//! Stage 4: derive grid shape and classification for one target.
//!
//! The expected cardinality comes from the band (`KF-001`). The stored form
//! count is compared against it (`KF-003`/`KF-004`); mismatches are reported
//! and never repaired. Ordering is only claimed for 0D/1D (`KF-006`); a
//! multi-dimensional stored order is never interpreted (`KF-005`).

use live2d_ir::{Confidence, Diagnostic, Severity};

use crate::axes::NormalizedBinding;
use crate::codes;
use crate::collect::TargetEvidence;
use crate::model::{
    AxisStatus, Cardinality, GridAxisSummary, GridLayout, GridOrdering, KeyformGrid, KeyformTarget,
    ReconstructionTrace, UnresolvedEntry,
};
use crate::rule::RuleId;

/// Grid derivation output for one target.
#[derive(Debug, Clone, PartialEq)]
pub struct GridDerivation {
    /// The grid record.
    pub grid: KeyformGrid,
    /// Diagnostics from this stage.
    pub diagnostics: Vec<Diagnostic>,
    /// Unresolved entries from this stage.
    pub unresolved: Vec<UnresolvedEntry>,
}

/// Derive the grid of one target from its band.
pub fn derive_grid_shape(
    grid_id: &str,
    band_id: &str,
    band: &crate::model::BindingBand,
    target: &KeyformTarget,
    stored_form_count: u64,
    form_indices: &[usize],
) -> GridDerivation {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut unresolved: Vec<UnresolvedEntry> = Vec::new();
    let mut rules: Vec<RuleId> = vec![RuleId::new("KF-001")];

    let ordering = if band.axes.len() <= 1 {
        rules.push(RuleId::new("KF-006"));
        GridOrdering::Derived
    } else {
        rules.push(RuleId::new("KF-005"));
        GridOrdering::Unknown
    };

    // Form span contiguity (KF-007): indices must be exactly 0..n.
    let span_ok = form_indices
        .iter()
        .enumerate()
        .all(|(expected, actual)| expected == *actual);
    if !span_ok {
        rules.push(RuleId::new("KF-007"));
        diagnostics.push(Diagnostic::new(
            Severity::Warning,
            codes::KEYFORM_FORM_SPAN_MISMATCH,
            format!(
                "target '{}' stores non-contiguous form indices (expected 0..{}); indices are preserved without reindexing",
                target.id_text(),
                stored_form_count.saturating_sub(1)
            ),
        ));
        unresolved.push(UnresolvedEntry {
            code: codes::KEYFORM_FORM_SPAN_MISMATCH.to_string(),
            target: Some(target.clone()),
            band: Some(band_id.to_string()),
            grid: Some(grid_id.to_string()),
            detail: "stored form indices are not contiguous from zero".to_string(),
        });
    }

    let mut layout;
    let mut confidence;
    match band.expected_cardinality {
        Cardinality::Exact { value } => {
            if stored_form_count == value {
                rules.push(RuleId::new("KF-003"));
                layout = GridLayout::Dense;
                confidence = if band.provenance.confidence == Confidence::Exact {
                    Confidence::Derived
                } else {
                    Confidence::Unknown
                };
            } else if stored_form_count < value {
                rules.push(RuleId::new("KF-004"));
                layout = GridLayout::Sparse;
                confidence = Confidence::Derived;
                diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    codes::KEYFORM_GRID_CARDINALITY_MISMATCH,
                    format!(
                        "target '{}' stores {stored_form_count} form(s) but the band expects {value}; the grid stays partial (no padding, no duplication)",
                        target.id_text()
                    ),
                ));
                unresolved.push(UnresolvedEntry {
                    code: codes::KEYFORM_GRID_CARDINALITY_MISMATCH.to_string(),
                    target: Some(target.clone()),
                    band: Some(band_id.to_string()),
                    grid: Some(grid_id.to_string()),
                    detail: format!("stored {stored_form_count} of {value} expected forms"),
                });
            } else {
                rules.push(RuleId::new("KF-004"));
                layout = GridLayout::UnknownLayout;
                confidence = Confidence::Unknown;
                diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    codes::KEYFORM_GRID_CARDINALITY_MISMATCH,
                    format!(
                        "target '{}' stores {stored_form_count} form(s) but the band expects {value}; the layout is not classified",
                        target.id_text()
                    ),
                ));
                unresolved.push(UnresolvedEntry {
                    code: codes::KEYFORM_GRID_CARDINALITY_MISMATCH.to_string(),
                    target: Some(target.clone()),
                    band: Some(band_id.to_string()),
                    grid: Some(grid_id.to_string()),
                    detail: format!("stored {stored_form_count} forms, {value} expected"),
                });
            }
        }
        Cardinality::Overflow | Cardinality::LimitExceeded => {
            rules.push(RuleId::new("KF-002"));
            layout = GridLayout::UnknownLayout;
            confidence = Confidence::Unknown;
        }
    }

    if !span_ok {
        layout = GridLayout::UnknownLayout;
        confidence = Confidence::Unknown;
    }
    if band.experimental || band.provenance.confidence == Confidence::Unknown {
        confidence = Confidence::Unknown;
    }

    let axes: Vec<GridAxisSummary> = band
        .axes
        .iter()
        .map(|axis| GridAxisSummary {
            parameter: axis.parameter.clone(),
            key_count: axis.stored_key_count,
            resolved: axis.resolved,
        })
        .collect();

    rules.sort();
    rules.dedup();

    GridDerivation {
        grid: KeyformGrid {
            id: grid_id.to_string(),
            band: band_id.to_string(),
            target: target.clone(),
            axes,
            expected_cardinality: band.expected_cardinality,
            stored_form_count,
            layout,
            ordering,
            confidence,
            rules,
        },
        diagnostics,
        unresolved,
    }
}

/// Build the axis entries of a reconstruction trace for a band.
pub fn axis_traces(band: &crate::model::BindingBand) -> Vec<crate::model::AxisTrace> {
    band.axes
        .iter()
        .map(|axis| {
            let status = if !axis.resolved {
                AxisStatus::Dangling
            } else if !axis.non_finite_key_indices.is_empty() {
                AxisStatus::NonFiniteKeys
            } else if axis.rules.iter().any(|rule| rule.as_str() == "KB-005") {
                AxisStatus::Experimental
            } else {
                AxisStatus::Resolved
            };
            crate::model::AxisTrace {
                parameter: axis.parameter.clone(),
                key_count: axis.stored_key_count,
                keys: axis.keys.clone(),
                non_finite_key_indices: axis.non_finite_key_indices.clone(),
                status,
            }
        })
        .collect()
}

/// Build the trace of one target.
pub fn build_trace(
    band: &crate::model::BindingBand,
    grid: &KeyformGrid,
    notes: Vec<String>,
) -> ReconstructionTrace {
    let mut rules: Vec<RuleId> = band.rules.clone();
    rules.extend(grid.rules.iter().cloned());
    rules.sort();
    rules.dedup();
    ReconstructionTrace {
        target: grid.target.clone(),
        band: band.id.clone(),
        grid: grid.id.clone(),
        axes: axis_traces(band),
        expected_cardinality: grid.expected_cardinality,
        stored_form_count: grid.stored_form_count,
        layout: grid.layout,
        ordering: grid.ordering,
        confidence: grid.confidence,
        rules,
        notes,
    }
}

/// The grid id scheme: `grid:{binding}:{kind}:{target-id}` or
/// `grid:static:{kind}:{target-id}`.
pub fn grid_id_for_target(
    target: &KeyformTarget,
    binding: &Option<live2d_ir::BindingId>,
) -> String {
    match binding {
        Some(binding) => format!(
            "grid:{}:{}:{}",
            binding.as_str(),
            target.kind_label(),
            target.id_text()
        ),
        None => format!("grid:static:{}:{}", target.kind_label(), target.id_text()),
    }
}

/// Whether a target evidence entry belongs to a bound band whose binding is
/// missing from the model (dangling binding reference).
pub fn missing_binding_reference(
    evidence: &TargetEvidence,
    normalized: &[NormalizedBinding],
) -> bool {
    match &evidence.binding {
        Some(binding) => !normalized.iter().any(|entry| &entry.id == binding),
        None => false,
    }
}
