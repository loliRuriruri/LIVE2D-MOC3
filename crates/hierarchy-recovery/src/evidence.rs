//! Evidence collection: stored relations (Exact) and policy-gated heuristics.
//!
//! Nothing here resolves parents; it only records *why* a parent relation is
//! plausible, with a rule id and a confidence (work order sections 6-8).

use std::collections::{BTreeMap, BTreeSet};

use live2d_ir::{BindingTarget, Confidence, Deformer, Diagnostic, Live2DModel, Severity};

use crate::graph::{EvidenceEdge, EvidenceKind, NodeId, NodeKind, RecoveryGraph};
use crate::policy::RecoveryPolicy;
use crate::rule::RuleId;

/// Build the recovery graph (nodes + evidence edges + raw diagnostics).
///
/// Candidate filtering and resolution happen in the next stages.
pub fn build_recovery_graph(model: &Live2DModel, policy: &RecoveryPolicy) -> RecoveryGraph {
    let nodes = RecoveryGraph::collect_nodes(model);
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut edges: Vec<EvidenceEdge> = Vec::new();

    let node_kind: BTreeMap<&str, NodeKind> = nodes
        .iter()
        .map(|node| (node.id.as_str(), node.kind))
        .collect();
    let push_edge = |edges: &mut Vec<EvidenceEdge>,
                     child: &str,
                     parent: &str,
                     kind: EvidenceKind,
                     confidence: Confidence,
                     rule: &str,
                     note: &str| {
        edges.push(EvidenceEdge {
            child: NodeId::new(child),
            parent: NodeId::new(parent),
            kind,
            confidence,
            rule: RuleId::new(rule),
            note: Some(note.to_string()),
        });
    };

    // HR-001: part -> parent part.
    for part in &model.parts {
        if let Some(parent) = &part.parent {
            push_edge(
                &mut edges,
                part.id.as_str(),
                parent.as_str(),
                EvidenceKind::ExplicitStoredRelation,
                Confidence::Exact,
                "HR-001",
                "stored part.parent",
            );
        }
    }

    // HR-002 / HR-003: deformer -> parent deformer / parent part.
    for deformer in &model.deformers {
        let (id, parent_deformer, parent_part) = match deformer {
            Deformer::Warp(warp) => (
                &warp.common.id,
                &warp.common.parent_deformer,
                &warp.common.parent_part,
            ),
            Deformer::Rotation(rotation) => (
                &rotation.common.id,
                &rotation.common.parent_deformer,
                &rotation.common.parent_part,
            ),
        };
        if let Some(parent) = parent_deformer {
            push_edge(
                &mut edges,
                id.as_str(),
                parent.as_str(),
                EvidenceKind::ExplicitStoredRelation,
                Confidence::Exact,
                "HR-002",
                "stored deformer.parent_deformer",
            );
        }
        if let Some(parent) = parent_part {
            push_edge(
                &mut edges,
                id.as_str(),
                parent.as_str(),
                EvidenceKind::ExplicitStoredRelation,
                Confidence::Exact,
                "HR-003",
                "stored deformer.parent_part",
            );
        }
    }

    // HR-005 / HR-004: art mesh -> parent deformer / parent part.
    for mesh in &model.art_meshes {
        if let Some(parent) = &mesh.parent_deformer {
            push_edge(
                &mut edges,
                mesh.id.as_str(),
                parent.as_str(),
                EvidenceKind::ExplicitStoredRelation,
                Confidence::Exact,
                "HR-005",
                "stored art_mesh.parent_deformer",
            );
        }
        if let Some(parent) = &mesh.parent_part {
            push_edge(
                &mut edges,
                mesh.id.as_str(),
                parent.as_str(),
                EvidenceKind::ExplicitStoredRelation,
                Confidence::Exact,
                "HR-004",
                "stored art_mesh.parent_part",
            );
        }
    }

    // HR-010 (heuristic, policy-gated at candidate time): orphan art meshes
    // whose keyform binding is used by parts. The edge is always collected so
    // the trace can explain why it was not used; the policy decides.
    let children_with_explicit_evidence: BTreeSet<String> = edges
        .iter()
        .filter(|edge| edge.kind == EvidenceKind::ExplicitStoredRelation)
        .map(|edge| edge.child.0.clone())
        .collect();
    // Index bindings once: the HR-010 lookup used to be a linear scan per
    // orphan mesh (O(meshes * bindings) on the default path).
    let binding_by_id: BTreeMap<&str, &live2d_ir::Binding> = model
        .bindings
        .iter()
        .map(|binding| (binding.id.as_str(), binding))
        .collect();
    let mut blocked_heuristic = 0usize;
    for mesh in &model.art_meshes {
        if children_with_explicit_evidence.contains(mesh.id.as_str()) {
            continue;
        }
        let Some(binding_id) = &mesh.binding else {
            continue;
        };
        let Some(binding) = binding_by_id.get(binding_id.as_str()).copied() else {
            continue;
        };
        let mut parts: Vec<&str> = binding
            .used_by
            .iter()
            .filter_map(|target| match target {
                BindingTarget::Part(id) => Some(id.as_str()),
                _ => None,
            })
            .collect();
        parts.sort_unstable();
        parts.dedup();
        if parts.is_empty() {
            continue;
        }
        if !policy.allow_heuristic_parenting {
            blocked_heuristic += 1;
        }
        for part in parts {
            push_edge(
                &mut edges,
                mesh.id.as_str(),
                part,
                EvidenceKind::BindingRelationship,
                Confidence::Heuristic,
                "HR-010",
                "shares a keyform binding with this part",
            );
        }
    }
    if blocked_heuristic > 0 {
        diagnostics.push(Diagnostic::new(
            Severity::Info,
            "heuristic_parenting_disabled",
            format!(
                "{blocked_heuristic} orphan node(s) could receive heuristic parents (HR-010); \
                 pass --allow-heuristic to enable. Unresolved nodes are preserved."
            ),
        ));
    }

    // Defensive structural check: every edge endpoint must exist. Stored IR
    // references are validated upstream, but hand-built graphs may not be.
    // Budgeted so hostile inputs cannot amplify diagnostics linearly.
    let mut budget = crate::candidates::DiagnosticBudget::new();
    let mut endpoint_diagnostics: Vec<Diagnostic> = Vec::new();
    for (index, edge) in edges.iter().enumerate() {
        for endpoint in [&edge.child, &edge.parent] {
            if !node_kind.contains_key(endpoint.as_str()) {
                budget.push(
                    &mut endpoint_diagnostics,
                    Diagnostic::new(
                        Severity::Warning,
                        "dangling_candidate",
                        format!(
                            "evidence edge {index} ({}) references unknown node '{endpoint}'",
                            edge.rule
                        ),
                    ),
                );
            }
        }
    }
    budget.finish(&mut endpoint_diagnostics);
    diagnostics.extend(endpoint_diagnostics);

    RecoveryGraph {
        nodes,
        edges,
        candidates: Vec::new(),
        diagnostics,
    }
}
