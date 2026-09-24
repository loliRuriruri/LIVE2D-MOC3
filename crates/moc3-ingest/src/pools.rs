//! Bulk data pools extracted on demand for downstream stages (AGENT.2+).
//!
//! The AGENT.1 inspection path deliberately reads only ids, counts and
//! ranges; the Live2D IR needs the actual values. `parse_full` therefore
//! returns [`ParsedModel`], which pairs the same [`RawMoc3`] the inspector
//! uses with the flat pools below.
//!
//! Pools are flat, source-ordered arrays. Per-object slices are described by
//! the ranges already present in `RawMoc3` (for example
//! `art_mesh_keyform_position_begin` indexes into `keyform_positions` with
//! `2 * vertex_count` floats per keyform). Nothing here stores file offsets.

use serde::Serialize;

use crate::raw::RawMoc3;

/// Flat value pools read from the file after structural validation.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ModelPools {
    /// Key values of all key tables (`key.values`).
    pub key_values: Vec<f32>,
    /// Begin index (floats) of per-parameter extension keys (4.2+).
    pub parameter_extension_key_begin: Vec<i32>,
    /// Extension key count per parameter (4.2+).
    pub parameter_extension_key_count: Vec<i32>,
    /// UV floats, two per vertex (`uv.xy`).
    pub uvs: Vec<f32>,
    /// Triangle indices (`position_indices`).
    pub indices: Vec<u16>,
    /// Keyform vertex position floats, two per vertex (`keyform_position.xy`).
    pub keyform_positions: Vec<f32>,
    /// Draw order per part keyform (`part_keyform.draw_order`).
    pub part_keyform_draw_orders: Vec<f32>,
    /// Begin index into the warp keyform space per warp deformer.
    pub warp_keyform_begin: Vec<i32>,
    /// Keyform count per warp deformer.
    pub warp_keyform_count: Vec<i32>,
    /// Opacity per warp deformer keyform.
    pub warp_keyform_opacity: Vec<f32>,
    /// Begin index (floats) per warp deformer keyform position block.
    pub warp_keyform_position_begin: Vec<i32>,
    /// Begin index into the rotation keyform space per rotation deformer.
    pub rotation_keyform_begin: Vec<i32>,
    /// Keyform count per rotation deformer.
    pub rotation_keyform_count: Vec<i32>,
    /// Opacity per rotation deformer keyform.
    pub rotation_keyform_opacity: Vec<f32>,
    /// Angle per rotation deformer keyform.
    pub rotation_keyform_angle: Vec<f32>,
    /// Origin X per rotation deformer keyform.
    pub rotation_keyform_origin_x: Vec<f32>,
    /// Origin Y per rotation deformer keyform.
    pub rotation_keyform_origin_y: Vec<f32>,
    /// Scale per rotation deformer keyform.
    pub rotation_keyform_scale: Vec<f32>,
    /// Reflect-X flag per rotation deformer keyform.
    pub rotation_keyform_reflect_x: Vec<i32>,
    /// Reflect-Y flag per rotation deformer keyform.
    pub rotation_keyform_reflect_y: Vec<i32>,
    /// Opacity per art mesh keyform.
    pub art_mesh_keyform_opacity: Vec<f32>,
    /// Draw order per art mesh keyform.
    pub art_mesh_keyform_draw_order: Vec<f32>,
    /// Begin index (floats) per art mesh keyform position block.
    pub art_mesh_keyform_position_begin: Vec<i32>,
    /// Glue info weights.
    pub glue_info_weight: Vec<f32>,
    /// Glue info position indices.
    pub glue_info_position_index: Vec<u16>,
    /// Intensity per glue keyform.
    pub glue_keyform_intensity: Vec<f32>,
}

impl ModelPools {
    /// Total number of scalar elements across all pools.
    pub fn total_elements(&self) -> u64 {
        let mut total: u64 = 0;
        total = total.saturating_add(self.key_values.len() as u64);
        total = total.saturating_add(self.parameter_extension_key_begin.len() as u64);
        total = total.saturating_add(self.parameter_extension_key_count.len() as u64);
        total = total.saturating_add(self.uvs.len() as u64);
        total = total.saturating_add(self.indices.len() as u64);
        total = total.saturating_add(self.keyform_positions.len() as u64);
        total = total.saturating_add(self.part_keyform_draw_orders.len() as u64);
        total = total.saturating_add(self.warp_keyform_begin.len() as u64);
        total = total.saturating_add(self.warp_keyform_count.len() as u64);
        total = total.saturating_add(self.warp_keyform_opacity.len() as u64);
        total = total.saturating_add(self.warp_keyform_position_begin.len() as u64);
        total = total.saturating_add(self.rotation_keyform_begin.len() as u64);
        total = total.saturating_add(self.rotation_keyform_count.len() as u64);
        total = total.saturating_add(self.rotation_keyform_opacity.len() as u64);
        total = total.saturating_add(self.rotation_keyform_angle.len() as u64);
        total = total.saturating_add(self.rotation_keyform_origin_x.len() as u64);
        total = total.saturating_add(self.rotation_keyform_origin_y.len() as u64);
        total = total.saturating_add(self.rotation_keyform_scale.len() as u64);
        total = total.saturating_add(self.rotation_keyform_reflect_x.len() as u64);
        total = total.saturating_add(self.rotation_keyform_reflect_y.len() as u64);
        total = total.saturating_add(self.art_mesh_keyform_opacity.len() as u64);
        total = total.saturating_add(self.art_mesh_keyform_draw_order.len() as u64);
        total = total.saturating_add(self.art_mesh_keyform_position_begin.len() as u64);
        total = total.saturating_add(self.glue_info_weight.len() as u64);
        total = total.saturating_add(self.glue_info_position_index.len() as u64);
        total = total.saturating_add(self.glue_keyform_intensity.len() as u64);
        total
    }

    /// True when no pool element was extracted.
    pub fn is_empty(&self) -> bool {
        self.total_elements() == 0
    }
}

/// Parser output for downstream stages: raw structure plus bulk pools.
#[derive(Debug, Clone, Serialize)]
pub struct ParsedModel {
    /// The same raw model the inspection report is built from.
    pub raw: RawMoc3,
    /// Bulk value pools referenced by the raw model's ranges.
    pub pools: ModelPools,
}
