//! Hierarchy reconstruction over the normalized Live2D IR (AGENT.3).
//!
//! Pipeline:
//!
//! ```text
//! Live2D IR
//!   -> collect nodes + evidence edges      (evidence.rs, graph.rs)
//!   -> candidates (filter + rank)          (candidates.rs)
//!   -> resolution (confidence, ambiguity)  (resolve.rs)
//!   -> cycle detection (iterative)         (cycles.rs)
//!   -> RecoveredProject                    (project.rs)
//!   -> hierarchy validation                (validate.rs)
//! ```
//!
//! Hard rules:
//!
//! - only `live2d-ir` is a dependency; binary parser details are unknown here,
//! - stored relations stay `Exact`; derived facts are never promoted,
//! - ambiguity is preserved, never resolved by "first one wins",
//! - orphans are preserved under a clearly marked synthetic container,
//! - the synthetic root and containers use the reserved `$` prefix,
//! - geometry is never copied: ids and evidence only,
//! - traversal and cycle detection are iterative.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod candidates;
mod cycles;
mod evidence;

pub mod graph;
pub mod policy;
pub mod project;
pub mod resolve;
pub mod rule;
pub mod text;
pub mod validate;

pub use graph::{
    confidence_rank, Candidate, EvidenceEdge, EvidenceKind, GraphNode, NodeId, NodeKind,
    ParentCandidates, RecoveryGraph, RejectedCandidate, RejectionReason, CYCLES_ID, ROOT_ID,
    SYNTHETIC_PREFIX, UNRESOLVED_ID,
};
pub use policy::{ConflictStrategy, RecoveryPolicy, UnresolvedStrategy};
pub use project::{
    build_recovered_project, compute_statistics, ContainerKind, CycleGroup, HierarchyStatistics,
    RecoveredNode, RecoveredProject, SyntheticContainer, SyntheticRoot, PROJECT_SCHEMA,
};
pub use resolve::{ResolutionState, ResolutionTrace, SelectedParent};
pub use rule::{rule_info, RuleId, RuleInfo, RULES};
pub use validate::validate_hierarchy;

use live2d_ir::Live2DModel;

/// Reconstruct the hierarchy for a model and validate the result.
///
/// The returned project contains graph, traces, statistics and all
/// diagnostics (guard diagnostics, resolution diagnostics and validation
/// findings).
pub fn reconstruct(model: &Live2DModel, policy: &RecoveryPolicy) -> RecoveredProject {
    let mut project = build_recovered_project(model, policy);
    let validation = validate_hierarchy(&project);
    project.diagnostics.extend(validation);
    project.statistics.diagnostics = project.diagnostics.len();
    project
}

/// Explain the resolution trace of one node (canonical id or source name).
pub fn explain(project: &RecoveredProject, query: &str) -> Option<String> {
    text::render_explain(project, query)
}

/// Run the candidate stage on an existing graph (advanced/testing API).
///
/// Replaces the candidate sets and appends guard diagnostics; intended to be
/// called once per graph (calling it repeatedly on the same graph would
/// duplicate diagnostics). Use this to test structural filters against
/// hand-built graphs.
pub fn analyze_graph(graph: &mut RecoveryGraph, policy: &RecoveryPolicy) {
    candidates::build_candidates(graph, policy);
}
