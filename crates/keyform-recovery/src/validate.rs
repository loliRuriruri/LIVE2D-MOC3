//! Validator for recovered keyform documents.
//!
//! `validate_recovered_keyforms` re-checks the document against itself, the
//! Live2D IR and (optionally) the recovered project. Structural corruption is
//! Fatal and blocks export; expected unresolved states (dangling parameters,
//! mismatched forms, experimental blend shapes) stay non-fatal by design.

use std::collections::BTreeSet;

use hierarchy_recovery::RecoveredProject;
use live2d_ir::{Confidence, Diagnostic, Severity};

use crate::codes;
use crate::model::{Cardinality, GridLayout, KeyformTarget, RecoveredKeyformModel};
use crate::targets::target_exists;

/// Validate a recovered keyform document.
pub fn validate_recovered_keyforms(
    document: &RecoveredKeyformModel,
    model: &live2d_ir::Live2DModel,
    project: Option<&RecoveredProject>,
) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    if document.schema != crate::model::SCHEMA_ID {
        diagnostics.push(Diagnostic::new(
            Severity::Fatal,
            codes::SCHEMA_MISMATCH,
            format!(
                "document schema '{}' is not '{}'",
                document.schema,
                crate::model::SCHEMA_ID
            ),
        ));
    }

    // Structural uniqueness.
    let mut band_ids: BTreeSet<&str> = BTreeSet::new();
    for band in &document.binding_bands {
        if !band_ids.insert(band.id.as_str()) {
            diagnostics.push(Diagnostic::new(
                Severity::Fatal,
                codes::DUPLICATE_BAND_ID,
                format!("band id '{}' appears more than once", band.id),
            ));
        }
        let mut axis_ids: BTreeSet<&str> = BTreeSet::new();
        for axis in &band.axes {
            if !axis_ids.insert(axis.parameter.as_str()) {
                diagnostics.push(Diagnostic::new(
                    Severity::Fatal,
                    codes::DUPLICATE_AXIS,
                    format!(
                        "band '{}' contains parameter '{}' more than once",
                        band.id,
                        axis.parameter.as_str()
                    ),
                ));
            }
            let usable_claim = axis.usable;
            let usable_fact = axis.resolved
                && axis.non_finite_key_indices.is_empty()
                && axis.keys.len() == axis.stored_key_count;
            if usable_claim != usable_fact {
                diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    codes::CONFIDENCE_PROVENANCE_INCONSISTENT,
                    format!(
                        "axis '{}' of band '{}' claims usable={} but stored facts disagree",
                        axis.parameter.as_str(),
                        band.id,
                        usable_claim
                    ),
                ));
            }
        }
        // Confidence contract: Exact bands must be fully usable, non
        // experimental and have an exact cardinality.
        let fully_usable = band.axes.iter().all(|axis| axis.usable);
        let expect_exact = fully_usable
            && !band.experimental
            && matches!(band.expected_cardinality, Cardinality::Exact { .. });
        let is_exact = band.provenance.confidence == Confidence::Exact;
        if is_exact != expect_exact {
            diagnostics.push(Diagnostic::new(
                Severity::Warning,
                codes::CONFIDENCE_PROVENANCE_INCONSISTENT,
                format!(
                    "band '{}' confidence {:?} disagrees with its axis/cardinality state",
                    band.id, band.provenance.confidence
                ),
            ));
        }
        if let (Cardinality::Exact { value }, Some(recomputed)) =
            (band.expected_cardinality, recomputed_product(&band.axes))
        {
            if value != recomputed {
                diagnostics.push(Diagnostic::new(
                    Severity::Fatal,
                    codes::UNRESOLVED_ACCOUNTING_MISMATCH,
                    format!(
                        "band '{}' declares cardinality {value} but its axes multiply to {recomputed}",
                        band.id
                    ),
                ));
            }
        }
    }
    let known_bands: BTreeSet<&str> = document
        .binding_bands
        .iter()
        .map(|band| band.id.as_str())
        .collect();

    let mut grid_ids: BTreeSet<&str> = BTreeSet::new();
    for grid in &document.keyform_grids {
        if !grid_ids.insert(grid.id.as_str()) {
            diagnostics.push(Diagnostic::new(
                Severity::Fatal,
                codes::DUPLICATE_GRID_ID,
                format!("grid id '{}' appears more than once", grid.id),
            ));
        }
        if !known_bands.contains(grid.band.as_str()) {
            diagnostics.push(Diagnostic::new(
                Severity::Fatal,
                codes::DANGLING_BAND_REFERENCE,
                format!("grid '{}' references unknown band '{}'", grid.id, grid.band),
            ));
        }
        if !target_exists(model, &grid.target) {
            diagnostics.push(Diagnostic::new(
                Severity::Fatal,
                codes::WRONG_TARGET_TYPE,
                format!(
                    "grid '{}' targets '{}' which does not exist with that kind in the IR",
                    grid.id,
                    grid.target.id_text()
                ),
            ));
        }
    }

    // Target uniqueness and grid integrity.
    let mut seen_targets: BTreeSet<KeyformTarget> = BTreeSet::new();
    for entry in &document.target_keyforms {
        let target = entry.target();
        if !seen_targets.insert(target.clone()) {
            diagnostics.push(Diagnostic::new(
                Severity::Fatal,
                codes::DUPLICATE_TARGET_ENTRY,
                format!("target '{}' appears more than once", target.id_text()),
            ));
        }
        let grid = document.grid(&grid_ref(entry));
        let Some(grid) = grid else {
            diagnostics.push(Diagnostic::new(
                Severity::Fatal,
                codes::DANGLING_BAND_REFERENCE,
                format!("target '{}' references a missing grid", target.id_text()),
            ));
            continue;
        };
        if grid.target != target {
            diagnostics.push(Diagnostic::new(
                Severity::Fatal,
                codes::WRONG_TARGET_TYPE,
                format!(
                    "target '{}' points at grid '{}' which belongs to a different target",
                    target.id_text(),
                    grid.id
                ),
            ));
        }
        let form_count = entry.form_count() as u64;
        if form_count != grid.stored_form_count {
            diagnostics.push(Diagnostic::new(
                Severity::Fatal,
                codes::UNRESOLVED_ACCOUNTING_MISMATCH,
                format!(
                    "target '{}' carries {form_count} form(s) but grid '{}' records {}",
                    target.id_text(),
                    grid.id,
                    grid.stored_form_count
                ),
            ));
        }
    }

    // Statistics must match a fresh recomputation.
    let recomputed = crate::compute_statistics(document);
    if recomputed != document.statistics {
        diagnostics.push(Diagnostic::new(
            Severity::Fatal,
            codes::UNRESOLVED_ACCOUNTING_MISMATCH,
            "statistics do not match the document body".to_string(),
        ));
    }

    // Cross-layer checks.
    if let Some(project) = project {
        let node_ids: BTreeSet<&str> = project.nodes.iter().map(|node| node.id.as_str()).collect();
        for entry in &document.target_keyforms {
            let target = entry.target();
            if !node_ids.contains(target.id_text()) {
                diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    codes::TARGET_NOT_IN_PROJECT,
                    format!(
                        "target '{}' is not present in the recovered project; keyform data is preserved but not hierarchy-linked",
                        target.id_text()
                    ),
                ));
            }
        }
    }

    // Grid layout/confidence contract.
    for grid in &document.keyform_grids {
        let band = document.band(&grid.band);
        if let Some(band) = band {
            if band.experimental && grid.confidence != Confidence::Unknown {
                diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    codes::CONFIDENCE_PROVENANCE_INCONSISTENT,
                    format!(
                        "grid '{}' has a confident layout inside an experimental band",
                        grid.id
                    ),
                ));
            }
        }
        if grid.layout == GridLayout::Sparse && !grid.expected_cardinality.is_exact() {
            diagnostics.push(Diagnostic::new(
                Severity::Warning,
                codes::CONFIDENCE_PROVENANCE_INCONSISTENT,
                format!(
                    "grid '{}' is Sparse but its cardinality is unresolved",
                    grid.id
                ),
            ));
        }
    }

    diagnostics
}

fn grid_ref(entry: &crate::model::TargetKeyforms) -> String {
    match entry {
        crate::model::TargetKeyforms::Part(entry) => entry.grid.clone(),
        crate::model::TargetKeyforms::WarpDeformer(entry) => entry.grid.clone(),
        crate::model::TargetKeyforms::RotationDeformer(entry) => entry.grid.clone(),
        crate::model::TargetKeyforms::ArtMesh(entry) => entry.grid.clone(),
    }
}

fn recomputed_product(axes: &[crate::model::ParameterAxis]) -> Option<u64> {
    let mut product: u64 = 1;
    for axis in axes {
        product = product.checked_mul(axis.stored_key_count as u64)?;
    }
    Some(product)
}
