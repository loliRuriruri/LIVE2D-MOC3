//! Raw (on-disk shaped) model description produced by the parser.
//!
//! Arrays here intentionally mirror the file's table layout; normalization
//! into the Live2D IR is a separate stage (AGENT.2). Field order and array
//! order are deterministic, and unknown data is preserved in dedicated
//! `unknown_*` collections instead of being dropped.

use serde::Serialize;

use crate::counts::CountInfo;
use crate::reader::IdField;
use crate::version::ByteOrder;

/// Confidence label attached to format knowledge (master spec section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldConfidence {
    /// Not yet analyzed.
    Unknown,
    /// Seen in one source but not independently verified.
    Unverified,
    /// Documented/observed in at least one independent reference.
    Observed,
    /// Plausible interpretation that still needs differential testing.
    Hypothesis,
    /// Confirmed by at least two independent sources and/or fixtures.
    Confirmed,
}

/// Parsed header information.
#[derive(Debug, Clone, Serialize)]
pub struct RawHeader {
    /// Raw version byte.
    pub version_byte: u8,
    /// Human-readable version label, e.g. `2 (3.3.00-3.3.03)`.
    pub version_label: String,
    /// Byte order of the payload.
    pub byte_order: ByteOrder,
    /// Non-zero bytes inside the reserved header padding (offset, value).
    pub header_padding_nonzero: Vec<UnknownByte>,
}

/// A preserved non-zero byte in a region whose meaning is unknown.
#[derive(Debug, Clone, Serialize)]
pub struct UnknownByte {
    /// File offset.
    pub offset: u64,
    /// Byte value.
    pub value: u8,
}

/// Canvas information (pixels per unit, origin, size, flags).
#[derive(Debug, Clone, Serialize)]
pub struct RawCanvas {
    /// Pixels per unit used by the editor canvas.
    pub pixels_per_unit: f32,
    /// Canvas X origin.
    pub origin_x: f32,
    /// Canvas Y origin.
    pub origin_y: f32,
    /// Canvas width.
    pub width: f32,
    /// Canvas height.
    pub height: f32,
    /// Raw flag byte.
    pub flags_raw: u8,
    /// `flags_raw & 0x01`: Y axis is reversed.
    pub flag_y_reversed: bool,
}

/// One parameter.
#[derive(Debug, Clone, Serialize)]
pub struct RawParameter {
    /// Index in file order.
    pub index: usize,
    /// Parameter identifier (64-byte field).
    pub id: IdField,
    /// Minimum value.
    pub minimum: f32,
    /// Maximum value.
    pub maximum: f32,
    /// Default value.
    pub default: f32,
    /// Repeat/loop flag.
    pub repeat: bool,
    /// Decimal places shown in the editor.
    pub decimal_places: i64,
    /// Begin index into the key table array.
    pub key_table_begin: i64,
    /// Number of key tables bound to this parameter.
    pub key_table_count: i64,
    /// Parameter type (4.2+): `normal` or `blend_shape`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_type: Option<ParameterType>,
    /// Blend shape key table begin (4.2+).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blend_key_table_begin: Option<i64>,
    /// Blend shape key table count (4.2+).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blend_key_table_count: Option<i64>,
}

/// Parameter kind (4.2+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    /// Ordinary parameter driven by the user/animator.
    Normal,
    /// Blend shape parameter.
    BlendShape,
}

/// One part.
#[derive(Debug, Clone, Serialize)]
pub struct RawPart {
    /// Index in file order.
    pub index: usize,
    /// Part identifier.
    pub id: IdField,
    /// Keyform binding index.
    pub binding_index: i64,
    /// Begin index into part keyforms.
    pub keyform_begin: i64,
    /// Number of part keyforms.
    pub keyform_count: i64,
    /// Visibility flag.
    pub visible: bool,
    /// Enabled flag.
    pub enabled: bool,
    /// Parent part index (`None` when -1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_part: Option<u32>,
    /// Offscreen surface index (5.3+; `None` when -1 or absent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offscreen_index: Option<i64>,
}

/// One deformer (warp or rotation).
#[derive(Debug, Clone, Serialize)]
pub struct RawDeformer {
    /// Index in file order.
    pub index: usize,
    /// Deformer identifier.
    pub id: IdField,
    /// Deformer kind plus kind-specific raw fields.
    pub specific: DeformerSpecific,
    /// Index into the matching (warp or rotation) specific array.
    pub local_index: i64,
    /// Keyform binding index.
    pub binding_index: i64,
    /// Visibility flag.
    pub visible: bool,
    /// Enabled flag.
    pub enabled: bool,
    /// Parent part index (`None` when -1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_part: Option<u32>,
    /// Parent deformer index (`None` when -1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_deformer: Option<u32>,
}

/// Kind-specific deformer payload.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeformerSpecific {
    /// Warp (mesh) deformer.
    Warp {
        /// Total vertex count `(rows + 1) * (columns + 1)`.
        vertex_count: i64,
        /// Grid rows.
        rows: i64,
        /// Grid columns.
        columns: i64,
        /// Quad transform flag (3.3+).
        #[serde(skip_serializing_if = "Option::is_none")]
        quad_transform: Option<i64>,
    },
    /// Rotation deformer.
    Rotation {
        /// Base angle in degrees.
        base_angle: f32,
    },
}

/// One art mesh (drawable).
#[derive(Debug, Clone, Serialize)]
pub struct RawArtMesh {
    /// Index in file order.
    pub index: usize,
    /// Art mesh identifier.
    pub id: IdField,
    /// Keyform binding index.
    pub binding_index: i64,
    /// Begin index into art mesh keyforms.
    pub keyform_begin: i64,
    /// Number of art mesh keyforms.
    pub keyform_count: i64,
    /// Visibility flag.
    pub visible: bool,
    /// Enabled flag.
    pub enabled: bool,
    /// Parent part index (`None` when -1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_part: Option<u32>,
    /// Parent deformer index (`None` when -1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_deformer: Option<u32>,
    /// Texture page number referenced by this mesh.
    pub texture_number: i64,
    /// Raw drawable flag byte.
    pub drawable_flags_raw: u8,
    /// Blend mode bits (2 LSBs of the flag byte).
    pub blend_mode_bits: u8,
    /// Double sided bit (bit 2).
    pub double_sided: bool,
    /// Inverted mask bit (bit 3).
    pub inverted_mask: bool,
    /// Vertex count.
    pub vertex_count: i64,
    /// Begin index into the UV float array (2 floats per vertex).
    pub uv_begin: i64,
    /// Begin index into the triangle index array.
    pub index_begin: i64,
    /// Number of triangle indices.
    pub index_count: i64,
    /// Begin index into the drawable mask array.
    pub mask_begin: i64,
    /// Number of drawable mask entries.
    pub mask_count: i64,
    /// Resolved masking drawable indices (entry values, may contain -1).
    pub mask_drawables: Vec<i64>,
    /// Keyform color begin (4.2+).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyform_color_begin: Option<i64>,
    /// Blend mode value (5.3+).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blend_mode_5_3: Option<i64>,
}

/// One entry of the global drawable mask index array.
#[derive(Debug, Clone, Serialize)]
pub struct RawMaskEntry {
    /// Index in the global mask array.
    pub index: usize,
    /// Referenced art mesh index (-1 allowed).
    pub art_mesh: i64,
}

/// One draw order group.
#[derive(Debug, Clone, Serialize)]
pub struct RawDrawOrderGroup {
    /// Index in file order.
    pub index: usize,
    /// Begin index into draw order items.
    pub object_begin: i64,
    /// Number of draw order items.
    pub object_count: i64,
    /// Total count including nested groups.
    pub object_total_count: i64,
    /// Maximum draw order in this group.
    pub max_order: i64,
    /// Minimum draw order in this group.
    pub min_order: i64,
}

/// One draw order group object.
#[derive(Debug, Clone, Serialize)]
pub struct RawDrawOrderItem {
    /// Index in file order.
    pub index: usize,
    /// `0` = art mesh, `1` = part.
    pub object_type: i64,
    /// Target index (art mesh or part).
    pub object_index: i64,
    /// Nested group index (-1 allowed for art meshes).
    pub self_group: i64,
}

/// One glue object.
#[derive(Debug, Clone, Serialize)]
pub struct RawGlue {
    /// Index in file order.
    pub index: usize,
    /// Glue identifier.
    pub id: IdField,
    /// Keyform binding index.
    pub binding_index: i64,
    /// Begin index into glue keyforms.
    pub keyform_begin: i64,
    /// Number of glue keyforms.
    pub keyform_count: i64,
    /// First glued art mesh.
    pub art_mesh_a: i64,
    /// Second glued art mesh.
    pub art_mesh_b: i64,
    /// Begin index into glue info entries.
    pub info_begin: i64,
    /// Number of glue info entries.
    pub info_count: i64,
}

/// One keyform binding (resolved key table indices).
#[derive(Debug, Clone, Serialize)]
pub struct RawBinding {
    /// Index in file order.
    pub index: usize,
    /// Key table indices bound to this binding.
    pub key_tables: Vec<i64>,
}

/// One key table.
#[derive(Debug, Clone, Serialize)]
pub struct RawKeyTable {
    /// Index in file order.
    pub index: usize,
    /// Begin index into the key value array.
    pub key_begin: i64,
    /// Number of keys.
    pub key_count: i64,
}

/// Runtime pointer scratch section that Cubism Core fills at load time.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSection {
    /// Canonical section name.
    pub name: &'static str,
    /// File offset stored in the offset table.
    pub offset: u32,
    /// Number of pointer-sized elements reserved.
    pub elements: u64,
    /// Reserved byte size on disk.
    pub byte_size: u64,
}

/// A non-zero value in an offset table slot not used by this version.
#[derive(Debug, Clone, Serialize)]
pub struct UnknownSlot {
    /// Slot index inside the offset table.
    pub index: usize,
    /// Raw value found.
    pub value: u32,
}

/// Structured, non-fatal diagnostic recorded while parsing.
#[derive(Debug, Clone, Serialize)]
pub struct Anomaly {
    /// Stable machine-readable code.
    pub code: &'static str,
    /// Human-readable description.
    pub message: String,
    /// Related file offset, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// Related element index, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
}

/// Data the parser intentionally did not extract in AGENT.1.
#[derive(Debug, Clone, Serialize)]
pub struct NotExtracted {
    /// Field name.
    pub field: &'static str,
    /// Number of elements present in the file.
    pub count: u64,
    /// Why it was skipped and which phase will handle it.
    pub reason: &'static str,
}

/// Complete raw parse result.
#[derive(Debug, Clone, Serialize)]
pub struct RawMoc3 {
    /// Header info.
    pub header: RawHeader,
    /// Canvas info.
    pub canvas: RawCanvas,
    /// Count info table.
    pub counts: CountInfo,
    /// Parsed parameters.
    pub parameters: Vec<RawParameter>,
    /// Parsed parts.
    pub parts: Vec<RawPart>,
    /// Parsed deformers.
    pub deformers: Vec<RawDeformer>,
    /// Parsed art meshes.
    pub art_meshes: Vec<RawArtMesh>,
    /// Global drawable mask index array.
    pub masks: Vec<RawMaskEntry>,
    /// Draw order groups.
    pub draw_order_groups: Vec<RawDrawOrderGroup>,
    /// Draw order group objects.
    pub draw_order_items: Vec<RawDrawOrderItem>,
    /// Glue objects.
    pub glues: Vec<RawGlue>,
    /// Keyform bindings.
    pub bindings: Vec<RawBinding>,
    /// Key tables (key value ranges; key values themselves are indexed data).
    pub key_tables: Vec<RawKeyTable>,
    /// Runtime pointer scratch sections (content not interpreted).
    pub runtime_sections: Vec<RuntimeSection>,
    /// Non-zero values in unused offset table slots.
    pub unknown_slots: Vec<UnknownSlot>,
    /// Non-fatal diagnostics.
    pub anomalies: Vec<Anomaly>,
    /// Fields deferred to later phases.
    pub not_extracted: Vec<NotExtracted>,
}
