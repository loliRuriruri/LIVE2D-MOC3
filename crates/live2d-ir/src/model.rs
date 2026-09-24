//! The normalized Live2D model.
//!
//! These types describe *semantics*, not the binary layout. Collections are
//! in canonical order (source order unless documented otherwise) and all
//! references are typed identifiers. Unknown data never appears on an entity;
//! it lives in [`Unknowns`].

use serde::{Deserialize, Serialize};

use crate::diagnostics::Diagnostic;
use crate::geometry::{Uv, Vec2};
use crate::ids::{
    ArtMeshId, BindingId, DeformerId, DrawOrderGroupId, GlueId, MaskGroupId, ParameterId, PartId,
    TextureId,
};
use crate::provenance::{FieldProvenance, Provenance};

/// Identifier of this IR schema revision.
pub const SCHEMA_ID: &str = "live2d-ir/1";

/// Lifecycle status of the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaStatus {
    /// Schema may still change; no real-file validation has happened yet.
    Experimental,
    /// Schema is frozen.
    Stable,
}

/// Byte order of the source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Endianness {
    /// Little endian.
    Little,
    /// Big endian.
    Big,
}

/// Description of the file the IR was produced from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFormat {
    /// Source container type (always `moc3` in AGENT.2).
    pub format: String,
    /// Raw source format version byte.
    pub version_byte: u8,
    /// Human-readable version label.
    pub version_label: String,
    /// Byte order of the payload.
    pub byte_order: Endianness,
}

/// Entity counts, mirroring the array lengths for quick inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityCounts {
    /// Number of parameters.
    pub parameters: usize,
    /// Number of parts.
    pub parts: usize,
    /// Number of deformers (warp + rotation).
    pub deformers: usize,
    /// Number of warp deformers.
    pub warp_deformers: usize,
    /// Number of rotation deformers.
    pub rotation_deformers: usize,
    /// Number of art meshes.
    pub art_meshes: usize,
    /// Number of draw order groups.
    pub draw_order_groups: usize,
    /// Number of mask groups.
    pub mask_groups: usize,
    /// Number of texture pages.
    pub textures: usize,
    /// Number of glue entries.
    pub glue: usize,
    /// Number of keyform bindings.
    pub bindings: usize,
}

/// Build metadata (no timestamps by design).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// Generator name.
    pub generator: String,
    /// Pipeline phase that produced this file.
    pub generator_phase: String,
    /// Entity counts.
    pub entity_counts: EntityCounts,
}

/// Canvas description.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Canvas {
    /// Canvas width.
    pub width: f32,
    /// Canvas height.
    pub height: f32,
    /// Canvas origin (meaning is only partially confirmed; see provenance).
    pub origin: Vec2,
    /// Editor pixels per unit.
    pub pixels_per_unit: f32,
    /// Whether the source stores a Y-reversed coordinate space.
    pub y_axis_reversed: bool,
    /// Per-field provenance overrides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_provenance: Vec<FieldProvenance>,
    /// Entity provenance.
    pub provenance: Provenance,
}

/// Parameter kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterKind {
    /// Ordinary parameter.
    Normal,
    /// Blend shape parameter (4.2+).
    BlendShape,
}

/// One parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    /// Stable identifier.
    pub id: ParameterId,
    /// Stored identifier text when present and unique.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    /// Minimum value.
    pub minimum: f32,
    /// Maximum value.
    pub maximum: f32,
    /// Default value.
    pub default: f32,
    /// Current value (never stored in the file; always `None` in AGENT.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<f32>,
    /// Repeat flag.
    pub repeat: bool,
    /// Editor decimal places.
    pub decimal_places: i32,
    /// Parameter kind.
    pub kind: ParameterKind,
    /// Key values of this parameter's key table(s), in key table order.
    pub key_values: Vec<f32>,
    /// Extension key values (blend shape parameters, 4.2+).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension_key_values: Vec<f32>,
    /// Per-field provenance overrides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_provenance: Vec<FieldProvenance>,
    /// Entity provenance.
    pub provenance: Provenance,
}

/// One part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Part {
    /// Stable identifier.
    pub id: PartId,
    /// Stored identifier text when present and unique.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    /// Parent part (stored relation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PartId>,
    /// Child parts (derived reverse links, source order).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<PartId>,
    /// Base opacity (not stored in the file; `None` in AGENT.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    /// Base draw order (not stored; per-keyform values are in
    /// `keyform_draw_orders`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draw_order: Option<f32>,
    /// Draw order per part keyform (raw normalized order).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyform_draw_orders: Vec<f32>,
    /// Keyform binding used by this part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingId>,
    /// Stored visibility flag.
    pub visible: bool,
    /// Stored enable flag.
    pub enabled: bool,
    /// Per-field provenance overrides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_provenance: Vec<FieldProvenance>,
    /// Entity provenance.
    pub provenance: Provenance,
}

/// A deformer: warp or rotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Deformer {
    /// Warp (mesh) deformer.
    Warp(WarpDeformer),
    /// Rotation deformer.
    Rotation(RotationDeformer),
}

/// Common fields shared by both deformer kinds (kept inline per kind to keep
/// the JSON flat and explicit).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeformerCommon {
    /// Stable identifier.
    pub id: DeformerId,
    /// Stored identifier text when present and unique.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    /// Parent part (stored relation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_part: Option<PartId>,
    /// Parent deformer (stored relation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_deformer: Option<DeformerId>,
    /// Child deformers (derived reverse links, source order).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children_deformers: Vec<DeformerId>,
    /// Art meshes parented to this deformer (derived reverse links).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children_art_meshes: Vec<ArtMeshId>,
    /// Keyform binding used by this deformer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingId>,
    /// Stored visibility flag.
    pub visible: bool,
    /// Stored enable flag.
    pub enabled: bool,
    /// Per-field provenance overrides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_provenance: Vec<FieldProvenance>,
    /// Entity provenance.
    pub provenance: Provenance,
}

/// Warp deformer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarpDeformer {
    /// Common deformer fields.
    #[serde(flatten)]
    pub common: DeformerCommon,
    /// Grid rows (`row + 1` control vertices per column).
    pub rows: u32,
    /// Grid columns (`column + 1` control vertices per row).
    pub columns: u32,
    /// Total control vertex count: `(rows + 1) * (columns + 1)`.
    pub vertex_count: usize,
    /// Quad transform flag (3.3+).
    pub quad_transform: bool,
    /// Keyforms in declared order.
    pub keyforms: Vec<WarpKeyform>,
}

/// One warp deformer keyform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarpKeyform {
    /// Keyform slot index inside the object's declared range.
    pub index: usize,
    /// Keyform opacity.
    pub opacity: f32,
    /// Control vertex positions (source order).
    pub positions: Vec<Vec2>,
}

/// Rotation deformer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotationDeformer {
    /// Common deformer fields.
    #[serde(flatten)]
    pub common: DeformerCommon,
    /// Base angle in degrees.
    pub base_angle: f32,
    /// Keyforms in declared order.
    pub keyforms: Vec<RotationKeyform>,
}

/// One rotation deformer keyform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotationKeyform {
    /// Keyform slot index inside the object's declared range.
    pub index: usize,
    /// Keyform opacity.
    pub opacity: f32,
    /// Angle in degrees.
    pub angle: f32,
    /// Origin of the rotation.
    pub origin: Vec2,
    /// Uniform scale.
    pub scale: f32,
    /// Reflect on X flag.
    pub reflect_x: bool,
    /// Reflect on Y flag.
    pub reflect_y: bool,
}

/// Drawable flag bits (semantic view of the stored flag byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawableFlags {
    /// Additive blending bit (v5.2 and earlier).
    pub additive: bool,
    /// Multiplicative blending bit (v5.2 and earlier).
    pub multiplicative: bool,
    /// Double sided bit (disables backface culling).
    pub double_sided: bool,
    /// Inverted mask bit.
    pub inverted_mask: bool,
    /// Preserved unknown high bits (always 0 for clean files).
    #[serde(skip_serializing_if = "is_zero_u8", default)]
    pub unknown_bits: u8,
}

fn is_zero_u8(value: &u8) -> bool {
    *value == 0
}

/// Normalized blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    /// Normal blending.
    Normal,
    /// Legacy compatible additive blend.
    AddCompatible,
    /// Legacy compatible multiplicative blend.
    MultiplyCompatible,
    /// Additive blend (5.3+ extended enum).
    Add,
    /// Additive glow blend.
    AddGlow,
    /// Darken blend.
    Darken,
    /// Multiply blend.
    Multiply,
    /// Color burn blend.
    ColorBurn,
    /// Linear burn blend.
    LinearBurn,
    /// Lighten blend.
    Lighten,
    /// Screen blend.
    Screen,
    /// Color dodge blend.
    ColorDodge,
    /// Overlay blend.
    Overlay,
    /// Soft light blend.
    SoftLight,
    /// Hard light blend.
    HardLight,
    /// Linear light blend.
    LinearLight,
    /// Hue blend.
    Hue,
    /// Color blend.
    Color,
    /// Blend value this build does not recognize (raw value preserved).
    Unknown,
}

/// One art mesh keyform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtMeshKeyform {
    /// Keyform slot index inside the object's declared range.
    pub index: usize,
    /// Keyform opacity.
    pub opacity: f32,
    /// Keyform draw order.
    pub draw_order: f32,
    /// Vertex positions (source order, one per vertex).
    pub positions: Vec<Vec2>,
}

/// One art mesh (drawable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtMesh {
    /// Stable identifier.
    pub id: ArtMeshId,
    /// Stored identifier text when present and unique.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    /// Parent part (stored relation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_part: Option<PartId>,
    /// Parent deformer (stored relation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_deformer: Option<DeformerId>,
    /// Referenced texture page (from the stored page number).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<TextureId>,
    /// Vertex count.
    pub vertex_count: usize,
    /// Static texture coordinates (one per vertex).
    pub uvs: Vec<Uv>,
    /// Triangle indices (flat, three per triangle, source order).
    pub indices: Vec<u16>,
    /// Keyforms in declared order.
    pub keyforms: Vec<ArtMeshKeyform>,
    /// Drawable flag bits.
    pub flags: DrawableFlags,
    /// Normalized blend mode.
    pub blend_mode: BlendMode,
    /// Raw blend mode value (only stored from 5.3 on).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blend_mode_raw: Option<i32>,
    /// Base opacity (not stored; `None` in AGENT.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    /// Base draw order (not stored; `None` in AGENT.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draw_order: Option<f32>,
    /// Keyform binding used by this art mesh.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingId>,
    /// Stored visibility flag.
    pub visible: bool,
    /// Stored enable flag.
    pub enabled: bool,
    /// Mask groups whose target is this art mesh.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mask_groups: Vec<MaskGroupId>,
    /// Per-field provenance overrides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_provenance: Vec<FieldProvenance>,
    /// Entity provenance.
    pub provenance: Provenance,
}

/// What a draw order item points at.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum DrawOrderTarget {
    /// A part.
    Part(PartId),
    /// An art mesh.
    ArtMesh(ArtMeshId),
}

/// One draw order item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawOrderItem {
    /// Referenced object.
    pub object: DrawOrderTarget,
    /// Nested draw order group (part items always reference one; art mesh
    /// items may have none, which is stored as `-1` and mapped to `null`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_group: Option<DrawOrderGroupId>,
}

/// One draw order group (stored structure; visual resolution is a later
/// phase).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawOrderGroup {
    /// Stable identifier.
    pub id: DrawOrderGroupId,
    /// Items in source order.
    pub items: Vec<DrawOrderItem>,
    /// Maximum draw order stored for the group.
    pub maximum_order: i32,
    /// Minimum draw order stored for the group.
    pub minimum_order: i32,
    /// Entity provenance.
    pub provenance: Provenance,
}

/// A mask relationship: which art meshes clip a target art mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaskGroup {
    /// Stable identifier.
    pub id: MaskGroupId,
    /// The masked (target) art mesh.
    pub target: ArtMeshId,
    /// Masking art meshes (source order).
    pub sources: Vec<ArtMeshId>,
    /// Entity provenance.
    pub provenance: Provenance,
}

/// A texture page referenced by art meshes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Texture {
    /// Stable identifier.
    pub id: TextureId,
    /// Page number as stored in art meshes.
    pub page_index: u32,
    /// Image width (unknown without `model3.json`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Image height (unknown without `model3.json`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Image path (unknown without `model3.json`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// Entity provenance.
    pub provenance: Provenance,
}

/// One glue info entry (weight and vertex position index).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GlueInfoEntry {
    /// Weight value.
    pub weight: f32,
    /// Vertex position index.
    pub position_index: u16,
}

/// A glue entry connecting two art meshes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Glue {
    /// Stable identifier.
    pub id: GlueId,
    /// Stored identifier text when present and unique.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    /// First glued art mesh.
    pub art_mesh_a: ArtMeshId,
    /// Second glued art mesh.
    pub art_mesh_b: ArtMeshId,
    /// Keyform binding used by this glue entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<BindingId>,
    /// Intensities per glue keyform.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyform_intensities: Vec<f32>,
    /// Glue info entries (source order).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub info: Vec<GlueInfoEntry>,
    /// Per-field provenance overrides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_provenance: Vec<FieldProvenance>,
    /// Entity provenance.
    pub provenance: Provenance,
}

/// One parameter participating in a binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindingParameter {
    /// The parameter.
    pub parameter: ParameterId,
    /// Key values of the key table that references this parameter.
    pub key_values: Vec<f32>,
}

/// Object referencing a keyform binding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum BindingTarget {
    /// A part.
    Part(PartId),
    /// A deformer.
    Deformer(DeformerId),
    /// An art mesh.
    ArtMesh(ArtMeshId),
    /// A glue entry.
    Glue(GlueId),
}

/// A keyform binding: parameters, their key values, and the declared keyform
/// count. Interpolation is out of scope here (AGENT.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    /// Stable identifier.
    pub id: BindingId,
    /// Product of the key counts of the bound parameters (the object's
    /// reachable keyform grid size).
    pub keyform_grid_size: usize,
    /// Key counts per bound parameter (aligned with `parameters`).
    pub keyform_grid: Vec<usize>,
    /// Bound parameters with their key values.
    pub parameters: Vec<BindingParameter>,
    /// Objects using this binding (derived reverse links, deterministic
    /// order: parts, deformers, art meshes, glue).
    pub used_by: Vec<BindingTarget>,
    /// Entity provenance.
    pub provenance: Provenance,
}

/// A preserved non-zero value in an offset table slot undefined for the
/// source version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UndefinedSlot {
    /// Slot index inside the proprietary offset table (kept only for future
    /// format research; never used by semantic consumers).
    pub slot: usize,
    /// Raw value found.
    pub value: u32,
}

/// A preserved non-zero count field outside the documented range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnknownCountField {
    /// Field index inside the count table.
    pub index: usize,
    /// Raw value found.
    pub value: i32,
}

/// Runtime pointer scratch section inventory (contents not interpreted).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSectionInfo {
    /// Section name.
    pub name: String,
    /// Number of reserved pointer-sized elements.
    pub element_count: u64,
    /// Reserved byte size.
    pub byte_size: u64,
}

/// Unknown information preserved separately from the semantic model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unknowns {
    /// Number of non-zero bytes in reserved header padding.
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub header_reserved_nonzero_bytes: usize,
    /// Non-zero values in undefined offset table slots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub undefined_offset_slots: Vec<UndefinedSlot>,
    /// Non-zero values in undocumented count fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown_count_fields: Vec<UnknownCountField>,
    /// Runtime scratch sections.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_sections: Vec<RuntimeSectionInfo>,
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

/// The complete normalized model.
///
/// Field order is the canonical JSON order; arrays are in canonical order:
/// parameters, parts, deformers, art meshes, mask groups, textures, glue and
/// bindings follow source order (textures ascend by page number; mask groups
/// ascend by target art mesh).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Live2DModel {
    /// Schema identifier (`live2d-ir/1`).
    pub schema: String,
    /// Schema lifecycle status.
    pub schema_status: SchemaStatus,
    /// Source file description.
    pub source: SourceFormat,
    /// Build metadata.
    pub metadata: Metadata,
    /// Canvas.
    pub canvas: Canvas,
    /// Parameters.
    pub parameters: Vec<Parameter>,
    /// Parts.
    pub parts: Vec<Part>,
    /// Deformers (warp and rotation, source order).
    pub deformers: Vec<Deformer>,
    /// Art meshes (source order).
    pub art_meshes: Vec<ArtMesh>,
    /// Drawable list (art mesh ids in source order; resolved draw order is a
    /// later-phase concern).
    pub drawables: Vec<ArtMeshId>,
    /// Draw order groups (source order).
    pub draw_order_groups: Vec<DrawOrderGroup>,
    /// Mask groups (ascending by target art mesh).
    pub mask_groups: Vec<MaskGroup>,
    /// Texture pages (ascending by page number).
    pub textures: Vec<Texture>,
    /// Glue entries (source order).
    pub glue: Vec<Glue>,
    /// Keyform bindings (source order).
    pub bindings: Vec<Binding>,
    /// Preserved unknown information.
    pub unknowns: Unknowns,
    /// Parser anomalies, mapper notes and validator findings.
    pub diagnostics: Vec<Diagnostic>,
}

impl Live2DModel {
    /// Entity counts derived from the current arrays.
    pub fn entity_counts(&self) -> EntityCounts {
        let mut warp_deformers = 0usize;
        let mut rotation_deformers = 0usize;
        for deformer in &self.deformers {
            match deformer {
                Deformer::Warp(_) => warp_deformers += 1,
                Deformer::Rotation(_) => rotation_deformers += 1,
            }
        }
        EntityCounts {
            parameters: self.parameters.len(),
            parts: self.parts.len(),
            deformers: self.deformers.len(),
            warp_deformers,
            rotation_deformers,
            art_meshes: self.art_meshes.len(),
            draw_order_groups: self.draw_order_groups.len(),
            mask_groups: self.mask_groups.len(),
            textures: self.textures.len(),
            glue: self.glue.len(),
            bindings: self.bindings.len(),
        }
    }

    /// Look up a deformer by identifier.
    pub fn deformer(&self, id: &DeformerId) -> Option<&Deformer> {
        self.deformers.iter().find(|deformer| match deformer {
            Deformer::Warp(warp) => &warp.common.id == id,
            Deformer::Rotation(rotation) => &rotation.common.id == id,
        })
    }
}
