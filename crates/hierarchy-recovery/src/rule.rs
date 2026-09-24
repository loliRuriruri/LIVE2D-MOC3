//! Stable rule identifiers and their documentation.
//!
//! Every inference or structural decision in hierarchy reconstruction names a
//! rule here. `ResolutionTrace` records the rule id so that a wrong hierarchy
//! can be traced back to the exact rule (AGENT.3 work order sections 47/48).

use std::fmt;

use live2d_ir::Confidence;
use serde::{Deserialize, Serialize};

/// Stable rule identifier such as `HR-001`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuleId(pub String);

impl RuleId {
    /// Construct a rule id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the rule id text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Documentation record for one rule (also mirrored in
/// `docs/HIERARCHY_RECOVERY.md`; a test keeps both in sync).
#[derive(Debug, Clone, Copy)]
pub struct RuleInfo {
    /// Stable id (`HR-001`, ...).
    pub id: &'static str,
    /// What the rule does.
    pub description: &'static str,
    /// Which evidence it consumes.
    pub evidence: &'static str,
    /// Confidence the rule can produce.
    pub confidence: Confidence,
    /// What can go wrong when the rule fires.
    pub failure_mode: &'static str,
    /// Fixture that exercises it.
    pub fixture: &'static str,
}

/// All rules implemented in AGENT.3.
pub const RULES: &[RuleInfo] = &[
    RuleInfo {
        id: "HR-001",
        description: "Stored part parent index becomes the hierarchy parent of a part.",
        evidence: "moc3 part.parent_part (IR Part.parent)",
        confidence: Confidence::Exact,
        failure_mode: "None known; the value is stored in the file.",
        fixture: "hierarchy-006-multiple-parts",
    },
    RuleInfo {
        id: "HR-002",
        description: "Stored deformer parent index becomes the hierarchy parent of a deformer.",
        evidence: "moc3 deformer.parent_deformer (IR DeformerCommon.parent_deformer)",
        confidence: Confidence::Exact,
        failure_mode: "Stored cycles or self parents are possible; they are rejected or reported.",
        fixture: "hierarchy-004-nested-warp",
    },
    RuleInfo {
        id: "HR-003",
        description: "Stored part association becomes the hierarchy parent of a deformer that has no parent deformer.",
        evidence: "moc3 deformer.parent_part (IR DeformerCommon.parent_part)",
        confidence: Confidence::Exact,
        failure_mode: "None known; part association is stored.",
        fixture: "hierarchy-002-part-warp-artmesh",
    },
    RuleInfo {
        id: "HR-004",
        description: "Stored part association becomes the hierarchy parent of an art mesh that has no parent deformer.",
        evidence: "moc3 art_mesh.parent_part (IR ArtMesh.parent_part)",
        confidence: Confidence::Exact,
        failure_mode: "None known; part association is stored.",
        fixture: "hierarchy-001-part-artmesh",
    },
    RuleInfo {
        id: "HR-005",
        description: "Stored parent deformer becomes the hierarchy parent of an art mesh; the part association is kept separately.",
        evidence: "moc3 art_mesh.parent_deformer (IR ArtMesh.parent_deformer)",
        confidence: Confidence::Exact,
        failure_mode: "Deformer precedence can differ from authoring intent in non-standard files.",
        fixture: "hierarchy-002-part-warp-artmesh",
    },
    RuleInfo {
        id: "HR-006",
        description: "When a node has both a deformer parent candidate and a part parent candidate, the part candidate is demoted to a logical association (not a hierarchy parent).",
        evidence: "Structural rule over stored relations; part association is implied by the deformer chain.",
        confidence: Confidence::Exact,
        failure_mode: "If a file uses parent_part as the structural parent and parent_deformer only as a modifier, the association role is still preserved.",
        fixture: "hierarchy-002-part-warp-artmesh",
    },
    RuleInfo {
        id: "HR-007",
        description: "Checks that the stored part association agrees with the effective part of the selected deformer chain and reports mismatches.",
        evidence: "part association versus deformer chain part (both stored)",
        confidence: Confidence::Exact,
        failure_mode: "Mismatch indicates a corrupt or hand-edited file; both values are preserved and reported.",
        fixture: "hierarchy-010-part-mismatch",
    },
    RuleInfo {
        id: "HR-010",
        description: "Heuristic fallback: an orphan art mesh whose keyform binding is used by parts gets those parts as heuristic parent candidates.",
        evidence: "binding.used_by parts (IR Binding.used_by)",
        confidence: Confidence::Heuristic,
        failure_mode: "Shared bindings are common; the rule can propose a wrong parent. Disabled by default and never treated as Exact.",
        fixture: "hierarchy-008-ambiguous-parent",
    },
];

/// Look up rule documentation by id.
pub fn rule_info(id: &RuleId) -> Option<&'static RuleInfo> {
    RULES.iter().find(|info| info.id == id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_ids_are_unique() {
        let mut ids: Vec<&str> = RULES.iter().map(|info| info.id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate rule id");
    }

    #[test]
    fn only_heuristic_rules_are_heuristic() {
        for info in RULES {
            if info.confidence == Confidence::Heuristic {
                assert!(info.id.starts_with("HR-01"), "heuristic rule {info:?}");
            }
        }
    }
}
