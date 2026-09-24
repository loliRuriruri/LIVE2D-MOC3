//! Stable rule identifiers for keyform recovery.
//!
//! Every structural decision in this crate names a rule here. Binding rules
//! use the `KB-` prefix, keyform/grid rules use `KF-`. Traces and grid
//! records store the applied rule ids so a wrong reconstruction can be traced
//! back to the exact rule (AGENT.4 work order sections 29/30).

use std::fmt;

use live2d_ir::Confidence;
use serde::{Deserialize, Serialize};

/// Stable rule identifier such as `KB-002`.
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

/// Documentation record for one rule (mirrored in `docs/KEYFORM_RECOVERY.md`;
/// a test keeps both in sync).
#[derive(Debug, Clone, Copy)]
pub struct RuleInfo {
    /// Stable id (`KB-001`, `KF-001`, ...).
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

/// All rules implemented in AGENT.4.
pub const RULES: &[RuleInfo] = &[
    RuleInfo {
        id: "KB-001",
        description: "A stored binding reference on a target object is collected as binding evidence (the binding itself is never inferred).",
        evidence: "moc3 object.binding_index (IR Part.binding / DeformerCommon.binding / ArtMesh.binding)",
        confidence: Confidence::Exact,
        failure_mode: "None known; the reference is stored. Targets without a binding are treated as static (KB-007).",
        fixture: "keyform-001-zero-dimensional",
    },
    RuleInfo {
        id: "KB-002",
        description: "Each parameter axis is the stored key table of the binding parameter, with keys kept in stored order.",
        evidence: "IR Binding.parameters[].key_values (stored key tables)",
        confidence: Confidence::Exact,
        failure_mode: "Duplicate or unsorted stored keys are preserved as-is and reported (KB-004).",
        fixture: "keyform-002-1d-three-keys",
    },
    RuleInfo {
        id: "KB-003",
        description: "A binding parameter that does not resolve to a parameter entity produces an unresolved axis (dangling_parameter); the axis is preserved with its stored keys.",
        evidence: "IR Binding.parameters[].parameter versus IR parameters",
        confidence: Confidence::Unknown,
        failure_mode: "Only possible in hand-edited or partially recovered IR; no axis semantics are claimed.",
        fixture: "ir-negative-dangling-parameter",
    },
    RuleInfo {
        id: "KB-004",
        description: "Stored axis keys are validated without adjustment: duplicates, unsorted order and values outside the parameter range are reported, never fixed.",
        evidence: "IR Parameter.minimum/maximum versus Binding.parameters[].key_values",
        confidence: Confidence::Exact,
        failure_mode: "A file with out-of-range keys may rely on extension/extrapolation semantics we do not model yet.",
        fixture: "keyform-012-duplicate-key",
    },
    RuleInfo {
        id: "KB-005",
        description: "A binding that involves a blend-shape parameter is marked experimental: the axis is preserved but no interpolation semantics are claimed.",
        evidence: "IR Parameter.kind == blend_shape / extension_key_values",
        confidence: Confidence::Unknown,
        failure_mode: "Blend-shape semantics are documented as insufficiently validated (AGENT.3.5 audit).",
        fixture: "ir-negative-blend-shape-axis",
    },
    RuleInfo {
        id: "KB-006",
        description: "Glue bindings are preserved as evidence but never promoted to keyform targets in AGENT.4 (glue pairing semantics are still Unknown).",
        evidence: "IR Binding.used_by glue entries / Glue.binding",
        confidence: Confidence::Unknown,
        failure_mode: "Glue intensity semantics stay unmapped until they are independently validated.",
        fixture: "ir-negative-glue-deferred",
    },
    RuleInfo {
        id: "KB-007",
        description: "A target without any stored binding is represented as a zero-dimensional band (no axes, cardinality 1) using its stored forms as the single static form.",
        evidence: "absence of a binding reference plus stored keyform count of the target",
        confidence: Confidence::Derived,
        failure_mode: "A target with several stored forms but no binding is suspicious; it stays Unknown layout.",
        fixture: "keyform-001-zero-dimensional",
    },
    RuleInfo {
        id: "KF-001",
        description: "Expected grid cardinality is the product of the axis key counts, computed with checked arithmetic.",
        evidence: "BindingBand axes key counts",
        confidence: Confidence::Derived,
        failure_mode: "Overflow or products above the configured limit become unresolved (KF-002), never truncated.",
        fixture: "keyform-003-2d-3x3",
    },
    RuleInfo {
        id: "KF-002",
        description: "Cardinality overflow and cardinality above the hard limit are preserved as unresolved entries; the Cartesian space is never materialized.",
        evidence: "checked product of axis key counts versus MAX_GRID_CARDINALITY",
        confidence: Confidence::Derived,
        failure_mode: "Legitimate very large grids stay unresolved in AGENT.4 by design.",
        fixture: "ir-negative-cardinality-limit",
    },
    RuleInfo {
        id: "KF-003",
        description: "A grid whose stored form count equals the expected cardinality is Dense; equality is a count comparison, not proof of a Cartesian layout.",
        evidence: "target stored form count versus expected cardinality",
        confidence: Confidence::Derived,
        failure_mode: "A dense count can still hide a permuted form order; ordering stays Unknown (KF-005).",
        fixture: "keyform-003-2d-3x3",
    },
    RuleInfo {
        id: "KF-004",
        description: "A stored form count that differs from the expected cardinality raises KeyformGridCardinalityMismatch and the grid becomes Sparse (fewer) or UnknownLayout (more); no padding, no duplication.",
        evidence: "target stored form count versus expected cardinality",
        confidence: Confidence::Derived,
        failure_mode: "The extra or missing forms are preserved exactly; the grid stays partial/unresolved.",
        fixture: "keyform-014-cardinality-mismatch",
    },
    RuleInfo {
        id: "KF-005",
        description: "Multi-dimensional grid ordering is never guessed: ordering stays Unknown unless evidence exists (reserved Observed).",
        evidence: "absence of an independently validated fastest-axis convention",
        confidence: Confidence::Unknown,
        failure_mode: "Consumer must not assume row-major/column-major from this document.",
        fixture: "keyform-003-2d-3x3",
    },
    RuleInfo {
        id: "KF-006",
        description: "Zero- and one-dimensional grids carry the stored sequence order only; ordering is Derived from the single stored axis ordering.",
        evidence: "single stored key sequence (no axis cross-product)",
        confidence: Confidence::Derived,
        failure_mode: "Stored axis keys themselves may be unsorted; that is reported separately (KB-004).",
        fixture: "keyform-002-1d-three-keys",
    },
    RuleInfo {
        id: "KF-007",
        description: "Stored form indices must be contiguous from zero; gaps or duplicates raise KeyformFormSpanMismatch and are preserved without reindexing.",
        evidence: "keyform index fields of the target",
        confidence: Confidence::Derived,
        failure_mode: "A hand-edited IR with reordered indices stays visible as a span mismatch.",
        fixture: "ir-negative-form-span-gap",
    },
    RuleInfo {
        id: "KF-008",
        description: "Target-specific stored payloads (opacity, draw order, angle, origin, scale, reflect flags) are mapped per form; fields not stored in the file are absent, not defaulted.",
        evidence: "IR keyform arrays per object kind",
        confidence: Confidence::Exact,
        failure_mode: "An absent field is a format fact, not a value of zero.",
        fixture: "keyform-009-artmesh-forms",
    },
    RuleInfo {
        id: "KF-009",
        description: "Geometry stays in the Live2D IR: the keyform model references positions by form index and count and never copies vertex/control-point payloads.",
        evidence: "recovered-keyforms/1 versus live2d-ir/1 ownership boundary",
        confidence: Confidence::Exact,
        failure_mode: "Consumers must resolve geometry through the IR, not this document.",
        fixture: "keyform-020-large-dataset",
    },
    RuleInfo {
        id: "KF-010",
        description: "No combinatorial materialization: grid coordinates are represented as axis counts and a cardinality, never as a generated tuple list.",
        evidence: "document structure (axes + cardinality only)",
        confidence: Confidence::Derived,
        failure_mode: "Very large grids remain unresolved rather than expanded (KF-002).",
        fixture: "keyform-020-large-dataset",
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
    fn docs_mention_every_rule() {
        let docs = include_str!("../../../docs/KEYFORM_RECOVERY.md");
        for info in RULES {
            assert!(
                docs.contains(info.id),
                "docs/KEYFORM_RECOVERY.md is missing rule {}",
                info.id
            );
        }
    }

    /// Fixture stems shipped by `fixture-gen` (kept in sync manually; the
    /// rule table must reference real artifacts).
    const KEYFORM_STEMS: &[&str] = &[
        "keyform-001-zero-dimensional",
        "keyform-002-1d-three-keys",
        "keyform-003-2d-3x3",
        "keyform-004-2d-2x3",
        "keyform-005-3d-small-grid",
        "keyform-006-part-draw-order",
        "keyform-007-warp-forms",
        "keyform-008-rotation-forms",
        "keyform-009-artmesh-forms",
        "keyform-010-multiple-bindings",
        "keyform-011-multiple-targets",
        "keyform-012-duplicate-key",
        "keyform-013-unsorted-keys",
        "keyform-014-cardinality-mismatch",
        "keyform-016-non-finite-key",
        "keyform-019-deep-nested-targets",
        "keyform-020-large-dataset",
    ];

    #[test]
    fn fixture_references_are_resolvable() {
        for info in RULES {
            if info.fixture.starts_with("ir-negative-") {
                continue;
            }
            assert!(
                KEYFORM_STEMS.contains(&info.fixture),
                "rule {} references unknown fixture {}",
                info.id,
                info.fixture
            );
        }
    }

    #[test]
    fn rule_prefixes_match_their_family() {
        for info in RULES {
            assert!(
                info.id.starts_with("KB-") || info.id.starts_with("KF-"),
                "unexpected rule id {}",
                info.id
            );
        }
    }
}
