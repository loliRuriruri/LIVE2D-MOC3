//! Stage 3: construct normalized binding bands.
//!
//! A band is the axis set of one binding (or of one static target without a
//! binding, KB-007) plus the targets that use it. No grid or form
//! interpretation happens here.

use std::collections::BTreeMap;

use live2d_ir::{Diagnostic, Provenance, Severity};

use crate::axes::{checked_cardinality, NormalizedBinding};
use crate::codes;
use crate::collect::TargetEvidence;
use crate::model::{BindingBand, Cardinality, KeyformTarget};
use crate::rule::RuleId;

/// Result of the band construction stage.
#[derive(Debug, Clone, PartialEq)]
pub struct BandBuild {
    /// Bands in deterministic order (bound bands in binding order, then
    /// static bands in target order).
    pub bands: Vec<BindingBand>,
    /// Band id per source binding.
    pub band_by_binding: BTreeMap<live2d_ir::BindingId, String>,
    /// Band id per static target (no binding).
    pub static_band_by_target: BTreeMap<KeyformTarget, String>,
    /// Diagnostics from this stage.
    pub diagnostics: Vec<Diagnostic>,
    /// Unresolved entries from this stage.
    pub unresolved: Vec<crate::model::UnresolvedEntry>,
}

/// Build binding bands from normalized bindings and collected targets.
pub fn build_binding_bands(
    normalized: &[NormalizedBinding],
    targets: &[TargetEvidence],
) -> BandBuild {
    let mut bands: Vec<BindingBand> = Vec::with_capacity(normalized.len() + targets.len());
    let mut band_by_binding: BTreeMap<live2d_ir::BindingId, String> = BTreeMap::new();
    let mut static_band_by_target: BTreeMap<KeyformTarget, String> = BTreeMap::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut unresolved: Vec<crate::model::UnresolvedEntry> = Vec::new();

    for binding in normalized {
        let id = format!("band:{}", binding.id.as_str());
        let mut rules: Vec<RuleId> = vec![RuleId::new("KB-001")];
        let mut usable = true;
        let mut experimental = false;

        for axis in &binding.axes {
            for rule in &axis.rules {
                rules.push(rule.clone());
            }
            // The band confidence contract is defined in terms of the axis's
            // own usability (resolution + finite + duplicates); never
            // re-derive it from a subset of those conditions.
            if !axis.usable {
                usable = false;
            }
            if !axis.resolved {
                usable = false;
                unresolved.push(crate::model::UnresolvedEntry {
                    code: codes::DANGLING_PARAMETER.to_string(),
                    target: None,
                    band: Some(id.clone()),
                    grid: None,
                    detail: format!(
                        "axis parameter '{}' does not resolve to an IR parameter",
                        axis.parameter.as_str()
                    ),
                });
            }
            if !axis.non_finite_key_indices.is_empty() {
                usable = false;
                unresolved.push(crate::model::UnresolvedEntry {
                    code: codes::NON_FINITE_PARAMETER_KEY.to_string(),
                    target: None,
                    band: Some(id.clone()),
                    grid: None,
                    detail: format!(
                        "axis parameter '{}' stores non-finite keys at position(s) {:?}",
                        axis.parameter.as_str(),
                        axis.non_finite_key_indices
                    ),
                });
            }
            if axis.keys.len() != axis.stored_key_count {
                usable = false;
            }
            if axis.rules.iter().any(|rule| rule.as_str() == "KB-005") {
                experimental = true;
            }
        }

        if binding.glue_used {
            rules.push(RuleId::new("KB-006"));
            diagnostics.push(Diagnostic::new(
                Severity::Info,
                codes::GLUE_KEYFORM_BINDING_DEFERRED,
                format!(
                    "binding '{}' is referenced by a glue entry; glue targets are preserved but deferred in AGENT.4",
                    binding.id.as_str()
                ),
            ));
            unresolved.push(crate::model::UnresolvedEntry {
                code: codes::GLUE_KEYFORM_BINDING_DEFERRED.to_string(),
                target: None,
                band: Some(id.clone()),
                grid: None,
                detail: format!(
                    "binding '{}' is used by glue; glue pairing semantics remain Unknown",
                    binding.id.as_str()
                ),
            });
        }

        let expected_cardinality = checked_cardinality(&binding.axes);
        match expected_cardinality {
            Cardinality::Overflow => {
                rules.push(RuleId::new("KF-002"));
                diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    codes::KEYFORM_CARDINALITY_OVERFLOW,
                    format!(
                        "band '{id}' cardinality overflows u64; the grid stays unresolved and is never expanded"
                    ),
                ));
                unresolved.push(crate::model::UnresolvedEntry {
                    code: codes::KEYFORM_CARDINALITY_OVERFLOW.to_string(),
                    target: None,
                    band: Some(id.clone()),
                    grid: None,
                    detail: "axis key count product overflows u64".to_string(),
                });
            }
            Cardinality::LimitExceeded => {
                rules.push(RuleId::new("KF-002"));
                diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    codes::KEYFORM_CARDINALITY_LIMIT_EXCEEDED,
                    format!(
                        "band '{id}' cardinality exceeds the hard limit of {}; the grid stays unresolved and is never expanded",
                        crate::model::MAX_GRID_CARDINALITY
                    ),
                ));
                unresolved.push(crate::model::UnresolvedEntry {
                    code: codes::KEYFORM_CARDINALITY_LIMIT_EXCEEDED.to_string(),
                    target: None,
                    band: Some(id.clone()),
                    grid: None,
                    detail: format!(
                        "axis key count product exceeds {}",
                        crate::model::MAX_GRID_CARDINALITY
                    ),
                });
            }
            Cardinality::Exact { .. } => {}
        }

        let confidence = if !usable || experimental || !expected_cardinality.is_exact() {
            live2d_ir::Confidence::Unknown
        } else {
            live2d_ir::Confidence::Exact
        };
        let provenance = match confidence {
            live2d_ir::Confidence::Exact => {
                Provenance::exact("moc3:binding")
            }
            _ => Provenance::unknown(
                "moc3:binding",
                "band axes are unresolved or experimental; no complete binding semantics are claimed",
            ),
        };

        let mut band_targets = binding.targets.clone();
        band_targets.sort();
        band_targets.dedup();

        rules.sort();
        rules.dedup();
        band_by_binding.insert(binding.id.clone(), id.clone());
        bands.push(BindingBand {
            id,
            binding: binding.id.clone(),
            axes: binding.axes.clone(),
            expected_cardinality,
            targets: band_targets,
            experimental,
            provenance,
            rules,
        });
    }

    // Static bands (KB-007): targets without a binding reference.
    for target_evidence in targets {
        if target_evidence.binding.is_some() || target_evidence.form_indices.is_empty() {
            continue;
        }
        let id = format!(
            "band:static:{}:{}",
            target_evidence.target.kind_label(),
            target_evidence.target.id_text()
        );
        let cardinality = Cardinality::Exact { value: 1 };
        if target_evidence.form_indices.len() > 1 {
            diagnostics.push(Diagnostic::new(
                Severity::Warning,
                codes::STATIC_TARGET_MULTIPLE_FORMS,
                format!(
                    "target '{}' stores {} forms but has no binding; static semantics are unclear",
                    target_evidence.target.id_text(),
                    target_evidence.form_indices.len()
                ),
            ));
            unresolved.push(crate::model::UnresolvedEntry {
                code: codes::STATIC_TARGET_MULTIPLE_FORMS.to_string(),
                target: Some(target_evidence.target.clone()),
                band: Some(id.clone()),
                grid: None,
                detail: "no binding is stored but multiple forms exist; layout is not guessed"
                    .to_string(),
            });
        }
        static_band_by_target.insert(target_evidence.target.clone(), id.clone());
        bands.push(BindingBand {
            id,
            binding: live2d_ir::BindingId::new(""),
            axes: Vec::new(),
            expected_cardinality: cardinality,
            targets: vec![target_evidence.target.clone()],
            experimental: false,
            provenance: Provenance::derived("moc3:absence_of_binding"),
            rules: vec![RuleId::new("KB-007")],
        });
    }

    BandBuild {
        bands,
        band_by_binding,
        static_band_by_target,
        diagnostics,
        unresolved,
    }
}
