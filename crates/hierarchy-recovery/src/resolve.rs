//! Resolution: pick a parent only when the evidence justifies it.
//!
//! Rules (work order sections 8, 9, 14-16, 27):
//!
//! - exactly one candidate in the highest confidence tier -> resolved
//! - several candidates share the highest tier -> ambiguous (never "first
//!   one wins"; no automatic tie-break is implemented because none is
//!   justified by the format)
//! - no candidates: art meshes are unresolved orphans, parts/deformers are
//!   exact top-level nodes (the file stores no parent for them)
//! - cycles keep their stored edges and are reported; nothing is deleted

use std::collections::{BTreeMap, BTreeSet};

use live2d_ir::{Diagnostic, Severity};
use serde::{Deserialize, Serialize};

use crate::candidates::DiagnosticBudget;
use crate::cycles::detect_cycles;
use crate::graph::{
    confidence_rank, Candidate, NodeId, NodeKind, RecoveryGraph, RejectedCandidate, RejectionReason,
};
use crate::rule::RuleId;

/// Outcome of hierarchy resolution for one node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionState {
    /// A parent was selected from candidates.
    Resolved,
    /// No parent exists and none is expected for this kind (parts and
    /// deformers without stored parents).
    TopLevel,
    /// No parent could be determined (orphans, policy-blocked heuristics).
    Unresolved,
    /// Several candidates share the highest confidence.
    Ambiguous,
    /// A parent is selected but the node participates in a stored cycle.
    CycleDetected,
}

/// The parent selected for a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectedParent {
    /// Selected parent.
    pub parent: NodeId,
    /// Confidence of the selection.
    pub confidence: live2d_ir::Confidence,
    /// Rule that produced the selected evidence.
    pub rule: RuleId,
    /// Index into [`RecoveryGraph::edges`].
    pub evidence_index: usize,
}

/// Full resolution trace for one node (work order section 9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolutionTrace {
    /// Node this trace is about.
    pub child: NodeId,
    /// Node kind.
    pub kind: NodeKind,
    /// Resolution state.
    pub state: ResolutionState,
    /// Selected parent, when resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<SelectedParent>,
    /// Candidates that shared the highest confidence but could not be
    /// chosen (ambiguity).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tied_candidates: Vec<NodeId>,
    /// Every candidate that was not selected, with the reason.
    pub rejected: Vec<RejectedCandidate>,
    /// Optional explanation (for example: heuristics disabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Effective part of a deformer chain: first stored part association walking
/// up parent deformers (iterative, cycle-safe, memoized).
///
/// The memo keeps chains of arbitrary depth linear: every visited node stores
/// its answer, so a 20k-deep chain is O(n) instead of O(n * depth).
fn effective_part<'a>(
    start: &'a str,
    part_of: &BTreeMap<&'a str, Option<&'a str>>,
    parents: &BTreeMap<&'a str, &'a str>,
    cache: &mut BTreeMap<&'a str, Option<&'a str>>,
) -> Option<&'a str> {
    if let Some(cached) = cache.get(start) {
        return *cached;
    }
    let mut path: Vec<&'a str> = Vec::new();
    let mut seen: BTreeSet<&'a str> = BTreeSet::new();
    let mut current = Some(start);
    let mut result: Option<&'a str> = None;
    while let Some(node) = current {
        if let Some(cached) = cache.get(node) {
            result = *cached;
            break;
        }
        if !seen.insert(node) {
            result = None; // cycle: no trustworthy effective part
            break;
        }
        if let Some(Some(part)) = part_of.get(node) {
            result = Some(part);
            break;
        }
        path.push(node);
        current = parents.get(node).copied();
    }
    for node in path {
        cache.insert(node, result);
    }
    result
}

/// Resolve candidates for every node and detect cycles.
///
/// Returns traces in canonical node order, the detected cycle groups, and
/// diagnostics.
pub fn resolve_candidates(
    graph: &RecoveryGraph,
) -> (Vec<ResolutionTrace>, Vec<Vec<NodeId>>, Vec<Diagnostic>) {
    let mut budget = DiagnosticBudget::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut traces: Vec<ResolutionTrace> = Vec::with_capacity(graph.nodes.len());

    // Candidates are built aligned with the node order; index access keeps
    // resolution O(n log n) instead of scanning per node.
    for (node_index, node) in graph.nodes.iter().enumerate() {
        let entry = graph.candidates.get(node_index);
        let (candidates, mut rejected) = match entry {
            Some(entry) => (entry.candidates.clone(), entry.rejected.clone()),
            None => (Vec::new(), Vec::new()),
        };

        let blocked_heuristics = rejected
            .iter()
            .filter(|candidate| candidate.reason == RejectionReason::PolicyDisabledHeuristic)
            .count();
        let mut note = None;
        if blocked_heuristics > 0 {
            note = Some(format!(
                "{blocked_heuristics} heuristic candidate(s) blocked by policy (HR-010); \
                 enable --allow-heuristic to consider them"
            ));
        }

        if candidates.is_empty() {
            // Illegal/duplicate/dangling evidence means something claimed a
            // parent but it could not be used; such a node is unresolved, not
            // a legitimate top-level object.
            let had_invalid_evidence = rejected.iter().any(|candidate| {
                matches!(
                    candidate.reason,
                    RejectionReason::IllegalSelfParent
                        | RejectionReason::IllegalRelation
                        | RejectionReason::MissingNode
                        | RejectionReason::DuplicateRelation
                )
            });
            let state = if node.kind == NodeKind::ArtMesh || had_invalid_evidence {
                budget.push(
                    &mut diagnostics,
                    Diagnostic::new(
                        Severity::Warning,
                        "orphan_node",
                        format!(
                            "node '{}' ({}) was left unresolved ({} rejected candidate(s))",
                            node.id,
                            node.kind.label(),
                            rejected.len()
                        ),
                    ),
                );
                ResolutionState::Unresolved
            } else {
                ResolutionState::TopLevel
            };
            traces.push(ResolutionTrace {
                child: node.id.clone(),
                kind: node.kind,
                state,
                selected: None,
                tied_candidates: Vec::new(),
                rejected,
                note,
            });
            continue;
        }

        let top_rank = candidates
            .first()
            .map(|candidate| confidence_rank(candidate.confidence))
            .unwrap_or(0);
        let tier: Vec<&Candidate> = candidates
            .iter()
            .filter(|candidate| confidence_rank(candidate.confidence) == top_rank)
            .collect();

        if tier.len() == 1 {
            let selected = tier.first().copied();
            let selected_parent = selected.map(|candidate| SelectedParent {
                parent: candidate.parent.clone(),
                confidence: candidate.confidence,
                rule: candidate.rule.clone(),
                evidence_index: candidate.evidence_index,
            });
            for candidate in &candidates {
                let is_selected = selected
                    .map(|chosen| chosen.parent == candidate.parent)
                    .unwrap_or(false);
                if !is_selected {
                    rejected.push(RejectedCandidate {
                        parent: candidate.parent.clone(),
                        confidence: candidate.confidence,
                        rule: candidate.rule.clone(),
                        reason: RejectionReason::LowerConfidence,
                    });
                }
            }
            traces.push(ResolutionTrace {
                child: node.id.clone(),
                kind: node.kind,
                state: ResolutionState::Resolved,
                selected: selected_parent,
                tied_candidates: Vec::new(),
                rejected,
                note,
            });
        } else {
            let tied: Vec<NodeId> = tier
                .iter()
                .map(|candidate| candidate.parent.clone())
                .collect();
            for candidate in &candidates {
                if !tier
                    .iter()
                    .any(|tied_candidate| tied_candidate.parent == candidate.parent)
                {
                    rejected.push(RejectedCandidate {
                        parent: candidate.parent.clone(),
                        confidence: candidate.confidence,
                        rule: candidate.rule.clone(),
                        reason: RejectionReason::LowerConfidence,
                    });
                }
            }
            let mut listed: Vec<String> = tied.iter().map(|parent| parent.0.clone()).collect();
            listed.truncate(8);
            let suffix = if tied.len() > listed.len() {
                ", ..."
            } else {
                ""
            };
            budget.push(
                &mut diagnostics,
                Diagnostic::new(
                    Severity::Warning,
                    "ambiguous_parent",
                    format!(
                        "node '{}' has {} equally-strong parent candidates: {}{}",
                        node.id,
                        tied.len(),
                        listed.join(", "),
                        suffix
                    ),
                ),
            );
            traces.push(ResolutionTrace {
                child: node.id.clone(),
                kind: node.kind,
                state: ResolutionState::Ambiguous,
                selected: None,
                tied_candidates: tied,
                rejected,
                note: note.or_else(|| {
                    Some("no deterministic tie-break rule is justified; left ambiguous".to_string())
                }),
            });
        }
    }

    // Cycle detection over selected parents (iterative; no recursion).
    let mut parents: BTreeMap<&str, &str> = BTreeMap::new();
    for trace in &traces {
        if let Some(selected) = &trace.selected {
            parents.insert(trace.child.as_str(), selected.parent.as_str());
        }
    }

    // HR-007: stored part association must agree with the effective part of
    // the selected deformer chain; both values are preserved either way.
    let part_of: BTreeMap<&str, Option<&str>> = graph
        .nodes
        .iter()
        .map(|node| {
            (
                node.id.as_str(),
                node.part_association.as_ref().map(|part| part.as_str()),
            )
        })
        .collect();
    let kind_of: BTreeMap<&str, NodeKind> = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.kind))
        .collect();
    let mut effective_cache: BTreeMap<&str, Option<&str>> = BTreeMap::new();
    for trace in &traces {
        let Some(selected) = &trace.selected else {
            continue;
        };
        let parent_kind = kind_of
            .get(selected.parent.as_str())
            .copied()
            .unwrap_or(NodeKind::Root);
        if !matches!(
            parent_kind,
            NodeKind::WarpDeformer | NodeKind::RotationDeformer
        ) {
            continue;
        }
        let effective = effective_part(
            selected.parent.as_str(),
            &part_of,
            &parents,
            &mut effective_cache,
        );
        let stored = part_of.get(trace.child.as_str()).copied().flatten();
        if let (Some(effective), Some(stored)) = (effective, stored) {
            if effective != stored {
                budget.push(
                    &mut diagnostics,
                    Diagnostic::new(
                        Severity::Warning,
                        "exact_relation_conflict",
                        format!(
                            "node '{}' part association '{stored}' disagrees with effective part \
                             '{effective}' of deformer chain '{}' (rule HR-007); both values are preserved",
                            trace.child, selected.parent
                        ),
                    ),
                );
            }
        }
    }
    let cycles = detect_cycles(&parents);
    for cycle in &cycles {
        let mut member_ids: Vec<String> = cycle.iter().map(|node| node.0.clone()).collect();
        let total = member_ids.len();
        member_ids.truncate(12);
        let suffix = if total > member_ids.len() {
            ", ..."
        } else {
            ""
        };
        budget.push(
            &mut diagnostics,
            Diagnostic::new(
                Severity::Warning,
                "hierarchy_cycle",
                format!(
                    "stored parent chain contains a cycle of {total} node(s): {}{}",
                    member_ids.join(" -> "),
                    suffix
                ),
            ),
        );
    }
    let cycle_members: BTreeSet<&str> = cycles
        .iter()
        .flat_map(|cycle| cycle.iter().map(|node| node.as_str()))
        .collect();
    for trace in &mut traces {
        if cycle_members.contains(trace.child.as_str()) {
            trace.state = ResolutionState::CycleDetected;
        }
    }

    budget.finish(&mut diagnostics);
    (traces, cycles, diagnostics)
}
