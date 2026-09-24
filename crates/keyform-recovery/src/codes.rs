//! Stable diagnostic codes produced by keyform recovery.
//!
//! Codes are part of the machine-readable contract (CLI `--strict` and
//! differential tooling match on them); they are never reused for different
//! meanings.

/// A binding parameter reference did not resolve to a parameter entity.
pub const DANGLING_PARAMETER: &str = "dangling_parameter";
/// A stored axis contains duplicate key values (preserved, never deduplicated).
pub const DUPLICATE_PARAMETER_KEY: &str = "duplicate_parameter_key";
/// Stored axis keys are not in ascending order (order is preserved).
pub const UNSORTED_PARAMETER_KEYS: &str = "unsorted_parameter_keys";
/// Stored axis keys lie outside the declared parameter range.
pub const OUT_OF_RANGE_PARAMETER_KEY: &str = "out_of_range_parameter_key";
/// Stored axis keys contain non-finite values (reported positionally).
pub const NON_FINITE_PARAMETER_KEY: &str = "non_finite_parameter_key";
/// A blend-shape parameter participates in a binding (experimental).
pub const BLEND_SHAPE_BINDING_EXPERIMENTAL: &str = "blend_shape_binding_experimental";
/// A glue entry references a binding; glue targets are deferred (KB-006).
pub const GLUE_KEYFORM_BINDING_DEFERRED: &str = "glue_keyform_binding_deferred";
/// A target without a binding stores more than one form.
pub const STATIC_TARGET_MULTIPLE_FORMS: &str = "static_target_multiple_forms";
/// Stored form count differs from the expected cardinality (KF-004).
pub const KEYFORM_GRID_CARDINALITY_MISMATCH: &str = "keyform_grid_cardinality_mismatch";
/// A band declares more axes than the hard limit allows.
pub const AXIS_COUNT_LIMIT_EXCEEDED: &str = "axis_count_limit_exceeded";
/// An axis stores more keys than the hard limit allows.
pub const AXIS_KEY_LIMIT_EXCEEDED: &str = "axis_key_limit_exceeded";
/// Expected cardinality overflowed `u64` (KF-002).
pub const KEYFORM_CARDINALITY_OVERFLOW: &str = "keyform_cardinality_overflow";
/// Expected cardinality exceeded [`MAX_GRID_CARDINALITY`] (KF-002).
pub const KEYFORM_CARDINALITY_LIMIT_EXCEEDED: &str = "keyform_cardinality_limit_exceeded";
/// Stored form indices are not contiguous from zero (KF-007).
pub const KEYFORM_FORM_SPAN_MISMATCH: &str = "keyform_form_span_mismatch";
/// A binding references a target object that does not exist in the IR.
pub const DANGLING_KEYFORM_TARGET: &str = "dangling_keyform_target";
/// A target references a binding that does not exist in the IR.
pub const DANGLING_BINDING_REFERENCE: &str = "dangling_binding_reference";
/// The validator found a duplicate band id.
pub const DUPLICATE_BAND_ID: &str = "duplicate_band_id";
/// The validator found a duplicate grid id.
pub const DUPLICATE_GRID_ID: &str = "duplicate_grid_id";
/// The validator found two entries for the same target.
pub const DUPLICATE_TARGET_ENTRY: &str = "duplicate_target_entry";
/// The validator found a reference to a band that does not exist.
pub const DANGLING_BAND_REFERENCE: &str = "dangling_band_reference";
/// The validator found a target type that disagrees with the IR entity.
pub const WRONG_TARGET_TYPE: &str = "wrong_target_type";
/// The validator found two axes for the same parameter inside one band.
pub const DUPLICATE_AXIS: &str = "duplicate_axis";
/// The validator found a stored form index outside `0..stored_form_count`.
pub const FORM_INDEX_OUT_OF_BOUNDS: &str = "form_index_out_of_bounds";
/// Statistics or unresolved accounting disagrees with the document body.
pub const UNRESOLVED_ACCOUNTING_MISMATCH: &str = "unresolved_accounting_mismatch";
/// Document schema id is not the expected one.
pub const SCHEMA_MISMATCH: &str = "schema_mismatch";
/// A confidence/provenance combination is internally inconsistent.
pub const CONFIDENCE_PROVENANCE_INCONSISTENT: &str = "confidence_provenance_inconsistent";
/// A target does not exist in the recovered project (cross-layer check).
pub const TARGET_NOT_IN_PROJECT: &str = "target_not_in_project";
