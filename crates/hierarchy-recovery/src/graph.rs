//! Recovery graph: nodes, evidence edges and parent candidates.
//!
//! The graph never copies geometry; it stores identifiers, kinds, evidence
//! and diagnostics only (AGENT.3 work order section 42). Hierarchy resolution
//! runs on this graph, not by mutating the IR.

use std::fmt;

use live2d_ir::{
    ArtMeshId, BindingId, Confidence, Deformer, DeformerId, Diagnostic, Live2DModel, PartId,
};
use serde::{Deserialize, Serialize};

use crate::rule::RuleId;

/// Kind of a hierarchy node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// Synthetic recovered root (never corresponds to a model object).
    Root,
    /// A part.
    Part,
    /// A warp deformer.
    WarpDeformer,
    /// A rotation deformer.
    RotationDeformer,
    /// An art mesh.
    ArtMesh,
}

impl NodeKind {
    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            NodeKind::Root => "Root",
            NodeKind::Part => "Part",
            NodeKind::WarpDeformer => "WarpDeformer",
            NodeKind::RotationDeformer => "RotationDeformer",
            NodeKind::ArtMesh => "ArtMesh",
        }
    }

    /// Canonical ordering rank used for deterministic node order.
    pub fn rank(self) -> u8 {
        match self {
            NodeKind::Root => 0,
            NodeKind::Part => 1,
            NodeKind::WarpDeformer | NodeKind::RotationDeformer => 2,
            NodeKind::ArtMesh => 3,
        }
    }

    /// Whether a node of this kind may be the hierarchy parent of `child`.
    ///
    /// This is the typed parent matrix from the work order section 12:
    /// parts contain parts/deformers/art meshes, deformers contain
    /// deformers/art meshes, art meshes contain nothing. The synthetic root
    /// never becomes a stored parent (orphan fallback under the root is
    /// intentionally not implemented; unresolved nodes are grouped under a
    /// clearly marked synthetic container instead).
    pub fn accepts_child(self, child: NodeKind) -> bool {
        match self {
            NodeKind::Root => false,
            NodeKind::Part => matches!(
                child,
                NodeKind::Part
                    | NodeKind::WarpDeformer
                    | NodeKind::RotationDeformer
                    | NodeKind::ArtMesh
            ),
            NodeKind::WarpDeformer | NodeKind::RotationDeformer => matches!(
                child,
                NodeKind::WarpDeformer | NodeKind::RotationDeformer | NodeKind::ArtMesh
            ),
            NodeKind::ArtMesh => false,
        }
    }
}

/// Stable hierarchy node identifier (canonical IR entity id, or `root`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub String);

impl NodeId {
    /// Construct a node id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the id text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Synthetic root id. Synthetic ids always start with `$`; the hierarchy
/// validator rejects colliding model ids.
pub const ROOT_ID: &str = "$root";

/// Prefix shared by every synthetic (non-model) node or container.
pub const SYNTHETIC_PREFIX: &str = "$";

/// Synthetic container for unresolved and ambiguous nodes.
pub const UNRESOLVED_ID: &str = "$unresolved";

/// Synthetic container for cycle groups.
pub const CYCLES_ID: &str = "$cycles";

/// Deterministic confidence ordering (higher wins).
pub fn confidence_rank(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::Exact => 3,
        Confidence::Derived => 2,
        Confidence::Heuristic => 1,
        Confidence::Unknown => 0,
    }
}

/// One recovery graph node (metadata only, no geometry).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Canonical identifier (IR entity id).
    pub id: NodeId,
    /// Node kind.
    pub kind: NodeKind,
    /// Source index inside the IR array of that kind.
    pub source_index: usize,
    /// Stored identifier text when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    /// Stored part association (kept separate from the hierarchy parent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_association: Option<PartId>,
    /// Keyform binding used by the object (AGENT.4 will need this identity).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingId>,
}

/// Evidence kinds collected by the graph builder.
///
/// AGENT.3 emits `ExplicitStoredRelation` (stored parent fields) and
/// `HeuristicFallback` (HR-010). The remaining variants are reserved for
/// later evidence work and are documented in `docs/HIERARCHY_RECOVERY.md`;
/// they are never claimed without an implementing rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Directly stored parent relation.
    ExplicitStoredRelation,
    /// Reverse link confirming a stored relation.
    ReverseLink,
    /// Observed object range grouping (reserved).
    ObjectRange,
    /// Observed deformer range grouping (reserved).
    DeformerRange,
    /// Binding relationship evidence (heuristic only).
    BindingRelationship,
    /// Draw order relationship evidence (reserved).
    DrawOrderRelationship,
    /// Structural constraint (for example deformer precedence).
    StructuralConstraint,
    /// Consistency derived from two stored values.
    DerivedConsistency,
    /// Policy-gated heuristic fallback.
    HeuristicFallback,
}

/// One evidence edge for a potential parent relation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceEdge {
    /// Child node.
    pub child: NodeId,
    /// Proposed parent node.
    pub parent: NodeId,
    /// Evidence kind.
    pub kind: EvidenceKind,
    /// Confidence this single edge can support.
    pub confidence: Confidence,
    /// Rule that produced the edge.
    pub rule: RuleId,
    /// Optional human-readable note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One surviving parent candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    /// Candidate parent.
    pub parent: NodeId,
    /// Candidate confidence.
    pub confidence: Confidence,
    /// Rule that produced the candidate.
    pub rule: RuleId,
    /// Index into [`RecoveryGraph::edges`].
    pub evidence_index: usize,
}

/// Why a candidate was rejected before resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    /// A higher-confidence candidate exists.
    LowerConfidence,
    /// The node would be its own parent.
    IllegalSelfParent,
    /// The parent kind cannot contain the child kind.
    IllegalRelation,
    /// The parent node does not exist in the graph.
    MissingNode,
    /// A policy-gated heuristic candidate while the policy is disabled.
    PolicyDisabledHeuristic,
    /// Candidate demoted to a logical association (deformer precedence).
    DemotedToAssociation,
    /// The same (child, parent, rule) relation appeared more than once.
    DuplicateRelation,
}

impl RejectionReason {
    /// Stable label used in traces and explain output.
    pub fn label(self) -> &'static str {
        match self {
            RejectionReason::LowerConfidence => "lower confidence",
            RejectionReason::IllegalSelfParent => "illegal self parent",
            RejectionReason::IllegalRelation => "illegal relation",
            RejectionReason::MissingNode => "missing parent node",
            RejectionReason::PolicyDisabledHeuristic => "heuristic parenting disabled by policy",
            RejectionReason::DemotedToAssociation => "demoted to part association",
            RejectionReason::DuplicateRelation => "duplicate relation",
        }
    }
}

/// A rejected candidate with its reason.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RejectedCandidate {
    /// Rejected parent.
    pub parent: NodeId,
    /// Confidence the candidate had.
    pub confidence: Confidence,
    /// Rule that produced it.
    pub rule: RuleId,
    /// Why it was rejected.
    pub reason: RejectionReason,
}

/// Potential parents of one child.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParentCandidates {
    /// Child node.
    pub child: NodeId,
    /// Candidates considered for resolution, sorted by confidence then id.
    pub candidates: Vec<Candidate>,
    /// Candidates rejected before resolution (kept for traceability).
    pub rejected: Vec<RejectedCandidate>,
}

/// The recovery graph, in canonical node order (parts, deformers, art
/// meshes; source order inside each kind).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryGraph {
    /// Nodes in canonical order.
    pub nodes: Vec<GraphNode>,
    /// Evidence edges in collection order.
    pub edges: Vec<EvidenceEdge>,
    /// Candidate sets, aligned with `nodes` order (root excluded).
    pub candidates: Vec<ParentCandidates>,
    /// Non-fatal graph diagnostics (also merged into the project report).
    #[serde(skip)]
    pub diagnostics: Vec<Diagnostic>,
}

impl RecoveryGraph {
    /// Build the node list from the IR (metadata only).
    pub fn collect_nodes(model: &Live2DModel) -> Vec<GraphNode> {
        let mut nodes: Vec<GraphNode> =
            Vec::with_capacity(model.parts.len() + model.deformers.len() + model.art_meshes.len());
        for (index, part) in model.parts.iter().enumerate() {
            nodes.push(GraphNode {
                id: NodeId::new(part.id.as_str()),
                kind: NodeKind::Part,
                source_index: index,
                source_name: part.source_name.clone(),
                part_association: part.parent.clone(),
                binding: part.binding.clone(),
            });
        }
        for (index, deformer) in model.deformers.iter().enumerate() {
            let (id, source_name, part_association, binding, kind) = match deformer {
                Deformer::Warp(warp) => (
                    &warp.common.id,
                    warp.common.source_name.clone(),
                    warp.common.parent_part.clone(),
                    warp.common.binding.clone(),
                    NodeKind::WarpDeformer,
                ),
                Deformer::Rotation(rotation) => (
                    &rotation.common.id,
                    rotation.common.source_name.clone(),
                    rotation.common.parent_part.clone(),
                    rotation.common.binding.clone(),
                    NodeKind::RotationDeformer,
                ),
            };
            nodes.push(GraphNode {
                id: NodeId::new(id.as_str()),
                kind,
                source_index: index,
                source_name,
                part_association,
                binding,
            });
        }
        for (index, mesh) in model.art_meshes.iter().enumerate() {
            nodes.push(GraphNode {
                id: NodeId::new(mesh.id.as_str()),
                kind: NodeKind::ArtMesh,
                source_index: index,
                source_name: mesh.source_name.clone(),
                part_association: mesh.parent_part.clone(),
                binding: mesh.binding.clone(),
            });
        }
        nodes
    }

    /// Find a node by id.
    pub fn node(&self, id: &NodeId) -> Option<&GraphNode> {
        self.nodes.iter().find(|node| &node.id == id)
    }

    /// Candidate set for a child.
    pub fn candidates_for(&self, child: &NodeId) -> Option<&ParentCandidates> {
        self.candidates.iter().find(|entry| &entry.child == child)
    }

    /// Deterministic entity reference helper for diagnostics.
    pub fn entity_ref(node: &GraphNode) -> live2d_ir::EntityRef {
        match node.kind {
            NodeKind::Part => live2d_ir::EntityRef::Part(PartId::new(node.id.as_str())),
            NodeKind::WarpDeformer | NodeKind::RotationDeformer => {
                live2d_ir::EntityRef::Deformer(DeformerId::new(node.id.as_str()))
            }
            NodeKind::ArtMesh => live2d_ir::EntityRef::ArtMesh(ArtMeshId::new(node.id.as_str())),
            NodeKind::Root => live2d_ir::EntityRef::Model(Default::default()),
        }
    }
}
