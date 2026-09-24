//! Semantic keyform / parameter binding recovery (AGENT.4).
//!
//! Pipeline (work order sections 4, 23, 31 - deliberately split, never one
//! giant algorithm):
//!
//! ```text
//! Live2D IR
//!   -> collect stored binding evidence        (collect.rs)
//!   -> normalize parameter axes               (axes.rs)
//!   -> build binding bands                    (bands.rs)
//!   -> derive grid shape / classification     (grids.rs)
//!   -> map target-specific forms              (targets.rs)
//!   -> RecoveredKeyformModel                  (model.rs)
//!   -> validation                             (validate.rs)
//! ```
//!
//! Hard rules:
//!
//! - only `live2d-ir` + `hierarchy-recovery` inputs; binary layout details
//!   are unknown here (no offsets, no table slots in the output),
//! - stored evidence stays `Exact`; grid cardinality is `Derived`; ordering
//!   and unresolved semantics stay `Unknown`,
//! - nothing is repaired: missing forms are never padded, duplicates never
//!   deduplicated, unsorted keys never sorted,
//! - interpolation/evaluation is runtime territory and is not modeled,
//! - geometry is never copied; forms reference IR payloads by index/count,
//! - deterministic output: same IR + project in, byte-identical JSON out.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod codes;
pub mod model;
pub mod rule;

mod axes;
mod bands;
mod codec;
mod collect;
mod grids;
mod targets;
mod text;
mod validate;

use std::collections::BTreeMap;

use hierarchy_recovery::RecoveredProject;
use live2d_ir::{Diagnostic, Live2DModel, Severity};

pub use codec::{from_json_str, to_json_str, KeyformExportError, KeyformImportError};
pub use model::{
    ArtMeshKeyformForm, ArtMeshTargetKeyforms, AxisStatus, AxisTrace, BindingBand, Cardinality,
    FormGeometryRef, GridAxisSummary, GridLayout, GridOrdering, KeyformGrid, KeyformStatistics,
    KeyformTarget, ParameterAxis, PartKeyformForm, PartTargetKeyforms, ReconstructionTrace,
    RecoveredKeyformModel, RotationKeyformForm, RotationTargetKeyforms, SourceModelRef,
    TargetKeyforms, UnresolvedEntry, WarpKeyformForm, WarpTargetKeyforms, MAX_AXES_PER_BAND,
    MAX_AXIS_KEYS, MAX_GRID_CARDINALITY, SCHEMA_ID,
};
pub use rule::{rule_info, RuleId, RuleInfo, RULES};
pub use validate::validate_recovered_keyforms;

/// Recover the keyform model for a Live2D IR document.
///
/// When a recovered project is provided it is used for cross-layer checks
/// only; it never changes the keyform semantics (work order section 69).
pub fn recover(model: &Live2DModel, project: Option<&RecoveredProject>) -> RecoveredKeyformModel {
    let evidence = collect::collect_binding_evidence(model);
    let (normalized, mut diagnostics) = axes::normalize_axes(model, &evidence.bindings);
    let band_build = bands::build_binding_bands(&normalized, &evidence.targets);
    let bands::BandBuild {
        bands: band_list,
        band_by_binding,
        static_band_by_target,
        diagnostics: band_diagnostics,
        unresolved: band_unresolved,
    } = band_build;
    diagnostics.extend(band_diagnostics);
    let mut unresolved = band_unresolved;

    // Targets referenced by a band but absent from the model.
    let evidence_targets: BTreeMap<KeyformTarget, &collect::TargetEvidence> = evidence
        .targets
        .iter()
        .map(|entry| (entry.target.clone(), entry))
        .collect();
    for band in &band_list {
        for target in &band.targets {
            if !evidence_targets.contains_key(target) && targets::target_exists(model, target) {
                // The object exists; the mismatch is elsewhere (kind
                // ambiguity), which validation reports. Nothing to do here.
                continue;
            }
            if !targets::target_exists(model, target) {
                diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    codes::DANGLING_KEYFORM_TARGET,
                    format!(
                        "binding '{}' references target '{}' which does not exist in the IR; the target is preserved but has no forms",
                        band.binding.as_str(),
                        target.id_text()
                    ),
                ));
                unresolved.push(UnresolvedEntry {
                    code: codes::DANGLING_KEYFORM_TARGET.to_string(),
                    target: Some(target.clone()),
                    band: Some(band.id.clone()),
                    grid: None,
                    detail: "target object missing in the IR".to_string(),
                });
            }
        }
    }

    let mut keyform_grids: Vec<KeyformGrid> = Vec::new();
    let mut target_keyforms: Vec<TargetKeyforms> = Vec::new();
    let mut traces: Vec<ReconstructionTrace> = Vec::new();

    for target_evidence in &evidence.targets {
        if grids::missing_binding_reference(target_evidence, &normalized) {
            diagnostics.push(Diagnostic::new(
                Severity::Warning,
                codes::DANGLING_BINDING_REFERENCE,
                format!(
                    "target '{}' references binding '{}' which does not exist in the IR",
                    target_evidence.target.id_text(),
                    target_evidence
                        .binding
                        .as_ref()
                        .map(|binding| binding.as_str())
                        .unwrap_or("-")
                ),
            ));
            unresolved.push(UnresolvedEntry {
                code: codes::DANGLING_BINDING_REFERENCE.to_string(),
                target: Some(target_evidence.target.clone()),
                band: None,
                grid: None,
                detail: "binding reference missing in the IR".to_string(),
            });
            continue;
        }
        let band_id = match &target_evidence.binding {
            Some(binding) => band_by_binding.get(binding),
            None => static_band_by_target.get(&target_evidence.target),
        };
        let Some(band) = band_id.and_then(|id| band_list.iter().find(|band| &band.id == id)) else {
            continue;
        };
        let grid_id = grids::grid_id_for_target(&target_evidence.target, &target_evidence.binding);
        let derivation = grids::derive_grid_shape(
            &grid_id,
            &band.id,
            band,
            &target_evidence.target,
            target_evidence.form_indices.len() as u64,
            &target_evidence.form_indices,
        );
        diagnostics.extend(derivation.diagnostics);
        unresolved.extend(derivation.unresolved);

        let Some((forms, notes)) = targets::map_forms(model, target_evidence, &band.id, &grid_id)
        else {
            diagnostics.push(Diagnostic::new(
                Severity::Warning,
                codes::DANGLING_KEYFORM_TARGET,
                format!(
                    "target '{}' could not be mapped to an IR entity",
                    target_evidence.target.id_text()
                ),
            ));
            unresolved.push(UnresolvedEntry {
                code: codes::DANGLING_KEYFORM_TARGET.to_string(),
                target: Some(target_evidence.target.clone()),
                band: Some(band.id.clone()),
                grid: Some(grid_id),
                detail: "target mapping failed; kind mismatch suspected".to_string(),
            });
            continue;
        };

        let (payload_diagnostics, payload_unresolved) = targets::scan_payload_finiteness(&forms);
        diagnostics.extend(payload_diagnostics);
        unresolved.extend(payload_unresolved);

        traces.push(grids::build_trace(band, &derivation.grid, notes));
        keyform_grids.push(derivation.grid);
        target_keyforms.push(forms);
    }

    let mut document = RecoveredKeyformModel {
        schema: SCHEMA_ID.to_string(),
        schema_status: live2d_ir::SchemaStatus::Experimental,
        source_model: SourceModelRef {
            ir_schema: model.schema.clone(),
            project_schema: project
                .map(|project| project.schema.clone())
                .unwrap_or_else(|| "-".to_string()),
            source_version_label: model.source.version_label.clone(),
            source_version_byte: model.source.version_byte,
            source_byte_order: model.source.byte_order,
        },
        binding_bands: band_list,
        keyform_grids,
        target_keyforms,
        unresolved,
        diagnostics,
        traces,
        statistics: empty_statistics(),
    };
    document.statistics = compute_statistics(&document);
    let validation = validate_recovered_keyforms(&document, model, project);
    document.diagnostics.extend(validation);
    document.diagnostics = cap_diagnostics(document.diagnostics);
    document.unresolved = cap_unresolved(document.unresolved);
    document.statistics = compute_statistics(&document);
    document
}

/// Per-code diagnostic cap (work order section 59: diagnostic amplification).
const MAX_DIAGNOSTICS_PER_CODE: usize = 256;

/// Hard cap for unresolved entries.
const MAX_UNRESOLVED_ENTRIES: usize = 4_096;

fn cap_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut kept: Vec<Diagnostic> = Vec::with_capacity(diagnostics.len());
    for diagnostic in diagnostics {
        let entry = counts.entry(diagnostic.code.clone()).or_insert((0, 0));
        entry.1 += 1;
        if entry.0 < MAX_DIAGNOSTICS_PER_CODE {
            entry.0 += 1;
            kept.push(diagnostic);
        }
    }
    let mut summaries: Vec<Diagnostic> = Vec::new();
    for (code, (kept_count, total)) in &counts {
        if kept_count < total {
            summaries.push(Diagnostic::new(
                Severity::Info,
                codes::DIAGNOSTIC_CAP_REACHED,
                format!("diagnostic cap: kept {kept_count} of {total} '{code}' finding(s)"),
            ));
        }
    }
    kept.extend(summaries);
    kept
}

fn cap_unresolved(mut unresolved: Vec<UnresolvedEntry>) -> Vec<UnresolvedEntry> {
    if unresolved.len() <= MAX_UNRESOLVED_ENTRIES {
        return unresolved;
    }
    let suppressed = unresolved.len() - MAX_UNRESOLVED_ENTRIES;
    unresolved.truncate(MAX_UNRESOLVED_ENTRIES);
    unresolved.push(UnresolvedEntry {
        code: codes::UNRESOLVED_CAP_REACHED.to_string(),
        target: None,
        band: None,
        grid: None,
        detail: format!("{suppressed} additional unresolved entr(ies) suppressed by the cap"),
    });
    unresolved
}

/// Recompute statistics from the document body (also used by the validator).
pub fn compute_statistics(document: &RecoveredKeyformModel) -> KeyformStatistics {
    let mut statistics = empty_statistics();
    statistics.diagnostics = document.diagnostics.len();
    statistics.unresolved_entries = document.unresolved.len();
    statistics.targets = document.target_keyforms.len();
    statistics.bindings = document.binding_bands.len();
    statistics.parameter_axes = document
        .binding_bands
        .iter()
        .map(|band| band.axes.len())
        .sum();
    statistics.stored_keyforms = document
        .target_keyforms
        .iter()
        .map(|entry| entry.form_count() as u64)
        .sum();
    for band in &document.binding_bands {
        match band.provenance.confidence {
            live2d_ir::Confidence::Exact => statistics.exact_bindings += 1,
            live2d_ir::Confidence::Derived => statistics.derived_bindings += 1,
            live2d_ir::Confidence::Heuristic => statistics.heuristic_bindings += 1,
            live2d_ir::Confidence::Unknown => statistics.unknown_bindings += 1,
        }
    }
    for grid in &document.keyform_grids {
        match grid.layout {
            GridLayout::Dense => statistics.dense_grids += 1,
            GridLayout::Sparse => statistics.sparse_grids += 1,
            GridLayout::UnknownLayout => statistics.unknown_layout_grids += 1,
        }
        if grid.confidence == live2d_ir::Confidence::Unknown {
            statistics.unresolved_grids += 1;
        }
    }
    statistics
}

fn empty_statistics() -> KeyformStatistics {
    KeyformStatistics {
        targets: 0,
        bindings: 0,
        parameter_axes: 0,
        stored_keyforms: 0,
        exact_bindings: 0,
        derived_bindings: 0,
        heuristic_bindings: 0,
        unknown_bindings: 0,
        dense_grids: 0,
        sparse_grids: 0,
        unknown_layout_grids: 0,
        unresolved_grids: 0,
        unresolved_entries: 0,
        diagnostics: 0,
    }
}

/// Explain one target, band or grid (canonical id or stored target name).
pub fn explain(document: &RecoveredKeyformModel, query: &str) -> Option<String> {
    text::render_explain(document, query)
}

/// Render the human statistics report.
pub fn render_human(document: &RecoveredKeyformModel) -> String {
    text::render_human(document)
}

/// Diagnostic codes that make `--strict` fail (work order section 52).
pub const STRICT_FAILURE_CODES: &[&str] = &[
    codes::DANGLING_PARAMETER,
    codes::NON_FINITE_PARAMETER_KEY,
    codes::KEYFORM_CARDINALITY_OVERFLOW,
    codes::KEYFORM_CARDINALITY_LIMIT_EXCEEDED,
    codes::KEYFORM_GRID_CARDINALITY_MISMATCH,
    codes::KEYFORM_FORM_SPAN_MISMATCH,
    codes::DANGLING_KEYFORM_TARGET,
    codes::DANGLING_BINDING_REFERENCE,
    codes::WRONG_TARGET_TYPE,
];

/// True when any diagnostic uses one of the strict failure codes.
pub fn strict_violations(document: &RecoveredKeyformModel) -> Vec<&str> {
    let mut failures: Vec<&str> = STRICT_FAILURE_CODES
        .iter()
        .copied()
        .filter(|code| {
            document
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == *code)
        })
        .collect();
    failures.sort_unstable();
    failures
}

/// Number of diagnostics with the given severity (re-export convenience).
pub fn count_severity(diagnostics: &[Diagnostic], severity: Severity) -> usize {
    live2d_ir::count_severity(diagnostics, severity)
}
