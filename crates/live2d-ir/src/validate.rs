//! IR validation: structural, referential and numeric checks.
//!
//! `validate_ir` is deterministic and allocation-bounded by the model size.
//! `Fatal` diagnostics mean the IR must not be exported or consumed; the
//! `Warning`/`Recoverable`/`Info` severities describe conditions that are
//! preserved for later phases.
//!
//! Hierarchy cycle *recovery* is AGENT.3 work; this validator only reports
//! cycles that exist in the stored relations.

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostics::{Diagnostic, EntityRef, Severity};
use crate::model::{Deformer, DrawOrderTarget, Live2DModel, SCHEMA_ID};
use crate::provenance::Confidence;

/// Validate an IR model and return all diagnostics in deterministic order.
pub fn validate_ir(model: &Live2DModel) -> Vec<Diagnostic> {
    let mut validator = Validator {
        model,
        out: Vec::new(),
    };
    validator.run();
    validator.out
}

struct Validator<'a> {
    model: &'a Live2DModel,
    out: Vec<Diagnostic>,
}

impl<'a> Validator<'a> {
    fn push(&mut self, severity: Severity, code: &str, message: String, entity: Option<EntityRef>) {
        self.out.push(Diagnostic {
            severity,
            code: code.to_string(),
            message,
            entity,
        });
    }

    fn fatal(&mut self, code: &str, message: String, entity: Option<EntityRef>) {
        self.push(Severity::Fatal, code, message, entity);
    }

    fn run(&mut self) {
        self.check_schema();
        self.check_metadata();
        let sets = IdSets::collect(self.model);
        self.check_duplicate_ids(&sets);
        self.check_textures();
        self.check_drawables(&sets);
        self.check_parameters();
        self.check_canvas();
        self.check_parts(&sets);
        self.check_deformers(&sets);
        self.check_art_meshes(&sets);
        self.check_draw_order_groups(&sets);
        self.check_mask_groups(&sets);
        self.check_glue(&sets);
        self.check_bindings(&sets);
        self.check_cycles();
    }

    fn check_schema(&mut self) {
        if self.model.schema != SCHEMA_ID {
            self.fatal(
                "schema_mismatch",
                format!(
                    "expected schema '{SCHEMA_ID}', found '{}'",
                    self.model.schema
                ),
                Some(EntityRef::Model(Default::default())),
            );
        }
    }

    fn check_metadata(&mut self) {
        let counts = self.model.entity_counts();
        let declared = &self.model.metadata.entity_counts;
        let pairs: [(&str, usize, usize); 11] = [
            ("parameters", declared.parameters, counts.parameters),
            ("parts", declared.parts, counts.parts),
            ("deformers", declared.deformers, counts.deformers),
            (
                "warp_deformers",
                declared.warp_deformers,
                counts.warp_deformers,
            ),
            (
                "rotation_deformers",
                declared.rotation_deformers,
                counts.rotation_deformers,
            ),
            ("art_meshes", declared.art_meshes, counts.art_meshes),
            (
                "draw_order_groups",
                declared.draw_order_groups,
                counts.draw_order_groups,
            ),
            ("mask_groups", declared.mask_groups, counts.mask_groups),
            ("textures", declared.textures, counts.textures),
            ("glue", declared.glue, counts.glue),
            ("bindings", declared.bindings, counts.bindings),
        ];
        for (field, declared_value, actual) in pairs {
            if declared_value != actual {
                self.fatal(
                    "metadata_count_mismatch",
                    format!(
                        "metadata.entity_counts.{field} = {declared_value} but the model has {actual}"
                    ),
                    Some(EntityRef::Model(Default::default())),
                );
            }
        }
    }

    fn check_duplicate_ids(&mut self, sets: &IdSets) {
        let parameter_duplicates =
            sets.duplicates_in(self.model.parameters.iter().map(|p| &p.id.0));
        for duplicate in parameter_duplicates {
            self.fatal(
                "duplicate_id",
                format!("duplicate parameter identifier '{duplicate}'"),
                Some(EntityRef::Parameter(crate::ids::ParameterId::new(
                    duplicate,
                ))),
            );
        }
        let part_duplicates = sets.duplicates_in(self.model.parts.iter().map(|p| &p.id.0));
        for duplicate in part_duplicates {
            self.fatal(
                "duplicate_id",
                format!("duplicate part identifier '{duplicate}'"),
                Some(EntityRef::Part(crate::ids::PartId::new(duplicate))),
            );
        }
        let deformer_duplicates =
            sets.duplicates_in(self.model.deformers.iter().map(|deformer| match deformer {
                Deformer::Warp(warp) => &warp.common.id.0,
                Deformer::Rotation(rotation) => &rotation.common.id.0,
            }));
        for duplicate in deformer_duplicates {
            self.fatal(
                "duplicate_id",
                format!("duplicate deformer identifier '{duplicate}'"),
                Some(EntityRef::Deformer(crate::ids::DeformerId::new(duplicate))),
            );
        }
        let mesh_duplicates = sets.duplicates_in(self.model.art_meshes.iter().map(|m| &m.id.0));
        for duplicate in mesh_duplicates {
            self.fatal(
                "duplicate_id",
                format!("duplicate art mesh identifier '{duplicate}'"),
                Some(EntityRef::ArtMesh(crate::ids::ArtMeshId::new(duplicate))),
            );
        }
        let mask_duplicates = sets.duplicates_in(self.model.mask_groups.iter().map(|m| &m.id.0));
        for duplicate in mask_duplicates {
            self.fatal(
                "duplicate_id",
                format!("duplicate mask group identifier '{duplicate}'"),
                Some(EntityRef::MaskGroup(crate::ids::MaskGroupId::new(
                    duplicate,
                ))),
            );
        }
        let texture_duplicates = sets.duplicates_in(self.model.textures.iter().map(|t| &t.id.0));
        for duplicate in texture_duplicates {
            self.fatal(
                "duplicate_id",
                format!("duplicate texture identifier '{duplicate}'"),
                Some(EntityRef::Texture(crate::ids::TextureId::new(duplicate))),
            );
        }
        let glue_duplicates = sets.duplicates_in(self.model.glue.iter().map(|g| &g.id.0));
        for duplicate in glue_duplicates {
            self.fatal(
                "duplicate_id",
                format!("duplicate glue identifier '{duplicate}'"),
                Some(EntityRef::Glue(crate::ids::GlueId::new(duplicate))),
            );
        }
        let binding_duplicates = sets.duplicates_in(self.model.bindings.iter().map(|b| &b.id.0));
        for duplicate in binding_duplicates {
            self.fatal(
                "duplicate_id",
                format!("duplicate binding identifier '{duplicate}'"),
                Some(EntityRef::Binding(crate::ids::BindingId::new(duplicate))),
            );
        }
    }

    fn check_textures(&mut self) {
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        for texture in &self.model.textures {
            if !seen.insert(texture.page_index) {
                self.fatal(
                    "duplicate_texture_page",
                    format!("texture page {} appears more than once", texture.page_index),
                    Some(EntityRef::Texture(texture.id.clone())),
                );
            }
        }
        for pair in self.model.textures.windows(2) {
            if let [first, second] = pair {
                if first.page_index > second.page_index {
                    self.fatal(
                        "non_canonical_order",
                        "textures must ascend by page number".to_string(),
                        Some(EntityRef::Texture(second.id.clone())),
                    );
                }
            }
        }
    }

    fn check_drawables(&mut self, sets: &IdSets) {
        let expected: Vec<&str> = self
            .model
            .art_meshes
            .iter()
            .map(|mesh| mesh.id.as_str())
            .collect();
        let actual: Vec<&str> = self.model.drawables.iter().map(|id| id.as_str()).collect();
        if expected != actual {
            self.fatal(
                "drawables_mismatch",
                "drawables must list art mesh ids in source order".to_string(),
                None,
            );
        }
        for id in &self.model.drawables {
            if !sets.art_meshes.contains(id.as_str()) {
                self.fatal(
                    "dangling_reference",
                    format!("drawables references unknown art mesh '{id}'"),
                    None,
                );
            }
        }
    }

    fn check_parameters(&mut self) {
        for parameter in &self.model.parameters {
            let entity = EntityRef::Parameter(parameter.id.clone());
            if parameter.current.is_some() {
                self.fatal(
                    "field_not_stored",
                    format!(
                        "parameter '{}' has a current value, which is not stored in .moc3",
                        parameter.id
                    ),
                    Some(entity.clone()),
                );
            } else {
                expect_field_provenance(
                    &mut self.out,
                    parameter.field_provenance.as_slice(),
                    "current",
                    &entity,
                );
            }
            check_finite(
                &mut self.out,
                parameter.minimum,
                "parameter.minimum",
                parameter.id.as_str(),
                entity.clone(),
            );
            check_finite(
                &mut self.out,
                parameter.maximum,
                "parameter.maximum",
                parameter.id.as_str(),
                entity.clone(),
            );
            check_finite(
                &mut self.out,
                parameter.default,
                "parameter.default",
                parameter.id.as_str(),
                entity.clone(),
            );
            if !(parameter.minimum <= parameter.default && parameter.default <= parameter.maximum) {
                self.fatal(
                    "parameter_range_invalid",
                    format!(
                        "parameter '{}' must satisfy minimum <= default <= maximum ({} <= {} <= {})",
                        parameter.id, parameter.minimum, parameter.default, parameter.maximum
                    ),
                    Some(entity.clone()),
                );
            }
            for (index, value) in parameter.key_values.iter().enumerate() {
                check_finite(
                    &mut self.out,
                    *value,
                    &format!("parameter.key_values[{index}]"),
                    parameter.id.as_str(),
                    entity.clone(),
                );
            }
            for (index, value) in parameter.extension_key_values.iter().enumerate() {
                check_finite(
                    &mut self.out,
                    *value,
                    &format!("parameter.extension_key_values[{index}]"),
                    parameter.id.as_str(),
                    entity.clone(),
                );
            }
        }
    }

    fn check_canvas(&mut self) {
        let canvas = &self.model.canvas;
        for (value, field) in [
            (canvas.width, "canvas.width"),
            (canvas.height, "canvas.height"),
            (canvas.origin.x, "canvas.origin.x"),
            (canvas.origin.y, "canvas.origin.y"),
            (canvas.pixels_per_unit, "canvas.pixels_per_unit"),
        ] {
            if !value.is_finite() {
                self.fatal(
                    "non_finite_value",
                    format!("{field} is not finite"),
                    Some(EntityRef::Model(Default::default())),
                );
            }
        }
    }

    fn check_parts(&mut self, sets: &IdSets) {
        for part in &self.model.parts {
            let entity = EntityRef::Part(part.id.clone());
            if let Some(parent) = &part.parent {
                if !sets.parts.contains(parent.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!("part '{}' references unknown parent '{parent}'", part.id),
                        Some(entity.clone()),
                    );
                }
            }
            for child in &part.children {
                if !sets.parts.contains(child.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!("part '{}' lists unknown child '{child}'", part.id),
                        Some(entity.clone()),
                    );
                    continue;
                }
                let reverse_ok = self
                    .model
                    .parts
                    .iter()
                    .find(|candidate| &candidate.id == child)
                    .map(|candidate| candidate.parent.as_ref() == Some(&part.id))
                    .unwrap_or(false);
                if !reverse_ok {
                    self.fatal(
                        "inconsistent_reverse_link",
                        format!(
                            "part '{}' lists '{}' as a child but the link is not reciprocal",
                            part.id, child
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            if let Some(binding) = &part.binding {
                if !sets.bindings.contains(binding.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!("part '{}' references unknown binding '{binding}'", part.id),
                        Some(entity.clone()),
                    );
                }
            }
            for (index, value) in part.keyform_draw_orders.iter().enumerate() {
                check_finite(
                    &mut self.out,
                    *value,
                    &format!("part.keyform_draw_orders[{index}]"),
                    part.id.as_str(),
                    entity.clone(),
                );
            }
            if part.opacity.is_some() || part.draw_order.is_some() {
                self.fatal(
                    "field_not_stored",
                    format!(
                        "part '{}' has a base opacity/draw order, which is keyform-dependent and not stored in .moc3",
                        part.id
                    ),
                    Some(entity.clone()),
                );
            } else {
                expect_field_provenance(
                    &mut self.out,
                    part.field_provenance.as_slice(),
                    "opacity",
                    &entity,
                );
                expect_field_provenance(
                    &mut self.out,
                    part.field_provenance.as_slice(),
                    "draw_order",
                    &entity,
                );
            }
        }
        // Reverse-link completeness: a part that declares a parent must be
        // listed by that parent.
        for part in &self.model.parts {
            let Some(parent_id) = &part.parent else {
                continue;
            };
            let listed = sets
                .part_children
                .get(parent_id.as_str())
                .map(|children| children.contains(&part.id))
                .unwrap_or(false);
            if !listed {
                self.fatal(
                    "inconsistent_reverse_link",
                    format!(
                        "part '{}' declares parent '{parent_id}' that does not list it as a child",
                        part.id
                    ),
                    Some(EntityRef::Part(part.id.clone())),
                );
            }
        }
    }

    fn check_deformers(&mut self, sets: &IdSets) {
        for deformer in &self.model.deformers {
            let (id, common) = match deformer {
                Deformer::Warp(warp) => (warp.common.id.clone(), &warp.common),
                Deformer::Rotation(rotation) => (rotation.common.id.clone(), &rotation.common),
            };
            let entity = EntityRef::Deformer(id.clone());
            if let Some(parent) = &common.parent_part {
                if !sets.parts.contains(parent.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!("deformer '{id}' references unknown parent part '{parent}'"),
                        Some(entity.clone()),
                    );
                }
            }
            if let Some(parent) = &common.parent_deformer {
                if !sets.deformers.contains(parent.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!("deformer '{id}' references unknown parent deformer '{parent}'"),
                        Some(entity.clone()),
                    );
                }
            }
            let child_deformers: BTreeSet<&str> = common
                .children_deformers
                .iter()
                .map(|child| child.as_str())
                .collect();
            for child in &common.children_deformers {
                if !sets.deformers.contains(child.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!("deformer '{id}' lists unknown child '{child}'"),
                        Some(entity.clone()),
                    );
                } else if sets.deformer_parents.get(child.as_str()).copied().flatten()
                    != Some(id.as_str())
                {
                    self.fatal(
                        "inconsistent_reverse_link",
                        format!(
                            "deformer '{id}' lists '{child}' as a child but the link is not reciprocal"
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            for (candidate, parent) in &sets.deformer_parents {
                if *parent == Some(id.as_str()) && !child_deformers.contains(candidate) {
                    self.fatal(
                        "inconsistent_reverse_link",
                        format!(
                            "deformer '{candidate}' declares '{id}' as its parent but is not listed as a child"
                        ),
                        Some(entity.clone()),
                    );
                }
            }

            let child_meshes: BTreeSet<&str> = common
                .children_art_meshes
                .iter()
                .map(|mesh| mesh.as_str())
                .collect();
            for mesh in &common.children_art_meshes {
                if !sets.art_meshes.contains(mesh.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!("deformer '{id}' lists unknown art mesh '{mesh}'"),
                        Some(entity.clone()),
                    );
                } else if sets
                    .mesh_parent_deformer
                    .get(mesh.as_str())
                    .copied()
                    .flatten()
                    != Some(id.as_str())
                {
                    self.fatal(
                        "inconsistent_reverse_link",
                        format!(
                            "deformer '{id}' lists '{mesh}' as a child art mesh but the link is not reciprocal"
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            for (mesh, parent) in &sets.mesh_parent_deformer {
                if *parent == Some(id.as_str()) && !child_meshes.contains(mesh) {
                    self.fatal(
                        "inconsistent_reverse_link",
                        format!(
                            "art mesh '{mesh}' declares '{id}' as its parent deformer but is not listed as a child"
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            if let Some(binding) = &common.binding {
                if !sets.bindings.contains(binding.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!("deformer '{id}' references unknown binding '{binding}'"),
                        Some(entity.clone()),
                    );
                }
            }
            match deformer {
                Deformer::Warp(warp) => {
                    let expected = (warp.rows as u64 + 1).saturating_mul(warp.columns as u64 + 1);
                    if warp.vertex_count as u64 != expected {
                        self.fatal(
                            "warp_grid_mismatch",
                            format!(
                                "warp deformer '{id}' vertex_count {} != (rows + 1) * (columns + 1) ({expected})",
                                warp.vertex_count
                            ),
                            Some(entity.clone()),
                        );
                    }
                    for (expected, keyform) in warp.keyforms.iter().enumerate() {
                        check_keyform_index(&mut self.out, keyform.index, expected, &entity);
                        check_finite(
                            &mut self.out,
                            keyform.opacity,
                            "warp.keyform.opacity",
                            id.as_str(),
                            entity.clone(),
                        );
                        if keyform.positions.len() != warp.vertex_count {
                            self.fatal(
                                "vertex_count_mismatch",
                                format!(
                                    "warp deformer '{id}' keyform {} has {} positions but vertex_count is {}",
                                    keyform.index,
                                    keyform.positions.len(),
                                    warp.vertex_count
                                ),
                                Some(entity.clone()),
                            );
                        }
                        for (position_index, position) in keyform.positions.iter().enumerate() {
                            if !position.is_finite() {
                                self.fatal(
                                    "non_finite_value",
                                    format!(
                                        "warp deformer '{id}' keyform {} position[{position_index}] is not finite",
                                        keyform.index
                                    ),
                                    Some(entity.clone()),
                                );
                            }
                        }
                    }
                }
                Deformer::Rotation(rotation) => {
                    check_finite(
                        &mut self.out,
                        rotation.base_angle,
                        "rotation.base_angle",
                        id.as_str(),
                        entity.clone(),
                    );
                    for (expected, keyform) in rotation.keyforms.iter().enumerate() {
                        check_keyform_index(&mut self.out, keyform.index, expected, &entity);
                        check_finite(
                            &mut self.out,
                            keyform.opacity,
                            "rotation.keyform.opacity",
                            id.as_str(),
                            entity.clone(),
                        );
                        check_finite(
                            &mut self.out,
                            keyform.angle,
                            "rotation.keyform.angle",
                            id.as_str(),
                            entity.clone(),
                        );
                        if !keyform.origin.is_finite() {
                            self.fatal(
                                "non_finite_value",
                                format!(
                                    "rotation deformer '{id}' keyform {} origin is not finite",
                                    keyform.index
                                ),
                                Some(entity.clone()),
                            );
                        }
                        check_finite(
                            &mut self.out,
                            keyform.scale,
                            "rotation.keyform.scale",
                            id.as_str(),
                            entity.clone(),
                        );
                    }
                }
            }
        }
    }

    fn check_art_meshes(&mut self, sets: &IdSets) {
        for mesh in &self.model.art_meshes {
            let entity = EntityRef::ArtMesh(mesh.id.clone());
            if let Some(parent) = &mesh.parent_part {
                if !sets.parts.contains(parent.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!(
                            "art mesh '{}' references unknown parent part '{parent}'",
                            mesh.id
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            if let Some(parent) = &mesh.parent_deformer {
                if !sets.deformers.contains(parent.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!(
                            "art mesh '{}' references unknown parent deformer '{parent}'",
                            mesh.id
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            if let Some(texture) = &mesh.texture {
                if !sets.textures.contains(texture.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!(
                            "art mesh '{}' references unknown texture '{texture}'",
                            mesh.id
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            if let Some(binding) = &mesh.binding {
                if !sets.bindings.contains(binding.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!(
                            "art mesh '{}' references unknown binding '{binding}'",
                            mesh.id
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            for group in &mesh.mask_groups {
                if !sets.mask_groups.contains(group.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!(
                            "art mesh '{}' references unknown mask group '{group}'",
                            mesh.id
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            if mesh.uvs.len() != mesh.vertex_count {
                self.fatal(
                    "uv_count_mismatch",
                    format!(
                        "art mesh '{}' has {} UVs but vertex_count is {}",
                        mesh.id,
                        mesh.uvs.len(),
                        mesh.vertex_count
                    ),
                    Some(entity.clone()),
                );
            }
            for (index, uv) in mesh.uvs.iter().enumerate() {
                if !uv.is_finite() {
                    self.fatal(
                        "non_finite_value",
                        format!("art mesh '{}' uv[{index}] is not finite", mesh.id),
                        Some(entity.clone()),
                    );
                }
            }
            for (index, vertex) in mesh.indices.iter().enumerate() {
                if usize::from(*vertex) >= mesh.vertex_count {
                    self.fatal(
                        "index_out_of_range",
                        format!(
                            "art mesh '{}' index[{index}] = {vertex} >= vertex_count {}",
                            mesh.id, mesh.vertex_count
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            if mesh.indices.len() % 3 != 0 {
                self.push(
                    Severity::Warning,
                    "index_count_not_triangular",
                    format!(
                        "art mesh '{}' has {} indices, which is not a multiple of 3",
                        mesh.id,
                        mesh.indices.len()
                    ),
                    Some(entity.clone()),
                );
            }
            for (expected_index, keyform) in mesh.keyforms.iter().enumerate() {
                check_keyform_index(&mut self.out, keyform.index, expected_index, &entity);
                check_finite(
                    &mut self.out,
                    keyform.opacity,
                    "art_mesh.keyform.opacity",
                    mesh.id.as_str(),
                    entity.clone(),
                );
                check_finite(
                    &mut self.out,
                    keyform.draw_order,
                    "art_mesh.keyform.draw_order",
                    mesh.id.as_str(),
                    entity.clone(),
                );
                if keyform.positions.len() != mesh.vertex_count {
                    self.fatal(
                        "vertex_count_mismatch",
                        format!(
                            "art mesh '{}' keyform {} has {} positions but vertex_count is {}",
                            mesh.id,
                            keyform.index,
                            keyform.positions.len(),
                            mesh.vertex_count
                        ),
                        Some(entity.clone()),
                    );
                }
                for (position_index, position) in keyform.positions.iter().enumerate() {
                    if !position.is_finite() {
                        self.fatal(
                            "non_finite_value",
                            format!(
                                "art mesh '{}' keyform {} position[{position_index}] is not finite",
                                mesh.id, keyform.index
                            ),
                            Some(entity.clone()),
                        );
                    }
                }
            }
        }
    }

    fn check_mask_groups(&mut self, sets: &IdSets) {
        for group in &self.model.mask_groups {
            let entity = EntityRef::MaskGroup(group.id.clone());
            if !sets.art_meshes.contains(group.target.as_str()) {
                self.fatal(
                    "dangling_reference",
                    format!(
                        "mask group '{}' targets unknown art mesh '{}'",
                        group.id, group.target
                    ),
                    Some(entity.clone()),
                );
            }
            for source in &group.sources {
                if !sets.art_meshes.contains(source.as_str()) {
                    self.fatal(
                        "mask_reference_missing",
                        format!(
                            "mask group '{}' references unknown source '{source}'",
                            group.id
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            // The target mesh must list this group (reverse link).
            let listed_by_target = sets
                .mesh_mask_groups
                .get(group.target.as_str())
                .map(|groups| groups.contains(&group.id))
                .unwrap_or(false);
            if !listed_by_target && sets.art_meshes.contains(group.target.as_str()) {
                self.fatal(
                    "inconsistent_reverse_link",
                    format!(
                        "mask group '{}' targets '{}' which does not list the group",
                        group.id, group.target
                    ),
                    Some(entity.clone()),
                );
            }
            if group.sources.is_empty() {
                self.push(
                    Severity::Warning,
                    "empty_mask_group",
                    format!("mask group '{}' has no sources", group.id),
                    Some(entity.clone()),
                );
            }
        }
        // Mesh side: every listed group must exist and target the mesh.
        for mesh in &self.model.art_meshes {
            for group_id in &mesh.mask_groups {
                match sets.mask_group_targets.get(group_id.as_str()) {
                    None => self.fatal(
                        "dangling_reference",
                        format!(
                            "art mesh '{}' references unknown mask group '{group_id}'",
                            mesh.id
                        ),
                        Some(EntityRef::ArtMesh(mesh.id.clone())),
                    ),
                    Some(target) if *target != mesh.id.as_str() => self.fatal(
                        "inconsistent_reverse_link",
                        format!(
                            "art mesh '{}' lists mask group '{group_id}' that targets '{target}'",
                            mesh.id
                        ),
                        Some(EntityRef::ArtMesh(mesh.id.clone())),
                    ),
                    Some(_) => {}
                }
            }
        }
    }

    fn check_draw_order_groups(&mut self, sets: &IdSets) {
        let group_ids: BTreeSet<&str> = self
            .model
            .draw_order_groups
            .iter()
            .map(|group| group.id.as_str())
            .collect();
        for group in &self.model.draw_order_groups {
            let entity = EntityRef::Model(Default::default());
            for item in &group.items {
                match &item.object {
                    DrawOrderTarget::Part(id) => {
                        if !sets.parts.contains(id.as_str()) {
                            self.fatal(
                                "dangling_reference",
                                format!(
                                    "draw order group '{}' references unknown part '{id}'",
                                    group.id
                                ),
                                Some(entity.clone()),
                            );
                        }
                        if item.self_group.is_none() {
                            self.fatal(
                                "invalid_reference",
                                format!(
                                    "draw order group '{}' has a part item without a nested group",
                                    group.id
                                ),
                                Some(entity.clone()),
                            );
                        }
                    }
                    DrawOrderTarget::ArtMesh(id) => {
                        if !sets.art_meshes.contains(id.as_str()) {
                            self.fatal(
                                "dangling_reference",
                                format!(
                                    "draw order group '{}' references unknown art mesh '{id}'",
                                    group.id
                                ),
                                Some(entity.clone()),
                            );
                        }
                    }
                }
                if let Some(nested) = &item.self_group {
                    if !group_ids.contains(nested.as_str()) {
                        self.fatal(
                            "dangling_reference",
                            format!(
                                "draw order group '{}' references unknown nested group '{nested}'",
                                group.id
                            ),
                            Some(entity.clone()),
                        );
                    }
                }
            }
        }
        // Nested group cycles are reported but not fatal (resolution is a
        // later-phase concern).
        let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for group in &self.model.draw_order_groups {
            let nested: Vec<&str> = group
                .items
                .iter()
                .filter_map(|item| item.self_group.as_ref().map(|id| id.as_str()))
                .collect();
            edges.insert(group.id.as_str(), nested);
        }
        for id in find_cycles_multi(&edges) {
            self.push(
                Severity::Warning,
                "hierarchy_cycle",
                format!("draw order group '{id}' participates in a nesting cycle"),
                Some(EntityRef::Model(Default::default())),
            );
        }
    }

    fn check_glue(&mut self, sets: &IdSets) {
        for glue in &self.model.glue {
            let entity = EntityRef::Glue(glue.id.clone());
            for (field, value) in [
                ("art_mesh_a", &glue.art_mesh_a),
                ("art_mesh_b", &glue.art_mesh_b),
            ] {
                if !sets.art_meshes.contains(value.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!(
                            "glue '{}' {field} references unknown art mesh '{value}'",
                            glue.id
                        ),
                        Some(entity.clone()),
                    );
                }
            }
            if let Some(binding) = &glue.binding {
                if !sets.bindings.contains(binding.as_str()) {
                    self.fatal(
                        "dangling_reference",
                        format!("glue '{}' references unknown binding '{binding}'", glue.id),
                        Some(entity.clone()),
                    );
                }
            }
            for (index, value) in glue.keyform_intensities.iter().enumerate() {
                check_finite(
                    &mut self.out,
                    *value,
                    &format!("glue.keyform_intensities[{index}]"),
                    glue.id.as_str(),
                    entity.clone(),
                );
            }
            for (index, entry) in glue.info.iter().enumerate() {
                check_finite(
                    &mut self.out,
                    entry.weight,
                    &format!("glue.info[{index}].weight"),
                    glue.id.as_str(),
                    entity.clone(),
                );
            }
        }
    }

    fn check_bindings(&mut self, sets: &IdSets) {
        for binding in &self.model.bindings {
            let entity = EntityRef::Binding(binding.id.clone());
            if binding.keyform_grid.len() != binding.parameters.len() {
                self.fatal(
                    "binding_grid_mismatch",
                    format!(
                        "binding '{}' has {} grid entries but {} parameters",
                        binding.id,
                        binding.keyform_grid.len(),
                        binding.parameters.len()
                    ),
                    Some(entity.clone()),
                );
            }
            let product = binding
                .keyform_grid
                .iter()
                .map(|count| (*count).max(1))
                .fold(1usize, |acc, value| acc.saturating_mul(value));
            if binding.keyform_grid_size != product {
                self.fatal(
                    "binding_grid_mismatch",
                    format!(
                        "binding '{}' declares grid size {} but the grid dimensions multiply to {product}",
                        binding.id, binding.keyform_grid_size
                    ),
                    Some(entity.clone()),
                );
            }
            for (index, binding_parameter) in binding.parameters.iter().enumerate() {
                if !sets
                    .parameters
                    .contains(binding_parameter.parameter.as_str())
                {
                    self.fatal(
                        "dangling_reference",
                        format!(
                            "binding '{}' parameter[{index}] references unknown parameter '{}'",
                            binding.id, binding_parameter.parameter
                        ),
                        Some(entity.clone()),
                    );
                }
                for (value_index, value) in binding_parameter.key_values.iter().enumerate() {
                    check_finite(
                        &mut self.out,
                        *value,
                        &format!("binding.parameters[{index}].key_values[{value_index}]"),
                        binding.id.as_str(),
                        entity.clone(),
                    );
                }
            }
            for target in &binding.used_by {
                let known = match target {
                    crate::model::BindingTarget::Part(id) => sets.parts.contains(id.as_str()),
                    crate::model::BindingTarget::Deformer(id) => {
                        sets.deformers.contains(id.as_str())
                    }
                    crate::model::BindingTarget::ArtMesh(id) => {
                        sets.art_meshes.contains(id.as_str())
                    }
                    crate::model::BindingTarget::Glue(id) => sets.glue.contains(id.as_str()),
                };
                if !known {
                    self.fatal(
                        "dangling_reference",
                        format!("binding '{}' lists an unknown user {target:?}", binding.id),
                        Some(entity.clone()),
                    );
                }
            }
        }

        // Reverse-link consistency between objects and their bindings.
        let mut expected: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for part in &self.model.parts {
            if let Some(binding) = &part.binding {
                expected
                    .entry(binding.as_str())
                    .or_default()
                    .insert(part.id.as_str());
            }
        }
        for deformer in &self.model.deformers {
            let (id, binding) = match deformer {
                Deformer::Warp(warp) => (&warp.common.id, &warp.common.binding),
                Deformer::Rotation(rotation) => (&rotation.common.id, &rotation.common.binding),
            };
            if let Some(binding) = binding {
                expected
                    .entry(binding.as_str())
                    .or_default()
                    .insert(id.as_str());
            }
        }
        for mesh in &self.model.art_meshes {
            if let Some(binding) = &mesh.binding {
                expected
                    .entry(binding.as_str())
                    .or_default()
                    .insert(mesh.id.as_str());
            }
        }
        for glue in &self.model.glue {
            if let Some(binding) = &glue.binding {
                expected
                    .entry(binding.as_str())
                    .or_default()
                    .insert(glue.id.as_str());
            }
        }
        for binding in &self.model.bindings {
            let expected_users = expected.remove(binding.id.as_str()).unwrap_or_default();
            let mut actual: BTreeSet<&str> = BTreeSet::new();
            for target in &binding.used_by {
                let text = match target {
                    crate::model::BindingTarget::Part(id) => id.as_str(),
                    crate::model::BindingTarget::Deformer(id) => id.as_str(),
                    crate::model::BindingTarget::ArtMesh(id) => id.as_str(),
                    crate::model::BindingTarget::Glue(id) => id.as_str(),
                };
                actual.insert(text);
            }
            if actual != expected_users {
                self.fatal(
                    "inconsistent_reverse_link",
                    format!(
                        "binding '{}' used_by does not match the objects referencing it",
                        binding.id
                    ),
                    Some(EntityRef::Binding(binding.id.clone())),
                );
            }
        }
    }

    fn check_cycles(&mut self) {
        // Parts.
        let part_parents: BTreeMap<&str, Option<&str>> = self
            .model
            .parts
            .iter()
            .map(|part| (part.id.as_str(), part.parent.as_ref().map(|id| id.as_str())))
            .collect();
        for id in find_cycles(&part_parents) {
            self.push(
                Severity::Warning,
                "hierarchy_cycle",
                format!("part '{id}' participates in a parent cycle"),
                Some(EntityRef::Part(crate::ids::PartId::new(id))),
            );
        }

        // Deformers.
        let deformer_parents: BTreeMap<&str, Option<&str>> = self
            .model
            .deformers
            .iter()
            .map(|deformer| {
                let (id, parent) = match deformer {
                    Deformer::Warp(warp) => (
                        warp.common.id.as_str(),
                        warp.common.parent_deformer.as_ref().map(|id| id.as_str()),
                    ),
                    Deformer::Rotation(rotation) => (
                        rotation.common.id.as_str(),
                        rotation
                            .common
                            .parent_deformer
                            .as_ref()
                            .map(|id| id.as_str()),
                    ),
                };
                (id, parent)
            })
            .collect();
        for id in find_cycles(&deformer_parents) {
            self.push(
                Severity::Warning,
                "hierarchy_cycle",
                format!("deformer '{id}' participates in a parent cycle"),
                Some(EntityRef::Deformer(crate::ids::DeformerId::new(id))),
            );
        }
    }
}

/// Identifier sets used for reference checks.
struct IdSets<'a> {
    parameters: BTreeSet<&'a str>,
    parts: BTreeSet<&'a str>,
    deformers: BTreeSet<&'a str>,
    art_meshes: BTreeSet<&'a str>,
    textures: BTreeSet<&'a str>,
    mask_groups: BTreeSet<&'a str>,
    glue: BTreeSet<&'a str>,
    bindings: BTreeSet<&'a str>,
    /// Part id -> declared children (reciprocity checks).
    part_children: BTreeMap<&'a str, &'a [crate::ids::PartId]>,
    /// Deformer id -> declared parent deformer.
    deformer_parents: BTreeMap<&'a str, Option<&'a str>>,
    /// Art mesh id -> declared parent deformer.
    mesh_parent_deformer: BTreeMap<&'a str, Option<&'a str>>,
    /// Mask group id -> target art mesh id.
    mask_group_targets: BTreeMap<&'a str, &'a str>,
    /// Art mesh id -> listed mask group ids.
    mesh_mask_groups: BTreeMap<&'a str, &'a [crate::ids::MaskGroupId]>,
}

impl<'a> IdSets<'a> {
    fn collect(model: &'a Live2DModel) -> Self {
        Self {
            parameters: model.parameters.iter().map(|e| e.id.as_str()).collect(),
            parts: model.parts.iter().map(|e| e.id.as_str()).collect(),
            deformers: model
                .deformers
                .iter()
                .map(|deformer| match deformer {
                    Deformer::Warp(warp) => warp.common.id.as_str(),
                    Deformer::Rotation(rotation) => rotation.common.id.as_str(),
                })
                .collect(),
            art_meshes: model.art_meshes.iter().map(|e| e.id.as_str()).collect(),
            textures: model.textures.iter().map(|e| e.id.as_str()).collect(),
            mask_groups: model.mask_groups.iter().map(|e| e.id.as_str()).collect(),
            glue: model.glue.iter().map(|e| e.id.as_str()).collect(),
            bindings: model.bindings.iter().map(|e| e.id.as_str()).collect(),
            part_children: model
                .parts
                .iter()
                .map(|part| (part.id.as_str(), part.children.as_slice()))
                .collect(),
            deformer_parents: model
                .deformers
                .iter()
                .map(|deformer| match deformer {
                    Deformer::Warp(warp) => (
                        warp.common.id.as_str(),
                        warp.common.parent_deformer.as_ref().map(|id| id.as_str()),
                    ),
                    Deformer::Rotation(rotation) => (
                        rotation.common.id.as_str(),
                        rotation
                            .common
                            .parent_deformer
                            .as_ref()
                            .map(|id| id.as_str()),
                    ),
                })
                .collect(),
            mesh_parent_deformer: model
                .art_meshes
                .iter()
                .map(|mesh| {
                    (
                        mesh.id.as_str(),
                        mesh.parent_deformer.as_ref().map(|id| id.as_str()),
                    )
                })
                .collect(),
            mask_group_targets: model
                .mask_groups
                .iter()
                .map(|group| (group.id.as_str(), group.target.as_str()))
                .collect(),
            mesh_mask_groups: model
                .art_meshes
                .iter()
                .map(|mesh| (mesh.id.as_str(), mesh.mask_groups.as_slice()))
                .collect(),
        }
    }

    fn duplicates_in<'b, I>(&self, ids: I) -> Vec<String>
    where
        I: IntoIterator<Item = &'b String>,
    {
        let mut seen: BTreeSet<&'b str> = BTreeSet::new();
        let mut duplicates: Vec<String> = Vec::new();
        for id in ids {
            if !seen.insert(id.as_str()) && !duplicates.iter().any(|entry| entry == id) {
                duplicates.push(id.clone());
            }
        }
        duplicates
    }
}

fn check_keyform_index(
    out: &mut Vec<Diagnostic>,
    index: usize,
    expected: usize,
    entity: &EntityRef,
) {
    if index != expected {
        out.push(Diagnostic {
            severity: Severity::Fatal,
            code: "non_canonical_order".to_string(),
            message: format!("keyform index {index} does not match its position {expected}"),
            entity: Some(entity.clone()),
        });
    }
}

fn check_finite(
    out: &mut Vec<Diagnostic>,
    value: f32,
    field: &str,
    owner: &str,
    entity: EntityRef,
) {
    if !value.is_finite() {
        out.push(Diagnostic {
            severity: Severity::Fatal,
            code: "non_finite_value".to_string(),
            message: format!("{field} of '{owner}' is not finite"),
            entity: Some(entity),
        });
    }
}

fn expect_field_provenance(
    out: &mut Vec<Diagnostic>,
    fields: &[crate::provenance::FieldProvenance],
    field: &str,
    entity: &EntityRef,
) {
    let documented = fields
        .iter()
        .any(|entry| entry.field == field && entry.provenance.confidence == Confidence::Unknown);
    if !documented {
        out.push(Diagnostic {
            severity: Severity::Warning,
            code: "missing_field_provenance".to_string(),
            message: format!("field '{field}' is absent but has no Unknown provenance entry"),
            entity: Some(entity.clone()),
        });
    }
}

/// Iterative cycle detection over a directed graph (deterministic).
fn find_cycles_multi(edges: &BTreeMap<&str, Vec<&str>>) -> Vec<String> {
    let mut state: BTreeMap<&str, u8> = BTreeMap::new(); // 0 unknown, 1 done, 2 on stack
    let mut cyclic: BTreeSet<&str> = BTreeSet::new();
    for start in edges.keys() {
        if state.get(start).copied().unwrap_or(0) != 0 {
            continue;
        }
        state.insert(start, 2);
        let mut path: Vec<&str> = vec![start];
        let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
        while let Some((node, index)) = stack.pop() {
            let children = edges.get(node).map(|value| value.as_slice()).unwrap_or(&[]);
            if let Some(child) = children.get(index).copied() {
                stack.push((node, index + 1));
                match state.get(child).copied().unwrap_or(0) {
                    0 => {
                        state.insert(child, 2);
                        path.push(child);
                        stack.push((child, 0));
                    }
                    2 => {
                        if let Some(position) = path.iter().position(|value| *value == child) {
                            for marked in path.iter().skip(position) {
                                cyclic.insert(marked);
                            }
                        }
                    }
                    _ => {}
                }
            } else {
                state.insert(node, 1);
                if path.last().copied() == Some(node) {
                    path.pop();
                }
            }
        }
    }
    cyclic.into_iter().map(str::to_string).collect()
}

/// Iterative cycle detection over a parent map (deterministic).
fn find_cycles(parents: &BTreeMap<&str, Option<&str>>) -> Vec<String> {
    let mut state: BTreeMap<&str, u8> = BTreeMap::new(); // 0 unknown, 1 done, 2 on stack
    let mut cyclic: BTreeSet<&str> = BTreeSet::new();
    for start in parents.keys() {
        if state.get(start).copied().unwrap_or(0) != 0 {
            continue;
        }
        let mut path: Vec<&str> = Vec::new();
        let mut current: Option<&str> = Some(*start);
        while let Some(node) = current {
            match state.get(node).copied().unwrap_or(0) {
                0 => {
                    state.insert(node, 2);
                    path.push(node);
                    current = parents.get(node).copied().flatten();
                }
                2 => {
                    if let Some(position) = path.iter().position(|value| *value == node) {
                        for marked in path.iter().skip(position) {
                            cyclic.insert(marked);
                        }
                    }
                    break;
                }
                _ => break,
            }
        }
        for node in &path {
            state.insert(node, 1);
        }
    }
    cyclic.into_iter().map(|value| value.to_string()).collect()
}
