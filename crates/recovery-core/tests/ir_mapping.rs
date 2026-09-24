//! MOC3 -> IR mapping tests over the synthetic fixtures.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use live2d_ir::{BlendMode, Deformer, Live2DModel, Severity};
use recovery_core::{
    build_ir_from_bytes, build_ir_from_file, export_ir_json, import_ir_json, InspectOptions,
};

fn build(name: &str) -> Live2DModel {
    let bytes = support::read_fixture(name);
    build_ir_from_bytes(&bytes, &InspectOptions::default())
        .unwrap_or_else(|error| panic!("{name} failed to map: {error}"))
}

fn has_code(model: &Live2DModel, code: &str) -> bool {
    model
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}

#[test]
fn every_fixture_maps_without_fatal_diagnostics() {
    for name in fixture_gen::FIXTURE_NAMES {
        if *name == "keyform-016-non-finite-key.moc3" {
            // Non-finite keys are intentionally a Fatal IR diagnostic; the
            // keyform layer still reports them positionally (AGENT.4).
            continue;
        }
        let model = build(name);
        assert!(
            !live2d_ir::has_fatal(&model.diagnostics),
            "{name} produced fatal diagnostics: {:?}",
            model
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == Severity::Fatal)
                .collect::<Vec<_>>()
        );
        assert_eq!(model.schema, "live2d-ir/1");
        assert_eq!(model.metadata.entity_counts, model.entity_counts());
    }
}

#[test]
fn fixture_001_maps_single_artmesh() {
    let model = build("fixture-001-single-artmesh.moc3");
    assert_eq!(model.parameters.len(), 0);
    assert_eq!(model.parts.len(), 0);
    assert_eq!(model.art_meshes.len(), 1);
    let mesh = &model.art_meshes[0];
    assert_eq!(mesh.id.as_str(), "ArtMesh_Synthetic_00");
    assert_eq!(mesh.vertex_count, 4);
    assert_eq!(mesh.uvs.len(), 4);
    assert_eq!(mesh.indices.len(), 6);
    assert_eq!(mesh.keyforms.len(), 1);
    assert_eq!(mesh.keyforms[0].positions.len(), 4);
    assert!(mesh.parent_part.is_none());
    // Flags 0x05: additive blending + double sided.
    assert_eq!(mesh.blend_mode, BlendMode::AddCompatible);
    assert!(mesh.flags.additive);
    assert!(mesh.flags.double_sided);
    assert!(mesh.visible);
    assert!(mesh.enabled);
    assert!(model.canvas.y_axis_reversed);
    assert_eq!(model.textures.len(), 1);
    assert_eq!(model.textures[0].page_index, 0);
    assert_eq!(model.drawables, vec![mesh.id.clone()]);
    // Draw order group coverage.
    assert_eq!(model.draw_order_groups.len(), 1);
    let group = &model.draw_order_groups[0];
    assert_eq!(group.items.len(), 1);
    assert_eq!(
        group.items[0].object,
        live2d_ir::DrawOrderTarget::ArtMesh(mesh.id.clone())
    );
    assert!(group.items[0].self_group.is_none());
}

#[test]
fn fixture_002_maps_parameter_and_binding() {
    let model = build("fixture-002-artmesh-param.moc3");
    assert_eq!(model.parameters.len(), 1);
    let parameter = &model.parameters[0];
    assert_eq!(parameter.id.as_str(), "ParamSynthetic_AngleX");
    assert_eq!(parameter.key_values, vec![-30.0, 30.0]);
    assert!(parameter.current.is_none());
    assert!(parameter
        .field_provenance
        .iter()
        .any(|entry| entry.field == "current"));

    assert_eq!(model.parts[0].id.as_str(), "Part_Synthetic_00");
    assert_eq!(model.parts[0].keyform_draw_orders.len(), 2);
    assert!(model.parts[0].opacity.is_none());

    let mesh = &model.art_meshes[0];
    assert_eq!(
        mesh.parent_part.as_ref().map(|id| id.as_str()),
        Some("Part_Synthetic_00")
    );
    assert_eq!(mesh.keyforms.len(), 2);
    assert_eq!(mesh.keyforms[1].positions[0].x, 2.0);

    assert_eq!(model.bindings.len(), 1);
    let binding = &model.bindings[0];
    assert_eq!(binding.keyform_grid_size, 2);
    assert_eq!(binding.keyform_grid, vec![2]);
    assert_eq!(binding.parameters.len(), 1);
    assert_eq!(
        binding.parameters[0].parameter.as_str(),
        "ParamSynthetic_AngleX"
    );
    assert_eq!(
        binding.used_by.len(),
        2,
        "part and art mesh share the binding"
    );
}

#[test]
fn fixture_003_maps_warp_and_reverse_links() {
    let model = build("fixture-003-warp.moc3");
    assert_eq!(model.deformers.len(), 1);
    let Deformer::Warp(warp) = &model.deformers[0] else {
        panic!("expected warp deformer");
    };
    assert_eq!(warp.common.id.as_str(), "WarpDeformer_Synthetic_00");
    assert_eq!(warp.rows, 1);
    assert_eq!(warp.columns, 1);
    assert_eq!(warp.vertex_count, 4);
    assert!(!warp.quad_transform);
    assert_eq!(warp.keyforms.len(), 1);
    assert_eq!(warp.keyforms[0].positions.len(), 4);
    assert_eq!(warp.common.children_art_meshes.len(), 1);
    assert_eq!(
        warp.common.children_art_meshes[0].as_str(),
        "ArtMesh_Synthetic_00"
    );
    let mesh = &model.art_meshes[0];
    assert_eq!(
        mesh.parent_deformer.as_ref().map(|id| id.as_str()),
        Some("WarpDeformer_Synthetic_00")
    );
}

#[test]
fn fixture_004_maps_nested_deformer_links() {
    let model = build("fixture-004-nested-warp.moc3");
    assert_eq!(model.deformers.len(), 2);
    let Deformer::Warp(first) = &model.deformers[0] else {
        panic!("expected warp");
    };
    let Deformer::Warp(second) = &model.deformers[1] else {
        panic!("expected warp");
    };
    assert_eq!(
        second.common.parent_deformer.as_ref(),
        Some(&first.common.id)
    );
    assert_eq!(
        first.common.children_deformers,
        vec![second.common.id.clone()]
    );
}

#[test]
fn fixture_005_maps_rotation() {
    let model = build("fixture-005-rotation.moc3");
    let Deformer::Rotation(rotation) = &model.deformers[0] else {
        panic!("expected rotation");
    };
    assert_eq!(rotation.base_angle, 30.0);
    assert_eq!(rotation.keyforms.len(), 1);
    assert_eq!(rotation.keyforms[0].angle, 30.0);
    assert_eq!(rotation.keyforms[0].origin.x, 50.0);
    assert_eq!(rotation.keyforms[0].origin.y, 50.0);
}

#[test]
fn fixture_006_maps_mask_groups() {
    let model = build("fixture-006-clipping-mask.moc3");
    assert_eq!(model.mask_groups.len(), 1);
    let group = &model.mask_groups[0];
    assert_eq!(group.target.as_str(), "ArtMesh_Synthetic_01");
    assert_eq!(group.sources.len(), 1);
    assert_eq!(group.sources[0].as_str(), "ArtMesh_Synthetic_00");
    assert_eq!(model.art_meshes[1].mask_groups, vec![group.id.clone()]);
}

#[test]
fn fixture_007_maps_multi_bindings() {
    let model = build("fixture-007-multi-params.moc3");
    assert_eq!(model.parameters.len(), 5);
    assert_eq!(model.bindings.len(), 4);
    let sizes: Vec<usize> = model
        .bindings
        .iter()
        .map(|binding| binding.keyform_grid_size)
        .collect();
    assert_eq!(sizes, vec![4, 8, 2, 1]);
    assert_eq!(
        model.parts[1].parent.as_ref().map(|id| id.as_str()),
        Some("Part_Synthetic_00")
    );
    assert_eq!(model.parts[0].children.len(), 1);
    // The v4.2 color pools are deliberately not mapped yet.
    assert!(has_code(&model, "unmapped_sections"));
    // Nested draw order groups: group 0 nests group 1 through a part item.
    assert_eq!(model.draw_order_groups.len(), 2);
    let outer = &model.draw_order_groups[0];
    assert_eq!(outer.items.len(), 1);
    assert_eq!(
        outer.items[0].object,
        live2d_ir::DrawOrderTarget::Part(live2d_ir::PartId::new("Part_Synthetic_00"))
    );
    assert_eq!(
        outer.items[0].self_group,
        Some(model.draw_order_groups[1].id.clone())
    );
    assert_eq!(model.draw_order_groups[1].items.len(), 2);
    assert!(!live2d_ir::has_fatal(&model.diagnostics));
}

#[test]
fn fixture_008_maps_textures_ascending() {
    let model = build("fixture-008-multi-textures.moc3");
    assert_eq!(model.textures.len(), 2);
    assert_eq!(model.textures[0].page_index, 0);
    assert_eq!(model.textures[1].page_index, 1);
    assert_eq!(
        model.art_meshes[0].texture.as_ref().map(|id| id.as_str()),
        Some("texture:000000")
    );
    assert_eq!(
        model.art_meshes[1].texture.as_ref().map(|id| id.as_str()),
        Some("texture:000001")
    );
}

#[test]
fn fixture_009_and_010_carry_version_and_blend_mode() {
    let model = build("fixture-009-v50.moc3");
    assert_eq!(model.source.version_byte, 5);
    let model = build("fixture-010-v53.moc3");
    assert_eq!(model.source.version_byte, 6);
    assert_eq!(model.art_meshes[0].blend_mode_raw, Some(0));
    assert_eq!(model.art_meshes[0].blend_mode, BlendMode::Normal);
}

#[test]
fn id_assignment_is_deterministic_for_duplicates() {
    let mut spec = fixture_gen::SyntheticModel {
        version: 2,
        parameters: Vec::new(),
        art_meshes: vec![
            fixture_gen::ArtMeshSpec::quad("Dup", 0),
            fixture_gen::ArtMeshSpec::quad("Dup", 0),
            fixture_gen::ArtMeshSpec::quad("", 0),
        ],
        ..fixture_gen::SyntheticModel::default()
    };
    spec.version = 2;
    let bytes = fixture_gen::build(&spec);
    let first = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    let second = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();

    let ids: Vec<&str> = first
        .art_meshes
        .iter()
        .map(|mesh| mesh.id.as_str())
        .collect();
    assert_eq!(ids, vec!["Dup", "artmesh:000001", "artmesh:000002"]);
    assert!(has_code(&first, "duplicate_source_id"));
    assert!(has_code(&first, "empty_source_id"));
    assert_eq!(
        export_ir_json(&first, false).unwrap(),
        export_ir_json(&second, false).unwrap()
    );
}

#[test]
fn fixture_013_maps_glue() {
    let model = build("fixture-013-glue.moc3");
    assert_eq!(model.glue.len(), 1);
    let glue = &model.glue[0];
    assert_eq!(glue.id.as_str(), "Glue_Synthetic_00");
    assert_eq!(glue.art_mesh_a.as_str(), "ArtMesh_Synthetic_00");
    assert_eq!(glue.art_mesh_b.as_str(), "ArtMesh_Synthetic_01");
    assert_eq!(glue.info.len(), 4);
    assert_eq!(glue.info[0].weight, 0.5);
    assert_eq!(glue.info[0].position_index, 0);
    assert_eq!(glue.info[2].weight, 1.0);
    assert_eq!(glue.info[2].position_index, 1);
    assert_eq!(glue.keyform_intensities, vec![1.0]);
    assert!(!live2d_ir::has_fatal(&model.diagnostics));
}

#[test]
fn art_mesh_visibility_flags_are_mapped() {
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let visible_offset = support::section_offset(&bytes, "art_mesh.visible");
    support::write_u32(&mut bytes, visible_offset, 0);
    let model = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    assert!(!model.art_meshes[0].visible);
    assert!(model.art_meshes[0].enabled);
}

#[test]
fn conflicting_blend_bits_are_reported() {
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let flags_offset = support::section_offset(&bytes, "art_mesh.drawable_flags");
    let at = usize::try_from(flags_offset).unwrap();
    *bytes.get_mut(at).unwrap() = 0x03; // additive + multiplicative
    let model = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    assert_eq!(model.art_meshes[0].blend_mode, BlendMode::AddCompatible);
    assert!(has_code(&model, "conflicting_blend_bits"));
    assert_eq!(model.art_meshes[0].flags.unknown_bits, 0);
}

#[test]
fn unknown_blend_mode_values_are_preserved() {
    let mut bytes = support::read_fixture("fixture-010-v53.moc3");
    let blend_offset = support::section_offset(&bytes, "art_mesh.blend_mode");
    support::write_u32(&mut bytes, blend_offset, 99);
    let model = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    assert_eq!(model.art_meshes[0].blend_mode, BlendMode::Unknown);
    assert_eq!(model.art_meshes[0].blend_mode_raw, Some(99));
    assert!(has_code(&model, "unknown_blend_mode"));
}

#[test]
fn unknown_bytes_are_preserved() {
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    support::write_u32(&mut bytes, 6, 0xAB); // first reserved header byte
    let model = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    assert_eq!(model.unknowns.header_reserved_nonzero_bytes, 1);
    assert!(has_code(&model, "moc3_header_padding_nonzero"));
}

#[test]
fn non_finite_parameter_values_fail_validation() {
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let default_offset = support::section_offset(&bytes, "parameter.default");
    support::write_f32(&mut bytes, default_offset, f32::NAN);
    let model = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    assert!(model.parameters[0].default.is_nan());
    assert!(live2d_ir::has_fatal(&model.diagnostics));
    assert!(has_code(&model, "non_finite_value"));
    let error = export_ir_json(&model, true).unwrap_err();
    assert_eq!(error.code, "ValidationFailed");
}

#[test]
fn invalid_parameter_ranges_fail_validation() {
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let minimum_offset = support::section_offset(&bytes, "parameter.minimum");
    support::write_f32(&mut bytes, minimum_offset, 100.0);
    let model = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    assert!(has_code(&model, "parameter_range_invalid"));
    assert!(export_ir_json(&model, true).is_err());
}

#[test]
fn ir_round_trip_is_semantic_and_byte_stable() {
    for name in fixture_gen::FIXTURE_NAMES {
        if *name == "keyform-016-non-finite-key.moc3" {
            // Canonical IR cannot carry NaN (Fatal diagnostic), so there is
            // no round-trip contract for this fixture at the IR layer.
            continue;
        }
        let path = support::fixtures_dir().join(name);
        let model = build_ir_from_file(&path, &InspectOptions::default()).unwrap();
        let json = export_ir_json(&model, true).unwrap();
        let imported = import_ir_json(&json).unwrap();
        let json_again = export_ir_json(&imported, true).unwrap();
        assert_eq!(json, json_again, "round-trip changed JSON bytes for {name}");
        assert_eq!(model, imported, "round-trip changed the model for {name}");
    }
}

#[test]
fn build_ir_is_deterministic_across_runs() {
    let bytes = support::read_fixture("fixture-006-clipping-mask.moc3");
    let first = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    let second = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    assert_eq!(
        export_ir_json(&first, true).unwrap(),
        export_ir_json(&second, true).unwrap()
    );
}
