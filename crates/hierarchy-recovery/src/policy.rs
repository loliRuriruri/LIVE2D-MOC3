//! Recovery policy: how aggressive hierarchy resolution may be.
//!
//! AGENT.3 defaults are deliberately conservative (work order section 27):
//! without enough evidence a node stays unresolved instead of being attached
//! to a guessed parent.

use serde::{Deserialize, Serialize};

/// What to do with nodes that cannot be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnresolvedStrategy {
    /// Keep unresolved nodes, grouped under a clearly marked synthetic
    /// container (only implemented mode).
    Preserve,
}

/// What to do when several parent candidates share the highest confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    /// Keep the node ambiguous; never pick "the first one" (only
    /// implemented mode).
    LeaveAmbiguous,
}

/// Hierarchy recovery policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPolicy {
    /// Enable policy-gated heuristic rules (currently HR-010). Default:
    /// `false`.
    pub allow_heuristic_parenting: bool,
    /// Unresolved node handling.
    pub unresolved_strategy: UnresolvedStrategy,
    /// Conflict handling.
    pub conflict_strategy: ConflictStrategy,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            allow_heuristic_parenting: false,
            unresolved_strategy: UnresolvedStrategy::Preserve,
            conflict_strategy: ConflictStrategy::LeaveAmbiguous,
        }
    }
}

impl RecoveryPolicy {
    /// Conservative policy with heuristics enabled (used by tests and the
    /// `--allow-heuristic` flag).
    pub fn with_heuristics() -> Self {
        Self {
            allow_heuristic_parenting: true,
            ..Self::default()
        }
    }
}
