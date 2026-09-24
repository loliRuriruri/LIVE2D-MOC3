//! Safety limits applied before any allocation or loop.
//!
//! Malformed or hostile files can declare huge counts. Every count read from
//! the file is checked against these limits first; only then is memory
//! reserved (with `try_reserve`, so allocation failure becomes a structured
//! error instead of an abort). Defaults are deliberately generous for real
//! models while keeping worst-case memory bounded.

use crate::error::{ErrorKind, Moc3Error, Moc3Result};

/// Configurable parser safety limits.
#[derive(Debug, Clone)]
pub struct Limits {
    /// Maximum accepted input file size in bytes (default 512 MiB).
    pub max_file_size: u64,
    /// Maximum number of parts.
    pub max_parts: u64,
    /// Maximum number of deformers (warp + rotation).
    pub max_deformers: u64,
    /// Maximum number of art meshes.
    pub max_art_meshes: u64,
    /// Maximum number of parameters.
    pub max_parameters: u64,
    /// Maximum number of glue objects.
    pub max_glues: u64,
    /// Maximum number of keyforms per keyform category.
    pub max_keyforms_per_category: u64,
    /// Maximum total number of keyform position floats.
    pub max_keyform_position_floats: u64,
    /// Maximum number of parameter binding indices.
    pub max_binding_indices: u64,
    /// Maximum number of keyform bindings.
    pub max_bindings: u64,
    /// Maximum number of key tables.
    pub max_key_tables: u64,
    /// Maximum number of key values.
    pub max_keys: u64,
    /// Maximum number of UV floats.
    pub max_uv_floats: u64,
    /// Maximum number of triangle indices.
    pub max_position_indices: u64,
    /// Maximum number of drawable mask indices.
    pub max_masks: u64,
    /// Maximum number of draw order groups.
    pub max_draw_order_groups: u64,
    /// Maximum number of draw order group objects.
    pub max_draw_order_items: u64,
    /// Maximum number of glue info entries.
    pub max_glue_info: u64,
    /// Maximum number of keyform color entries.
    pub max_keyform_colors: u64,
    /// Maximum number of blend shape entries (per blend shape category).
    pub max_blend_shape_entries: u64,
    /// Maximum number of offscreen rendering surfaces (5.3+).
    pub max_offscreens: u64,
    /// Aggregate element budget across all arrays.
    pub max_total_elements: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_size: 512 * 1024 * 1024,
            max_parts: 100_000,
            max_deformers: 200_000,
            max_art_meshes: 200_000,
            max_parameters: 20_000,
            max_glues: 100_000,
            max_keyforms_per_category: 5_000_000,
            max_keyform_position_floats: 50_000_000,
            max_binding_indices: 5_000_000,
            max_bindings: 5_000_000,
            max_key_tables: 1_000_000,
            max_keys: 20_000_000,
            max_uv_floats: 50_000_000,
            max_position_indices: 50_000_000,
            max_masks: 5_000_000,
            max_draw_order_groups: 100_000,
            max_draw_order_items: 2_000_000,
            max_glue_info: 20_000_000,
            max_keyform_colors: 5_000_000,
            max_blend_shape_entries: 2_000_000,
            max_offscreens: 10_000,
            max_total_elements: 120_000_000,
        }
    }
}

impl Limits {
    /// Check one count against a limit.
    pub fn check_count(&self, field: &'static str, count: u64, limit: u64) -> Moc3Result<()> {
        if count > limit {
            return Err(Moc3Error::new(ErrorKind::LimitExceeded {
                field,
                count,
                limit,
            }));
        }
        Ok(())
    }

    /// Check the input file size against the configured maximum.
    pub fn check_file_size(&self, size: u64) -> Moc3Result<()> {
        self.check_count("file.size", size, self.max_file_size)
    }
}
