//! Mapper: raw MOC3 parse result -> normalized Live2D IR.
//!
//! This module lives in `recovery-core` because `live2d-ir` must stay
//! independent from binary parsing. The mapper
//!
//! - assigns stable typed identifiers (source name when unique, otherwise a
//!   deterministic fallback),
//! - resolves stored indices into typed references,
//! - derives only structural facts (reverse links, texture pages, mask
//!   groups) and marks them as `Derived`,
//! - carries every parser anomaly and every lossy decision into the IR
//!   diagnostic list,
//! - never invents names, opacity or draw order values that the file does
//!   not store.

use std::collections::{BTreeMap, BTreeSet};

use live2d_ir::{
    ArtMesh, ArtMeshId, ArtMeshKeyform, Binding, BindingId, BindingParameter, BindingTarget,
    BlendMode, Canvas, Confidence, Deformer, DeformerCommon, DeformerId, Diagnostic,
    DrawOrderGroup, DrawOrderGroupId, DrawOrderItem, DrawOrderTarget, DrawableFlags, Endianness,
    EntityCounts, EntityRef, FieldProvenance, Glue, GlueId, GlueInfoEntry, IdAssigner, IdOutcome,
    Live2DModel, MaskGroup, MaskGroupId, Metadata, Parameter, ParameterId, ParameterKind, Part,
    PartId, Provenance, RotationDeformer, RotationKeyform, RuntimeSectionInfo, SchemaStatus,
    Severity, SourceFormat, Texture, TextureId, UndefinedSlot, UnknownCountField, Unknowns, Uv,
    Vec2, WarpDeformer, WarpKeyform, SCHEMA_ID,
};
use moc3_ingest::raw::{DeformerSpecific, ParameterType};
use moc3_ingest::reader::IdField;
use moc3_ingest::{ModelPools, ParsedModel};

/// Map a parsed MOC3 model into the normalized Live2D IR.
///
/// The returned model already contains parser anomalies, mapper diagnostics
/// and a full validation pass in `diagnostics`.
pub fn map_parsed_model(parsed: ParsedModel) -> Live2DModel {
    let ParsedModel { raw, pools } = parsed;
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    for anomaly in &raw.anomalies {
        diagnostics.push(Diagnostic::new(
            severity_for_anomaly(anomaly.code),
            format!("moc3_{}", anomaly.code),
            anomaly.message.clone(),
        ));
    }
    push_unmapped_section_diagnostics(&raw, &mut diagnostics);

    // ---- identifiers ------------------------------------------------------
    let mut parameter_assigner = IdAssigner::new("parameter");
    let parameter_ids: Vec<ParameterId> = raw
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let source = source_text(&parameter.id);
            let assignment = parameter_assigner.assign(index, source);
            let id = ParameterId::new(assignment.id.clone());
            record_id_outcome(
                &mut diagnostics,
                "parameter",
                source,
                &assignment.id,
                assignment.outcome,
                EntityRef::Parameter(id.clone()),
            );
            id
        })
        .collect();

    let mut part_assigner = IdAssigner::new("part");
    let part_ids: Vec<PartId> = raw
        .parts
        .iter()
        .enumerate()
        .map(|(index, part)| {
            let source = source_text(&part.id);
            let assignment = part_assigner.assign(index, source);
            let id = PartId::new(assignment.id.clone());
            record_id_outcome(
                &mut diagnostics,
                "part",
                source,
                &assignment.id,
                assignment.outcome,
                EntityRef::Part(id.clone()),
            );
            id
        })
        .collect();

    let mut deformer_assigner = IdAssigner::new("deformer");
    let deformer_ids: Vec<DeformerId> = raw
        .deformers
        .iter()
        .enumerate()
        .map(|(index, deformer)| {
            let prefix = match &deformer.specific {
                DeformerSpecific::Warp { .. } => "warp",
                DeformerSpecific::Rotation { .. } => "rotation",
            };
            let source = source_text(&deformer.id);
            let assignment = deformer_assigner.assign_with_prefix(index, source, prefix);
            let id = DeformerId::new(assignment.id.clone());
            record_id_outcome(
                &mut diagnostics,
                "deformer",
                source,
                &assignment.id,
                assignment.outcome,
                EntityRef::Deformer(id.clone()),
            );
            id
        })
        .collect();

    let mut mesh_assigner = IdAssigner::new("artmesh");
    let art_mesh_ids: Vec<ArtMeshId> = raw
        .art_meshes
        .iter()
        .enumerate()
        .map(|(index, mesh)| {
            let source = source_text(&mesh.id);
            let assignment = mesh_assigner.assign(index, source);
            let id = ArtMeshId::new(assignment.id.clone());
            record_id_outcome(
                &mut diagnostics,
                "art_mesh",
                source,
                &assignment.id,
                assignment.outcome,
                EntityRef::ArtMesh(id.clone()),
            );
            id
        })
        .collect();

    let mut binding_assigner = IdAssigner::new("binding");
    let binding_ids: Vec<BindingId> = raw
        .bindings
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let assignment = binding_assigner.assign(index, None);
            BindingId::new(assignment.id)
        })
        .collect();

    let mut glue_assigner = IdAssigner::new("glue");
    let glue_ids: Vec<GlueId> = raw
        .glues
        .iter()
        .enumerate()
        .map(|(index, glue)| {
            let source = source_text(&glue.id);
            let assignment = glue_assigner.assign(index, source);
            let id = GlueId::new(assignment.id.clone());
            record_id_outcome(
                &mut diagnostics,
                "glue",
                source,
                &assignment.id,
                assignment.outcome,
                EntityRef::Glue(id.clone()),
            );
            id
        })
        .collect();

    // ---- textures (derived from referenced page numbers) -------------------
    let mut pages: BTreeSet<u32> = BTreeSet::new();
    for mesh in &raw.art_meshes {
        if let Ok(page) = u32::try_from(mesh.texture_number) {
            pages.insert(page);
        }
    }
    let mut texture_assigner = IdAssigner::new("texture");
    let mut textures: Vec<Texture> = Vec::new();
    let mut texture_by_page: BTreeMap<u32, TextureId> = BTreeMap::new();
    for (index, page) in pages.iter().enumerate() {
        let assignment = texture_assigner.assign(index, None);
        let id = TextureId::new(assignment.id);
        texture_by_page.insert(*page, id.clone());
        textures.push(Texture {
            id,
            page_index: *page,
            width: None,
            height: None,
            source_path: None,
            provenance: Provenance::derived("moc3:art_mesh.texture_number").with_note(
                "page exists because at least one art mesh references it; \
                 dimensions and image path require model3.json",
            ),
        });
    }

    // ---- mask groups -------------------------------------------------------
    let mut mask_assigner = IdAssigner::new("mask");
    let mut mask_groups: Vec<MaskGroup> = Vec::new();
    let mut mask_groups_by_mesh: Vec<Vec<MaskGroupId>> = vec![Vec::new(); raw.art_meshes.len()];
    for (index, mesh) in raw.art_meshes.iter().enumerate() {
        let mut sources: Vec<ArtMeshId> = Vec::new();
        let mut saw_padding = false;
        for mask in &mesh.mask_drawables {
            if *mask < 0 {
                saw_padding = true;
                continue;
            }
            if let Some(id) = usize::try_from(*mask)
                .ok()
                .and_then(|position| art_mesh_ids.get(position))
            {
                sources.push(id.clone());
            }
        }
        if saw_padding {
            if let Some(mesh_id) = art_mesh_ids.get(index) {
                diagnostics.push(Diagnostic::for_entity(
                    Severity::Info,
                    "mask_padding_entry",
                    format!("art mesh '{mesh_id}' mask list contains non-reference (-1) entries"),
                    EntityRef::ArtMesh(mesh_id.clone()),
                ));
            }
        }
        if !sources.is_empty() {
            let assignment = mask_assigner.assign(mask_groups.len(), None);
            let id = MaskGroupId::new(assignment.id);
            let target = art_mesh_ids
                .get(index)
                .cloned()
                .unwrap_or_else(|| ArtMeshId::new(format!("artmesh:{index:06}")));
            mask_groups.push(MaskGroup {
                id: id.clone(),
                target,
                sources,
                provenance: Provenance::exact("moc3:art_mesh.mask").with_note(
                    "sources come from the stored drawable mask list of the target mesh",
                ),
            });
            if let Some(slot) = mask_groups_by_mesh.get_mut(index) {
                slot.push(id);
            }
        }
    }

    // ---- key table -> parameter ownership ---------------------------------
    let mut table_owner: BTreeMap<usize, usize> = BTreeMap::new();
    for (parameter_index, parameter) in raw.parameters.iter().enumerate() {
        let begin = usize::try_from(parameter.key_table_begin.max(0)).unwrap_or(usize::MAX);
        let count = usize::try_from(parameter.key_table_count.max(0)).unwrap_or(0);
        for table in begin..begin.saturating_add(count) {
            table_owner.entry(table).or_insert(parameter_index);
        }
    }

    // ---- parameters --------------------------------------------------------
    let parameters: Vec<Parameter> = raw
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let key_values = collect_parameter_keys(parameter, &raw.key_tables, &pools.key_values);
            let extension_key_values = extension_keys(index, &pools);
            let kind = match parameter.parameter_type {
                Some(ParameterType::BlendShape) => ParameterKind::BlendShape,
                _ => ParameterKind::Normal,
            };
            Parameter {
                id: parameter_ids
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| ParameterId::new(format!("parameter:{index:06}"))),
                source_name: source_text(&parameter.id).map(str::to_string),
                minimum: parameter.minimum,
                maximum: parameter.maximum,
                default: parameter.default,
                current: None,
                repeat: parameter.repeat,
                decimal_places: i32::try_from(parameter.decimal_places).unwrap_or(0),
                kind,
                key_values,
                extension_key_values,
                field_provenance: parameter_field_provenance(&raw, kind),
                provenance: Provenance::exact("moc3:parameter"),
            }
        })
        .collect();

    // ---- parts -------------------------------------------------------------
    // Offscreen surfaces (5.3+) are not modeled in AGENT.2; keep the stored
    // relation visible as a diagnostic instead of a raw index on the entity.
    for (index, part) in raw.parts.iter().enumerate() {
        if let (Some(surface), Some(id)) = (part.offscreen_index, part_ids.get(index)) {
            if surface >= 0 {
                diagnostics.push(Diagnostic::for_entity(
                    Severity::Warning,
                    "unmapped_reference",
                    format!(
                        "part '{id}' references offscreen surface index {surface}; \
                         offscreen surfaces are not modeled in AGENT.2"
                    ),
                    EntityRef::Part(id.clone()),
                ));
            }
        }
    }
    let mut parts: Vec<Part> = raw
        .parts
        .iter()
        .enumerate()
        .map(|(index, part)| Part {
            id: part_ids
                .get(index)
                .cloned()
                .unwrap_or_else(|| PartId::new(format!("part:{index:06}"))),
            source_name: source_text(&part.id).map(str::to_string),
            parent: part
                .parent_part
                .and_then(|parent| part_ids.get(parent as usize))
                .cloned(),
            children: Vec::new(),
            opacity: None,
            draw_order: None,
            keyform_draw_orders: f32_slice(
                &pools.part_keyform_draw_orders,
                part.keyform_begin,
                usize::try_from(part.keyform_count.max(0)).unwrap_or(0),
            )
            .to_vec(),
            binding: binding_ids
                .get(usize::try_from(part.binding_index.max(0)).unwrap_or(usize::MAX))
                .cloned(),
            visible: part.visible,
            enabled: part.enabled,
            field_provenance: vec![
                field_provenance(
                    "opacity",
                    Provenance::unknown(
                        "moc3:part",
                        "base opacity is keyform/runtime dependent and not stored",
                    ),
                ),
                field_provenance(
                    "draw_order",
                    Provenance::unknown(
                        "moc3:part",
                        "base draw order is keyform dependent; see keyform_draw_orders",
                    ),
                ),
            ],
            provenance: Provenance::exact("moc3:part"),
        })
        .collect();

    // Reverse part links (source order).
    for index in 0..parts.len() {
        if let Some(parent) = raw
            .parts
            .get(index)
            .and_then(|part| part.parent_part)
            .and_then(|parent| usize::try_from(parent).ok())
        {
            let child_id = parts.get(index).map(|part| part.id.clone());
            if let (Some(child_id), Some(parent_part)) = (child_id, parts.get_mut(parent)) {
                parent_part.children.push(child_id);
            }
        }
    }

    // ---- deformers ---------------------------------------------------------
    let mut deformers: Vec<Deformer> = Vec::with_capacity(raw.deformers.len());
    for (index, deformer) in raw.deformers.iter().enumerate() {
        let id = deformer_ids
            .get(index)
            .cloned()
            .unwrap_or_else(|| DeformerId::new(format!("deformer:{index:06}")));
        let common = DeformerCommon {
            id,
            source_name: source_text(&deformer.id).map(str::to_string),
            parent_part: deformer
                .parent_part
                .and_then(|parent| part_ids.get(parent as usize))
                .cloned(),
            parent_deformer: deformer
                .parent_deformer
                .and_then(|parent| deformer_ids.get(parent as usize))
                .cloned(),
            children_deformers: Vec::new(),
            children_art_meshes: Vec::new(),
            binding: binding_ids
                .get(usize::try_from(deformer.binding_index.max(0)).unwrap_or(usize::MAX))
                .cloned(),
            visible: deformer.visible,
            enabled: deformer.enabled,
            field_provenance: Vec::new(),
            provenance: Provenance::exact("moc3:deformer"),
        };
        match &deformer.specific {
            DeformerSpecific::Warp {
                vertex_count,
                rows,
                columns,
                quad_transform,
            } => {
                let vertex_count = usize::try_from((*vertex_count).max(0)).unwrap_or(0);
                deformers.push(Deformer::Warp(WarpDeformer {
                    common,
                    rows: u32::try_from((*rows).max(0)).unwrap_or(0),
                    columns: u32::try_from((*columns).max(0)).unwrap_or(0),
                    vertex_count,
                    quad_transform: quad_transform.map(|value| value != 0).unwrap_or(false),
                    keyforms: warp_keyforms(
                        usize::try_from(deformer.local_index.max(0)).unwrap_or(usize::MAX),
                        vertex_count,
                        &pools,
                    ),
                }));
            }
            DeformerSpecific::Rotation { base_angle } => {
                deformers.push(Deformer::Rotation(RotationDeformer {
                    common,
                    base_angle: *base_angle,
                    keyforms: rotation_keyforms(
                        usize::try_from(deformer.local_index.max(0)).unwrap_or(usize::MAX),
                        &pools,
                    ),
                }));
            }
        }
    }

    // Reverse deformer links (source order).
    for index in 0..deformers.len() {
        if let Some(parent) = raw
            .deformers
            .get(index)
            .and_then(|deformer| deformer.parent_deformer)
            .and_then(|parent| usize::try_from(parent).ok())
        {
            let child_id = deformer_common(deformers.get(index)).map(|common| common.id.clone());
            if let (Some(child_id), Some(parent_deformer)) = (child_id, deformers.get_mut(parent)) {
                deformer_common_mut(parent_deformer)
                    .children_deformers
                    .push(child_id);
            }
        }
    }

    // ---- art meshes --------------------------------------------------------
    let mut art_meshes: Vec<ArtMesh> = Vec::with_capacity(raw.art_meshes.len());
    for (index, mesh) in raw.art_meshes.iter().enumerate() {
        let vertex_count = usize::try_from(mesh.vertex_count.max(0)).unwrap_or(0);
        let uvs = uv_slice(&pools.uvs, mesh.uv_begin, vertex_count.saturating_mul(2));
        let indices = u16_slice(
            &pools.indices,
            mesh.index_begin,
            usize::try_from(mesh.index_count.max(0)).unwrap_or(0),
        );
        let keyforms =
            art_mesh_keyforms(mesh.keyform_begin, mesh.keyform_count, vertex_count, &pools);
        let (blend_mode, blend_mode_raw) =
            blend_mode_of(mesh, &mut diagnostics, index, &art_mesh_ids);
        let flags = DrawableFlags {
            additive: mesh.drawable_flags_raw & 0x01 != 0,
            multiplicative: mesh.drawable_flags_raw & 0x02 != 0,
            double_sided: mesh.double_sided,
            inverted_mask: mesh.inverted_mask,
            unknown_bits: mesh.drawable_flags_raw & 0xF0,
        };
        let mesh_id = art_mesh_ids
            .get(index)
            .cloned()
            .unwrap_or_else(|| ArtMeshId::new(format!("artmesh:{index:06}")));
        art_meshes.push(ArtMesh {
            id: mesh_id,
            source_name: source_text(&mesh.id).map(str::to_string),
            parent_part: mesh
                .parent_part
                .and_then(|parent| part_ids.get(parent as usize))
                .cloned(),
            parent_deformer: mesh
                .parent_deformer
                .and_then(|parent| deformer_ids.get(parent as usize))
                .cloned(),
            texture: u32::try_from(mesh.texture_number)
                .ok()
                .and_then(|page| texture_by_page.get(&page))
                .cloned(),
            vertex_count,
            uvs,
            indices: indices.to_vec(),
            keyforms,
            flags,
            blend_mode,
            blend_mode_raw,
            opacity: None,
            draw_order: None,
            binding: binding_ids
                .get(usize::try_from(mesh.binding_index.max(0)).unwrap_or(usize::MAX))
                .cloned(),
            visible: mesh.visible,
            enabled: mesh.enabled,
            mask_groups: mask_groups_by_mesh.get(index).cloned().unwrap_or_default(),
            field_provenance: vec![
                field_provenance(
                    "opacity",
                    Provenance::unknown(
                        "moc3:art_mesh",
                        "base opacity is keyform/runtime dependent and not stored",
                    ),
                ),
                field_provenance(
                    "draw_order",
                    Provenance::unknown(
                        "moc3:art_mesh",
                        "base draw order is keyform dependent; see keyforms[].draw_order",
                    ),
                ),
                field_provenance(
                    "blend_mode",
                    Provenance::derived("moc3:art_mesh.drawable_flags").with_note(
                        "derived from flag bits (pre-5.3) or the blend mode field (5.3+)",
                    ),
                ),
            ],
            provenance: Provenance::exact("moc3:art_mesh"),
        });
    }

    // Art meshes parented to deformers (reverse links).
    for index in 0..art_meshes.len() {
        if let Some(parent) = raw
            .art_meshes
            .get(index)
            .and_then(|mesh| mesh.parent_deformer)
            .and_then(|parent| usize::try_from(parent).ok())
        {
            let child_id = art_meshes.get(index).map(|mesh| mesh.id.clone());
            if let (Some(child_id), Some(parent_deformer)) = (child_id, deformers.get_mut(parent)) {
                deformer_common_mut(parent_deformer)
                    .children_art_meshes
                    .push(child_id);
            }
        }
    }

    // ---- glue --------------------------------------------------------------
    let mut glue_entries: Vec<Glue> = Vec::with_capacity(raw.glues.len());
    for (index, glue) in raw.glues.iter().enumerate() {
        let info_count = usize::try_from(glue.info_count.max(0)).unwrap_or(0);
        let mut info: Vec<GlueInfoEntry> = Vec::with_capacity(info_count.min(1_000_000));
        for slot in 0..info_count {
            let at = usize::try_from(glue.info_begin.max(0))
                .unwrap_or(usize::MAX)
                .saturating_add(slot);
            info.push(GlueInfoEntry {
                weight: pools.glue_info_weight.get(at).copied().unwrap_or_default(),
                position_index: pools
                    .glue_info_position_index
                    .get(at)
                    .copied()
                    .unwrap_or_default(),
            });
        }
        glue_entries.push(Glue {
            id: glue_ids
                .get(index)
                .cloned()
                .unwrap_or_else(|| GlueId::new(format!("glue:{index:06}"))),
            source_name: source_text(&glue.id).map(str::to_string),
            art_mesh_a: glue_mesh_ref(
                glue.art_mesh_a,
                "a",
                index,
                &art_mesh_ids,
                &mut diagnostics,
            ),
            art_mesh_b: glue_mesh_ref(
                glue.art_mesh_b,
                "b",
                index,
                &art_mesh_ids,
                &mut diagnostics,
            ),
            binding: binding_ids
                .get(usize::try_from(glue.binding_index.max(0)).unwrap_or(usize::MAX))
                .cloned(),
            keyform_intensities: f32_slice(
                &pools.glue_keyform_intensity,
                glue.keyform_begin,
                usize::try_from(glue.keyform_count.max(0)).unwrap_or(0),
            )
            .to_vec(),
            info,
            field_provenance: vec![field_provenance(
                "info",
                Provenance::unknown(
                    "moc3:glue.info",
                    "weight/position pairing semantics are not confirmed; entries are preserved raw",
                ),
            )],
            provenance: Provenance::exact("moc3:glue"),
        });
    }

    // ---- bindings ----------------------------------------------------------
    let mut bindings: Vec<Binding> = Vec::with_capacity(raw.bindings.len());
    for (index, binding) in raw.bindings.iter().enumerate() {
        let mut parameters: Vec<BindingParameter> = Vec::new();
        let mut grid: Vec<usize> = Vec::new();
        let mut grid_size: usize = 1;
        let mut unmapped: Vec<usize> = Vec::new();
        for table_index in &binding.key_tables {
            let table_position = usize::try_from((*table_index).max(0)).unwrap_or(usize::MAX);
            let Some(table) = raw.key_tables.get(table_position) else {
                unmapped.push(table_position);
                continue;
            };
            let keys = f32_slice(
                &pools.key_values,
                table.key_begin,
                usize::try_from(table.key_count.max(0)).unwrap_or(0),
            )
            .to_vec();
            match table_owner
                .get(&table_position)
                .and_then(|owner| parameter_ids.get(*owner))
            {
                Some(owner) => {
                    grid.push(keys.len());
                    if keys.len() > 1 {
                        grid_size = grid_size.saturating_mul(keys.len());
                    }
                    parameters.push(BindingParameter {
                        parameter: owner.clone(),
                        key_values: keys,
                    });
                }
                None => unmapped.push(table_position),
            }
        }
        if !unmapped.is_empty() {
            if let Some(binding_id) = binding_ids.get(index) {
                diagnostics.push(Diagnostic::for_entity(
                    Severity::Warning,
                    "orphan_key_table",
                    format!("binding '{binding_id}' references key table(s) not owned by any parameter: {unmapped:?}"),
                    EntityRef::Binding(binding_id.clone()),
                ));
            }
        }
        bindings.push(Binding {
            id: binding_ids
                .get(index)
                .cloned()
                .unwrap_or_else(|| BindingId::new(format!("binding:{index:06}"))),
            keyform_grid_size: grid_size,
            keyform_grid: grid,
            parameters,
            used_by: Vec::new(),
            provenance: Provenance::new(Confidence::Derived, "moc3:binding").with_note(
                "parameters resolved through the parameter key table ranges; reverse links derived",
            ),
        });
    }

    // Reverse binding links (parts, deformers, art meshes, glue).
    push_binding_users(
        &mut bindings,
        &binding_ids,
        &raw,
        &parts,
        &deformers,
        &art_meshes,
        &glue_entries,
    );

    // ---- draw order groups -------------------------------------------------
    let mut draw_group_assigner = IdAssigner::new("drawgroup");
    let draw_group_ids: Vec<DrawOrderGroupId> = raw
        .draw_order_groups
        .iter()
        .enumerate()
        .map(|(index, _)| DrawOrderGroupId::new(draw_group_assigner.assign(index, None).id))
        .collect();
    let mut draw_order_groups: Vec<DrawOrderGroup> =
        Vec::with_capacity(raw.draw_order_groups.len());
    for (index, group) in raw.draw_order_groups.iter().enumerate() {
        let item_count = usize::try_from(group.object_count.max(0)).unwrap_or(0);
        let mut items: Vec<DrawOrderItem> = Vec::with_capacity(item_count.min(1_000_000));
        for slot in 0..item_count {
            let at = usize::try_from(group.object_begin.max(0))
                .unwrap_or(usize::MAX)
                .saturating_add(slot);
            let Some(item) = raw.draw_order_items.get(at) else {
                continue;
            };
            let object = match item.object_type {
                0 => usize::try_from(item.object_index.max(0))
                    .ok()
                    .and_then(|position| art_mesh_ids.get(position))
                    .cloned()
                    .map(DrawOrderTarget::ArtMesh),
                1 => usize::try_from(item.object_index.max(0))
                    .ok()
                    .and_then(|position| part_ids.get(position))
                    .cloned()
                    .map(DrawOrderTarget::Part),
                _ => None,
            };
            let Some(object) = object else {
                continue;
            };
            let is_part = matches!(object, DrawOrderTarget::Part(_));
            let self_group = if item.self_group >= 0 {
                usize::try_from(item.self_group)
                    .ok()
                    .and_then(|position| draw_group_ids.get(position))
                    .cloned()
            } else {
                None
            };
            if is_part && self_group.is_none() {
                if let Some(group_id) = draw_group_ids.get(index) {
                    diagnostics.push(Diagnostic::for_entity(
                        Severity::Warning,
                        "invalid_reference",
                        format!(
                            "draw order group '{group_id}' has a part item without a nested group"
                        ),
                        EntityRef::Model(Default::default()),
                    ));
                }
            }
            items.push(DrawOrderItem { object, self_group });
        }
        if let Some(id) = draw_group_ids.get(index) {
            draw_order_groups.push(DrawOrderGroup {
                id: id.clone(),
                items,
                maximum_order: i32::try_from(group.max_order).unwrap_or(0),
                minimum_order: i32::try_from(group.min_order).unwrap_or(0),
                provenance: Provenance::exact("moc3:draw_group"),
            });
        }
    }

    // ---- model assembly ----------------------------------------------------
    let source = SourceFormat {
        format: "moc3".to_string(),
        version_byte: raw.header.version_byte,
        version_label: raw.header.version_label.clone(),
        byte_order: endianness_of(raw.header.byte_order),
    };
    let unknowns = Unknowns {
        header_reserved_nonzero_bytes: raw.header.header_padding_nonzero.len(),
        undefined_offset_slots: raw
            .unknown_slots
            .iter()
            .map(|slot| UndefinedSlot {
                slot: slot.index,
                value: slot.value,
            })
            .collect(),
        unknown_count_fields: raw
            .counts
            .unknown_nonzero
            .iter()
            .map(|(index, value)| UnknownCountField {
                index: *index,
                value: *value,
            })
            .collect(),
        runtime_sections: raw
            .runtime_sections
            .iter()
            .filter(|section| section.elements > 0)
            .map(|section| RuntimeSectionInfo {
                name: section.name.to_string(),
                element_count: section.elements,
                byte_size: section.byte_size,
            })
            .collect(),
    };
    let metadata = Metadata {
        generator: "live2d-recovery".to_string(),
        generator_phase: "AGENT.2".to_string(),
        entity_counts: EntityCounts {
            parameters: 0,
            parts: 0,
            deformers: 0,
            warp_deformers: 0,
            rotation_deformers: 0,
            art_meshes: 0,
            draw_order_groups: 0,
            mask_groups: 0,
            textures: 0,
            glue: 0,
            bindings: 0,
        },
    };
    let canvas = build_canvas(&raw);
    let mut model = Live2DModel {
        schema: SCHEMA_ID.to_string(),
        schema_status: SchemaStatus::Experimental,
        source,
        metadata,
        canvas,
        parameters,
        parts,
        deformers,
        art_meshes,
        drawables: art_mesh_ids.clone(),
        draw_order_groups,
        mask_groups,
        textures,
        glue: glue_entries,
        bindings,
        unknowns,
        diagnostics,
    };
    model.metadata.entity_counts = model.entity_counts();
    let validation = live2d_ir::validate_ir(&model);
    model.diagnostics.extend(validation);
    model
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn source_text(id: &IdField) -> Option<&str> {
    if id.text.is_empty() {
        None
    } else {
        Some(id.text.as_str())
    }
}

fn f32_slice(pool: &[f32], begin: i64, len: usize) -> &[f32] {
    let start = match usize::try_from(begin) {
        Ok(start) => start,
        Err(_) => return &[],
    };
    pool.get(start..)
        .and_then(|rest| rest.get(..len))
        .unwrap_or(&[])
}

fn u16_slice(pool: &[u16], begin: i64, len: usize) -> &[u16] {
    let start = match usize::try_from(begin) {
        Ok(start) => start,
        Err(_) => return &[],
    };
    pool.get(start..)
        .and_then(|rest| rest.get(..len))
        .unwrap_or(&[])
}

fn vec2_slice(values: &[f32]) -> Vec<Vec2> {
    let mut out = Vec::with_capacity(values.len() / 2);
    for pair in values.chunks_exact(2) {
        if let [x, y] = pair {
            out.push(Vec2::new(*x, *y));
        }
    }
    out
}

fn uv_slice(values: &[f32], begin: i64, len: usize) -> Vec<Uv> {
    let raw = f32_slice(values, begin, len);
    let mut out = Vec::with_capacity(raw.len() / 2);
    for pair in raw.chunks_exact(2) {
        if let [u, v] = pair {
            out.push(Uv::new(*u, *v));
        }
    }
    out
}

fn field_provenance(field: &str, provenance: Provenance) -> FieldProvenance {
    FieldProvenance {
        field: field.to_string(),
        provenance,
    }
}

fn record_id_outcome(
    diagnostics: &mut Vec<Diagnostic>,
    kind: &str,
    source: Option<&str>,
    id: &str,
    outcome: IdOutcome,
    entity: EntityRef,
) {
    match outcome {
        IdOutcome::SourceName => {}
        IdOutcome::EmptySourceName => diagnostics.push(Diagnostic::for_entity(
            Severity::Recoverable,
            "empty_source_id",
            format!("{kind} has an empty stored identifier; deterministic id '{id}' used"),
            entity,
        )),
        IdOutcome::DuplicateSourceName => diagnostics.push(Diagnostic::for_entity(
            Severity::Recoverable,
            "duplicate_source_id",
            format!(
                "{kind} stored identifier '{}' is duplicated; deterministic id '{id}' used",
                source.unwrap_or("")
            ),
            entity,
        )),
    }
}

fn severity_for_anomaly(code: &str) -> Severity {
    match code {
        "header_padding_nonzero"
        | "unknown_offset_slot"
        | "unknown_count_nonzero"
        | "unknown_canvas_flag_bits"
        | "unknown_drawable_flag_bits" => Severity::Info,
        "invalid_utf8_id" | "empty_id" | "duplicate_id" => Severity::Recoverable,
        _ => Severity::Warning,
    }
}

fn push_unmapped_section_diagnostics(
    raw: &moc3_ingest::RawMoc3,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let counts = &raw.counts;
    let mut unmapped: Vec<String> = Vec::new();
    let mut note = |name: &str, value: u32| {
        if value > 0 {
            unmapped.push(format!("{name}={value}"));
        }
    };
    note(
        "blend_shape_parameter_bindings",
        counts.blend_shape_parameter_bindings,
    );
    note(
        "blend_shape_keyform_bindings",
        counts.blend_shape_keyform_bindings,
    );
    note(
        "blend_shapes_warp_deformers",
        counts.blend_shapes_warp_deformers,
    );
    note("blend_shapes_art_meshes", counts.blend_shapes_art_meshes);
    note(
        "blend_shape_constraint_indices",
        counts.blend_shape_constraint_indices,
    );
    note("blend_shape_constraints", counts.blend_shape_constraints);
    note(
        "blend_shape_constraint_values",
        counts.blend_shape_constraint_values,
    );
    note("blend_shapes_parts", counts.blend_shapes_parts);
    note(
        "blend_shapes_rotation_deformers",
        counts.blend_shapes_rotation_deformers,
    );
    note("blend_shapes_glue", counts.blend_shapes_glue);
    note("blend_shapes_offscreens", counts.blend_shapes_offscreens);
    note("offscreens", counts.offscreens);
    note("offscreen_keyforms", counts.offscreen_keyforms);
    note("keyform_multiply_colors", counts.keyform_multiply_colors);
    note("keyform_screen_colors", counts.keyform_screen_colors);
    if !unmapped.is_empty() {
        diagnostics.push(Diagnostic::new(
            Severity::Warning,
            "unmapped_sections",
            format!(
                "source sections present but not mapped in AGENT.2: {}",
                unmapped.join(", ")
            ),
        ));
    }
}

fn collect_parameter_keys(
    parameter: &moc3_ingest::raw::RawParameter,
    key_tables: &[moc3_ingest::raw::RawKeyTable],
    key_values: &[f32],
) -> Vec<f32> {
    let begin = usize::try_from(parameter.key_table_begin.max(0)).unwrap_or(usize::MAX);
    let count = usize::try_from(parameter.key_table_count.max(0)).unwrap_or(0);
    let mut out: Vec<f32> = Vec::new();
    for table_position in begin..begin.saturating_add(count) {
        if let Some(table) = key_tables.get(table_position) {
            out.extend_from_slice(f32_slice(
                key_values,
                table.key_begin,
                usize::try_from(table.key_count.max(0)).unwrap_or(0),
            ));
        }
    }
    out
}

fn extension_keys(index: usize, pools: &ModelPools) -> Vec<f32> {
    let Some(begin) = pools.parameter_extension_key_begin.get(index).copied() else {
        return Vec::new();
    };
    let Some(count) = pools.parameter_extension_key_count.get(index).copied() else {
        return Vec::new();
    };
    f32_slice(
        &pools.key_values,
        i64::from(begin),
        usize::try_from(count.max(0)).unwrap_or(0),
    )
    .to_vec()
}

fn warp_keyforms(warp_index: usize, vertex_count: usize, pools: &ModelPools) -> Vec<WarpKeyform> {
    let begin = pools
        .warp_keyform_begin
        .get(warp_index)
        .copied()
        .unwrap_or(0);
    let count = pools
        .warp_keyform_count
        .get(warp_index)
        .copied()
        .unwrap_or(0)
        .max(0) as usize;
    let mut out: Vec<WarpKeyform> = Vec::with_capacity(count.min(1_000_000));
    for keyform in 0..count {
        let global = usize::try_from(begin.max(0))
            .unwrap_or(usize::MAX)
            .saturating_add(keyform);
        let opacity = pools
            .warp_keyform_opacity
            .get(global)
            .copied()
            .unwrap_or_default();
        let position_begin = pools
            .warp_keyform_position_begin
            .get(global)
            .copied()
            .unwrap_or(0);
        let positions = vec2_slice(f32_slice(
            &pools.keyform_positions,
            i64::from(position_begin),
            vertex_count.saturating_mul(2),
        ));
        out.push(WarpKeyform {
            index: keyform,
            opacity,
            positions,
        });
    }
    out
}

fn rotation_keyforms(rotation_index: usize, pools: &ModelPools) -> Vec<RotationKeyform> {
    let begin = pools
        .rotation_keyform_begin
        .get(rotation_index)
        .copied()
        .unwrap_or(0);
    let count = pools
        .rotation_keyform_count
        .get(rotation_index)
        .copied()
        .unwrap_or(0)
        .max(0) as usize;
    let mut out: Vec<RotationKeyform> = Vec::with_capacity(count.min(1_000_000));
    for keyform in 0..count {
        let global = usize::try_from(begin.max(0))
            .unwrap_or(usize::MAX)
            .saturating_add(keyform);
        out.push(RotationKeyform {
            index: keyform,
            opacity: pools
                .rotation_keyform_opacity
                .get(global)
                .copied()
                .unwrap_or_default(),
            angle: pools
                .rotation_keyform_angle
                .get(global)
                .copied()
                .unwrap_or_default(),
            origin: Vec2::new(
                pools
                    .rotation_keyform_origin_x
                    .get(global)
                    .copied()
                    .unwrap_or_default(),
                pools
                    .rotation_keyform_origin_y
                    .get(global)
                    .copied()
                    .unwrap_or_default(),
            ),
            scale: pools
                .rotation_keyform_scale
                .get(global)
                .copied()
                .unwrap_or_default(),
            reflect_x: pools
                .rotation_keyform_reflect_x
                .get(global)
                .copied()
                .unwrap_or(0)
                != 0,
            reflect_y: pools
                .rotation_keyform_reflect_y
                .get(global)
                .copied()
                .unwrap_or(0)
                != 0,
        });
    }
    out
}

fn art_mesh_keyforms(
    keyform_begin: i64,
    keyform_count: i64,
    vertex_count: usize,
    pools: &ModelPools,
) -> Vec<ArtMeshKeyform> {
    let begin = usize::try_from(keyform_begin.max(0)).unwrap_or(usize::MAX);
    let count = usize::try_from(keyform_count.max(0)).unwrap_or(0);
    let mut out: Vec<ArtMeshKeyform> = Vec::with_capacity(count.min(1_000_000));
    for keyform in 0..count {
        let global = begin.saturating_add(keyform);
        let position_begin = pools
            .art_mesh_keyform_position_begin
            .get(global)
            .copied()
            .unwrap_or(0);
        out.push(ArtMeshKeyform {
            index: keyform,
            opacity: pools
                .art_mesh_keyform_opacity
                .get(global)
                .copied()
                .unwrap_or_default(),
            draw_order: pools
                .art_mesh_keyform_draw_order
                .get(global)
                .copied()
                .unwrap_or_default(),
            positions: vec2_slice(f32_slice(
                &pools.keyform_positions,
                i64::from(position_begin),
                vertex_count.saturating_mul(2),
            )),
        });
    }
    out
}

fn blend_mode_of(
    mesh: &moc3_ingest::raw::RawArtMesh,
    diagnostics: &mut Vec<Diagnostic>,
    index: usize,
    art_mesh_ids: &[ArtMeshId],
) -> (BlendMode, Option<i32>) {
    if let Some(raw) = mesh.blend_mode_5_3 {
        let raw32 = i32::try_from(raw).unwrap_or(i32::MIN);
        let mode = blend_mode_from_raw(raw32);
        if mode == BlendMode::Unknown {
            if let Some(id) = art_mesh_ids.get(index) {
                diagnostics.push(Diagnostic::for_entity(
                    Severity::Warning,
                    "unknown_blend_mode",
                    format!("art mesh '{id}' uses blend mode value {raw32} this build does not recognize"),
                    EntityRef::ArtMesh(id.clone()),
                ));
            }
        }
        (mode, Some(raw32))
    } else {
        let additive = mesh.drawable_flags_raw & 0x01 != 0;
        let multiplicative = mesh.drawable_flags_raw & 0x02 != 0;
        if additive && multiplicative {
            if let Some(id) = art_mesh_ids.get(index) {
                diagnostics.push(Diagnostic::for_entity(
                    Severity::Warning,
                    "conflicting_blend_bits",
                    format!(
                        "art mesh '{id}' sets both the additive and multiplicative flag bits; \
                         treating the mesh as additive (raw flags preserved)"
                    ),
                    EntityRef::ArtMesh(id.clone()),
                ));
            }
        }
        let mode = if additive {
            BlendMode::AddCompatible
        } else if multiplicative {
            BlendMode::MultiplyCompatible
        } else {
            BlendMode::Normal
        };
        (mode, None)
    }
}

fn blend_mode_from_raw(raw: i32) -> BlendMode {
    match raw {
        0 => BlendMode::Normal,
        1 => BlendMode::AddCompatible,
        2 => BlendMode::MultiplyCompatible,
        3 => BlendMode::Add,
        4 => BlendMode::AddGlow,
        5 => BlendMode::Darken,
        6 => BlendMode::Multiply,
        7 => BlendMode::ColorBurn,
        8 => BlendMode::LinearBurn,
        9 => BlendMode::Lighten,
        10 => BlendMode::Screen,
        11 => BlendMode::ColorDodge,
        12 => BlendMode::Overlay,
        13 => BlendMode::SoftLight,
        14 => BlendMode::HardLight,
        15 => BlendMode::LinearLight,
        16 => BlendMode::Hue,
        17 => BlendMode::Color,
        _ => BlendMode::Unknown,
    }
}

/// Resolve a glue art mesh reference.
///
/// Parsed files are already validated, so the invalid branch is defensive
/// (only reachable for hand-edited IR inputs); it produces a deterministic
/// error placeholder id that can never collide with the documented ID policy
/// and is always reported.
fn glue_mesh_ref(
    raw_index: i64,
    side: &'static str,
    glue_index: usize,
    art_mesh_ids: &[ArtMeshId],
    diagnostics: &mut Vec<Diagnostic>,
) -> ArtMeshId {
    if let Some(id) = usize::try_from(raw_index.max(0))
        .ok()
        .and_then(|position| art_mesh_ids.get(position))
    {
        return id.clone();
    }
    let placeholder = ArtMeshId::new(format!("artmesh:invalid:{side}:{glue_index:06}"));
    diagnostics.push(Diagnostic::for_entity(
        Severity::Warning,
        "invalid_reference",
        format!(
            "glue[{glue_index}] art_mesh_{side} index {raw_index} does not resolve; \
             placeholder '{placeholder}' used and reported"
        ),
        EntityRef::ArtMesh(placeholder.clone()),
    ));
    placeholder
}

fn parameter_field_provenance(
    raw: &moc3_ingest::RawMoc3,
    kind: ParameterKind,
) -> Vec<FieldProvenance> {
    let mut entries = vec![field_provenance(
        "current",
        Provenance::unknown(
            "moc3:parameter",
            "current value is runtime state and is not stored in the file",
        ),
    )];
    if raw.header.version_byte < 4 && kind == ParameterKind::Normal {
        entries.push(field_provenance(
            "kind",
            Provenance::derived("moc3:parameter.type").with_note(
                "the explicit parameter type field exists from format version 4 (4.2); \
                 earlier files are mapped to normal",
            ),
        ));
    }
    entries
}

fn deformer_common(deformer: Option<&Deformer>) -> Option<&DeformerCommon> {
    match deformer {
        Some(Deformer::Warp(warp)) => Some(&warp.common),
        Some(Deformer::Rotation(rotation)) => Some(&rotation.common),
        None => None,
    }
}

fn deformer_common_mut(deformer: &mut Deformer) -> &mut DeformerCommon {
    match deformer {
        Deformer::Warp(warp) => &mut warp.common,
        Deformer::Rotation(rotation) => &mut rotation.common,
    }
}

fn push_binding_users(
    bindings: &mut [Binding],
    binding_ids: &[BindingId],
    raw: &moc3_ingest::RawMoc3,
    parts: &[Part],
    deformers: &[Deformer],
    art_meshes: &[ArtMesh],
    glue: &[Glue],
) {
    let binding_position = |index: i64| -> Option<usize> {
        usize::try_from(index.max(0))
            .ok()
            .filter(|position| *position < binding_ids.len())
    };
    // Parts.
    for (index, part) in raw.parts.iter().enumerate() {
        let (Some(position), Some(target)) = (
            binding_position(part.binding_index),
            parts.get(index).map(|part| part.id.clone()),
        ) else {
            continue;
        };
        if let Some(binding) = bindings.get_mut(position) {
            binding.used_by.push(BindingTarget::Part(target));
        }
    }
    // Deformers.
    for (index, deformer) in raw.deformers.iter().enumerate() {
        let (Some(position), Some(id)) = (
            binding_position(deformer.binding_index),
            deformer_common(deformers.get(index)).map(|common| common.id.clone()),
        ) else {
            continue;
        };
        if let Some(binding) = bindings.get_mut(position) {
            binding.used_by.push(BindingTarget::Deformer(id));
        }
    }
    // Art meshes.
    for (index, mesh) in raw.art_meshes.iter().enumerate() {
        let (Some(position), Some(target)) = (
            binding_position(mesh.binding_index),
            art_meshes.get(index).map(|mesh| mesh.id.clone()),
        ) else {
            continue;
        };
        if let Some(binding) = bindings.get_mut(position) {
            binding.used_by.push(BindingTarget::ArtMesh(target));
        }
    }
    // Glue.
    for (index, glue_entry) in raw.glues.iter().enumerate() {
        let (Some(position), Some(target)) = (
            binding_position(glue_entry.binding_index),
            glue.get(index).map(|glue| glue.id.clone()),
        ) else {
            continue;
        };
        if let Some(binding) = bindings.get_mut(position) {
            binding.used_by.push(BindingTarget::Glue(target));
        }
    }
}

fn endianness_of(order: moc3_ingest::ByteOrder) -> Endianness {
    match order {
        moc3_ingest::ByteOrder::Little => Endianness::Little,
        moc3_ingest::ByteOrder::Big => Endianness::Big,
    }
}

fn build_canvas(raw: &moc3_ingest::RawMoc3) -> Canvas {
    Canvas {
        width: raw.canvas.width,
        height: raw.canvas.height,
        origin: Vec2::new(raw.canvas.origin_x, raw.canvas.origin_y),
        pixels_per_unit: raw.canvas.pixels_per_unit,
        y_axis_reversed: raw.canvas.flag_y_reversed,
        field_provenance: vec![
            field_provenance(
                "origin",
                Provenance::exact("moc3:canvas_info.origin").with_note(
                    "stored values; the exact coordinate meaning is only partially confirmed",
                ),
            ),
            field_provenance(
                "y_axis_reversed",
                Provenance::derived("moc3:canvas_info.flags")
                    .with_note("bit 0 of the canvas flag byte"),
            ),
        ],
        provenance: Provenance::exact("moc3:canvas_info"),
    }
}
