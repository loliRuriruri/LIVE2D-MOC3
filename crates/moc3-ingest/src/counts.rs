//! The MOC3 count info table.
//!
//! The table lives at offset-table slot 0. Versions 1-4 store 32 `i32`
//! values (128 bytes); versions 5+ store 64 `i32` values (256 bytes).
//! Field order follows the documented layout; unknown or padding entries are
//! preserved in `unknown_nonzero` instead of being dropped.

use serde::Serialize;

use crate::error::{ErrorKind, Moc3Error, Moc3Result};
use crate::reader::ByteView;
use crate::version::MocVersion;

/// All count fields of a MOC3 file.
#[derive(Debug, Clone, Serialize)]
pub struct CountInfo {
    /// Number of parts.
    pub parts: u32,
    /// Number of deformers (warp + rotation).
    pub deformers: u32,
    /// Number of warp deformers.
    pub warp_deformers: u32,
    /// Number of rotation deformers.
    pub rotation_deformers: u32,
    /// Number of art meshes (drawables).
    pub art_meshes: u32,
    /// Number of parameters.
    pub parameters: u32,
    /// Number of part keyforms.
    pub part_keyforms: u32,
    /// Number of warp deformer keyforms.
    pub warp_deformer_keyforms: u32,
    /// Number of rotation deformer keyforms.
    pub rotation_deformer_keyforms: u32,
    /// Number of art mesh keyforms.
    pub art_mesh_keyforms: u32,
    /// Total keyform position floats (x, y pairs).
    pub keyform_positions: u32,
    /// Total parameter binding indices.
    pub parameter_binding_indices: u32,
    /// Number of keyform bindings.
    pub keyform_bindings: u32,
    /// Number of parameter bindings (key tables used by bindings).
    pub parameter_bindings: u32,
    /// Total key values in all key tables.
    pub keys: u32,
    /// Total UV floats.
    pub uvs: u32,
    /// Total triangle index entries.
    pub position_indices: u32,
    /// Total drawable mask entries.
    pub drawable_masks: u32,
    /// Number of draw order groups.
    pub draw_order_groups: u32,
    /// Number of draw order group objects.
    pub draw_order_group_objects: u32,
    /// Number of glue objects.
    pub glue: u32,
    /// Total glue info entries.
    pub glue_info: u32,
    /// Number of glue keyforms.
    pub glue_keyforms: u32,
    /// Number of multiply color keyform entries (4.2+).
    pub keyform_multiply_colors: u32,
    /// Number of screen color keyform entries (4.2+).
    pub keyform_screen_colors: u32,
    /// Number of blend shape parameter bindings (4.2+).
    pub blend_shape_parameter_bindings: u32,
    /// Number of blend shape keyform bindings (4.2+).
    pub blend_shape_keyform_bindings: u32,
    /// Number of blend shape warp targets (4.2+).
    pub blend_shapes_warp_deformers: u32,
    /// Number of blend shape art mesh targets (4.2+).
    pub blend_shapes_art_meshes: u32,
    /// Number of blend shape constraint indices (4.2+).
    pub blend_shape_constraint_indices: u32,
    /// Number of blend shape constraints (4.2+).
    pub blend_shape_constraints: u32,
    /// Number of blend shape constraint values (4.2+).
    pub blend_shape_constraint_values: u32,
    /// Number of blend shape part targets (5.0+).
    pub blend_shapes_parts: u32,
    /// Number of blend shape rotation targets (5.0+).
    pub blend_shapes_rotation_deformers: u32,
    /// Number of blend shape glue targets (5.0+).
    pub blend_shapes_glue: u32,
    /// Number of offscreen rendering surfaces (5.3+).
    pub offscreens: u32,
    /// Number of offscreen keyforms (5.3+).
    pub offscreen_keyforms: u32,
    /// Number of blend shape offscreen targets (5.3+).
    pub blend_shapes_offscreens: u32,
    /// Non-zero values found beyond the known fields, as `(index, value)`.
    pub unknown_nonzero: Vec<(usize, i32)>,
    /// Number of `i32` entries actually stored on disk (32 or 64).
    pub raw_int_count: usize,
}

/// Read and structurally validate the count info table.
pub fn parse_count_info(
    view: &ByteView<'_>,
    version: MocVersion,
    offset: u64,
    file_len: u64,
) -> Moc3Result<CountInfo> {
    let ints = version.count_info_ints();
    let bytes = (ints as u64) * 4;
    if offset % 4 != 0 {
        return Err(Moc3Error::in_context(
            ErrorKind::MisalignedSection {
                section: "count_info",
                offset,
                alignment: 4,
            },
            offset,
            "count_info",
        ));
    }
    let end = offset.checked_add(bytes).ok_or_else(|| {
        Moc3Error::in_context(
            ErrorKind::Internal {
                what: "count_info range overflow",
            },
            offset,
            "count_info",
        )
    })?;
    if end > file_len {
        return Err(Moc3Error::in_context(
            ErrorKind::SectionOutOfBounds {
                section: "count_info",
                offset,
                size: bytes,
                file_len,
            },
            offset,
            "count_info",
        ));
    }

    let raw = view.i32_vec(offset, ints as u64, "count_info")?;
    let mut unknown_nonzero = Vec::new();
    for (index, value) in raw.iter().enumerate() {
        let known = index < 38;
        if *value < 0 {
            let field = known_count_name(index).unwrap_or("count_info.unknown");
            return Err(Moc3Error::in_context(
                ErrorKind::InvalidCount {
                    field,
                    value: *value as i64,
                },
                offset + (index as u64) * 4,
                "count_info",
            ));
        }
        if !known && *value != 0 {
            unknown_nonzero.push((index, *value));
        }
    }

    let get = |index: usize| -> u32 {
        raw.get(index)
            .copied()
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0)
    };

    let counts = CountInfo {
        parts: get(0),
        deformers: get(1),
        warp_deformers: get(2),
        rotation_deformers: get(3),
        art_meshes: get(4),
        parameters: get(5),
        part_keyforms: get(6),
        warp_deformer_keyforms: get(7),
        rotation_deformer_keyforms: get(8),
        art_mesh_keyforms: get(9),
        keyform_positions: get(10),
        parameter_binding_indices: get(11),
        keyform_bindings: get(12),
        parameter_bindings: get(13),
        keys: get(14),
        uvs: get(15),
        position_indices: get(16),
        drawable_masks: get(17),
        draw_order_groups: get(18),
        draw_order_group_objects: get(19),
        glue: get(20),
        glue_info: get(21),
        glue_keyforms: get(22),
        keyform_multiply_colors: get(23),
        keyform_screen_colors: get(24),
        blend_shape_parameter_bindings: get(25),
        blend_shape_keyform_bindings: get(26),
        blend_shapes_warp_deformers: get(27),
        blend_shapes_art_meshes: get(28),
        blend_shape_constraint_indices: get(29),
        blend_shape_constraints: get(30),
        blend_shape_constraint_values: get(31),
        blend_shapes_parts: get(32),
        blend_shapes_rotation_deformers: get(33),
        blend_shapes_glue: get(34),
        offscreens: get(35),
        offscreen_keyforms: get(36),
        blend_shapes_offscreens: get(37),
        unknown_nonzero,
        raw_int_count: ints,
    };

    if counts.warp_deformers as u64 + counts.rotation_deformers as u64 != counts.deformers as u64 {
        return Err(Moc3Error::in_context(
            ErrorKind::DeformerCountMismatch {
                warps: counts.warp_deformers as i64,
                rotations: counts.rotation_deformers as i64,
                deformers: counts.deformers as i64,
            },
            offset,
            "count_info",
        ));
    }

    Ok(counts)
}

/// Canonical name of a known count field index.
pub fn known_count_name(index: usize) -> Option<&'static str> {
    const NAMES: [&str; 38] = [
        "parts",
        "deformers",
        "warp_deformers",
        "rotation_deformers",
        "art_meshes",
        "parameters",
        "part_keyforms",
        "warp_deformer_keyforms",
        "rotation_deformer_keyforms",
        "art_mesh_keyforms",
        "keyform_positions",
        "parameter_binding_indices",
        "keyform_bindings",
        "parameter_bindings",
        "keys",
        "uvs",
        "position_indices",
        "drawable_masks",
        "draw_order_groups",
        "draw_order_group_objects",
        "glue",
        "glue_info",
        "glue_keyforms",
        "keyform_multiply_colors",
        "keyform_screen_colors",
        "blend_shape_parameter_bindings",
        "blend_shape_keyform_bindings",
        "blend_shapes_warp_deformers",
        "blend_shapes_art_meshes",
        "blend_shape_constraint_indices",
        "blend_shape_constraints",
        "blend_shape_constraint_values",
        "blend_shapes_parts",
        "blend_shapes_rotation_deformers",
        "blend_shapes_glue",
        "offscreens",
        "offscreen_keyforms",
        "blend_shapes_offscreens",
    ];
    NAMES.get(index).copied()
}
