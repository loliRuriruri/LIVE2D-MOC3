//! The MOC3 parse pipeline.
//!
//! Order of operations (mirrors the structural verification used by
//! independent clean-room loaders):
//!
//! 1. header (magic, version, endianness, reserved padding)
//! 2. section offset table (bounds, then per-section alignment/bounds/order)
//! 3. count info table (non-negative counts, limits, deformer consistency)
//! 4. section content + cross-reference validation
//!
//! Nothing is read before it is bounds checked, and nothing is allocated
//! before its count is checked against [`Limits`].

use std::collections::BTreeMap;

use crate::counts::{parse_count_info, CountInfo};
use crate::error::{ErrorKind, Moc3Error, Moc3Result};
use crate::limits::Limits;
use crate::raw::*;
use crate::reader::{ByteView, IdField};
use crate::table::{self, Elem, SlotDef};
use crate::version::{ByteOrder, MocVersion};

/// Fixed header length on disk.
pub const HEADER_LEN: u64 = 64;
/// Section offset table base offset on disk.
pub const TABLE_BASE: u64 = 0x40;
/// Maximum number of key tables a single binding may reference.
pub const MAX_KEY_TABLES_PER_BINDING: i64 = 20;

/// One validated section.
#[derive(Debug, Clone, Copy)]
struct Slot {
    offset: u64,
    count: u64,
}

struct Sections {
    slots: Vec<Option<Slot>>,
}

impl Sections {
    fn slot(&self, index: usize, what: &'static str) -> Moc3Result<Slot> {
        self.slots
            .get(index)
            .and_then(|entry| *entry)
            .ok_or_else(|| Moc3Error::new(ErrorKind::Internal { what }))
    }

    fn i32_arr(&self, view: &ByteView<'_>, index: usize) -> Moc3Result<Vec<i32>> {
        let slot = self.slot(index, "i32 section missing")?;
        if slot.count == 0 {
            return Ok(Vec::new());
        }
        view.i32_vec(slot.offset, slot.count, section_name(index))
    }

    fn f32_arr(&self, view: &ByteView<'_>, index: usize) -> Moc3Result<Vec<f32>> {
        let slot = self.slot(index, "f32 section missing")?;
        if slot.count == 0 {
            return Ok(Vec::new());
        }
        view.f32_vec(slot.offset, slot.count, section_name(index))
    }

    fn u8_arr(&self, view: &ByteView<'_>, index: usize) -> Moc3Result<Vec<u8>> {
        let slot = self.slot(index, "u8 section missing")?;
        if slot.count == 0 {
            return Ok(Vec::new());
        }
        view.u8_vec(slot.offset, slot.count, section_name(index))
    }

    fn u16_arr(&self, view: &ByteView<'_>, index: usize) -> Moc3Result<Vec<u16>> {
        let slot = self.slot(index, "u16 section missing")?;
        if slot.count == 0 {
            return Ok(Vec::new());
        }
        view.u16_vec(slot.offset, slot.count, section_name(index))
    }

    fn ids(&self, view: &ByteView<'_>, index: usize) -> Moc3Result<Vec<IdField>> {
        let slot = self.slot(index, "id section missing")?;
        if slot.count == 0 {
            return Ok(Vec::new());
        }
        view.ids_at(slot.offset, slot.count, section_name(index))
    }
}

fn section_name(index: usize) -> &'static str {
    table::SLOT_DEFS
        .get(index)
        .map(|def| def.name)
        .unwrap_or("unknown_section")
}

fn pick<T: Clone>(arr: &[T], index: usize, what: &'static str) -> Moc3Result<T> {
    arr.get(index)
        .cloned()
        .ok_or_else(|| Moc3Error::new(ErrorKind::Internal { what }))
}

fn vec_with_capacity<T>(count: usize, field: &'static str) -> Moc3Result<Vec<T>> {
    let mut out: Vec<T> = Vec::new();
    out.try_reserve_exact(count).map_err(|_| {
        Moc3Error::new(ErrorKind::AllocationFailed {
            field,
            count: count as u64,
        })
    })?;
    Ok(out)
}

fn check_index(value: i64, bound: u64, field: &'static str, index: usize) -> Moc3Result<()> {
    if value < 0 || value as u64 >= bound {
        return Err(Moc3Error::new(ErrorKind::InvalidReference {
            field,
            index,
            value,
            bound,
        }));
    }
    Ok(())
}

fn check_index_or_neg1(
    value: i64,
    bound: u64,
    field: &'static str,
    index: usize,
) -> Moc3Result<Option<u32>> {
    if value < -1 || (value >= 0 && value as u64 >= bound) {
        return Err(Moc3Error::new(ErrorKind::InvalidReference {
            field,
            index,
            value,
            bound,
        }));
    }
    if value < 0 {
        Ok(None)
    } else {
        u32::try_from(value)
            .map(Some)
            .map_err(|_| Moc3Error::new(ErrorKind::Internal { what: field }))
    }
}

fn check_range(
    begin: i64,
    count: i64,
    total: u64,
    field: &'static str,
    index: usize,
) -> Moc3Result<()> {
    let fits = count == 0 || (begin >= 0 && (begin as u64).saturating_add(count as u64) <= total);
    if count < 0 || !fits {
        return Err(Moc3Error::new(ErrorKind::InvalidRange {
            field,
            index,
            begin,
            count,
            total,
        }));
    }
    Ok(())
}

/// Parse a whole MOC3 file.
///
/// The input slice is never modified and no state is shared between calls, so
/// repeated parses of the same bytes always produce the same result.
pub fn parse(data: &[u8], limits: &Limits) -> Moc3Result<RawMoc3> {
    let file_len = data.len() as u64;
    limits.check_file_size(file_len)?;

    if file_len < HEADER_LEN {
        return Err(Moc3Error::at(
            ErrorKind::InvalidHeader {
                reason: "file is shorter than the 64-byte header",
            },
            0,
        ));
    }

    let probe = ByteView::new(data, ByteOrder::Little);
    let magic = probe.slice(0, 4, "header.magic")?;
    if magic != b"MOC3" {
        return Err(Moc3Error::at(
            ErrorKind::InvalidHeader {
                reason: "magic bytes are not 'MOC3'",
            },
            0,
        ));
    }
    let version_byte = probe.u8_at(4, "header.version")?;
    let version = MocVersion::from_byte(version_byte).ok_or_else(|| {
        Moc3Error::at(
            ErrorKind::UnsupportedVersion {
                version: version_byte,
                max_supported: MocVersion::MAX_SUPPORTED,
            },
            4,
        )
    })?;
    let endian_flag = probe.u8_at(5, "header.endian")?;
    let byte_order = match endian_flag {
        0 => ByteOrder::Little,
        1 => ByteOrder::Big,
        _ => {
            return Err(Moc3Error::at(
                ErrorKind::InvalidHeader {
                    reason: "endian flag is neither 0 nor 1",
                },
                5,
            ))
        }
    };
    let view = ByteView::new(data, byte_order);

    let mut anomalies: Vec<Anomaly> = Vec::new();
    let mut header_padding_nonzero = Vec::new();
    for offset in 6..HEADER_LEN {
        let value = view.u8_at(offset, "header.padding")?;
        if value != 0 {
            header_padding_nonzero.push(UnknownByte { offset, value });
        }
    }
    if !header_padding_nonzero.is_empty() {
        let first = header_padding_nonzero
            .first()
            .map(|byte| byte.offset)
            .unwrap_or(6);
        anomalies.push(Anomaly {
            code: "header_padding_nonzero",
            message: format!(
                "{} non-zero byte(s) in reserved header padding starting at offset {first}",
                header_padding_nonzero.len()
            ),
            offset: Some(first),
            index: None,
        });
    }

    // --- section offset table ---------------------------------------------
    let offset_slots = version.offset_slots();
    let table_bytes = (offset_slots as u64) * 4;
    let table_end = TABLE_BASE + table_bytes;
    if file_len < table_end {
        return Err(Moc3Error::at(
            ErrorKind::UnexpectedEof {
                offset: TABLE_BASE,
                needed: table_bytes,
                available: file_len.saturating_sub(TABLE_BASE),
            },
            TABLE_BASE,
        ));
    }
    let offsets = view.u32_vec(TABLE_BASE, offset_slots as u64, "section offset table")?;
    for (slot, value) in offsets.iter().enumerate() {
        if *value as u64 > file_len {
            return Err(Moc3Error::at(
                ErrorKind::SectionTableOutOfRange {
                    slot,
                    value: *value,
                    file_len,
                },
                TABLE_BASE + (slot as u64) * 4,
            ));
        }
    }

    // --- count info --------------------------------------------------------
    let count_info_offset = offsets.first().copied().unwrap_or(0) as u64;
    if count_info_offset < table_end {
        return Err(Moc3Error::at(
            ErrorKind::InvalidValue {
                field: "count_info",
                index: 0,
                value: count_info_offset as i64,
                reason: "count info overlaps the section offset table",
            },
            0,
        ));
    }
    let counts = parse_count_info(&view, version, count_info_offset, file_len)?;
    enforce_limits(limits, &counts)?;

    // --- walk the slot table ----------------------------------------------
    let defs = table::defs_for(version);
    let mut slots: Vec<Option<Slot>> = vec![None; table::SLOT_DEFS.len()];
    let mut runtime_sections: Vec<RuntimeSection> = Vec::new();
    let mut prev_end = table_end;

    for (index, def) in defs.iter().enumerate() {
        let raw_offset = offsets.get(index).copied().unwrap_or(0) as u64;
        let count = table::count_for(def.count, &counts);
        let size = match def.elem {
            Elem::CountInfo => (version.count_info_ints() as u64) * 4,
            other => other.size(version).saturating_mul(count),
        };
        if raw_offset % 8 != 0 {
            return Err(Moc3Error::at(
                ErrorKind::MisalignedSection {
                    section: def.name,
                    offset: raw_offset,
                    alignment: 8,
                },
                raw_offset,
            ));
        }
        let end = raw_offset.checked_add(size).ok_or_else(|| {
            Moc3Error::at(
                ErrorKind::Internal {
                    what: "section range overflow",
                },
                raw_offset,
            )
        })?;
        if end > file_len {
            return Err(Moc3Error::at(
                ErrorKind::SectionOutOfBounds {
                    section: def.name,
                    offset: raw_offset,
                    size,
                    file_len,
                },
                raw_offset,
            ));
        }
        let is_static = matches!(def.elem, Elem::CountInfo | Elem::Canvas);
        if is_static {
            if raw_offset < table_end {
                return Err(Moc3Error::at(
                    ErrorKind::InvalidValue {
                        field: def.name,
                        index: 0,
                        value: raw_offset as i64,
                        reason: "section data overlaps the offset table",
                    },
                    raw_offset,
                ));
            }
            if end > prev_end {
                prev_end = end;
            }
        } else {
            if raw_offset < prev_end {
                return Err(Moc3Error::at(
                    ErrorKind::SectionsNotMonotonic {
                        section: def.name,
                        offset: raw_offset,
                        previous_end: prev_end,
                    },
                    raw_offset,
                ));
            }
            prev_end = end;
        }

        if def.runtime {
            runtime_sections.push(RuntimeSection {
                name: def.name,
                offset: raw_offset as u32,
                elements: count,
                byte_size: size,
            });
        }

        if let Some(entry) = slots.get_mut(index) {
            *entry = Some(Slot {
                offset: raw_offset,
                count,
            });
        }
    }

    let mut unknown_slots: Vec<UnknownSlot> = Vec::new();
    for index in defs.len()..offset_slots {
        if let Some(value) = offsets.get(index) {
            if *value != 0 {
                unknown_slots.push(UnknownSlot {
                    index,
                    value: *value,
                });
            }
        }
    }
    if !unknown_slots.is_empty() {
        anomalies.push(Anomaly {
            code: "unknown_offset_slot",
            message: format!(
                "{} unused offset table slot(s) hold non-zero values",
                unknown_slots.len()
            ),
            offset: None,
            index: unknown_slots.first().map(|slot| slot.index),
        });
    }
    for (index, value) in &counts.unknown_nonzero {
        anomalies.push(Anomaly {
            code: "unknown_count_nonzero",
            message: format!("unknown count field[{index}] = {value}"),
            offset: Some(count_info_offset + (*index as u64) * 4),
            index: Some(*index),
        });
    }

    let sections = Sections { slots };

    // --- canvas ------------------------------------------------------------
    let canvas_slot = sections.slot(table::S_CANVAS_INFO, "canvas slot missing")?;
    let flags_raw = view.u8_at(canvas_slot.offset + 20, "canvas.flags")?;
    let canvas = RawCanvas {
        pixels_per_unit: view.f32_at(canvas_slot.offset, "canvas.pixels_per_unit")?,
        origin_x: view.f32_at(canvas_slot.offset + 4, "canvas.origin_x")?,
        origin_y: view.f32_at(canvas_slot.offset + 8, "canvas.origin_y")?,
        width: view.f32_at(canvas_slot.offset + 12, "canvas.width")?,
        height: view.f32_at(canvas_slot.offset + 16, "canvas.height")?,
        flags_raw,
        flag_y_reversed: flags_raw & 0x01 != 0,
    };
    if canvas.flags_raw & !0x01 != 0 {
        anomalies.push(Anomaly {
            code: "unknown_canvas_flag_bits",
            message: format!(
                "canvas flag byte has unknown bits set: 0x{:02x}",
                canvas.flags_raw
            ),
            offset: Some(canvas_slot.offset + 20),
            index: None,
        });
    }

    // --- section content ---------------------------------------------------
    let key_table_index = sections.i32_arr(&view, table::S_KEY_TABLE_INDEX)?;
    let key_table_begin = sections.i32_arr(&view, table::S_KEY_TABLE_BEGIN)?;
    let key_table_count = sections.i32_arr(&view, table::S_KEY_TABLE_COUNT)?;

    let key_tables = read_key_tables(
        &key_table_begin,
        &key_table_count,
        counts.keys as u64,
        counts.parameter_bindings,
    )?;

    // Every entry of the global key table index array must reference a key table.
    for (index, value) in key_table_index.iter().enumerate() {
        check_index(
            i64::from(*value),
            counts.parameter_bindings as u64,
            "key_table_index",
            index,
        )?;
    }

    let bindings = read_bindings(&sections, &view, &counts, &key_table_index)?;

    let parameters = read_parameters(&sections, &view, version, &counts, &mut anomalies)?;
    let parts = read_parts(&sections, &view, version, &counts, &mut anomalies)?;
    let (deformers, warp_keyforms, rotation_keyforms) =
        read_deformers(&sections, &view, version, &counts, &mut anomalies)?;
    let art_meshes = read_art_meshes(&sections, &view, version, &counts, &mut anomalies)?;
    let masks = read_masks(&sections, &view, &counts)?;
    let draw_order_groups = read_draw_order_groups(&sections, &view, &counts)?;
    let draw_order_items = read_draw_order_items(&sections, &view, &counts)?;
    let glues = read_glues(&sections, &view, &counts)?;

    check_keyform_spans(
        "part",
        &parts
            .iter()
            .map(|p| (p.binding_index, p.keyform_begin, p.keyform_count))
            .collect::<Vec<_>>(),
        &bindings,
        &key_tables,
        counts.part_keyforms as u64,
    )?;
    check_keyform_spans(
        "warp",
        &warp_keyforms,
        &bindings,
        &key_tables,
        counts.warp_deformer_keyforms as u64,
    )?;
    check_keyform_spans(
        "rotation",
        &rotation_keyforms,
        &bindings,
        &key_tables,
        counts.rotation_deformer_keyforms as u64,
    )?;
    check_keyform_spans(
        "art_mesh",
        &art_meshes
            .iter()
            .map(|m| (m.binding_index, m.keyform_begin, m.keyform_count))
            .collect::<Vec<_>>(),
        &bindings,
        &key_tables,
        counts.art_mesh_keyforms as u64,
    )?;
    check_keyform_spans(
        "glue",
        &glues
            .iter()
            .map(|g| (g.binding_index, g.keyform_begin, g.keyform_count))
            .collect::<Vec<_>>(),
        &bindings,
        &key_tables,
        counts.glue_keyforms as u64,
    )?;

    validate_blend_shape_sections(&sections, &view, version, &counts)?;

    collect_id_anomalies(
        &mut anomalies,
        "parameter",
        &parameters.iter().map(|p| &p.id),
    );
    collect_id_anomalies(&mut anomalies, "part", &parts.iter().map(|p| &p.id));
    collect_id_anomalies(&mut anomalies, "deformer", &deformers.iter().map(|d| &d.id));
    collect_id_anomalies(
        &mut anomalies,
        "art_mesh",
        &art_meshes.iter().map(|m| &m.id),
    );
    collect_id_anomalies(&mut anomalies, "glue", &glues.iter().map(|g| &g.id));

    let not_extracted = build_not_extracted(&counts);
    let header = RawHeader {
        version_byte,
        version_label: version.label(),
        byte_order,
        header_padding_nonzero,
    };

    Ok(RawMoc3 {
        header,
        canvas,
        counts,
        parameters,
        parts,
        deformers,
        art_meshes,
        masks,
        draw_order_groups,
        draw_order_items,
        glues,
        bindings,
        key_tables,
        runtime_sections,
        unknown_slots,
        anomalies,
        not_extracted,
    })
}

fn enforce_limits(limits: &Limits, counts: &CountInfo) -> Moc3Result<()> {
    limits.check_count("parts", counts.parts as u64, limits.max_parts)?;
    limits.check_count("deformers", counts.deformers as u64, limits.max_deformers)?;
    limits.check_count(
        "art_meshes",
        counts.art_meshes as u64,
        limits.max_art_meshes,
    )?;
    limits.check_count(
        "parameters",
        counts.parameters as u64,
        limits.max_parameters,
    )?;
    limits.check_count("glue", counts.glue as u64, limits.max_glues)?;
    for (field, value) in [
        ("part_keyforms", counts.part_keyforms),
        ("warp_deformer_keyforms", counts.warp_deformer_keyforms),
        (
            "rotation_deformer_keyforms",
            counts.rotation_deformer_keyforms,
        ),
        ("art_mesh_keyforms", counts.art_mesh_keyforms),
        ("glue_keyforms", counts.glue_keyforms),
    ] {
        limits.check_count(field, value as u64, limits.max_keyforms_per_category)?;
    }
    limits.check_count(
        "keyform_positions",
        counts.keyform_positions as u64,
        limits.max_keyform_position_floats,
    )?;
    limits.check_count(
        "parameter_binding_indices",
        counts.parameter_binding_indices as u64,
        limits.max_binding_indices,
    )?;
    limits.check_count(
        "keyform_bindings",
        counts.keyform_bindings as u64,
        limits.max_bindings,
    )?;
    limits.check_count(
        "parameter_bindings",
        counts.parameter_bindings as u64,
        limits.max_key_tables,
    )?;
    limits.check_count("keys", counts.keys as u64, limits.max_keys)?;
    limits.check_count("uvs", counts.uvs as u64, limits.max_uv_floats)?;
    limits.check_count(
        "position_indices",
        counts.position_indices as u64,
        limits.max_position_indices,
    )?;
    limits.check_count(
        "drawable_masks",
        counts.drawable_masks as u64,
        limits.max_masks,
    )?;
    limits.check_count(
        "draw_order_groups",
        counts.draw_order_groups as u64,
        limits.max_draw_order_groups,
    )?;
    limits.check_count(
        "draw_order_group_objects",
        counts.draw_order_group_objects as u64,
        limits.max_draw_order_items,
    )?;
    limits.check_count("glue_info", counts.glue_info as u64, limits.max_glue_info)?;
    for (field, value) in [
        ("keyform_multiply_colors", counts.keyform_multiply_colors),
        ("keyform_screen_colors", counts.keyform_screen_colors),
    ] {
        limits.check_count(field, value as u64, limits.max_keyform_colors)?;
    }
    for (field, value) in [
        (
            "blend_shape_parameter_bindings",
            counts.blend_shape_parameter_bindings,
        ),
        (
            "blend_shape_keyform_bindings",
            counts.blend_shape_keyform_bindings,
        ),
        (
            "blend_shapes_warp_deformers",
            counts.blend_shapes_warp_deformers,
        ),
        ("blend_shapes_art_meshes", counts.blend_shapes_art_meshes),
        (
            "blend_shape_constraint_indices",
            counts.blend_shape_constraint_indices,
        ),
        ("blend_shape_constraints", counts.blend_shape_constraints),
        (
            "blend_shape_constraint_values",
            counts.blend_shape_constraint_values,
        ),
        ("blend_shapes_parts", counts.blend_shapes_parts),
        (
            "blend_shapes_rotation_deformers",
            counts.blend_shapes_rotation_deformers,
        ),
        ("blend_shapes_glue", counts.blend_shapes_glue),
        ("blend_shapes_offscreens", counts.blend_shapes_offscreens),
    ] {
        limits.check_count(field, value as u64, limits.max_blend_shape_entries)?;
    }
    limits.check_count(
        "offscreens",
        counts.offscreens as u64,
        limits.max_offscreens,
    )?;
    limits.check_count(
        "offscreen_keyforms",
        counts.offscreen_keyforms as u64,
        limits.max_keyforms_per_category,
    )?;

    let total: u64 = [
        counts.parts,
        counts.deformers,
        counts.art_meshes,
        counts.parameters,
        counts.part_keyforms,
        counts.warp_deformer_keyforms,
        counts.rotation_deformer_keyforms,
        counts.art_mesh_keyforms,
        counts.keyform_positions,
        counts.parameter_binding_indices,
        counts.keyform_bindings,
        counts.parameter_bindings,
        counts.keys,
        counts.uvs,
        counts.position_indices,
        counts.drawable_masks,
        counts.draw_order_groups,
        counts.draw_order_group_objects,
        counts.glue,
        counts.glue_info,
        counts.glue_keyforms,
        counts.keyform_multiply_colors,
        counts.keyform_screen_colors,
        counts.blend_shape_parameter_bindings,
        counts.blend_shape_keyform_bindings,
        counts.blend_shapes_warp_deformers,
        counts.blend_shapes_art_meshes,
        counts.blend_shape_constraint_indices,
        counts.blend_shape_constraints,
        counts.blend_shape_constraint_values,
        counts.blend_shapes_parts,
        counts.blend_shapes_rotation_deformers,
        counts.blend_shapes_glue,
        counts.offscreens,
        counts.offscreen_keyforms,
        counts.blend_shapes_offscreens,
    ]
    .iter()
    .map(|value| *value as u64)
    .fold(0u64, |acc, value| acc.saturating_add(value));
    limits.check_count("total_elements", total, limits.max_total_elements)?;
    Ok(())
}

fn read_key_tables(
    begins: &[i32],
    counts_arr: &[i32],
    total_keys: u64,
    table_count: u32,
) -> Moc3Result<Vec<RawKeyTable>> {
    let mut out = vec_with_capacity(table_count as usize, "key_tables")?;
    for index in 0..table_count as usize {
        let begin = i64::from(pick(begins, index, "key_table.begin")?);
        let count = i64::from(pick(counts_arr, index, "key_table.count")?);
        check_range(begin, count, total_keys, "key_table.keys", index)?;
        out.push(RawKeyTable {
            index,
            key_begin: begin,
            key_count: count,
        });
    }
    Ok(out)
}

fn read_bindings(
    sections: &Sections,
    view: &ByteView<'_>,
    counts: &CountInfo,
    key_table_index: &[i32],
) -> Moc3Result<Vec<RawBinding>> {
    let begins = sections.i32_arr(view, table::S_BINDING_KEY_TABLE_BEGIN)?;
    let counts_arr = sections.i32_arr(view, table::S_BINDING_KEY_TABLE_COUNT)?;
    let mut out = vec_with_capacity(counts.keyform_bindings as usize, "bindings")?;
    for index in 0..counts.keyform_bindings as usize {
        let begin = i64::from(pick(&begins, index, "binding.key_table_begin")?);
        let count = i64::from(pick(&counts_arr, index, "binding.key_table_count")?);
        check_range(
            begin,
            count,
            counts.parameter_binding_indices as u64,
            "binding.key_table_index",
            index,
        )?;
        if !(0..=MAX_KEY_TABLES_PER_BINDING).contains(&count) {
            return Err(Moc3Error::new(ErrorKind::InvalidValue {
                field: "binding.key_table_count",
                index,
                value: count,
                reason: "binding references more key tables than the format allows",
            }));
        }
        let mut tables = vec_with_capacity(count as usize, "binding.key_tables")?;
        for slot in 0..count as usize {
            let at = (begin as u64).saturating_add(slot as u64);
            let idx = *key_table_index.get(at as usize).ok_or_else(|| {
                Moc3Error::new(ErrorKind::Internal {
                    what: "key_table_index",
                })
            })?;
            check_index(
                i64::from(idx),
                counts.parameter_bindings as u64,
                "binding.key_table_index",
                index,
            )?;
            tables.push(i64::from(idx));
        }
        out.push(RawBinding {
            index,
            key_tables: tables,
        });
    }
    Ok(out)
}

fn read_parameters(
    sections: &Sections,
    view: &ByteView<'_>,
    version: MocVersion,
    counts: &CountInfo,
    anomalies: &mut Vec<Anomaly>,
) -> Moc3Result<Vec<RawParameter>> {
    let n = counts.parameters as usize;
    let ids = sections.ids(view, table::S_PARAMETER_ID)?;
    let maxima = sections.f32_arr(view, table::S_PARAMETER_MAX)?;
    let minima = sections.f32_arr(view, table::S_PARAMETER_MIN)?;
    let defaults = sections.f32_arr(view, table::S_PARAMETER_DEFAULT)?;
    let repeats = sections.i32_arr(view, table::S_PARAMETER_REPEAT)?;
    let decimals = sections.i32_arr(view, table::S_PARAMETER_DECIMALS)?;
    let kt_begin = sections.i32_arr(view, table::S_PARAMETER_KEY_TABLE_BEGIN)?;
    let kt_count = sections.i32_arr(view, table::S_PARAMETER_KEY_TABLE_COUNT)?;
    let types = if version.has_v42_sections() {
        Some(sections.i32_arr(view, table::S_PARAMETER_TYPE)?)
    } else {
        None
    };
    let blend_begin = if version.has_v42_sections() {
        Some(sections.i32_arr(view, table::S_PARAMETER_BLEND_BEGIN)?)
    } else {
        None
    };
    let blend_count = if version.has_v42_sections() {
        Some(sections.i32_arr(view, table::S_PARAMETER_BLEND_COUNT)?)
    } else {
        None
    };

    let mut out = vec_with_capacity(n, "parameters")?;
    for index in 0..n {
        let id = pick(&ids, index, "parameter.id")?.clone();
        let begin = i64::from(pick(&kt_begin, index, "parameter.key_table_begin")?);
        let count = i64::from(pick(&kt_count, index, "parameter.key_table_count")?);
        check_range(
            begin,
            count,
            counts.parameter_bindings as u64,
            "parameter.key_table",
            index,
        )?;
        let repeat_raw = pick(&repeats, index, "parameter.repeat")?;
        if repeat_raw != 0 && repeat_raw != 1 {
            anomalies.push(Anomaly {
                code: "non_boolean_flag",
                message: format!("parameter[{index}].repeat = {repeat_raw} (treated as true)"),
                offset: None,
                index: Some(index),
            });
        }
        let parameter_type = match types.as_ref() {
            Some(values) => {
                let raw = pick(values, index, "parameter.type")?;
                match raw {
                    0 => Some(ParameterType::Normal),
                    1 => Some(ParameterType::BlendShape),
                    other => {
                        return Err(Moc3Error::new(ErrorKind::InvalidValue {
                            field: "parameter.type",
                            index,
                            value: i64::from(other),
                            reason: "expected 0 (normal) or 1 (blend shape)",
                        }))
                    }
                }
            }
            None => None,
        };
        let (bb, bc) = match (blend_begin.as_ref(), blend_count.as_ref()) {
            (Some(begins), Some(counts_arr)) => {
                let b = i64::from(pick(begins, index, "parameter.blend_key_table_begin")?);
                let c = i64::from(pick(counts_arr, index, "parameter.blend_key_table_count")?);
                check_range(
                    b,
                    c,
                    counts.blend_shape_parameter_bindings as u64,
                    "parameter.blend_key_table",
                    index,
                )?;
                (Some(b), Some(c))
            }
            _ => (None, None),
        };
        out.push(RawParameter {
            index,
            id,
            minimum: pick(&minima, index, "parameter.minimum")?,
            maximum: pick(&maxima, index, "parameter.maximum")?,
            default: pick(&defaults, index, "parameter.default")?,
            repeat: repeat_raw != 0,
            decimal_places: i64::from(pick(&decimals, index, "parameter.decimal_places")?),
            key_table_begin: begin,
            key_table_count: count,
            parameter_type,
            blend_key_table_begin: bb,
            blend_key_table_count: bc,
        });
    }
    Ok(out)
}

fn bool_from_flag(
    value: i32,
    field: &'static str,
    index: usize,
    anomalies: &mut Vec<Anomaly>,
) -> bool {
    if value != 0 && value != 1 {
        anomalies.push(Anomaly {
            code: "non_boolean_flag",
            message: format!("{field}[{index}] = {value} (treated as true)"),
            offset: None,
            index: Some(index),
        });
    }
    value != 0
}

fn read_parts(
    sections: &Sections,
    view: &ByteView<'_>,
    version: MocVersion,
    counts: &CountInfo,
    anomalies: &mut Vec<Anomaly>,
) -> Moc3Result<Vec<RawPart>> {
    let n = counts.parts as usize;
    let ids = sections.ids(view, table::S_PART_ID)?;
    let bindings = sections.i32_arr(view, table::S_PART_BINDING)?;
    let kf_begin = sections.i32_arr(view, table::S_PART_KEYFORM_BEGIN)?;
    let kf_count = sections.i32_arr(view, table::S_PART_KEYFORM_COUNT)?;
    let visible = sections.i32_arr(view, table::S_PART_VISIBLE)?;
    let enabled = sections.i32_arr(view, table::S_PART_ENABLED)?;
    let parents = sections.i32_arr(view, table::S_PART_PARENT)?;
    let offscreens = if version.has_v53_sections() {
        Some(sections.i32_arr(view, table::S_PART_OFFSCREEN_INDEX)?)
    } else {
        None
    };

    let mut out = vec_with_capacity(n, "parts")?;
    for index in 0..n {
        let binding_index = i64::from(pick(&bindings, index, "part.binding_index")?);
        check_index(
            binding_index,
            counts.keyform_bindings as u64,
            "part.binding_index",
            index,
        )?;
        let begin = i64::from(pick(&kf_begin, index, "part.keyform_begin")?);
        let count = i64::from(pick(&kf_count, index, "part.keyform_count")?);
        check_range(
            begin,
            count,
            counts.part_keyforms as u64,
            "part.keyform",
            index,
        )?;
        let parent = check_index_or_neg1(
            i64::from(pick(&parents, index, "part.parent_part")?),
            counts.parts as u64,
            "part.parent_part",
            index,
        )?;
        if parent == Some(index as u32) {
            anomalies.push(Anomaly {
                code: "self_parent",
                message: format!("part[{index}] is its own parent"),
                offset: None,
                index: Some(index),
            });
        }
        let offscreen_index = match offscreens.as_ref() {
            Some(values) => {
                let raw = i64::from(pick(values, index, "part.offscreen_index")?);
                let checked = check_index_or_neg1(
                    raw,
                    counts.offscreens as u64,
                    "part.offscreen_index",
                    index,
                )?;
                checked.map(i64::from)
            }
            None => None,
        };
        out.push(RawPart {
            index,
            id: pick(&ids, index, "part.id")?.clone(),
            binding_index,
            keyform_begin: begin,
            keyform_count: count,
            visible: bool_from_flag(
                pick(&visible, index, "part.visible")?,
                "part.visible",
                index,
                anomalies,
            ),
            enabled: bool_from_flag(
                pick(&enabled, index, "part.enabled")?,
                "part.enabled",
                index,
                anomalies,
            ),
            parent_part: parent,
            offscreen_index,
        });
    }
    Ok(out)
}

type KeyformRefs = Vec<(i64, i64, i64)>;

fn read_deformers(
    sections: &Sections,
    view: &ByteView<'_>,
    version: MocVersion,
    counts: &CountInfo,
    anomalies: &mut Vec<Anomaly>,
) -> Moc3Result<(Vec<RawDeformer>, KeyformRefs, KeyformRefs)> {
    let n = counts.deformers as usize;
    let ids = sections.ids(view, table::S_DEFORMER_ID)?;
    let bindings = sections.i32_arr(view, table::S_DEFORMER_BINDING)?;
    let visible = sections.i32_arr(view, table::S_DEFORMER_VISIBLE)?;
    let enabled = sections.i32_arr(view, table::S_DEFORMER_ENABLED)?;
    let parent_parts = sections.i32_arr(view, table::S_DEFORMER_PARENT_PART)?;
    let parent_deformers = sections.i32_arr(view, table::S_DEFORMER_PARENT_DEFORMER)?;
    let types = sections.i32_arr(view, table::S_DEFORMER_TYPE)?;
    let local_indices = sections.i32_arr(view, table::S_DEFORMER_LOCAL_INDEX)?;

    let warp_vertices = sections.i32_arr(view, table::S_WARP_VERTEX_COUNT)?;
    let warp_rows = sections.i32_arr(view, table::S_WARP_ROWS)?;
    let warp_columns = sections.i32_arr(view, table::S_WARP_COLUMNS)?;
    let warp_quad = if version.has_quad_transform() {
        Some(sections.i32_arr(view, table::S_WARP_QUAD_TRANSFORM)?)
    } else {
        None
    };
    let rotation_angles = sections.f32_arr(view, table::S_ROTATION_BASE_ANGLE)?;

    let warp_bindings = sections.i32_arr(view, table::S_WARP_BINDING)?;
    let warp_kf_begin = sections.i32_arr(view, table::S_WARP_KEYFORM_BEGIN)?;
    let warp_kf_count = sections.i32_arr(view, table::S_WARP_KEYFORM_COUNT)?;
    let rotation_bindings = sections.i32_arr(view, table::S_ROTATION_BINDING)?;
    let rotation_kf_begin = sections.i32_arr(view, table::S_ROTATION_KEYFORM_BEGIN)?;
    let rotation_kf_count = sections.i32_arr(view, table::S_ROTATION_KEYFORM_COUNT)?;

    // Validate the warp-specific tables.
    let mut warp_keyforms = vec_with_capacity(counts.warp_deformers as usize, "warp_keyforms")?;
    for index in 0..counts.warp_deformers as usize {
        let binding = i64::from(pick(&warp_bindings, index, "warp.binding_index")?);
        check_index(
            binding,
            counts.keyform_bindings as u64,
            "warp.binding_index",
            index,
        )?;
        let begin = i64::from(pick(&warp_kf_begin, index, "warp.keyform_begin")?);
        let count = i64::from(pick(&warp_kf_count, index, "warp.keyform_count")?);
        check_range(
            begin,
            count,
            counts.warp_deformer_keyforms as u64,
            "warp.keyform",
            index,
        )?;
        let rows = pick(&warp_rows, index, "warp.rows")?;
        let columns = pick(&warp_columns, index, "warp.columns")?;
        let vertices = pick(&warp_vertices, index, "warp.vertex_count")?;
        if rows <= 0 || columns <= 0 {
            return Err(Moc3Error::new(ErrorKind::InvalidValue {
                field: "warp.grid",
                index,
                value: i64::from(rows) * i64::from(columns),
                reason: "warp grid rows/columns must be positive",
            }));
        }
        let expected = (rows as u64 + 1).saturating_mul(columns as u64 + 1);
        if vertices < 0 || vertices as u64 != expected {
            return Err(Moc3Error::new(ErrorKind::InvalidValue {
                field: "warp.vertex_count",
                index,
                value: i64::from(vertices),
                reason: "vertex count does not match (rows + 1) * (columns + 1)",
            }));
        }
        warp_keyforms.push((binding, begin, count));
    }

    let mut rotation_keyforms =
        vec_with_capacity(counts.rotation_deformers as usize, "rotation_keyforms")?;
    for index in 0..counts.rotation_deformers as usize {
        let binding = i64::from(pick(&rotation_bindings, index, "rotation.binding_index")?);
        check_index(
            binding,
            counts.keyform_bindings as u64,
            "rotation.binding_index",
            index,
        )?;
        let begin = i64::from(pick(&rotation_kf_begin, index, "rotation.keyform_begin")?);
        let count = i64::from(pick(&rotation_kf_count, index, "rotation.keyform_count")?);
        check_range(
            begin,
            count,
            counts.rotation_deformer_keyforms as u64,
            "rotation.keyform",
            index,
        )?;
        rotation_keyforms.push((binding, begin, count));
    }

    if version.has_v42_sections() {
        let mul_colors = counts.keyform_multiply_colors as u64;
        let warp_colors = sections.i32_arr(view, table::S_WARP_KEYFORM_COLOR_BEGIN)?;
        for index in 0..counts.warp_deformers as usize {
            let color_begin = i64::from(pick(&warp_colors, index, "warp.keyform_color_begin")?);
            let kf_count = i64::from(pick(&warp_kf_count, index, "warp.keyform_count")?);
            check_range(
                color_begin,
                kf_count.max(0),
                mul_colors,
                "warp.keyform_color",
                index,
            )?;
        }
        let rotation_colors = sections.i32_arr(view, table::S_ROTATION_KEYFORM_COLOR_BEGIN)?;
        for index in 0..counts.rotation_deformers as usize {
            let color_begin = i64::from(pick(
                &rotation_colors,
                index,
                "rotation.keyform_color_begin",
            )?);
            let kf_count = i64::from(pick(&rotation_kf_count, index, "rotation.keyform_count")?);
            check_range(
                color_begin,
                kf_count.max(0),
                mul_colors,
                "rotation.keyform_color",
                index,
            )?;
        }
    }

    let mut out = vec_with_capacity(n, "deformers")?;
    for index in 0..n {
        let binding_index = i64::from(pick(&bindings, index, "deformer.binding_index")?);
        check_index(
            binding_index,
            counts.keyform_bindings as u64,
            "deformer.binding_index",
            index,
        )?;
        let parent_part = check_index_or_neg1(
            i64::from(pick(&parent_parts, index, "deformer.parent_part")?),
            counts.parts as u64,
            "deformer.parent_part",
            index,
        )?;
        let parent_deformer = check_index_or_neg1(
            i64::from(pick(&parent_deformers, index, "deformer.parent_deformer")?),
            counts.deformers as u64,
            "deformer.parent_deformer",
            index,
        )?;
        if parent_deformer == Some(index as u32) {
            anomalies.push(Anomaly {
                code: "self_parent",
                message: format!("deformer[{index}] is its own parent"),
                offset: None,
                index: Some(index),
            });
        }
        let deformer_type = pick(&types, index, "deformer.type")?;
        let local_index = i64::from(pick(&local_indices, index, "deformer.local_index")?);
        let specific = match deformer_type {
            0 => {
                check_index(
                    local_index,
                    counts.warp_deformers as u64,
                    "deformer.local_index",
                    index,
                )?;
                let at = local_index as usize;
                DeformerSpecific::Warp {
                    vertex_count: i64::from(pick(&warp_vertices, at, "warp.vertex_count")?),
                    rows: i64::from(pick(&warp_rows, at, "warp.rows")?),
                    columns: i64::from(pick(&warp_columns, at, "warp.columns")?),
                    quad_transform: match warp_quad.as_ref() {
                        Some(values) => Some(i64::from(pick(values, at, "warp.quad_transform")?)),
                        None => None,
                    },
                }
            }
            1 => {
                check_index(
                    local_index,
                    counts.rotation_deformers as u64,
                    "deformer.local_index",
                    index,
                )?;
                DeformerSpecific::Rotation {
                    base_angle: pick(
                        &rotation_angles,
                        local_index as usize,
                        "rotation.base_angle",
                    )?,
                }
            }
            other => {
                return Err(Moc3Error::new(ErrorKind::InvalidValue {
                    field: "deformer.type",
                    index,
                    value: i64::from(other),
                    reason: "expected 0 (warp) or 1 (rotation)",
                }))
            }
        };
        out.push(RawDeformer {
            index,
            id: pick(&ids, index, "deformer.id")?.clone(),
            specific,
            local_index,
            binding_index,
            visible: bool_from_flag(
                pick(&visible, index, "deformer.visible")?,
                "deformer.visible",
                index,
                anomalies,
            ),
            enabled: bool_from_flag(
                pick(&enabled, index, "deformer.enabled")?,
                "deformer.enabled",
                index,
                anomalies,
            ),
            parent_part,
            parent_deformer,
        });
    }
    Ok((out, warp_keyforms, rotation_keyforms))
}

fn read_art_meshes(
    sections: &Sections,
    view: &ByteView<'_>,
    version: MocVersion,
    counts: &CountInfo,
    anomalies: &mut Vec<Anomaly>,
) -> Moc3Result<Vec<RawArtMesh>> {
    let n = counts.art_meshes as usize;
    let ids = sections.ids(view, table::S_ART_MESH_ID)?;
    let bindings = sections.i32_arr(view, table::S_ART_MESH_BINDING)?;
    let kf_begin = sections.i32_arr(view, table::S_ART_MESH_KEYFORM_BEGIN)?;
    let kf_count = sections.i32_arr(view, table::S_ART_MESH_KEYFORM_COUNT)?;
    let visible = sections.i32_arr(view, table::S_ART_MESH_VISIBLE)?;
    let enabled = sections.i32_arr(view, table::S_ART_MESH_ENABLED)?;
    let parent_parts = sections.i32_arr(view, table::S_ART_MESH_PARENT_PART)?;
    let parent_deformers = sections.i32_arr(view, table::S_ART_MESH_PARENT_DEFORMER)?;
    let textures = sections.i32_arr(view, table::S_ART_MESH_TEXTURE)?;
    let flags = sections.u8_arr(view, table::S_ART_MESH_FLAGS)?;
    let vertices = sections.i32_arr(view, table::S_ART_MESH_VERTEX_COUNT)?;
    let uv_begins = sections.i32_arr(view, table::S_ART_MESH_UV_BEGIN)?;
    let idx_begins = sections.i32_arr(view, table::S_ART_MESH_INDEX_BEGIN)?;
    let idx_counts = sections.i32_arr(view, table::S_ART_MESH_INDEX_COUNT)?;
    let mask_begins = sections.i32_arr(view, table::S_ART_MESH_MASK_BEGIN)?;
    let mask_counts = sections.i32_arr(view, table::S_ART_MESH_MASK_COUNT)?;
    let blend_modes = if version.has_v53_sections() {
        Some(sections.i32_arr(view, table::S_ART_MESH_BLEND_MODE)?)
    } else {
        None
    };

    let mask_values = if counts.drawable_masks > 0 {
        sections.i32_arr(view, table::S_DRAWABLE_MASK_INDICES)?
    } else {
        Vec::new()
    };

    // Read once, not per mesh: the array is indexed by art mesh.
    let keyform_colors = if version.has_v42_sections() {
        Some(sections.i32_arr(view, table::S_ART_MESH_KEYFORM_COLOR_BEGIN)?)
    } else {
        None
    };

    let mut out = vec_with_capacity(n, "art_meshes")?;
    for index in 0..n {
        let binding_index = i64::from(pick(&bindings, index, "art_mesh.binding_index")?);
        check_index(
            binding_index,
            counts.keyform_bindings as u64,
            "art_mesh.binding_index",
            index,
        )?;
        let begin = i64::from(pick(&kf_begin, index, "art_mesh.keyform_begin")?);
        let count = i64::from(pick(&kf_count, index, "art_mesh.keyform_count")?);
        check_range(
            begin,
            count,
            counts.art_mesh_keyforms as u64,
            "art_mesh.keyform",
            index,
        )?;
        let parent_part = check_index_or_neg1(
            i64::from(pick(&parent_parts, index, "art_mesh.parent_part")?),
            counts.parts as u64,
            "art_mesh.parent_part",
            index,
        )?;
        let parent_deformer = check_index_or_neg1(
            i64::from(pick(&parent_deformers, index, "art_mesh.parent_deformer")?),
            counts.deformers as u64,
            "art_mesh.parent_deformer",
            index,
        )?;

        let texture_number = i64::from(pick(&textures, index, "art_mesh.texture_number")?);
        if texture_number < 0 {
            anomalies.push(Anomaly {
                code: "negative_texture_number",
                message: format!(
                    "art_mesh[{index}] references texture {} (negative; ignored)",
                    texture_number
                ),
                offset: None,
                index: Some(index),
            });
        }

        let flag_byte = pick(&flags, index, "art_mesh.drawable_flags")?;
        if flag_byte & 0xF0 != 0 {
            anomalies.push(Anomaly {
                code: "unknown_drawable_flag_bits",
                message: format!(
                    "art_mesh[{index}] drawable flag 0x{flag_byte:02x} has unknown high bits"
                ),
                offset: None,
                index: Some(index),
            });
        }

        let vertex_count = i64::from(pick(&vertices, index, "art_mesh.vertex_count")?);
        let uv_begin = i64::from(pick(&uv_begins, index, "art_mesh.uv_begin")?);
        if vertex_count < 0 || uv_begin < 0 {
            return Err(Moc3Error::new(ErrorKind::InvalidValue {
                field: "art_mesh.uv",
                index,
                value: uv_begin,
                reason: "vertex count and uv begin must be non-negative",
            }));
        }
        let uv_floats = (vertex_count as u64).saturating_mul(2);
        if (uv_begin as u64).saturating_add(uv_floats) > counts.uvs as u64 {
            return Err(Moc3Error::new(ErrorKind::InvalidRange {
                field: "art_mesh.uv",
                index,
                begin: uv_begin,
                count: uv_floats as i64,
                total: counts.uvs as u64,
            }));
        }
        let index_begin = i64::from(pick(&idx_begins, index, "art_mesh.index_begin")?);
        let index_count = i64::from(pick(&idx_counts, index, "art_mesh.index_count")?);
        check_range(
            index_begin,
            index_count,
            counts.position_indices as u64,
            "art_mesh.index",
            index,
        )?;
        let mask_begin = i64::from(pick(&mask_begins, index, "art_mesh.mask_begin")?);
        let mask_count = i64::from(pick(&mask_counts, index, "art_mesh.mask_count")?);
        check_range(
            mask_begin,
            mask_count,
            counts.drawable_masks as u64,
            "art_mesh.mask",
            index,
        )?;
        let mut resolved_masks = vec_with_capacity(mask_count.max(0) as usize, "art_mesh.masks")?;
        for slot in 0..mask_count.max(0) as usize {
            let at = (mask_begin as u64).saturating_add(slot as u64);
            let value = *mask_values.get(at as usize).ok_or_else(|| {
                Moc3Error::new(ErrorKind::Internal {
                    what: "mask_values",
                })
            })?;
            check_index_or_neg1(
                i64::from(value),
                counts.art_meshes as u64,
                "art_mesh.mask_entry",
                index,
            )?;
            resolved_masks.push(i64::from(value));
        }

        let keyform_color_begin = if let Some(values) = keyform_colors.as_ref() {
            let color_begin = i64::from(pick(values, index, "art_mesh.keyform_color_begin")?);
            check_range(
                color_begin,
                count.max(0),
                counts.keyform_multiply_colors as u64,
                "art_mesh.keyform_color",
                index,
            )?;
            Some(color_begin)
        } else {
            None
        };

        out.push(RawArtMesh {
            index,
            id: pick(&ids, index, "art_mesh.id")?.clone(),
            binding_index,
            keyform_begin: begin,
            keyform_count: count,
            visible: bool_from_flag(
                pick(&visible, index, "art_mesh.visible")?,
                "art_mesh.visible",
                index,
                anomalies,
            ),
            enabled: bool_from_flag(
                pick(&enabled, index, "art_mesh.enabled")?,
                "art_mesh.enabled",
                index,
                anomalies,
            ),
            parent_part,
            parent_deformer,
            texture_number,
            drawable_flags_raw: flag_byte,
            blend_mode_bits: flag_byte & 0x03,
            double_sided: flag_byte & 0x04 != 0,
            inverted_mask: flag_byte & 0x08 != 0,
            vertex_count,
            uv_begin,
            index_begin,
            index_count,
            mask_begin,
            mask_count,
            mask_drawables: resolved_masks,
            keyform_color_begin,
            blend_mode_5_3: match blend_modes.as_ref() {
                Some(values) => Some(i64::from(pick(values, index, "art_mesh.blend_mode")?)),
                None => None,
            },
        });
    }
    Ok(out)
}

fn read_masks(
    sections: &Sections,
    view: &ByteView<'_>,
    counts: &CountInfo,
) -> Moc3Result<Vec<RawMaskEntry>> {
    let n = counts.drawable_masks as usize;
    let values = sections.i32_arr(view, table::S_DRAWABLE_MASK_INDICES)?;
    let mut out = vec_with_capacity(n, "masks")?;
    for index in 0..n {
        let value = i64::from(pick(&values, index, "drawable_mask.indices")?);
        if value < -1 || (value >= 0 && value as u64 >= counts.art_meshes as u64) {
            return Err(Moc3Error::new(ErrorKind::InvalidReference {
                field: "drawable_mask.indices",
                index,
                value,
                bound: counts.art_meshes as u64,
            }));
        }
        out.push(RawMaskEntry {
            index,
            art_mesh: value,
        });
    }
    Ok(out)
}

fn read_draw_order_groups(
    sections: &Sections,
    view: &ByteView<'_>,
    counts: &CountInfo,
) -> Moc3Result<Vec<RawDrawOrderGroup>> {
    let n = counts.draw_order_groups as usize;
    let begins = sections.i32_arr(view, table::S_DRAW_GROUP_OBJECT_BEGIN)?;
    let counts_arr = sections.i32_arr(view, table::S_DRAW_GROUP_OBJECT_COUNT)?;
    let totals = sections.i32_arr(view, table::S_DRAW_GROUP_OBJECT_TOTAL)?;
    let maxima = sections.i32_arr(view, table::S_DRAW_GROUP_MAX_ORDER)?;
    let minima = sections.i32_arr(view, table::S_DRAW_GROUP_MIN_ORDER)?;
    let mut out = vec_with_capacity(n, "draw_order_groups")?;
    for index in 0..n {
        let begin = i64::from(pick(&begins, index, "draw_group.object_begin")?);
        let count = i64::from(pick(&counts_arr, index, "draw_group.object_count")?);
        check_range(
            begin,
            count,
            counts.draw_order_group_objects as u64,
            "draw_group.objects",
            index,
        )?;
        out.push(RawDrawOrderGroup {
            index,
            object_begin: begin,
            object_count: count,
            object_total_count: i64::from(pick(&totals, index, "draw_group.object_total_count")?),
            max_order: i64::from(pick(&maxima, index, "draw_group.max_order")?),
            min_order: i64::from(pick(&minima, index, "draw_group.min_order")?),
        });
    }
    Ok(out)
}

fn read_draw_order_items(
    sections: &Sections,
    view: &ByteView<'_>,
    counts: &CountInfo,
) -> Moc3Result<Vec<RawDrawOrderItem>> {
    let n = counts.draw_order_group_objects as usize;
    let types = sections.i32_arr(view, table::S_DRAW_ITEM_TYPE)?;
    let indices = sections.i32_arr(view, table::S_DRAW_ITEM_INDEX)?;
    let groups = sections.i32_arr(view, table::S_DRAW_ITEM_SELF_GROUP)?;
    let mut out = vec_with_capacity(n, "draw_order_items")?;
    for index in 0..n {
        let object_type = i64::from(pick(&types, index, "draw_item.type")?);
        let object_index = i64::from(pick(&indices, index, "draw_item.index")?);
        let bound = match object_type {
            0 => counts.art_meshes as u64,
            1 => counts.parts as u64,
            other => {
                return Err(Moc3Error::new(ErrorKind::InvalidValue {
                    field: "draw_item.type",
                    index,
                    value: other,
                    reason: "expected 0 (art mesh) or 1 (part)",
                }))
            }
        };
        check_index(object_index, bound, "draw_item.index", index)?;
        let self_group = i64::from(pick(&groups, index, "draw_item.self_group")?);
        check_index_or_neg1(
            self_group,
            counts.draw_order_groups as u64,
            "draw_item.self_group",
            index,
        )?;
        if object_type == 1 && self_group < 0 {
            return Err(Moc3Error::new(ErrorKind::InvalidValue {
                field: "draw_item.self_group",
                index,
                value: self_group,
                reason: "part draw items must reference a group",
            }));
        }
        out.push(RawDrawOrderItem {
            index,
            object_type,
            object_index,
            self_group,
        });
    }
    Ok(out)
}

fn read_glues(
    sections: &Sections,
    view: &ByteView<'_>,
    counts: &CountInfo,
) -> Moc3Result<Vec<RawGlue>> {
    let n = counts.glue as usize;
    let ids = sections.ids(view, table::S_GLUE_ID)?;
    let bindings = sections.i32_arr(view, table::S_GLUE_BINDING)?;
    let kf_begin = sections.i32_arr(view, table::S_GLUE_KEYFORM_BEGIN)?;
    let kf_count = sections.i32_arr(view, table::S_GLUE_KEYFORM_COUNT)?;
    let mesh_a = sections.i32_arr(view, table::S_GLUE_ART_MESH_A)?;
    let mesh_b = sections.i32_arr(view, table::S_GLUE_ART_MESH_B)?;
    let info_begin = sections.i32_arr(view, table::S_GLUE_INFO_BEGIN)?;
    let info_count = sections.i32_arr(view, table::S_GLUE_INFO_COUNT)?;
    let vertices = sections.i32_arr(view, table::S_ART_MESH_VERTEX_COUNT)?;
    let pos_indices = if counts.glue_info > 0 {
        sections.u16_arr(view, table::S_GLUE_INFO_POSITION_INDEX)?
    } else {
        Vec::new()
    };

    let mut out = vec_with_capacity(n, "glues")?;
    for index in 0..n {
        let binding_index = i64::from(pick(&bindings, index, "glue.binding_index")?);
        check_index(
            binding_index,
            counts.keyform_bindings as u64,
            "glue.binding_index",
            index,
        )?;
        let begin = i64::from(pick(&kf_begin, index, "glue.keyform_begin")?);
        let count = i64::from(pick(&kf_count, index, "glue.keyform_count")?);
        check_range(
            begin,
            count,
            counts.glue_keyforms as u64,
            "glue.keyform",
            index,
        )?;
        let a = i64::from(pick(&mesh_a, index, "glue.art_mesh_a")?);
        let b = i64::from(pick(&mesh_b, index, "glue.art_mesh_b")?);
        check_index(a, counts.art_meshes as u64, "glue.art_mesh_a", index)?;
        check_index(b, counts.art_meshes as u64, "glue.art_mesh_b", index)?;
        let info_begin_value = i64::from(pick(&info_begin, index, "glue.info_begin")?);
        let info_count_value = i64::from(pick(&info_count, index, "glue.info_count")?);
        check_range(
            info_begin_value,
            info_count_value,
            counts.glue_info as u64,
            "glue.info",
            index,
        )?;
        if info_count_value % 2 != 0 {
            return Err(Moc3Error::new(ErrorKind::InvalidValue {
                field: "glue.info_count",
                index,
                value: info_count_value,
                reason: "glue info entries come in pairs",
            }));
        }
        let vc_a = pick(&vertices, a as usize, "glue.vertex_count_a")?;
        let vc_b = pick(&vertices, b as usize, "glue.vertex_count_b")?;
        if vc_a < 0 || vc_b < 0 {
            return Err(Moc3Error::new(ErrorKind::InvalidValue {
                field: "glue.vertex_count",
                index,
                value: i64::from(vc_a),
                reason: "vertex counts must be non-negative",
            }));
        }
        for slot in 0..info_count_value.max(0) as usize {
            let at = (info_begin_value as u64).saturating_add(slot as u64);
            let value = *pos_indices
                .get(at as usize)
                .ok_or_else(|| Moc3Error::new(ErrorKind::Internal { what: "glue_info" }))?;
            let vc = if slot % 2 == 0 { vc_a } else { vc_b };
            if u32::from(value) >= vc as u32 {
                return Err(Moc3Error::new(ErrorKind::InvalidReference {
                    field: "glue_info.position_index",
                    index: at as usize,
                    value: i64::from(value),
                    bound: vc as u64,
                }));
            }
        }
        out.push(RawGlue {
            index,
            id: pick(&ids, index, "glue.id")?.clone(),
            binding_index,
            keyform_begin: begin,
            keyform_count: count,
            art_mesh_a: a,
            art_mesh_b: b,
            info_begin: info_begin_value,
            info_count: info_count_value,
        });
    }
    Ok(out)
}

/// Count the keyform grid a binding produces (product of key counts > 1).
fn binding_keyform_count(
    binding_index: i64,
    bindings: &[RawBinding],
    key_tables: &[RawKeyTable],
) -> u64 {
    let mut product: u64 = 1;
    let binding = match bindings.get(binding_index.max(0) as usize) {
        Some(binding) => binding,
        None => return 1,
    };
    for table_index in &binding.key_tables {
        let table = match key_tables.get(*table_index as usize) {
            Some(table) => table,
            None => continue,
        };
        let keys = table.key_count;
        if keys <= 1 {
            continue;
        }
        product = product.saturating_mul(keys as u64);
    }
    product
}

fn check_keyform_spans(
    object_kind: &'static str,
    objects: &KeyformRefs,
    bindings: &[RawBinding],
    key_tables: &[RawKeyTable],
    _total_keyforms: u64,
) -> Moc3Result<()> {
    for (index, (binding_index, _begin, count)) in objects.iter().enumerate() {
        if *binding_index < 0 {
            continue;
        }
        let product = binding_keyform_count(*binding_index, bindings, key_tables);
        if *count < 0 || product > *count as u64 {
            return Err(Moc3Error::new(ErrorKind::InvalidValue {
                field: match object_kind {
                    "part" => "part.keyform_span",
                    "warp" => "warp.keyform_span",
                    "rotation" => "rotation.keyform_span",
                    "glue" => "glue.keyform_span",
                    _ => "art_mesh.keyform_span",
                },
                index,
                value: *count,
                reason: "declared keyforms do not cover the bound parameter grid",
            }));
        }
    }
    Ok(())
}

fn validate_blend_shape_sections(
    sections: &Sections,
    view: &ByteView<'_>,
    version: MocVersion,
    counts: &CountInfo,
) -> Moc3Result<()> {
    if !version.has_v42_sections() {
        return Ok(());
    }
    // Parameter extension keys point into the global key value pool.
    let ext_begin = sections.i32_arr(view, table::S_PARAMETER_EXT_KEY_BEGIN)?;
    let ext_count = sections.i32_arr(view, table::S_PARAMETER_EXT_KEY_COUNT)?;
    for index in 0..counts.parameters as usize {
        let begin = i64::from(pick(&ext_begin, index, "parameter.extension_key_begin")?);
        let count = i64::from(pick(&ext_count, index, "parameter.extension_key_count")?);
        check_range(
            begin,
            count,
            counts.keys as u64,
            "parameter.extension_key",
            index,
        )?;
    }

    // Blend key tables.
    let bkt_begin = sections.i32_arr(view, table::S_BLEND_KEY_TABLE_BEGIN)?;
    let bkt_count = sections.i32_arr(view, table::S_BLEND_KEY_TABLE_COUNT)?;
    for index in 0..counts.blend_shape_parameter_bindings as usize {
        let begin = i64::from(pick(&bkt_begin, index, "blend_key_table.begin")?);
        let count = i64::from(pick(&bkt_count, index, "blend_key_table.count")?);
        check_range(
            begin,
            count,
            counts.keys as u64,
            "blend_key_table.keys",
            index,
        )?;
    }

    // Blend bindings: key table index + constraint range.
    let bb_table = sections.i32_arr(view, table::S_BLEND_BINDING_KEY_TABLE_INDEX)?;
    let bb_constraint_begin = sections.i32_arr(view, table::S_BLEND_BINDING_CONSTRAINT_BEGIN)?;
    let bb_constraint_count = sections.i32_arr(view, table::S_BLEND_BINDING_CONSTRAINT_COUNT)?;
    for index in 0..counts.blend_shape_keyform_bindings as usize {
        let table = i64::from(pick(&bb_table, index, "blend_binding.key_table_index")?);
        check_index(
            table,
            counts.blend_shape_parameter_bindings as u64,
            "blend_binding.key_table_index",
            index,
        )?;
        let begin = i64::from(pick(
            &bb_constraint_begin,
            index,
            "blend_binding.constraint_begin",
        )?);
        let count = i64::from(pick(
            &bb_constraint_count,
            index,
            "blend_binding.constraint_count",
        )?);
        check_range(
            begin,
            count,
            counts.blend_shape_constraint_indices as u64,
            "blend_binding.constraints",
            index,
        )?;
    }

    // Blend shape warp targets.
    {
        let targets = sections.i32_arr(view, table::S_BS_WARP_TARGET)?;
        let begins = sections.i32_arr(view, table::S_BS_WARP_BINDING_BEGIN)?;
        let lengths = sections.i32_arr(view, table::S_BS_WARP_BINDING_COUNT)?;
        for index in 0..counts.blend_shapes_warp_deformers as usize {
            let target = i64::from(pick(&targets, index, "blend_shape.warp_target")?);
            check_index(
                target,
                counts.warp_deformers as u64,
                "blend_shape.warp_target",
                index,
            )?;
            let begin = i64::from(pick(&begins, index, "blend_shape.warp_binding_begin")?);
            let count = i64::from(pick(&lengths, index, "blend_shape.warp_binding_count")?);
            check_range(
                begin,
                count,
                counts.blend_shape_keyform_bindings as u64,
                "blend_shape.warp_bindings",
                index,
            )?;
        }
    }

    // Blend shape art mesh targets.
    {
        let targets = sections.i32_arr(view, table::S_BS_ART_MESH_TARGET)?;
        let begins = sections.i32_arr(view, table::S_BS_ART_MESH_BINDING_BEGIN)?;
        let lengths = sections.i32_arr(view, table::S_BS_ART_MESH_BINDING_COUNT)?;
        for index in 0..counts.blend_shapes_art_meshes as usize {
            let target = i64::from(pick(&targets, index, "blend_shape.art_mesh_target")?);
            check_index(
                target,
                counts.art_meshes as u64,
                "blend_shape.art_mesh_target",
                index,
            )?;
            let begin = i64::from(pick(&begins, index, "blend_shape.art_mesh_binding_begin")?);
            let count = i64::from(pick(&lengths, index, "blend_shape.art_mesh_binding_count")?);
            check_range(
                begin,
                count,
                counts.blend_shape_keyform_bindings as u64,
                "blend_shape.art_mesh_bindings",
                index,
            )?;
        }
    }

    // Constraint indices and constraint parameter/value ranges.
    let constraint_idx = sections.i32_arr(view, table::S_BS_CONSTRAINT_INDEX)?;
    for index in 0..counts.blend_shape_constraint_indices as usize {
        let value = i64::from(pick(&constraint_idx, index, "blend_constraint_index")?);
        check_index(
            value,
            counts.blend_shape_constraints as u64,
            "blend_constraint_index",
            index,
        )?;
    }
    let constraint_param = sections.i32_arr(view, table::S_BS_CONSTRAINT_PARAMETER)?;
    let constraint_v_begin = sections.i32_arr(view, table::S_BS_CONSTRAINT_VALUE_BEGIN)?;
    let constraint_v_count = sections.i32_arr(view, table::S_BS_CONSTRAINT_VALUE_COUNT)?;
    for index in 0..counts.blend_shape_constraints as usize {
        let parameter = i64::from(pick(
            &constraint_param,
            index,
            "blend_constraint.parameter",
        )?);
        check_index(
            parameter,
            counts.parameters as u64,
            "blend_constraint.parameter",
            index,
        )?;
        let begin = i64::from(pick(
            &constraint_v_begin,
            index,
            "blend_constraint.value_begin",
        )?);
        let count = i64::from(pick(
            &constraint_v_count,
            index,
            "blend_constraint.value_count",
        )?);
        check_range(
            begin,
            count,
            counts.blend_shape_constraint_values as u64,
            "blend_constraint.values",
            index,
        )?;
    }

    if version.has_v50_sections() {
        // Blend shape part targets.
        let targets = sections.i32_arr(view, table::S_BS_PART_TARGET)?;
        let begins = sections.i32_arr(view, table::S_BS_PART_BINDING_BEGIN)?;
        let lengths = sections.i32_arr(view, table::S_BS_PART_BINDING_COUNT)?;
        for index in 0..counts.blend_shapes_parts as usize {
            let target = i64::from(pick(&targets, index, "blend_shape.part_target")?);
            check_index(
                target,
                counts.parts as u64,
                "blend_shape.part_target",
                index,
            )?;
            let begin = i64::from(pick(&begins, index, "blend_shape.part_binding_begin")?);
            let count = i64::from(pick(&lengths, index, "blend_shape.part_binding_count")?);
            check_range(
                begin,
                count,
                counts.blend_shape_keyform_bindings as u64,
                "blend_shape.part_bindings",
                index,
            )?;
        }

        // Blend shape rotation targets.
        let targets = sections.i32_arr(view, table::S_BS_ROTATION_TARGET)?;
        let begins = sections.i32_arr(view, table::S_BS_ROTATION_BINDING_BEGIN)?;
        let lengths = sections.i32_arr(view, table::S_BS_ROTATION_BINDING_COUNT)?;
        for index in 0..counts.blend_shapes_rotation_deformers as usize {
            let target = i64::from(pick(&targets, index, "blend_shape.rotation_target")?);
            check_index(
                target,
                counts.rotation_deformers as u64,
                "blend_shape.rotation_target",
                index,
            )?;
            let begin = i64::from(pick(&begins, index, "blend_shape.rotation_binding_begin")?);
            let count = i64::from(pick(&lengths, index, "blend_shape.rotation_binding_count")?);
            check_range(
                begin,
                count,
                counts.blend_shape_keyform_bindings as u64,
                "blend_shape.rotation_bindings",
                index,
            )?;
        }

        // Blend shape glue targets.
        let targets = sections.i32_arr(view, table::S_BS_GLUE_TARGET)?;
        let begins = sections.i32_arr(view, table::S_BS_GLUE_BINDING_BEGIN)?;
        let lengths = sections.i32_arr(view, table::S_BS_GLUE_BINDING_COUNT)?;
        for index in 0..counts.blend_shapes_glue as usize {
            let target = i64::from(pick(&targets, index, "blend_shape.glue_target")?);
            check_index(target, counts.glue as u64, "blend_shape.glue_target", index)?;
            let begin = i64::from(pick(&begins, index, "blend_shape.glue_binding_begin")?);
            let count = i64::from(pick(&lengths, index, "blend_shape.glue_binding_count")?);
            check_range(
                begin,
                count,
                counts.blend_shape_keyform_bindings as u64,
                "blend_shape.glue_bindings",
                index,
            )?;
        }
    }

    if version.has_v53_sections() {
        let owner = sections.i32_arr(view, table::S_OFFSCREEN_OWNER_INDEX)?;
        let mask_begin = sections.i32_arr(view, table::S_OFFSCREEN_MASK_BEGIN)?;
        let mask_count = sections.i32_arr(view, table::S_OFFSCREEN_MASK_COUNT)?;
        for index in 0..counts.offscreens as usize {
            let owner_value = i64::from(pick(&owner, index, "offscreen.owner_index")?);
            check_index(
                owner_value,
                counts.parts as u64,
                "offscreen.owner_index",
                index,
            )?;
            let begin = i64::from(pick(&mask_begin, index, "offscreen.mask_begin")?);
            let count = i64::from(pick(&mask_count, index, "offscreen.mask_count")?);
            check_range(
                begin,
                count,
                counts.drawable_masks as u64,
                "offscreen.mask",
                index,
            )?;
        }
        let bs_target = sections.i32_arr(view, table::S_BS_OFFSCREEN_TARGET)?;
        let bs_begin = sections.i32_arr(view, table::S_BS_OFFSCREEN_BINDING_BEGIN)?;
        let bs_count = sections.i32_arr(view, table::S_BS_OFFSCREEN_BINDING_COUNT)?;
        for index in 0..counts.blend_shapes_offscreens as usize {
            let target = i64::from(pick(&bs_target, index, "blend_shape.offscreen_target")?);
            check_index(
                target,
                counts.offscreens as u64,
                "blend_shape.offscreen_target",
                index,
            )?;
            let begin = i64::from(pick(&bs_begin, index, "blend_shape.binding_begin")?);
            let count = i64::from(pick(&bs_count, index, "blend_shape.binding_count")?);
            check_range(
                begin,
                count,
                counts.blend_shape_keyform_bindings as u64,
                "blend_shape.bindings",
                index,
            )?;
        }
    }

    Ok(())
}

fn collect_id_anomalies<'a, I>(anomalies: &mut Vec<Anomaly>, kind: &'static str, ids: &I)
where
    I: Iterator<Item = &'a IdField> + Clone,
{
    for (index, id) in ids.clone().enumerate() {
        if !id.valid_utf8 {
            anomalies.push(Anomaly {
                code: "invalid_utf8_id",
                message: format!("{kind}[{index}] identifier is not valid UTF-8"),
                offset: None,
                index: Some(index),
            });
        }
        if !id.terminated {
            anomalies.push(Anomaly {
                code: "unterminated_id",
                message: format!(
                    "{kind}[{index}] identifier fills all 64 bytes without a terminator"
                ),
                offset: None,
                index: Some(index),
            });
        }
        if id.trailing_bytes_after_terminator {
            anomalies.push(Anomaly {
                code: "trailing_id_bytes",
                message: format!(
                    "{kind}[{index}] identifier has non-zero bytes after the terminator"
                ),
                offset: None,
                index: Some(index),
            });
        }
    }
    // BTreeMap keeps this O(n log n); anomalies are still emitted in
    // first-seen order, so output stays deterministic.
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, id) in ids.clone().enumerate() {
        if id.text.is_empty() {
            anomalies.push(Anomaly {
                code: "empty_id",
                message: format!("{kind}[{index}] identifier is empty"),
                offset: None,
                index: Some(index),
            });
            continue;
        }
        if seen.contains_key(id.text.as_str()) {
            anomalies.push(Anomaly {
                code: "duplicate_id",
                message: format!(
                    "{kind}[{index}] duplicates an earlier identifier '{}'",
                    id.text
                ),
                offset: None,
                index: Some(index),
            });
        } else {
            seen.insert(id.text.as_str(), index);
        }
    }
}

fn build_not_extracted(counts: &CountInfo) -> Vec<NotExtracted> {
    vec![
        NotExtracted {
            field: "keyform_position.xy",
            count: counts.keyform_positions as u64,
            reason: "vertex position values are read in AGENT.2 (Live2D IR)",
        },
        NotExtracted {
            field: "uv.xy",
            count: counts.uvs as u64,
            reason: "UV values are read in AGENT.2 (Live2D IR)",
        },
        NotExtracted {
            field: "position_indices",
            count: counts.position_indices as u64,
            reason: "triangle indices are read in AGENT.2 (Live2D IR)",
        },
        NotExtracted {
            field: "key.values",
            count: counts.keys as u64,
            reason: "key values are read in AGENT.2 (Live2D IR)",
        },
        NotExtracted {
            field: "part_keyform.draw_order",
            count: counts.part_keyforms as u64,
            reason: "keyform values are read in AGENT.2/AGENT.4",
        },
        NotExtracted {
            field: "warp_keyform.values",
            count: counts.warp_deformer_keyforms as u64,
            reason: "keyform values are read in AGENT.2/AGENT.4",
        },
        NotExtracted {
            field: "rotation_keyform.values",
            count: counts.rotation_deformer_keyforms as u64,
            reason: "keyform values are read in AGENT.2/AGENT.4",
        },
        NotExtracted {
            field: "art_mesh_keyform.values",
            count: counts.art_mesh_keyforms as u64,
            reason: "keyform values are read in AGENT.2/AGENT.4",
        },
        NotExtracted {
            field: "glue_info.weight",
            count: counts.glue_info as u64,
            reason: "glue weights are read in AGENT.2 (Live2D IR)",
        },
        NotExtracted {
            field: "keyform_color.*",
            count: (counts.keyform_multiply_colors as u64)
                .saturating_add(counts.keyform_screen_colors as u64),
            reason: "keyform colors are read in AGENT.2 (Live2D IR)",
        },
        NotExtracted {
            field: "blend_shape.*",
            count: [
                counts.blend_shape_parameter_bindings,
                counts.blend_shape_keyform_bindings,
                counts.blend_shapes_warp_deformers,
                counts.blend_shapes_art_meshes,
                counts.blend_shape_constraint_indices,
                counts.blend_shape_constraints,
                counts.blend_shape_constraint_values,
                counts.blend_shapes_parts,
                counts.blend_shapes_rotation_deformers,
                counts.blend_shapes_glue,
                counts.blend_shapes_offscreens,
            ]
            .iter()
            .map(|value| *value as u64)
            .fold(0u64, |acc, value| acc.saturating_add(value)),
            reason: "blend shapes are read in AGENT.2/AGENT.6",
        },
    ]
}

/// Slot definitions for the parsed version (exposed for report rendering).
pub fn slot_defs_for(version: MocVersion) -> &'static [SlotDef] {
    table::defs_for(version)
}
