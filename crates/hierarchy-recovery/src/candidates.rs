//! Candidate generation: classify evidence edges, apply structural and
//! policy filters, and rank the survivors.
//!
//! Filters (work order sections 13-15, 27):
//!
//! - missing parent node -> rejected (`dangling_candidate`)
//! - self parent -> rejected (`illegal_relation`)
//! - parent kind cannot contain the child kind -> rejected
//!   (`wrong_type_relation`)
//! - duplicate (child, parent, rule) relation -> rejected
//!   (`duplicate_relation`)
//! - heuristic edge while `allow_heuristic_parenting = false` -> rejected
//!   (`PolicyDisabledHeuristic`, visible in the trace, no diagnostic)
//! - deformer precedence (HR-006): when a node has an accepted deformer
//!   candidate, accepted part candidates are demoted to the logical part
//!   association (`DemotedToAssociation`)

use std::collections::BTreeMap;

use live2d_ir::{Confidence, Diagnostic, Severity};

use crate::graph::{
    confidence_rank, Candidate, EvidenceKind, NodeKind, ParentCandidates, RecoveryGraph,
    RejectedCandidate, RejectionReason,
};
use crate::policy::RecoveryPolicy;

/// Diagnostics cap per code before truncation (bounded diagnostics for
/// malicious inputs; work order section 55).
pub const DIAGNOSTIC_CAP_PER_CODE: usize = 256;

/// Small helper that caps how many diagnostics of one code are emitted.
///
/// Codes are keyed by value (not by a fixed bucket) so every code gets its
/// own summary line and no code is mislabeled.
pub(crate) struct DiagnosticBudget {
    counts: BTreeMap<String, usize>,
    truncated: BTreeMap<String, usize>,
}

impl DiagnosticBudget {
    pub(crate) fn new() -> Self {
        Self {
            counts: BTreeMap::new(),
            truncated: BTreeMap::new(),
        }
    }

    pub(crate) fn push(&mut self, out: &mut Vec<Diagnostic>, diagnostic: Diagnostic) {
        let count = self.counts.entry(diagnostic.code.clone()).or_insert(0);
        if *count < DIAGNOSTIC_CAP_PER_CODE {
            out.push(diagnostic);
        } else {
            *self.truncated.entry(diagnostic.code.clone()).or_insert(0) += 1;
        }
        *count += 1;
    }

    pub(crate) fn finish(&mut self, out: &mut Vec<Diagnostic>) {
        for (code, count) in &self.truncated {
            out.push(Diagnostic::new(
                Severity::Info,
                "diagnostics_truncated",
                format!("{count} additional '{code}' diagnostic(s) were suppressed by the cap"),
            ));
        }
    }
}

/// Build candidate sets for every node (root excluded) and append
/// diagnostics to the graph.
pub fn build_candidates(graph: &mut RecoveryGraph, policy: &RecoveryPolicy) {
    let mut budget = DiagnosticBudget::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    let mut kind_of: BTreeMap<&str, NodeKind> = BTreeMap::new();
    for node in &graph.nodes {
        kind_of.insert(node.id.as_str(), node.kind);
    }
    let mut edges_by_child: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        edges_by_child
            .entry(edge.child.as_str())
            .or_default()
            .push(index);
    }

    let mut candidates_out: Vec<ParentCandidates> = Vec::with_capacity(graph.nodes.len());
    let mut seen_relations: BTreeMap<(String, String, String), usize> = BTreeMap::new();

    for node in &graph.nodes {
        let mut accepted: Vec<Candidate> = Vec::new();
        let mut rejected: Vec<RejectedCandidate> = Vec::new();
        let mut accepted_edges: Vec<(usize, usize)> = Vec::new(); // (edge index, accepted slot)
        let mut has_deformer_candidate = false;

        let edge_indices = edges_by_child
            .get(node.id.as_str())
            .cloned()
            .unwrap_or_default();
        for index in edge_indices {
            let Some(edge) = graph.edges.get(index) else {
                continue;
            };
            let Some(parent_kind) = kind_of.get(edge.parent.as_str()).copied() else {
                rejected.push(RejectedCandidate {
                    parent: edge.parent.clone(),
                    confidence: edge.confidence,
                    rule: edge.rule.clone(),
                    reason: RejectionReason::MissingNode,
                });
                budget.push(
                    &mut diagnostics,
                    Diagnostic::new(
                        Severity::Warning,
                        "dangling_candidate",
                        format!(
                            "{} candidate for '{}': parent '{}' is not in the graph",
                            edge.rule, edge.child, edge.parent
                        ),
                    ),
                );
                continue;
            };
            if edge.child == edge.parent {
                rejected.push(RejectedCandidate {
                    parent: edge.parent.clone(),
                    confidence: edge.confidence,
                    rule: edge.rule.clone(),
                    reason: RejectionReason::IllegalSelfParent,
                });
                budget.push(
                    &mut diagnostics,
                    Diagnostic::new(
                        Severity::Warning,
                        "illegal_relation",
                        format!(
                            "node '{}' would be its own parent (rule {})",
                            edge.child, edge.rule
                        ),
                    ),
                );
                continue;
            }
            if !parent_kind.accepts_child(node.kind) {
                rejected.push(RejectedCandidate {
                    parent: edge.parent.clone(),
                    confidence: edge.confidence,
                    rule: edge.rule.clone(),
                    reason: RejectionReason::IllegalRelation,
                });
                budget.push(
                    &mut diagnostics,
                    Diagnostic::new(
                        Severity::Warning,
                        "wrong_type_relation",
                        format!(
                            "{} candidate for '{}': {} cannot contain {}",
                            edge.rule,
                            edge.child,
                            parent_kind.label(),
                            node.kind.label()
                        ),
                    ),
                );
                continue;
            }
            let relation_key = (
                edge.child.0.clone(),
                edge.parent.0.clone(),
                edge.rule.0.clone(),
            );
            if seen_relations.contains_key(&relation_key) {
                rejected.push(RejectedCandidate {
                    parent: edge.parent.clone(),
                    confidence: edge.confidence,
                    rule: edge.rule.clone(),
                    reason: RejectionReason::DuplicateRelation,
                });
                budget.push(
                    &mut diagnostics,
                    Diagnostic::new(
                        Severity::Warning,
                        "duplicate_relation",
                        format!(
                            "duplicate {} relation '{}' -> '{}'",
                            edge.rule, edge.child, edge.parent
                        ),
                    ),
                );
                continue;
            }
            seen_relations.insert(relation_key, index);

            if edge.kind == EvidenceKind::BindingRelationship
                && edge.confidence == Confidence::Heuristic
                && !policy.allow_heuristic_parenting
            {
                rejected.push(RejectedCandidate {
                    parent: edge.parent.clone(),
                    confidence: edge.confidence,
                    rule: edge.rule.clone(),
                    reason: RejectionReason::PolicyDisabledHeuristic,
                });
                continue;
            }

            // Deformer precedence: part candidates are demoted when an
            // accepted deformer candidate exists (HR-006).
            if matches!(
                parent_kind,
                NodeKind::WarpDeformer | NodeKind::RotationDeformer
            ) && edge.confidence == Confidence::Exact
            {
                has_deformer_candidate = true;
            }
            accepted_edges.push((index, accepted.len()));
            accepted.push(Candidate {
                parent: edge.parent.clone(),
                confidence: edge.confidence,
                rule: edge.rule.clone(),
                evidence_index: index,
            });
        }

        // Apply deformer precedence after the first pass.
        if has_deformer_candidate {
            let mut kept: Vec<Candidate> = Vec::with_capacity(accepted.len());
            for candidate in accepted.drain(..) {
                let is_part = kind_of
                    .get(candidate.parent.as_str())
                    .copied()
                    .map(|kind| kind == NodeKind::Part)
                    .unwrap_or(false);
                if is_part && candidate.confidence == Confidence::Exact {
                    rejected.push(RejectedCandidate {
                        parent: candidate.parent,
                        confidence: candidate.confidence,
                        rule: candidate.rule,
                        reason: RejectionReason::DemotedToAssociation,
                    });
                } else {
                    kept.push(candidate);
                }
            }
            accepted = kept;
        }

        // Deduplicate by parent keeping the strongest candidate.
        accepted.sort_by(|left, right| {
            confidence_rank(right.confidence)
                .cmp(&confidence_rank(left.confidence))
                .then_with(|| left.parent.cmp(&right.parent))
                .then_with(|| left.evidence_index.cmp(&right.evidence_index))
        });
        let mut deduped: Vec<Candidate> = Vec::with_capacity(accepted.len());
        for candidate in accepted {
            if deduped
                .iter()
                .any(|existing| existing.parent == candidate.parent)
            {
                rejected.push(RejectedCandidate {
                    parent: candidate.parent,
                    confidence: candidate.confidence,
                    rule: candidate.rule,
                    reason: RejectionReason::DuplicateRelation,
                });
            } else {
                deduped.push(candidate);
            }
        }

        candidates_out.push(ParentCandidates {
            child: node.id.clone(),
            candidates: deduped,
            rejected,
        });
    }

    budget.finish(&mut diagnostics);
    graph.candidates = candidates_out;
    graph.diagnostics.extend(diagnostics);
}
