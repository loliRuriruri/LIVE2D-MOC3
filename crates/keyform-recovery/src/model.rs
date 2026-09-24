//! The recovered keyform model (`live2d-recovery/recovered-keyforms/1`).
//!
//! This layer answers one question: *which parameter axis combination selects
//! or composes which stored form of which target object?* It consumes the
//! Live2D IR read-only; it never sees binary offsets, never copies geometry
//! and never interpolates (runtime interpolation is a separate concern).

use live2d_ir::{
    ArtMeshId, BindingId, Confidence, DeformerId, Diagnostic, Endianness, ParameterId, PartId,
    Provenance, SchemaStatus,
};
use serde::{Deserialize, Serialize};

use crate::rule::RuleId;

/// Identifier of this schema revision.
pub const SCHEMA_ID: &str = "live2d-recovery/recovered-keyforms/1";

/// Hard upper bound for a grid's expected Cartesian cardinality.
///
/// Larger products stay unresolved (`Cardinality::LimitExceeded`) and are
/// never materialized or expanded.
pub const MAX_GRID_CARDINALITY: u64 = 1_000_000;

/// Hard upper bound for axis key counts that enter cardinality math.
pub const MAX_AXIS_KEYS: usize = 65_536;

/// Hard upper bound for axes per binding band.
pub const MAX_AXES_PER_BAND: usize = 64;

/// Reference to a keyform target object (typed; glue is deliberately absent).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum KeyformTarget {
    /// A part.
    Part(PartId),
    /// A warp deformer.
    WarpDeformer(DeformerId),
    /// A rotation deformer.
    RotationDeformer(DeformerId),
    /// An art mesh.
    ArtMesh(ArtMeshId),
}

impl KeyformTarget {
    /// Kind label used in deterministic ids (`part`, `warp`, ...).
    pub fn kind_label(&self) -> &'static str {
        match self {
            KeyformTarget::Part(_) => "part",
            KeyformTarget::WarpDeformer(_) => "warp",
            KeyformTarget::RotationDeformer(_) => "rotation",
            KeyformTarget::ArtMesh(_) => "art_mesh",
        }
    }

    /// The referenced identifier text.
    pub fn id_text(&self) -> &str {
        match self {
            KeyformTarget::Part(id) => id.as_str(),
            KeyformTarget::WarpDeformer(id) | KeyformTarget::RotationDeformer(id) => id.as_str(),
            KeyformTarget::ArtMesh(id) => id.as_str(),
        }
    }
}

/// Expected Cartesian cardinality of a band or grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Cardinality {
    /// Product computed exactly.
    Exact {
        /// Product of axis key counts.
        value: u64,
    },
    /// Product overflowed `u64`; unresolved.
    Overflow,
    /// Product exceeded [`MAX_GRID_CARDINALITY`]; unresolved.
    LimitExceeded,
}

impl Cardinality {
    /// The exact value when known.
    pub fn value(self) -> Option<u64> {
        match self {
            Cardinality::Exact { value } => Some(value),
            Cardinality::Overflow | Cardinality::LimitExceeded => None,
        }
    }

    /// Whether the cardinality is usable for comparisons.
    pub fn is_exact(self) -> bool {
        matches!(self, Cardinality::Exact { .. })
    }
}

/// One normalized parameter axis of a binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterAxis {
    /// Parameter identifier from the binding.
    pub parameter: ParameterId,
    /// Whether the parameter resolved to an IR entity.
    pub resolved: bool,
    /// Number of stored keys before any non-finite filtering.
    pub stored_key_count: usize,
    /// Stored finite key values in stored order (never sorted, never
    /// deduplicated). Non-finite values are reported positionally instead of
    /// being embedded (canonical JSON cannot carry them).
    pub keys: Vec<f32>,
    /// Positions (0-based, in stored order) of non-finite stored keys.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub non_finite_key_indices: Vec<usize>,
    /// Whether the axis can be used for cardinality semantics (parameter
    /// resolved, no non-finite keys, no duplicate keys).
    pub usable: bool,
    /// Provenance of the axis (stored key table or unresolved reference).
    pub provenance: Provenance,
    /// Binding the axis came from.
    pub source_binding: BindingId,
    /// Applied rule ids.
    pub rules: Vec<RuleId>,
}

/// A normalized binding band: the parameter axes a binding uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindingBand {
    /// Deterministic band id (`band:000005`).
    pub id: String,
    /// Source binding.
    pub binding: BindingId,
    /// Normalized axes in binding order.
    pub axes: Vec<ParameterAxis>,
    /// Expected cardinality of the axis product (checked arithmetic).
    pub expected_cardinality: Cardinality,
    /// Targets using this binding (supported kinds only; glue excluded).
    pub targets: Vec<KeyformTarget>,
    /// Blend-shape participation flag (experimental semantics, see KB-005).
    pub experimental: bool,
    /// Provenance of the band.
    pub provenance: Provenance,
    /// Applied rule ids.
    pub rules: Vec<RuleId>,
}

/// Layout classification of a target's stored forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridLayout {
    /// Stored form count equals the expected cardinality.
    Dense,
    /// Fewer stored forms than the expected cardinality (partial grid).
    Sparse,
    /// Layout cannot be classified (extra forms, unresolved axes or
    /// cardinality).
    UnknownLayout,
}

/// Ordering evidence for a grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridOrdering {
    /// Ordering is backed by independently validated evidence (reserved;
    /// AGENT.4 never claims this).
    Observed,
    /// Ordering follows a single stored sequence (0D/1D).
    Derived,
    /// Ordering of a multi-dimensional grid is not interpreted.
    Unknown,
}

/// One axis summary inside a grid record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridAxisSummary {
    /// Parameter identifier.
    pub parameter: ParameterId,
    /// Number of stored keys on this axis.
    pub key_count: usize,
    /// Whether the parameter resolved.
    pub resolved: bool,
}

/// A keyform grid: the association of a target's stored forms with a band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyformGrid {
    /// Deterministic grid id (`grid:000005:art_mesh:000000`).
    pub id: String,
    /// Referenced band id.
    pub band: String,
    /// Target object.
    pub target: KeyformTarget,
    /// Axis summaries in binding order.
    pub axes: Vec<GridAxisSummary>,
    /// Expected cardinality inherited from the band.
    pub expected_cardinality: Cardinality,
    /// Number of stored forms on the target.
    pub stored_form_count: u64,
    /// Layout classification.
    pub layout: GridLayout,
    /// Ordering evidence.
    pub ordering: GridOrdering,
    /// Confidence of the grid association.
    pub confidence: Confidence,
    /// Applied rule ids.
    pub rules: Vec<RuleId>,
}

/// Reference to a stored form's geometry (the payload stays in the IR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormGeometryRef {
    /// Number of position pairs stored for this form in the Live2D IR
    /// (`ArtMesh.keyforms[i].positions` / `WarpDeformer.keyforms[i].positions`).
    ///
    /// The geometry itself is owned by the IR and is deliberately not copied
    /// into this document (rule KF-009).
    pub position_count: usize,
}

/// One part form: parts store only draw order values per form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartKeyformForm {
    /// Stored form index.
    pub index: usize,
    /// Stored draw order value for this form.
    pub draw_order: f32,
}

/// One warp deformer form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarpKeyformForm {
    /// Stored form index.
    pub index: usize,
    /// Stored opacity.
    pub opacity: f32,
    /// Control-point geometry reference (not a copy).
    pub geometry: FormGeometryRef,
}

/// One rotation deformer form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotationKeyformForm {
    /// Stored form index.
    pub index: usize,
    /// Stored opacity.
    pub opacity: f32,
    /// Stored angle in degrees.
    pub angle: f32,
    /// Stored origin.
    pub origin: [f32; 2],
    /// Stored uniform scale.
    pub scale: f32,
    /// Stored reflect-X flag.
    pub reflect_x: bool,
    /// Stored reflect-Y flag.
    pub reflect_y: bool,
}

/// One art mesh form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtMeshKeyformForm {
    /// Stored form index.
    pub index: usize,
    /// Stored opacity.
    pub opacity: f32,
    /// Stored draw order.
    pub draw_order: f32,
    /// Vertex geometry reference (not a copy).
    pub geometry: FormGeometryRef,
}

/// Recovered forms of a part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartTargetKeyforms {
    /// Part id.
    pub target: PartId,
    /// Referenced band id.
    pub band: String,
    /// Referenced grid id.
    pub grid: String,
    /// Source binding when present (absent means static, KB-007).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingId>,
    /// Forms in stored order.
    pub keyforms: Vec<PartKeyformForm>,
}

/// Recovered forms of a warp deformer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarpTargetKeyforms {
    /// Deformer id.
    pub target: DeformerId,
    /// Referenced band id.
    pub band: String,
    /// Referenced grid id.
    pub grid: String,
    /// Source binding when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingId>,
    /// Forms in stored order.
    pub keyforms: Vec<WarpKeyformForm>,
}

/// Recovered forms of a rotation deformer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotationTargetKeyforms {
    /// Deformer id.
    pub target: DeformerId,
    /// Referenced band id.
    pub band: String,
    /// Referenced grid id.
    pub grid: String,
    /// Source binding when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingId>,
    /// Forms in stored order.
    pub keyforms: Vec<RotationKeyformForm>,
}

/// Recovered forms of an art mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtMeshTargetKeyforms {
    /// Art mesh id.
    pub target: ArtMeshId,
    /// Referenced band id.
    pub band: String,
    /// Referenced grid id.
    pub grid: String,
    /// Source binding when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingId>,
    /// Forms in stored order.
    pub keyforms: Vec<ArtMeshKeyformForm>,
}

/// Typed target keyforms; each target kind has its own payload shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "target_type", rename_all = "snake_case")]
pub enum TargetKeyforms {
    /// Part forms.
    Part(PartTargetKeyforms),
    /// Warp deformer forms.
    WarpDeformer(WarpTargetKeyforms),
    /// Rotation deformer forms.
    RotationDeformer(RotationTargetKeyforms),
    /// Art mesh forms.
    ArtMesh(ArtMeshTargetKeyforms),
}

impl TargetKeyforms {
    /// The target this entry describes.
    pub fn target(&self) -> KeyformTarget {
        match self {
            TargetKeyforms::Part(entry) => KeyformTarget::Part(entry.target.clone()),
            TargetKeyforms::WarpDeformer(entry) => {
                KeyformTarget::WarpDeformer(entry.target.clone())
            }
            TargetKeyforms::RotationDeformer(entry) => {
                KeyformTarget::RotationDeformer(entry.target.clone())
            }
            TargetKeyforms::ArtMesh(entry) => KeyformTarget::ArtMesh(entry.target.clone()),
        }
    }

    /// Stored form count.
    pub fn form_count(&self) -> usize {
        match self {
            TargetKeyforms::Part(entry) => entry.keyforms.len(),
            TargetKeyforms::WarpDeformer(entry) => entry.keyforms.len(),
            TargetKeyforms::RotationDeformer(entry) => entry.keyforms.len(),
            TargetKeyforms::ArtMesh(entry) => entry.keyforms.len(),
        }
    }

    /// Referenced band id.
    pub fn band_id(&self) -> &str {
        match self {
            TargetKeyforms::Part(entry) => &entry.band,
            TargetKeyforms::WarpDeformer(entry) => &entry.band,
            TargetKeyforms::RotationDeformer(entry) => &entry.band,
            TargetKeyforms::ArtMesh(entry) => &entry.band,
        }
    }

    /// Referenced grid id.
    pub fn grid_id(&self) -> &str {
        match self {
            TargetKeyforms::Part(entry) => &entry.grid,
            TargetKeyforms::WarpDeformer(entry) => &entry.grid,
            TargetKeyforms::RotationDeformer(entry) => &entry.grid,
            TargetKeyforms::ArtMesh(entry) => &entry.grid,
        }
    }

    /// Source binding when present.
    pub fn binding(&self) -> Option<&BindingId> {
        match self {
            TargetKeyforms::Part(entry) => entry.binding.as_ref(),
            TargetKeyforms::WarpDeformer(entry) => entry.binding.as_ref(),
            TargetKeyforms::RotationDeformer(entry) => entry.binding.as_ref(),
            TargetKeyforms::ArtMesh(entry) => entry.binding.as_ref(),
        }
    }

    /// Stored form indices in stored order.
    pub fn form_indices(&self) -> Vec<usize> {
        match self {
            TargetKeyforms::Part(entry) => entry.keyforms.iter().map(|form| form.index).collect(),
            TargetKeyforms::WarpDeformer(entry) => {
                entry.keyforms.iter().map(|form| form.index).collect()
            }
            TargetKeyforms::RotationDeformer(entry) => {
                entry.keyforms.iter().map(|form| form.index).collect()
            }
            TargetKeyforms::ArtMesh(entry) => {
                entry.keyforms.iter().map(|form| form.index).collect()
            }
        }
    }
}

/// One unresolved condition (never a repaired value).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnresolvedEntry {
    /// Stable code (`dangling_parameter`, `cardinality_mismatch`, ...).
    pub code: String,
    /// Related target when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<KeyformTarget>,
    /// Related band when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub band: Option<String>,
    /// Related grid when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid: Option<String>,
    /// Human-readable detail.
    pub detail: String,
}

/// Axis status inside a trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisStatus {
    /// Parameter resolved and keys are usable as stored.
    Resolved,
    /// Parameter reference did not resolve (dangling).
    Dangling,
    /// Stored keys contain non-finite values.
    NonFiniteKeys,
    /// Axis participates in experimental (blend shape) semantics.
    Experimental,
}

/// One axis entry of a reconstruction trace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisTrace {
    /// Parameter identifier.
    pub parameter: ParameterId,
    /// Number of stored keys.
    pub key_count: usize,
    /// Stored finite keys (small; duplicated here for trace completeness
    /// only).
    pub keys: Vec<f32>,
    /// Positions of non-finite stored keys.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub non_finite_key_indices: Vec<usize>,
    /// Axis status.
    pub status: AxisStatus,
}

/// Full reconstruction trace for one target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconstructionTrace {
    /// Target object.
    pub target: KeyformTarget,
    /// Band id.
    pub band: String,
    /// Grid id.
    pub grid: String,
    /// Axis traces in binding order.
    pub axes: Vec<AxisTrace>,
    /// Expected cardinality.
    pub expected_cardinality: Cardinality,
    /// Stored form count.
    pub stored_form_count: u64,
    /// Layout classification.
    pub layout: GridLayout,
    /// Ordering evidence.
    pub ordering: GridOrdering,
    /// Overall confidence.
    pub confidence: Confidence,
    /// Applied rule ids (sorted).
    pub rules: Vec<RuleId>,
    /// Explanatory notes.
    pub notes: Vec<String>,
}

/// Keyform recovery statistics (exact counts only; never percentages).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyformStatistics {
    /// Target objects represented.
    pub targets: usize,
    /// Binding bands (one per used binding plus static bands).
    pub bindings: usize,
    /// Parameter axes across all bands.
    pub parameter_axes: usize,
    /// Stored forms represented.
    pub stored_keyforms: u64,
    /// Bands whose axes are all resolved and non-experimental.
    pub exact_bindings: usize,
    /// Bands derived from absence of a binding (static, KB-007).
    pub derived_bindings: usize,
    /// Bands produced by a heuristic rule (none in AGENT.4).
    pub heuristic_bindings: usize,
    /// Bands whose semantics stay unresolved (unusable or experimental axes).
    pub unknown_bindings: usize,
    /// Grids classified Dense.
    pub dense_grids: usize,
    /// Grids classified Sparse.
    pub sparse_grids: usize,
    /// Grids whose layout is Unknown.
    pub unknown_layout_grids: usize,
    /// Grids with unresolved axes or cardinality.
    pub unresolved_grids: usize,
    /// Unresolved entries.
    pub unresolved_entries: usize,
    /// Diagnostics in the document.
    pub diagnostics: usize,
}

/// Source references of a recovered keyform model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceModelRef {
    /// Schema of the IR document this was derived from.
    pub ir_schema: String,
    /// Schema of the recovered project document.
    pub project_schema: String,
    /// Source format version label (from the IR).
    pub source_version_label: String,
    /// Raw source version byte.
    pub source_version_byte: u8,
    /// Byte order of the source payload.
    pub source_byte_order: Endianness,
}

/// The complete recovered keyform model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveredKeyformModel {
    /// Schema identifier (`live2d-recovery/recovered-keyforms/1`).
    pub schema: String,
    /// Schema lifecycle status (experimental).
    pub schema_status: SchemaStatus,
    /// Source references.
    pub source_model: SourceModelRef,
    /// Binding bands in deterministic order.
    pub binding_bands: Vec<BindingBand>,
    /// Keyform grids in deterministic order.
    pub keyform_grids: Vec<KeyformGrid>,
    /// Target keyforms in deterministic order.
    pub target_keyforms: Vec<TargetKeyforms>,
    /// Unresolved conditions (never repaired).
    pub unresolved: Vec<UnresolvedEntry>,
    /// Diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// Reconstruction traces (one per target).
    pub traces: Vec<ReconstructionTrace>,
    /// Exact statistics.
    pub statistics: KeyformStatistics,
}

impl RecoveredKeyformModel {
    /// Look up a band by id.
    pub fn band(&self, id: &str) -> Option<&BindingBand> {
        self.binding_bands.iter().find(|band| band.id == id)
    }

    /// Look up a grid by id.
    pub fn grid(&self, id: &str) -> Option<&KeyformGrid> {
        self.keyform_grids.iter().find(|grid| grid.id == id)
    }

    /// Look up the trace for a target.
    pub fn trace_for(&self, target: &KeyformTarget) -> Option<&ReconstructionTrace> {
        self.traces.iter().find(|trace| &trace.target == target)
    }
}
