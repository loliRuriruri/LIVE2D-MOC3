//! Recovered project IR: the hierarchy reconstruction result.
//!
//! The IR model is never mutated; this is a separate layer that references IR
//! entities by id and never copies geometry (work order sections 10, 11, 42).

use std::collections::BTreeMap;

use live2d_ir::{
    BindingId, Confidence, Diagnostic, Live2DModel, PartId, SchemaStatus, SourceFormat,
};
use serde::Serialize;

use crate::candidates::build_candidates;
use crate::evidence::build_recovery_graph;
use crate::graph::{
    NodeId, NodeKind, RecoveryGraph, CYCLES_ID, ROOT_ID, SYNTHETIC_PREFIX, UNRESOLVED_ID,
};
use crate::policy::RecoveryPolicy;
use crate::resolve::{resolve_candidates, ResolutionState, ResolutionTrace};
use crate::rule::RuleId;

/// Schema identifier of the recovered project document.
pub const PROJECT_SCHEMA: &str = "live2d-recovery/recovered-project/1";

/// The clearly marked synthetic root (not a model object).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyntheticRoot {
    /// Always [`ROOT_ID`].
    pub id: String,
    /// Always `true`.
    pub synthetic: bool,
    /// Top-level resolved nodes followed by the synthetic containers.
    pub children: Vec<NodeId>,
}

/// Kind of a synthetic container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerKind {
    /// Unresolved and ambiguous nodes.
    Unresolved,
    /// Cycle groups.
    Cycles,
}

/// A synthetic grouping container (never a model object).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyntheticContainer {
    /// Container id (`$unresolved`, `$cycles`).
    pub id: String,
    /// Container kind.
    pub kind: ContainerKind,
    /// Always `true`.
    pub synthetic: bool,
    /// Member nodes in canonical node order.
    pub members: Vec<NodeId>,
}

/// One recovered node.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecoveredNode {
    /// IR entity id.
    pub id: NodeId,
    /// Node kind.
    pub kind: NodeKind,
    /// Source index inside the IR array of that kind.
    pub source_index: usize,
    /// Stored identifier text when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    /// Always `false` for model nodes (explicit so consumers can filter).
    pub synthetic: bool,
    /// Resolution state.
    pub status: ResolutionState,
    /// Selected hierarchy parent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NodeId>,
    /// Confidence of the selected parent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    /// Rule that selected the parent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<RuleId>,
    /// Stored logical part association (kept separate from the hierarchy
    /// parent, work order section 20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_association: Option<PartId>,
    /// Keyform binding identity (for AGENT.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingId>,
    /// Children under the selected hierarchy (canonical order).
    pub children: Vec<NodeId>,
    /// Full resolution trace.
    pub trace: ResolutionTrace,
}

/// One detected cycle group.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CycleGroup {
    /// Stable index (0-based, ordered by first member).
    pub index: usize,
    /// Members in traversal order (each has the next as parent, last wraps).
    pub nodes: Vec<NodeId>,
}

/// Hierarchy Resolution Statistics (never called recovery accuracy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HierarchyStatistics {
    /// Total model nodes (root excluded).
    pub nodes: usize,
    /// Nodes with a selected parent.
    pub resolved: usize,
    /// Nodes without a parent where none is expected.
    pub top_level: usize,
    /// Nodes without a determinable parent.
    pub unresolved: usize,
    /// Nodes with several equally-strong candidates.
    pub ambiguous: usize,
    /// Nodes inside a stored cycle.
    pub cycle_detected: usize,
    /// Resolved nodes whose parent evidence is Exact.
    pub exact: usize,
    /// Resolved nodes whose parent evidence is Derived.
    pub derived: usize,
    /// Resolved nodes whose parent evidence is Heuristic.
    pub heuristic: usize,
    /// Number of detected cycles.
    pub cycles: usize,
    /// Alias of `unresolved` (kept explicit for reports).
    pub orphans: usize,
    /// Total diagnostics in the project document.
    pub diagnostics: usize,
}

/// The recovered project.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecoveredProject {
    /// Schema identifier.
    pub schema: String,
    /// Schema lifecycle status (inherited: experimental).
    pub schema_status: SchemaStatus,
    /// Source format description (copied from the IR; small, no geometry).
    pub source: SourceFormat,
    /// Schema of the source document.
    pub source_schema: String,
    /// Policy used for this reconstruction.
    pub policy: RecoveryPolicy,
    /// Synthetic root.
    pub root: SyntheticRoot,
    /// Nodes in canonical order.
    pub nodes: Vec<RecoveredNode>,
    /// Synthetic containers.
    pub containers: Vec<SyntheticContainer>,
    /// Detected cycles.
    pub cycles: Vec<CycleGroup>,
    /// Statistics.
    pub statistics: HierarchyStatistics,
    /// Recovery graph (ids and evidence only).
    pub graph: RecoveryGraph,
    /// Non-fatal diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Build the recovered project (without the final validation pass).
pub fn build_recovered_project(model: &Live2DModel, policy: &RecoveryPolicy) -> RecoveredProject {
    let mut graph = build_recovery_graph(model, policy);
    build_candidates(&mut graph, policy);
    let (traces, cycles, resolve_diagnostics) = resolve_candidates(&graph);

    let mut diagnostics: Vec<Diagnostic> = graph.diagnostics.clone();
    diagnostics.extend(resolve_diagnostics);

    // Children lists and statuses.
    let mut children_by_parent: BTreeMap<&str, Vec<NodeId>> = BTreeMap::new();
    for trace in &traces {
        if let Some(selected) = &trace.selected {
            children_by_parent
                .entry(selected.parent.as_str())
                .or_default()
                .push(trace.child.clone());
        }
    }

    let mut nodes: Vec<RecoveredNode> = Vec::with_capacity(graph.nodes.len());
    for (graph_node, trace) in graph.nodes.iter().zip(traces.iter()) {
        let selected = trace.selected.as_ref();
        nodes.push(RecoveredNode {
            id: graph_node.id.clone(),
            kind: graph_node.kind,
            source_index: graph_node.source_index,
            source_name: graph_node.source_name.clone(),
            synthetic: false,
            status: trace.state,
            parent: selected.map(|selected| selected.parent.clone()),
            confidence: selected.map(|selected| selected.confidence),
            rule: selected.map(|selected| selected.rule.clone()),
            part_association: graph_node.part_association.clone(),
            binding: graph_node.binding.clone(),
            children: children_by_parent
                .get(graph_node.id.as_str())
                .cloned()
                .unwrap_or_default(),
            trace: trace.clone(),
        });
    }

    // Synthetic containers.
    let unresolved_members: Vec<NodeId> = nodes
        .iter()
        .filter(|node| {
            matches!(
                node.status,
                ResolutionState::Unresolved | ResolutionState::Ambiguous
            )
        })
        .map(|node| node.id.clone())
        .collect();
    let cycle_members: Vec<NodeId> = cycles
        .iter()
        .flat_map(|cycle| cycle.iter().cloned())
        .collect();

    let mut containers: Vec<SyntheticContainer> = Vec::new();
    if !unresolved_members.is_empty() {
        containers.push(SyntheticContainer {
            id: UNRESOLVED_ID.to_string(),
            kind: ContainerKind::Unresolved,
            synthetic: true,
            members: unresolved_members,
        });
    }
    if !cycle_members.is_empty() {
        containers.push(SyntheticContainer {
            id: CYCLES_ID.to_string(),
            kind: ContainerKind::Cycles,
            synthetic: true,
            members: cycle_members,
        });
    }

    let top_level: Vec<NodeId> = nodes
        .iter()
        .filter(|node| node.status == ResolutionState::TopLevel)
        .map(|node| node.id.clone())
        .collect();
    let mut root_children = top_level;
    root_children.extend(
        containers
            .iter()
            .map(|container| NodeId::new(&container.id)),
    );

    let cycle_groups: Vec<CycleGroup> = cycles
        .iter()
        .enumerate()
        .map(|(index, nodes_in_cycle)| CycleGroup {
            index,
            nodes: nodes_in_cycle.clone(),
        })
        .collect();

    let statistics = compute_statistics(&nodes, &cycle_groups, diagnostics.len());

    RecoveredProject {
        schema: PROJECT_SCHEMA.to_string(),
        schema_status: SchemaStatus::Experimental,
        source: model.source.clone(),
        source_schema: model.schema.clone(),
        policy: *policy,
        root: SyntheticRoot {
            id: ROOT_ID.to_string(),
            synthetic: true,
            children: root_children,
        },
        nodes,
        containers,
        cycles: cycle_groups,
        statistics,
        graph,
        diagnostics,
    }
}

/// Recompute statistics from node states (also used by the validator).
pub fn compute_statistics(
    nodes: &[RecoveredNode],
    cycles: &[CycleGroup],
    diagnostic_count: usize,
) -> HierarchyStatistics {
    let mut statistics = HierarchyStatistics {
        nodes: nodes.len(),
        resolved: 0,
        top_level: 0,
        unresolved: 0,
        ambiguous: 0,
        cycle_detected: 0,
        exact: 0,
        derived: 0,
        heuristic: 0,
        cycles: cycles.len(),
        orphans: 0,
        diagnostics: diagnostic_count,
    };
    for node in nodes {
        match node.status {
            ResolutionState::Resolved => {
                statistics.resolved += 1;
                match node.confidence {
                    Some(Confidence::Exact) => statistics.exact += 1,
                    Some(Confidence::Derived) => statistics.derived += 1,
                    Some(Confidence::Heuristic) => statistics.heuristic += 1,
                    Some(Confidence::Unknown) | None => {}
                }
            }
            ResolutionState::TopLevel => statistics.top_level += 1,
            ResolutionState::Unresolved => {
                statistics.unresolved += 1;
                statistics.orphans += 1;
            }
            ResolutionState::Ambiguous => statistics.ambiguous += 1,
            ResolutionState::CycleDetected => {
                statistics.cycle_detected += 1;
                match node.confidence {
                    Some(Confidence::Exact) => statistics.exact += 1,
                    Some(Confidence::Derived) => statistics.derived += 1,
                    Some(Confidence::Heuristic) => statistics.heuristic += 1,
                    Some(Confidence::Unknown) | None => {}
                }
            }
        }
    }
    statistics
}

/// True when an id is reserved for synthetic structures.
pub fn is_synthetic_id(id: &str) -> bool {
    id.starts_with(SYNTHETIC_PREFIX)
}
