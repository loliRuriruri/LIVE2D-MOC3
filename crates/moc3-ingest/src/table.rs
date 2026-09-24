//! Section offset table layout.
//!
//! The MOC3 file stores a fixed array of `u32` file offsets at `0x40`
//! (160 slots for format versions 1-5, 480 for version 6+). Each slot
//! describes one section. Version blocks are cumulative prefixes of a single
//! canonical order:
//!
//! - v3.0 (101 slots)
//! - v3.3 adds 1 slot (`warp.quad_transform`) -> 102
//! - v4.2 adds 35 slots (parameter extensions, colors, blend shapes) -> 137
//! - v5.0 adds 15 slots (keyform colors, blend shape targets) -> 152
//! - v5.3 adds 15 slots (offscreen rendering) -> 167
//!
//! All offsets are validated (alignment, bounds, monotonic order) while
//! walking this table before any section content is decoded.

use crate::counts::CountInfo;
use crate::version::MocVersion;

/// Number of slots actually defined per version byte (index = byte - 1):
/// 3.0 = 101, 3.3 = 102, 4.0 = 102, 4.2 = 137, 5.0 = 152, 5.3+ = 167.
pub const SLOT_COUNT_BY_VERSION: [usize; 6] = [101, 102, 102, 137, 152, 167];

/// Element layout of a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elem {
    /// 64-byte identifier.
    Id,
    /// Runtime pointer scratch (native pointer sized on disk, 8 bytes).
    Ptr,
    /// `u32` scalar.
    U32,
    /// `i32` scalar.
    I32,
    /// `f32` scalar.
    F32,
    /// `u16` scalar.
    U16,
    /// `u8` scalar.
    U8,
    /// Count info table (128 or 256 bytes depending on version).
    CountInfo,
    /// Canvas info structure (24 bytes used, padding beyond).
    Canvas,
}

impl Elem {
    /// Byte size of one element for a given version.
    pub fn size(self, version: MocVersion) -> u64 {
        match self {
            Elem::Id => 64,
            Elem::Ptr => 8,
            Elem::U32 | Elem::I32 | Elem::F32 => 4,
            Elem::U16 => 2,
            Elem::U8 => 1,
            Elem::CountInfo => version.count_info_ints() as u64 * 4,
            Elem::Canvas => 24,
        }
    }
}

/// Which count info field gives the element count of a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountKey {
    /// Static single element section.
    None,
    /// `count_info.parts`.
    Parts,
    /// `count_info.deformers`.
    Deformers,
    /// `count_info.warp_deformers`.
    Warps,
    /// `count_info.rotation_deformers`.
    Rotations,
    /// `count_info.art_meshes`.
    ArtMeshes,
    /// `count_info.parameters`.
    Parameters,
    /// `count_info.part_keyforms`.
    PartKeyforms,
    /// `count_info.warp_deformer_keyforms`.
    WarpKeyforms,
    /// `count_info.rotation_deformer_keyforms`.
    RotationKeyforms,
    /// `count_info.art_mesh_keyforms`.
    ArtMeshKeyforms,
    /// `count_info.keyform_positions`.
    KeyformPos,
    /// `count_info.parameter_binding_indices`.
    KeyTableIdx,
    /// `count_info.keyform_bindings`.
    Bindings,
    /// `count_info.parameter_bindings`.
    KeyTables,
    /// `count_info.keys`.
    Keys,
    /// `count_info.uvs`.
    Uvs,
    /// `count_info.position_indices`.
    Idx,
    /// `count_info.drawable_masks`.
    Masks,
    /// `count_info.draw_order_groups`.
    DrawGroups,
    /// `count_info.draw_order_group_objects`.
    DrawItems,
    /// `count_info.glue`.
    Glues,
    /// `count_info.glue_info`.
    GlueInfo,
    /// `count_info.glue_keyforms`.
    GlueKeyforms,
    /// `count_info.keyform_multiply_colors`.
    KeyformMulColors,
    /// `count_info.keyform_screen_colors`.
    KeyformScrColors,
    /// `count_info.blend_shape_parameter_bindings`.
    BlendKeyTables,
    /// `count_info.blend_shape_keyform_bindings`.
    BlendBindings,
    /// `count_info.blend_shapes_warp_deformers`.
    BsWarps,
    /// `count_info.blend_shapes_art_meshes`.
    BsArtMeshes,
    /// `count_info.blend_shape_constraint_indices`.
    BsConstraintIdx,
    /// `count_info.blend_shape_constraints`.
    BsConstraints,
    /// `count_info.blend_shape_constraint_values`.
    BsConstraintVals,
    /// `count_info.blend_shapes_parts`.
    BsParts,
    /// `count_info.blend_shapes_rotation_deformers`.
    BsRotations,
    /// `count_info.blend_shapes_glue`.
    BsGlues,
    /// `count_info.offscreens`.
    Offscreens,
    /// `count_info.offscreen_keyforms`.
    OffscreenKeyforms,
    /// `count_info.blend_shapes_offscreens`.
    BsOffscreens,
}

/// Resolve a count key against a parsed count table.
pub fn count_for(key: CountKey, counts: &CountInfo) -> u64 {
    match key {
        CountKey::None => 1,
        CountKey::Parts => counts.parts as u64,
        CountKey::Deformers => counts.deformers as u64,
        CountKey::Warps => counts.warp_deformers as u64,
        CountKey::Rotations => counts.rotation_deformers as u64,
        CountKey::ArtMeshes => counts.art_meshes as u64,
        CountKey::Parameters => counts.parameters as u64,
        CountKey::PartKeyforms => counts.part_keyforms as u64,
        CountKey::WarpKeyforms => counts.warp_deformer_keyforms as u64,
        CountKey::RotationKeyforms => counts.rotation_deformer_keyforms as u64,
        CountKey::ArtMeshKeyforms => counts.art_mesh_keyforms as u64,
        CountKey::KeyformPos => counts.keyform_positions as u64,
        CountKey::KeyTableIdx => counts.parameter_binding_indices as u64,
        CountKey::Bindings => counts.keyform_bindings as u64,
        CountKey::KeyTables => counts.parameter_bindings as u64,
        CountKey::Keys => counts.keys as u64,
        CountKey::Uvs => counts.uvs as u64,
        CountKey::Idx => counts.position_indices as u64,
        CountKey::Masks => counts.drawable_masks as u64,
        CountKey::DrawGroups => counts.draw_order_groups as u64,
        CountKey::DrawItems => counts.draw_order_group_objects as u64,
        CountKey::Glues => counts.glue as u64,
        CountKey::GlueInfo => counts.glue_info as u64,
        CountKey::GlueKeyforms => counts.glue_keyforms as u64,
        CountKey::KeyformMulColors => counts.keyform_multiply_colors as u64,
        CountKey::KeyformScrColors => counts.keyform_screen_colors as u64,
        CountKey::BlendKeyTables => counts.blend_shape_parameter_bindings as u64,
        CountKey::BlendBindings => counts.blend_shape_keyform_bindings as u64,
        CountKey::BsWarps => counts.blend_shapes_warp_deformers as u64,
        CountKey::BsArtMeshes => counts.blend_shapes_art_meshes as u64,
        CountKey::BsConstraintIdx => counts.blend_shape_constraint_indices as u64,
        CountKey::BsConstraints => counts.blend_shape_constraints as u64,
        CountKey::BsConstraintVals => counts.blend_shape_constraint_values as u64,
        CountKey::BsParts => counts.blend_shapes_parts as u64,
        CountKey::BsRotations => counts.blend_shapes_rotation_deformers as u64,
        CountKey::BsGlues => counts.blend_shapes_glue as u64,
        CountKey::Offscreens => counts.offscreens as u64,
        CountKey::OffscreenKeyforms => counts.offscreen_keyforms as u64,
        CountKey::BsOffscreens => counts.blend_shapes_offscreens as u64,
    }
}

/// One entry of the canonical slot order.
#[derive(Debug, Clone, Copy)]
pub struct SlotDef {
    /// Canonical dotted section name.
    pub name: &'static str,
    /// Element layout.
    pub elem: Elem,
    /// Count info key giving the element count.
    pub count: CountKey,
    /// Whether the section is runtime pointer scratch (content not interpreted).
    pub runtime: bool,
}

const fn s(name: &'static str, elem: Elem, count: CountKey, runtime: bool) -> SlotDef {
    SlotDef {
        name,
        elem,
        count,
        runtime,
    }
}

use CountKey as C;
use Elem::{Canvas, Id, Ptr, F32, I32, U16, U8};

/// Canonical slot order for all versions (cumulative).
pub static SLOT_DEFS: &[SlotDef] = &[
    // v3.0 block (slots 0-100)
    s("count_info", Elem::CountInfo, C::None, false),
    s("canvas_info", Canvas, C::None, false),
    s("part.id_runtime", Ptr, C::Parts, true),
    s("part.id", Id, C::Parts, false),
    s("part.binding_index", I32, C::Parts, false),
    s("part.keyform_begin", I32, C::Parts, false),
    s("part.keyform_count", I32, C::Parts, false),
    s("part.visible", I32, C::Parts, false),
    s("part.enabled", I32, C::Parts, false),
    s("part.parent_part", I32, C::Parts, false),
    s("deformer.id_runtime", Ptr, C::Deformers, true),
    s("deformer.id", Id, C::Deformers, false),
    s("deformer.binding_index", I32, C::Deformers, false),
    s("deformer.visible", I32, C::Deformers, false),
    s("deformer.enabled", I32, C::Deformers, false),
    s("deformer.parent_part", I32, C::Deformers, false),
    s("deformer.parent_deformer", I32, C::Deformers, false),
    s("deformer.type", I32, C::Deformers, false),
    s("deformer.local_index", I32, C::Deformers, false),
    s("warp.binding_index", I32, C::Warps, false),
    s("warp.keyform_begin", I32, C::Warps, false),
    s("warp.keyform_count", I32, C::Warps, false),
    s("warp.vertex_count", I32, C::Warps, false),
    s("warp.rows", I32, C::Warps, false),
    s("warp.columns", I32, C::Warps, false),
    s("rotation.binding_index", I32, C::Rotations, false),
    s("rotation.keyform_begin", I32, C::Rotations, false),
    s("rotation.keyform_count", I32, C::Rotations, false),
    s("rotation.base_angle", F32, C::Rotations, false),
    s("art_mesh.id_runtime", Ptr, C::ArtMeshes, true),
    s("art_mesh.uv_runtime", Ptr, C::ArtMeshes, true),
    s("art_mesh.position_index_runtime", Ptr, C::ArtMeshes, true),
    s("art_mesh.drawable_mask_runtime", Ptr, C::ArtMeshes, true),
    s("art_mesh.id", Id, C::ArtMeshes, false),
    s("art_mesh.binding_index", I32, C::ArtMeshes, false),
    s("art_mesh.keyform_begin", I32, C::ArtMeshes, false),
    s("art_mesh.keyform_count", I32, C::ArtMeshes, false),
    s("art_mesh.visible", I32, C::ArtMeshes, false),
    s("art_mesh.enabled", I32, C::ArtMeshes, false),
    s("art_mesh.parent_part", I32, C::ArtMeshes, false),
    s("art_mesh.parent_deformer", I32, C::ArtMeshes, false),
    s("art_mesh.texture_number", I32, C::ArtMeshes, false),
    s("art_mesh.drawable_flags", U8, C::ArtMeshes, false),
    s("art_mesh.vertex_count", I32, C::ArtMeshes, false),
    s("art_mesh.uv_begin", I32, C::ArtMeshes, false),
    s("art_mesh.index_begin", I32, C::ArtMeshes, false),
    s("art_mesh.index_count", I32, C::ArtMeshes, false),
    s("art_mesh.mask_begin", I32, C::ArtMeshes, false),
    s("art_mesh.mask_count", I32, C::ArtMeshes, false),
    s("parameter.id_runtime", Ptr, C::Parameters, true),
    s("parameter.id", Id, C::Parameters, false),
    s("parameter.maximum", F32, C::Parameters, false),
    s("parameter.minimum", F32, C::Parameters, false),
    s("parameter.default", F32, C::Parameters, false),
    s("parameter.repeat", I32, C::Parameters, false),
    s("parameter.decimal_places", I32, C::Parameters, false),
    s("parameter.key_table_begin", I32, C::Parameters, false),
    s("parameter.key_table_count", I32, C::Parameters, false),
    s("part_keyform.draw_order", F32, C::PartKeyforms, false),
    s("warp_keyform.opacity", F32, C::WarpKeyforms, false),
    s("warp_keyform.position_begin", I32, C::WarpKeyforms, false),
    s("rotation_keyform.opacity", F32, C::RotationKeyforms, false),
    s("rotation_keyform.angle", F32, C::RotationKeyforms, false),
    s("rotation_keyform.origin_x", F32, C::RotationKeyforms, false),
    s("rotation_keyform.origin_y", F32, C::RotationKeyforms, false),
    s("rotation_keyform.scale", F32, C::RotationKeyforms, false),
    s(
        "rotation_keyform.reflect_x",
        I32,
        C::RotationKeyforms,
        false,
    ),
    s(
        "rotation_keyform.reflect_y",
        I32,
        C::RotationKeyforms,
        false,
    ),
    s("art_mesh_keyform.opacity", F32, C::ArtMeshKeyforms, false),
    s(
        "art_mesh_keyform.draw_order",
        F32,
        C::ArtMeshKeyforms,
        false,
    ),
    s(
        "art_mesh_keyform.position_begin",
        I32,
        C::ArtMeshKeyforms,
        false,
    ),
    s("keyform_position.xy", F32, C::KeyformPos, false),
    s("key_table_index", I32, C::KeyTableIdx, false),
    s("binding.key_table_begin", I32, C::Bindings, false),
    s("binding.key_table_count", I32, C::Bindings, false),
    s("key_table.begin", I32, C::KeyTables, false),
    s("key_table.count", I32, C::KeyTables, false),
    s("key.values", F32, C::Keys, false),
    s("uv.xy", F32, C::Uvs, false),
    s("position_indices", U16, C::Idx, false),
    s("drawable_mask.indices", I32, C::Masks, false),
    s("draw_group.object_begin", I32, C::DrawGroups, false),
    s("draw_group.object_count", I32, C::DrawGroups, false),
    s("draw_group.object_total_count", I32, C::DrawGroups, false),
    s("draw_group.max_order", I32, C::DrawGroups, false),
    s("draw_group.min_order", I32, C::DrawGroups, false),
    s("draw_item.type", I32, C::DrawItems, false),
    s("draw_item.index", I32, C::DrawItems, false),
    s("draw_item.self_group", I32, C::DrawItems, false),
    s("glue.id_runtime", Ptr, C::Glues, true),
    s("glue.id", Id, C::Glues, false),
    s("glue.binding_index", I32, C::Glues, false),
    s("glue.keyform_begin", I32, C::Glues, false),
    s("glue.keyform_count", I32, C::Glues, false),
    s("glue.art_mesh_a", I32, C::Glues, false),
    s("glue.art_mesh_b", I32, C::Glues, false),
    s("glue.info_begin", I32, C::Glues, false),
    s("glue.info_count", I32, C::Glues, false),
    s("glue_info.weight", F32, C::GlueInfo, false),
    s("glue_info.position_index", U16, C::GlueInfo, false),
    s("glue_keyform.intensity", F32, C::GlueKeyforms, false),
    // v3.3 block
    s("warp.quad_transform", I32, C::Warps, false),
    // v4.2 block
    s("parameter.key_runtime", Ptr, C::Parameters, true),
    s("parameter.extension_key_begin", I32, C::Parameters, false),
    s("parameter.extension_key_count", I32, C::Parameters, false),
    s("warp.keyform_color_begin", I32, C::Warps, false),
    s("rotation.keyform_color_begin", I32, C::Rotations, false),
    s("art_mesh.keyform_color_begin", I32, C::ArtMeshes, false),
    s("keyform_color.multiply.r", F32, C::KeyformMulColors, false),
    s("keyform_color.multiply.g", F32, C::KeyformMulColors, false),
    s("keyform_color.multiply.b", F32, C::KeyformMulColors, false),
    s("keyform_color.screen.r", F32, C::KeyformScrColors, false),
    s("keyform_color.screen.g", F32, C::KeyformScrColors, false),
    s("keyform_color.screen.b", F32, C::KeyformScrColors, false),
    s("parameter.type", I32, C::Parameters, false),
    s("parameter.blend_key_table_begin", I32, C::Parameters, false),
    s("parameter.blend_key_table_count", I32, C::Parameters, false),
    s("blend_key_table.begin", I32, C::BlendKeyTables, false),
    s("blend_key_table.count", I32, C::BlendKeyTables, false),
    s("blend_key_table.base_key", I32, C::BlendKeyTables, false),
    s(
        "blend_binding.key_table_index",
        I32,
        C::BlendBindings,
        false,
    ),
    s("blend_binding.keyform_begin", I32, C::BlendBindings, false),
    s("blend_binding.keyform_count", I32, C::BlendBindings, false),
    s(
        "blend_binding.constraint_begin",
        I32,
        C::BlendBindings,
        false,
    ),
    s(
        "blend_binding.constraint_count",
        I32,
        C::BlendBindings,
        false,
    ),
    s("blend_shape.warp_target", I32, C::BsWarps, false),
    s("blend_shape.warp_binding_begin", I32, C::BsWarps, false),
    s("blend_shape.warp_binding_count", I32, C::BsWarps, false),
    s("blend_shape.art_mesh_target", I32, C::BsArtMeshes, false),
    s(
        "blend_shape.art_mesh_binding_begin",
        I32,
        C::BsArtMeshes,
        false,
    ),
    s(
        "blend_shape.art_mesh_binding_count",
        I32,
        C::BsArtMeshes,
        false,
    ),
    s("blend_constraint_index", I32, C::BsConstraintIdx, false),
    s("blend_constraint.parameter", I32, C::BsConstraints, false),
    s("blend_constraint.value_begin", I32, C::BsConstraints, false),
    s("blend_constraint.value_count", I32, C::BsConstraints, false),
    s(
        "blend_constraint_value.key",
        F32,
        C::BsConstraintVals,
        false,
    ),
    s(
        "blend_constraint_value.weight",
        F32,
        C::BsConstraintVals,
        false,
    ),
    // v5.0 block
    s(
        "warp_keyform.multiply_color_begin",
        I32,
        C::WarpKeyforms,
        false,
    ),
    s(
        "warp_keyform.screen_color_begin",
        I32,
        C::WarpKeyforms,
        false,
    ),
    s(
        "rotation_keyform.multiply_color_begin",
        I32,
        C::RotationKeyforms,
        false,
    ),
    s(
        "rotation_keyform.screen_color_begin",
        I32,
        C::RotationKeyforms,
        false,
    ),
    s(
        "art_mesh_keyform.multiply_color_begin",
        I32,
        C::ArtMeshKeyforms,
        false,
    ),
    s(
        "art_mesh_keyform.screen_color_begin",
        I32,
        C::ArtMeshKeyforms,
        false,
    ),
    s("blend_shape.part_target", I32, C::BsParts, false),
    s("blend_shape.part_binding_begin", I32, C::BsParts, false),
    s("blend_shape.part_binding_count", I32, C::BsParts, false),
    s("blend_shape.rotation_target", I32, C::BsRotations, false),
    s(
        "blend_shape.rotation_binding_begin",
        I32,
        C::BsRotations,
        false,
    ),
    s(
        "blend_shape.rotation_binding_count",
        I32,
        C::BsRotations,
        false,
    ),
    s("blend_shape.glue_target", I32, C::BsGlues, false),
    s("blend_shape.glue_binding_begin", I32, C::BsGlues, false),
    s("blend_shape.glue_binding_count", I32, C::BsGlues, false),
    // v5.3 block
    s("part.offscreen_index", I32, C::Parts, false),
    s("art_mesh.blend_mode", I32, C::ArtMeshes, false),
    s("offscreen.drawable_mask_runtime", Ptr, C::Offscreens, true),
    s("offscreen.owner_index", I32, C::Offscreens, false),
    s("offscreen.drawable_flags", U8, C::Offscreens, false),
    s("offscreen.blend_mode", I32, C::Offscreens, false),
    s("offscreen.mask_begin", I32, C::Offscreens, false),
    s("offscreen.mask_count", I32, C::Offscreens, false),
    s("part_keyform.key_index", I32, C::PartKeyforms, false),
    s(
        "offscreen_keyform.opacity",
        F32,
        C::OffscreenKeyforms,
        false,
    ),
    s(
        "offscreen_keyform.multiply_color_begin",
        I32,
        C::OffscreenKeyforms,
        false,
    ),
    s(
        "offscreen_keyform.screen_color_begin",
        I32,
        C::OffscreenKeyforms,
        false,
    ),
    s("blend_shape.offscreen_target", I32, C::BsOffscreens, false),
    s(
        "blend_shape.offscreen_binding_begin",
        I32,
        C::BsOffscreens,
        false,
    ),
    s(
        "blend_shape.offscreen_binding_count",
        I32,
        C::BsOffscreens,
        false,
    ),
];

/// Slice of `SLOT_DEFS` active for `version`.
pub fn defs_for(version: MocVersion) -> &'static [SlotDef] {
    let used = version.used_slots();
    SLOT_DEFS.get(..used).unwrap_or(SLOT_DEFS)
}

// ---------------------------------------------------------------------------
// Named slot indices for sections the inspector reads directly.
// Each constant is covered by the `slot_names_match_defs` unit test.
// ---------------------------------------------------------------------------

/// `count_info` slot.
pub const S_COUNT_INFO: usize = 0;
/// `canvas_info` slot.
pub const S_CANVAS_INFO: usize = 1;
/// `part.id` slot.
pub const S_PART_ID: usize = 3;
/// `part.binding_index` slot.
pub const S_PART_BINDING: usize = 4;
/// `part.keyform_begin` slot.
pub const S_PART_KEYFORM_BEGIN: usize = 5;
/// `part.keyform_count` slot.
pub const S_PART_KEYFORM_COUNT: usize = 6;
/// `part.visible` slot.
pub const S_PART_VISIBLE: usize = 7;
/// `part.enabled` slot.
pub const S_PART_ENABLED: usize = 8;
/// `part.parent_part` slot.
pub const S_PART_PARENT: usize = 9;
/// `deformer.id` slot.
pub const S_DEFORMER_ID: usize = 11;
/// `deformer.binding_index` slot.
pub const S_DEFORMER_BINDING: usize = 12;
/// `deformer.visible` slot.
pub const S_DEFORMER_VISIBLE: usize = 13;
/// `deformer.enabled` slot.
pub const S_DEFORMER_ENABLED: usize = 14;
/// `deformer.parent_part` slot.
pub const S_DEFORMER_PARENT_PART: usize = 15;
/// `deformer.parent_deformer` slot.
pub const S_DEFORMER_PARENT_DEFORMER: usize = 16;
/// `deformer.type` slot.
pub const S_DEFORMER_TYPE: usize = 17;
/// `deformer.local_index` slot.
pub const S_DEFORMER_LOCAL_INDEX: usize = 18;
/// `warp.binding_index` slot.
pub const S_WARP_BINDING: usize = 19;
/// `warp.keyform_begin` slot.
pub const S_WARP_KEYFORM_BEGIN: usize = 20;
/// `warp.keyform_count` slot.
pub const S_WARP_KEYFORM_COUNT: usize = 21;
/// `warp.vertex_count` slot.
pub const S_WARP_VERTEX_COUNT: usize = 22;
/// `warp.rows` slot.
pub const S_WARP_ROWS: usize = 23;
/// `warp.columns` slot.
pub const S_WARP_COLUMNS: usize = 24;
/// `rotation.binding_index` slot.
pub const S_ROTATION_BINDING: usize = 25;
/// `rotation.keyform_begin` slot.
pub const S_ROTATION_KEYFORM_BEGIN: usize = 26;
/// `rotation.keyform_count` slot.
pub const S_ROTATION_KEYFORM_COUNT: usize = 27;
/// `rotation.base_angle` slot.
pub const S_ROTATION_BASE_ANGLE: usize = 28;
/// `art_mesh.id` slot.
pub const S_ART_MESH_ID: usize = 33;
/// `art_mesh.binding_index` slot.
pub const S_ART_MESH_BINDING: usize = 34;
/// `art_mesh.keyform_begin` slot.
pub const S_ART_MESH_KEYFORM_BEGIN: usize = 35;
/// `art_mesh.keyform_count` slot.
pub const S_ART_MESH_KEYFORM_COUNT: usize = 36;
/// `art_mesh.visible` slot.
pub const S_ART_MESH_VISIBLE: usize = 37;
/// `art_mesh.enabled` slot.
pub const S_ART_MESH_ENABLED: usize = 38;
/// `art_mesh.parent_part` slot.
pub const S_ART_MESH_PARENT_PART: usize = 39;
/// `art_mesh.parent_deformer` slot.
pub const S_ART_MESH_PARENT_DEFORMER: usize = 40;
/// `art_mesh.texture_number` slot.
pub const S_ART_MESH_TEXTURE: usize = 41;
/// `art_mesh.drawable_flags` slot.
pub const S_ART_MESH_FLAGS: usize = 42;
/// `art_mesh.vertex_count` slot.
pub const S_ART_MESH_VERTEX_COUNT: usize = 43;
/// `art_mesh.uv_begin` slot.
pub const S_ART_MESH_UV_BEGIN: usize = 44;
/// `art_mesh.index_begin` slot.
pub const S_ART_MESH_INDEX_BEGIN: usize = 45;
/// `art_mesh.index_count` slot.
pub const S_ART_MESH_INDEX_COUNT: usize = 46;
/// `art_mesh.mask_begin` slot.
pub const S_ART_MESH_MASK_BEGIN: usize = 47;
/// `art_mesh.mask_count` slot.
pub const S_ART_MESH_MASK_COUNT: usize = 48;
/// `parameter.id` slot.
pub const S_PARAMETER_ID: usize = 50;
/// `parameter.maximum` slot.
pub const S_PARAMETER_MAX: usize = 51;
/// `parameter.minimum` slot.
pub const S_PARAMETER_MIN: usize = 52;
/// `parameter.default` slot.
pub const S_PARAMETER_DEFAULT: usize = 53;
/// `parameter.repeat` slot.
pub const S_PARAMETER_REPEAT: usize = 54;
/// `parameter.decimal_places` slot.
pub const S_PARAMETER_DECIMALS: usize = 55;
/// `parameter.key_table_begin` slot.
pub const S_PARAMETER_KEY_TABLE_BEGIN: usize = 56;
/// `parameter.key_table_count` slot.
pub const S_PARAMETER_KEY_TABLE_COUNT: usize = 57;
/// `part_keyform.draw_order` slot.
pub const S_PART_KEYFORM_DRAW_ORDER: usize = 58;
/// `warp_keyform.opacity` slot.
pub const S_WARP_KEYFORM_OPACITY: usize = 59;
/// `warp_keyform.position_begin` slot.
pub const S_WARP_KEYFORM_POSITION_BEGIN: usize = 60;
/// `rotation_keyform.opacity` slot.
pub const S_ROTATION_KEYFORM_OPACITY: usize = 61;
/// `rotation_keyform.angle` slot.
pub const S_ROTATION_KEYFORM_ANGLE: usize = 62;
/// `rotation_keyform.origin_x` slot.
pub const S_ROTATION_KEYFORM_ORIGIN_X: usize = 63;
/// `rotation_keyform.origin_y` slot.
pub const S_ROTATION_KEYFORM_ORIGIN_Y: usize = 64;
/// `rotation_keyform.scale` slot.
pub const S_ROTATION_KEYFORM_SCALE: usize = 65;
/// `rotation_keyform.reflect_x` slot.
pub const S_ROTATION_KEYFORM_REFLECT_X: usize = 66;
/// `rotation_keyform.reflect_y` slot.
pub const S_ROTATION_KEYFORM_REFLECT_Y: usize = 67;
/// `art_mesh_keyform.opacity` slot.
pub const S_ART_MESH_KEYFORM_OPACITY: usize = 68;
/// `art_mesh_keyform.draw_order` slot.
pub const S_ART_MESH_KEYFORM_DRAW_ORDER: usize = 69;
/// `art_mesh_keyform.position_begin` slot.
pub const S_ART_MESH_KEYFORM_POSITION_BEGIN: usize = 70;
/// `keyform_position.xy` slot.
pub const S_KEYFORM_POSITION_XY: usize = 71;
/// `key_table_index` slot.
pub const S_KEY_TABLE_INDEX: usize = 72;
/// `binding.key_table_begin` slot.
pub const S_BINDING_KEY_TABLE_BEGIN: usize = 73;
/// `binding.key_table_count` slot.
pub const S_BINDING_KEY_TABLE_COUNT: usize = 74;
/// `key_table.begin` slot.
pub const S_KEY_TABLE_BEGIN: usize = 75;
/// `key_table.count` slot.
pub const S_KEY_TABLE_COUNT: usize = 76;
/// `key.values` slot.
pub const S_KEY_VALUES: usize = 77;
/// `uv.xy` slot.
pub const S_UV_XY: usize = 78;
/// `position_indices` slot.
pub const S_POSITION_INDICES: usize = 79;
/// `drawable_mask.indices` slot.
pub const S_DRAWABLE_MASK_INDICES: usize = 80;
/// `draw_group.object_begin` slot.
pub const S_DRAW_GROUP_OBJECT_BEGIN: usize = 81;
/// `draw_group.object_count` slot.
pub const S_DRAW_GROUP_OBJECT_COUNT: usize = 82;
/// `draw_group.object_total_count` slot.
pub const S_DRAW_GROUP_OBJECT_TOTAL: usize = 83;
/// `draw_group.max_order` slot.
pub const S_DRAW_GROUP_MAX_ORDER: usize = 84;
/// `draw_group.min_order` slot.
pub const S_DRAW_GROUP_MIN_ORDER: usize = 85;
/// `draw_item.type` slot.
pub const S_DRAW_ITEM_TYPE: usize = 86;
/// `draw_item.index` slot.
pub const S_DRAW_ITEM_INDEX: usize = 87;
/// `draw_item.self_group` slot.
pub const S_DRAW_ITEM_SELF_GROUP: usize = 88;
/// `glue.id` slot.
pub const S_GLUE_ID: usize = 90;
/// `glue.binding_index` slot.
pub const S_GLUE_BINDING: usize = 91;
/// `glue.keyform_begin` slot.
pub const S_GLUE_KEYFORM_BEGIN: usize = 92;
/// `glue.keyform_count` slot.
pub const S_GLUE_KEYFORM_COUNT: usize = 93;
/// `glue.art_mesh_a` slot.
pub const S_GLUE_ART_MESH_A: usize = 94;
/// `glue.art_mesh_b` slot.
pub const S_GLUE_ART_MESH_B: usize = 95;
/// `glue.info_begin` slot.
pub const S_GLUE_INFO_BEGIN: usize = 96;
/// `glue.info_count` slot.
pub const S_GLUE_INFO_COUNT: usize = 97;
/// `glue_info.weight` slot.
pub const S_GLUE_INFO_WEIGHT: usize = 98;
/// `glue_info.position_index` slot.
pub const S_GLUE_INFO_POSITION_INDEX: usize = 99;
/// `glue_keyform.intensity` slot.
pub const S_GLUE_KEYFORM_INTENSITY: usize = 100;
/// `warp.quad_transform` slot (3.3+).
pub const S_WARP_QUAD_TRANSFORM: usize = 101;
/// `parameter.type` slot (4.2+).
pub const S_PARAMETER_TYPE: usize = 114;
/// `parameter.blend_key_table_begin` slot (4.2+).
pub const S_PARAMETER_BLEND_BEGIN: usize = 115;
/// `parameter.blend_key_table_count` slot (4.2+).
pub const S_PARAMETER_BLEND_COUNT: usize = 116;
/// `parameter.extension_key_begin` slot (4.2+).
pub const S_PARAMETER_EXT_KEY_BEGIN: usize = 103;
/// `parameter.extension_key_count` slot (4.2+).
pub const S_PARAMETER_EXT_KEY_COUNT: usize = 104;
/// `warp.keyform_color_begin` slot (4.2+).
pub const S_WARP_KEYFORM_COLOR_BEGIN: usize = 105;
/// `rotation.keyform_color_begin` slot (4.2+).
pub const S_ROTATION_KEYFORM_COLOR_BEGIN: usize = 106;
/// `art_mesh.keyform_color_begin` slot (4.2+).
pub const S_ART_MESH_KEYFORM_COLOR_BEGIN: usize = 107;
/// `blend_key_table.begin` slot (4.2+).
pub const S_BLEND_KEY_TABLE_BEGIN: usize = 117;
/// `blend_key_table.count` slot (4.2+).
pub const S_BLEND_KEY_TABLE_COUNT: usize = 118;
/// `blend_key_table.base_key` slot (4.2+).
pub const S_BLEND_KEY_TABLE_BASE_KEY: usize = 119;
/// `blend_binding.key_table_index` slot (4.2+).
pub const S_BLEND_BINDING_KEY_TABLE_INDEX: usize = 120;
/// `blend_binding.keyform_begin` slot (4.2+).
pub const S_BLEND_BINDING_KEYFORM_BEGIN: usize = 121;
/// `blend_binding.keyform_count` slot (4.2+).
pub const S_BLEND_BINDING_KEYFORM_COUNT: usize = 122;
/// `blend_binding.constraint_begin` slot (4.2+).
pub const S_BLEND_BINDING_CONSTRAINT_BEGIN: usize = 123;
/// `blend_binding.constraint_count` slot (4.2+).
pub const S_BLEND_BINDING_CONSTRAINT_COUNT: usize = 124;
/// `blend_shape.warp_target` slot (4.2+).
pub const S_BS_WARP_TARGET: usize = 125;
/// `blend_shape.warp_binding_begin` slot (4.2+).
pub const S_BS_WARP_BINDING_BEGIN: usize = 126;
/// `blend_shape.warp_binding_count` slot (4.2+).
pub const S_BS_WARP_BINDING_COUNT: usize = 127;
/// `blend_shape.art_mesh_target` slot (4.2+).
pub const S_BS_ART_MESH_TARGET: usize = 128;
/// `blend_shape.art_mesh_binding_begin` slot (4.2+).
pub const S_BS_ART_MESH_BINDING_BEGIN: usize = 129;
/// `blend_shape.art_mesh_binding_count` slot (4.2+).
pub const S_BS_ART_MESH_BINDING_COUNT: usize = 130;
/// `blend_constraint_index` slot (4.2+).
pub const S_BS_CONSTRAINT_INDEX: usize = 131;
/// `blend_constraint.parameter` slot (4.2+).
pub const S_BS_CONSTRAINT_PARAMETER: usize = 132;
/// `blend_constraint.value_begin` slot (4.2+).
pub const S_BS_CONSTRAINT_VALUE_BEGIN: usize = 133;
/// `blend_constraint.value_count` slot (4.2+).
pub const S_BS_CONSTRAINT_VALUE_COUNT: usize = 134;
/// `blend_shape.part_target` slot (5.0+).
pub const S_BS_PART_TARGET: usize = 143;
/// `blend_shape.part_binding_begin` slot (5.0+).
pub const S_BS_PART_BINDING_BEGIN: usize = 144;
/// `blend_shape.part_binding_count` slot (5.0+).
pub const S_BS_PART_BINDING_COUNT: usize = 145;
/// `blend_shape.rotation_target` slot (5.0+).
pub const S_BS_ROTATION_TARGET: usize = 146;
/// `blend_shape.rotation_binding_begin` slot (5.0+).
pub const S_BS_ROTATION_BINDING_BEGIN: usize = 147;
/// `blend_shape.rotation_binding_count` slot (5.0+).
pub const S_BS_ROTATION_BINDING_COUNT: usize = 148;
/// `blend_shape.glue_target` slot (5.0+).
pub const S_BS_GLUE_TARGET: usize = 149;
/// `blend_shape.glue_binding_begin` slot (5.0+).
pub const S_BS_GLUE_BINDING_BEGIN: usize = 150;
/// `blend_shape.glue_binding_count` slot (5.0+).
pub const S_BS_GLUE_BINDING_COUNT: usize = 151;
/// `part.offscreen_index` slot (5.3+).
pub const S_PART_OFFSCREEN_INDEX: usize = 152;
/// `art_mesh.blend_mode` slot (5.3+).
pub const S_ART_MESH_BLEND_MODE: usize = 153;
/// `offscreen.owner_index` slot (5.3+).
pub const S_OFFSCREEN_OWNER_INDEX: usize = 155;
/// `offscreen.mask_begin` slot (5.3+).
pub const S_OFFSCREEN_MASK_BEGIN: usize = 158;
/// `offscreen.mask_count` slot (5.3+).
pub const S_OFFSCREEN_MASK_COUNT: usize = 159;
/// `blend_shape.offscreen_target` slot (5.3+).
pub const S_BS_OFFSCREEN_TARGET: usize = 164;
/// `blend_shape.offscreen_binding_begin` slot (5.3+).
pub const S_BS_OFFSCREEN_BINDING_BEGIN: usize = 165;
/// `blend_shape.offscreen_binding_count` slot (5.3+).
pub const S_BS_OFFSCREEN_BINDING_COUNT: usize = 166;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_count_matches_defs() {
        for byte in 1u8..=6 {
            // from_byte is Option-free for the supported range by construction
            if let Some(version) = MocVersion::from_byte(byte) {
                let defs = defs_for(version);
                let expected = SLOT_COUNT_BY_VERSION
                    .get((byte - 1) as usize)
                    .copied()
                    .unwrap_or(0);
                assert_eq!(
                    defs.len(),
                    expected,
                    "slot count mismatch for version {byte}"
                );
            }
        }
        assert_eq!(SLOT_DEFS.len(), 167, "canonical slot count");
    }

    #[test]
    fn slot_names_match_defs() {
        let expected: &[(usize, &str)] = &[
            (S_COUNT_INFO, "count_info"),
            (S_CANVAS_INFO, "canvas_info"),
            (S_PART_ID, "part.id"),
            (S_PART_BINDING, "part.binding_index"),
            (S_PART_KEYFORM_BEGIN, "part.keyform_begin"),
            (S_PART_KEYFORM_COUNT, "part.keyform_count"),
            (S_PART_VISIBLE, "part.visible"),
            (S_PART_ENABLED, "part.enabled"),
            (S_PART_PARENT, "part.parent_part"),
            (S_DEFORMER_ID, "deformer.id"),
            (S_DEFORMER_BINDING, "deformer.binding_index"),
            (S_DEFORMER_VISIBLE, "deformer.visible"),
            (S_DEFORMER_ENABLED, "deformer.enabled"),
            (S_DEFORMER_PARENT_PART, "deformer.parent_part"),
            (S_DEFORMER_PARENT_DEFORMER, "deformer.parent_deformer"),
            (S_DEFORMER_TYPE, "deformer.type"),
            (S_DEFORMER_LOCAL_INDEX, "deformer.local_index"),
            (S_WARP_BINDING, "warp.binding_index"),
            (S_WARP_KEYFORM_BEGIN, "warp.keyform_begin"),
            (S_WARP_KEYFORM_COUNT, "warp.keyform_count"),
            (S_WARP_VERTEX_COUNT, "warp.vertex_count"),
            (S_WARP_ROWS, "warp.rows"),
            (S_WARP_COLUMNS, "warp.columns"),
            (S_ROTATION_BINDING, "rotation.binding_index"),
            (S_ROTATION_KEYFORM_BEGIN, "rotation.keyform_begin"),
            (S_ROTATION_KEYFORM_COUNT, "rotation.keyform_count"),
            (S_ROTATION_BASE_ANGLE, "rotation.base_angle"),
            (S_ART_MESH_ID, "art_mesh.id"),
            (S_ART_MESH_BINDING, "art_mesh.binding_index"),
            (S_ART_MESH_KEYFORM_BEGIN, "art_mesh.keyform_begin"),
            (S_ART_MESH_KEYFORM_COUNT, "art_mesh.keyform_count"),
            (S_ART_MESH_VISIBLE, "art_mesh.visible"),
            (S_ART_MESH_ENABLED, "art_mesh.enabled"),
            (S_ART_MESH_PARENT_PART, "art_mesh.parent_part"),
            (S_ART_MESH_PARENT_DEFORMER, "art_mesh.parent_deformer"),
            (S_ART_MESH_TEXTURE, "art_mesh.texture_number"),
            (S_ART_MESH_FLAGS, "art_mesh.drawable_flags"),
            (S_ART_MESH_VERTEX_COUNT, "art_mesh.vertex_count"),
            (S_ART_MESH_UV_BEGIN, "art_mesh.uv_begin"),
            (S_ART_MESH_INDEX_BEGIN, "art_mesh.index_begin"),
            (S_ART_MESH_INDEX_COUNT, "art_mesh.index_count"),
            (S_ART_MESH_MASK_BEGIN, "art_mesh.mask_begin"),
            (S_ART_MESH_MASK_COUNT, "art_mesh.mask_count"),
            (S_PARAMETER_ID, "parameter.id"),
            (S_PARAMETER_MAX, "parameter.maximum"),
            (S_PARAMETER_MIN, "parameter.minimum"),
            (S_PARAMETER_DEFAULT, "parameter.default"),
            (S_PARAMETER_REPEAT, "parameter.repeat"),
            (S_PARAMETER_DECIMALS, "parameter.decimal_places"),
            (S_PARAMETER_KEY_TABLE_BEGIN, "parameter.key_table_begin"),
            (S_PARAMETER_KEY_TABLE_COUNT, "parameter.key_table_count"),
            (S_PART_KEYFORM_DRAW_ORDER, "part_keyform.draw_order"),
            (S_WARP_KEYFORM_OPACITY, "warp_keyform.opacity"),
            (S_WARP_KEYFORM_POSITION_BEGIN, "warp_keyform.position_begin"),
            (S_ROTATION_KEYFORM_OPACITY, "rotation_keyform.opacity"),
            (S_ROTATION_KEYFORM_ANGLE, "rotation_keyform.angle"),
            (S_ROTATION_KEYFORM_ORIGIN_X, "rotation_keyform.origin_x"),
            (S_ROTATION_KEYFORM_ORIGIN_Y, "rotation_keyform.origin_y"),
            (S_ROTATION_KEYFORM_SCALE, "rotation_keyform.scale"),
            (S_ROTATION_KEYFORM_REFLECT_X, "rotation_keyform.reflect_x"),
            (S_ROTATION_KEYFORM_REFLECT_Y, "rotation_keyform.reflect_y"),
            (S_ART_MESH_KEYFORM_OPACITY, "art_mesh_keyform.opacity"),
            (S_ART_MESH_KEYFORM_DRAW_ORDER, "art_mesh_keyform.draw_order"),
            (
                S_ART_MESH_KEYFORM_POSITION_BEGIN,
                "art_mesh_keyform.position_begin",
            ),
            (S_KEYFORM_POSITION_XY, "keyform_position.xy"),
            (S_KEY_TABLE_INDEX, "key_table_index"),
            (S_BINDING_KEY_TABLE_BEGIN, "binding.key_table_begin"),
            (S_BINDING_KEY_TABLE_COUNT, "binding.key_table_count"),
            (S_KEY_TABLE_BEGIN, "key_table.begin"),
            (S_KEY_TABLE_COUNT, "key_table.count"),
            (S_KEY_VALUES, "key.values"),
            (S_UV_XY, "uv.xy"),
            (S_POSITION_INDICES, "position_indices"),
            (S_DRAWABLE_MASK_INDICES, "drawable_mask.indices"),
            (S_DRAW_GROUP_OBJECT_BEGIN, "draw_group.object_begin"),
            (S_DRAW_GROUP_OBJECT_COUNT, "draw_group.object_count"),
            (S_DRAW_GROUP_OBJECT_TOTAL, "draw_group.object_total_count"),
            (S_DRAW_GROUP_MAX_ORDER, "draw_group.max_order"),
            (S_DRAW_GROUP_MIN_ORDER, "draw_group.min_order"),
            (S_DRAW_ITEM_TYPE, "draw_item.type"),
            (S_DRAW_ITEM_INDEX, "draw_item.index"),
            (S_DRAW_ITEM_SELF_GROUP, "draw_item.self_group"),
            (S_GLUE_ID, "glue.id"),
            (S_GLUE_BINDING, "glue.binding_index"),
            (S_GLUE_KEYFORM_BEGIN, "glue.keyform_begin"),
            (S_GLUE_KEYFORM_COUNT, "glue.keyform_count"),
            (S_GLUE_ART_MESH_A, "glue.art_mesh_a"),
            (S_GLUE_ART_MESH_B, "glue.art_mesh_b"),
            (S_GLUE_INFO_BEGIN, "glue.info_begin"),
            (S_GLUE_INFO_COUNT, "glue.info_count"),
            (S_GLUE_INFO_WEIGHT, "glue_info.weight"),
            (S_GLUE_INFO_POSITION_INDEX, "glue_info.position_index"),
            (S_GLUE_KEYFORM_INTENSITY, "glue_keyform.intensity"),
            (S_WARP_QUAD_TRANSFORM, "warp.quad_transform"),
            (S_PARAMETER_TYPE, "parameter.type"),
            (S_PARAMETER_BLEND_BEGIN, "parameter.blend_key_table_begin"),
            (S_PARAMETER_BLEND_COUNT, "parameter.blend_key_table_count"),
            (S_PART_OFFSCREEN_INDEX, "part.offscreen_index"),
            (S_ART_MESH_BLEND_MODE, "art_mesh.blend_mode"),
            (S_PARAMETER_EXT_KEY_BEGIN, "parameter.extension_key_begin"),
            (S_PARAMETER_EXT_KEY_COUNT, "parameter.extension_key_count"),
            (S_WARP_KEYFORM_COLOR_BEGIN, "warp.keyform_color_begin"),
            (
                S_ROTATION_KEYFORM_COLOR_BEGIN,
                "rotation.keyform_color_begin",
            ),
            (
                S_ART_MESH_KEYFORM_COLOR_BEGIN,
                "art_mesh.keyform_color_begin",
            ),
            (S_BLEND_KEY_TABLE_BEGIN, "blend_key_table.begin"),
            (S_BLEND_KEY_TABLE_COUNT, "blend_key_table.count"),
            (S_BLEND_KEY_TABLE_BASE_KEY, "blend_key_table.base_key"),
            (
                S_BLEND_BINDING_KEY_TABLE_INDEX,
                "blend_binding.key_table_index",
            ),
            (S_BLEND_BINDING_KEYFORM_BEGIN, "blend_binding.keyform_begin"),
            (S_BLEND_BINDING_KEYFORM_COUNT, "blend_binding.keyform_count"),
            (
                S_BLEND_BINDING_CONSTRAINT_BEGIN,
                "blend_binding.constraint_begin",
            ),
            (
                S_BLEND_BINDING_CONSTRAINT_COUNT,
                "blend_binding.constraint_count",
            ),
            (S_BS_WARP_TARGET, "blend_shape.warp_target"),
            (S_BS_WARP_BINDING_BEGIN, "blend_shape.warp_binding_begin"),
            (S_BS_WARP_BINDING_COUNT, "blend_shape.warp_binding_count"),
            (S_BS_ART_MESH_TARGET, "blend_shape.art_mesh_target"),
            (
                S_BS_ART_MESH_BINDING_BEGIN,
                "blend_shape.art_mesh_binding_begin",
            ),
            (
                S_BS_ART_MESH_BINDING_COUNT,
                "blend_shape.art_mesh_binding_count",
            ),
            (S_BS_CONSTRAINT_INDEX, "blend_constraint_index"),
            (S_BS_CONSTRAINT_PARAMETER, "blend_constraint.parameter"),
            (S_BS_CONSTRAINT_VALUE_BEGIN, "blend_constraint.value_begin"),
            (S_BS_CONSTRAINT_VALUE_COUNT, "blend_constraint.value_count"),
            (S_BS_PART_TARGET, "blend_shape.part_target"),
            (S_BS_PART_BINDING_BEGIN, "blend_shape.part_binding_begin"),
            (S_BS_PART_BINDING_COUNT, "blend_shape.part_binding_count"),
            (S_BS_ROTATION_TARGET, "blend_shape.rotation_target"),
            (
                S_BS_ROTATION_BINDING_BEGIN,
                "blend_shape.rotation_binding_begin",
            ),
            (
                S_BS_ROTATION_BINDING_COUNT,
                "blend_shape.rotation_binding_count",
            ),
            (S_BS_GLUE_TARGET, "blend_shape.glue_target"),
            (S_BS_GLUE_BINDING_BEGIN, "blend_shape.glue_binding_begin"),
            (S_BS_GLUE_BINDING_COUNT, "blend_shape.glue_binding_count"),
            (S_OFFSCREEN_OWNER_INDEX, "offscreen.owner_index"),
            (S_OFFSCREEN_MASK_BEGIN, "offscreen.mask_begin"),
            (S_OFFSCREEN_MASK_COUNT, "offscreen.mask_count"),
            (S_BS_OFFSCREEN_TARGET, "blend_shape.offscreen_target"),
            (
                S_BS_OFFSCREEN_BINDING_BEGIN,
                "blend_shape.offscreen_binding_begin",
            ),
            (
                S_BS_OFFSCREEN_BINDING_COUNT,
                "blend_shape.offscreen_binding_count",
            ),
        ];
        for (index, name) in expected {
            let def = SLOT_DEFS.get(*index);
            assert_eq!(
                def.map(|d| d.name),
                Some(*name),
                "slot {index} name mismatch"
            );
        }
    }
}
