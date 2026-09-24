//! Validator and JSON contract tests, built from hand-made models.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{minimal_model, push_warp};
use live2d_ir::{
    from_json_str, has_fatal, to_json_str, validate_ir, BlendMode, Deformer, EntityRef, MaskGroup,
    MaskGroupId, PartId, Severity, Texture, TextureId,
};

fn codes(diagnostics: &[live2d_ir::Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.clone())
        .collect()
}

#[test]
fn minimal_model_is_valid() {
    let model = minimal_model();
    let diagnostics = validate_ir(&model);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, got: {diagnostics:?}"
    );
}

#[test]
fn schema_mismatch_is_fatal() {
    let mut model = minimal_model();
    model.schema = "live2d-ir/999".to_string();
    let diagnostics = validate_ir(&model);
    assert!(has_fatal(&diagnostics));
    assert!(codes(&diagnostics).contains(&"schema_mismatch".to_string()));
}

#[test]
fn metadata_count_mismatch_is_fatal() {
    let mut model = minimal_model();
    model.metadata.entity_counts.parameters = 99;
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"metadata_count_mismatch".to_string()));
}

#[test]
fn duplicate_ids_are_fatal() {
    let mut model = minimal_model();
    let mut clone = model.parts[0].clone();
    clone.id = PartId::new("PartHead");
    clone.children.clear();
    model.parts.push(clone);
    model.metadata.entity_counts = model.entity_counts();
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"duplicate_id".to_string()));
    assert!(has_fatal(&diagnostics));
}

#[test]
fn dangling_reference_is_fatal() {
    let mut model = minimal_model();
    model.parts[0].parent = Some(PartId::new("DoesNotExist"));
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"dangling_reference".to_string()));
}

#[test]
fn inconsistent_reverse_link_is_fatal() {
    let mut model = minimal_model();
    let mut second = model.parts[0].clone();
    second.id = PartId::new("PartBody");
    second.parent = None;
    second.children.clear();
    model.parts.push(second);
    model.parts[0].children.push(PartId::new("PartBody"));
    model.metadata.entity_counts = model.entity_counts();
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"inconsistent_reverse_link".to_string()));
}

#[test]
fn parameter_range_violation_is_fatal() {
    let mut model = minimal_model();
    model.parameters[0].minimum = 100.0;
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"parameter_range_invalid".to_string()));
}

#[test]
fn non_finite_values_are_fatal() {
    let mut model = minimal_model();
    model.parameters[0].default = f32::NAN;
    model.art_meshes[0].uvs[0].u = f32::INFINITY;
    model.canvas.width = f32::NAN;
    let diagnostics = validate_ir(&model);
    let non_finite = codes(&diagnostics)
        .iter()
        .filter(|code| *code == "non_finite_value")
        .count();
    assert!(non_finite >= 3, "expected 3+ non-finite findings");
}

#[test]
fn geometry_mismatches_are_fatal() {
    let mut model = minimal_model();
    model.art_meshes[0].uvs.pop();
    model.art_meshes[0].indices.push(9);
    let diagnostics = validate_ir(&model);
    let codes = codes(&diagnostics);
    assert!(codes.contains(&"uv_count_mismatch".to_string()));
    assert!(codes.contains(&"index_out_of_range".to_string()));
}

#[test]
fn non_triangular_index_count_is_a_warning() {
    let mut model = minimal_model();
    model.art_meshes[0].indices.push(0);
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"index_count_not_triangular".to_string()));
    assert!(!has_fatal(&diagnostics));
}

#[test]
fn warp_grid_mismatch_is_fatal() {
    let mut model = minimal_model();
    push_warp(&mut model);
    if let Some(Deformer::Warp(warp)) = model.deformers.first_mut() {
        warp.vertex_count = 5;
    }
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"warp_grid_mismatch".to_string()));
}

#[test]
fn cycles_are_warnings_not_fatal() {
    let mut model = minimal_model();
    let mut second = model.parts[0].clone();
    second.id = PartId::new("PartBody");
    second.parent = Some(PartId::new("PartHead"));
    second.children = vec![PartId::new("PartHead")];
    model.parts.push(second);
    if let Some(parent) = model.parts.first_mut() {
        parent.parent = Some(PartId::new("PartBody"));
        parent.children = vec![PartId::new("PartBody")];
    }
    model.metadata.entity_counts = model.entity_counts();
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"hierarchy_cycle".to_string()));
    assert!(!has_fatal(&diagnostics));
}

#[test]
fn empty_mask_group_is_a_warning() {
    let mut model = minimal_model();
    model.mask_groups.push(MaskGroup {
        id: MaskGroupId::new("mask:000000"),
        target: live2d_ir::ArtMeshId::new("ArtMeshFace"),
        sources: Vec::new(),
        provenance: live2d_ir::Provenance::exact("test:mask".to_string()),
    });
    if let Some(mesh) = model.art_meshes.first_mut() {
        mesh.mask_groups.push(MaskGroupId::new("mask:000000"));
    }
    model.metadata.entity_counts = model.entity_counts();
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"empty_mask_group".to_string()));
    assert!(!has_fatal(&diagnostics));
}

#[test]
fn missing_reverse_child_link_is_fatal() {
    let mut model = minimal_model();
    let mut second = model.parts[0].clone();
    second.id = PartId::new("PartBody");
    second.parent = Some(PartId::new("PartHead"));
    second.children.clear();
    model.parts.push(second);
    // PartHead has no children entry for PartBody -> incomplete reverse link.
    model.metadata.entity_counts = model.entity_counts();
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"inconsistent_reverse_link".to_string()));
    assert!(has_fatal(&diagnostics));
}

#[test]
fn missing_reverse_deformer_links_are_fatal() {
    let mut model = minimal_model();
    push_warp(&mut model);
    let mut child = model.deformers[0].clone();
    if let live2d_ir::Deformer::Warp(warp) = &mut child {
        warp.common.id = live2d_ir::DeformerId::new("warp:000001");
        warp.common.parent_deformer = Some(live2d_ir::DeformerId::new("warp:000000"));
        warp.common.children_deformers.clear();
    }
    model.deformers.push(child);
    // Mesh declares the warp as its parent but the warp does not list it.
    if let Some(mesh) = model.art_meshes.first_mut() {
        mesh.parent_deformer = Some(live2d_ir::DeformerId::new("warp:000000"));
    }
    model.metadata.entity_counts = model.entity_counts();
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"inconsistent_reverse_link".to_string()));
}

#[test]
fn missing_reverse_mask_link_is_fatal() {
    let mut model = minimal_model();
    model.mask_groups.push(MaskGroup {
        id: MaskGroupId::new("mask:000000"),
        target: live2d_ir::ArtMeshId::new("ArtMeshFace"),
        sources: vec![live2d_ir::ArtMeshId::new("ArtMeshFace")],
        provenance: live2d_ir::Provenance::exact("test:mask".to_string()),
    });
    // The mesh does not list the group -> reverse link missing.
    model.metadata.entity_counts = model.entity_counts();
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"inconsistent_reverse_link".to_string()));

    // With the link present the model is valid again.
    if let Some(mesh) = model.art_meshes.first_mut() {
        mesh.mask_groups.push(MaskGroupId::new("mask:000000"));
    }
    let diagnostics = validate_ir(&model);
    assert!(!has_fatal(&diagnostics), "{diagnostics:?}");
}

#[test]
fn stored_but_unmodeled_fields_are_rejected_when_filled() {
    let mut model = minimal_model();
    model.parameters[0].current = Some(10.0);
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"field_not_stored".to_string()));
    assert!(has_fatal(&diagnostics));

    let mut model = minimal_model();
    model.parts[0].draw_order = Some(1.0);
    let diagnostics = validate_ir(&model);
    assert!(codes(&diagnostics).contains(&"field_not_stored".to_string()));
}

#[test]
fn draw_order_groups_validate_references_and_cycles() {
    let mut model = minimal_model();
    model.draw_order_groups.push(live2d_ir::DrawOrderGroup {
        id: live2d_ir::DrawOrderGroupId::new("drawgroup:000000"),
        items: vec![live2d_ir::DrawOrderItem {
            object: live2d_ir::DrawOrderTarget::ArtMesh(live2d_ir::ArtMeshId::new("ArtMeshFace")),
            self_group: None,
        }],
        maximum_order: 0,
        minimum_order: 0,
        provenance: live2d_ir::Provenance::exact("test:draw_group".to_string()),
    });
    model.metadata.entity_counts = model.entity_counts();
    let diagnostics = validate_ir(&model);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    // Dangling object reference.
    let mut broken = model.clone();
    if let Some(group) = broken.draw_order_groups.first_mut() {
        group.items[0].object =
            live2d_ir::DrawOrderTarget::ArtMesh(live2d_ir::ArtMeshId::new("Gone"));
    }
    let diagnostics = validate_ir(&broken);
    assert!(codes(&diagnostics).contains(&"dangling_reference".to_string()));

    // Part item without a nested group.
    let mut broken = model.clone();
    if let Some(group) = broken.draw_order_groups.first_mut() {
        group.items[0].object = live2d_ir::DrawOrderTarget::Part(PartId::new("PartHead"));
        group.items[0].self_group = None;
    }
    let diagnostics = validate_ir(&broken);
    assert!(codes(&diagnostics).contains(&"invalid_reference".to_string()));

    // Nesting cycle (Warning, not fatal): two groups referencing each other.
    let mut cyc = model.clone();
    cyc.draw_order_groups.push(live2d_ir::DrawOrderGroup {
        id: live2d_ir::DrawOrderGroupId::new("drawgroup:000001"),
        items: vec![live2d_ir::DrawOrderItem {
            object: live2d_ir::DrawOrderTarget::Part(PartId::new("PartHead")),
            self_group: Some(live2d_ir::DrawOrderGroupId::new("drawgroup:000000")),
        }],
        maximum_order: 0,
        minimum_order: 0,
        provenance: live2d_ir::Provenance::exact("test:draw_group".to_string()),
    });
    if let Some(group) = cyc.draw_order_groups.first_mut() {
        if let Some(item) = group.items.first_mut() {
            item.object = live2d_ir::DrawOrderTarget::Part(PartId::new("PartHead"));
            item.self_group = Some(live2d_ir::DrawOrderGroupId::new("drawgroup:000001"));
        }
    }
    cyc.metadata.entity_counts = cyc.entity_counts();
    let diagnostics = validate_ir(&cyc);
    assert!(codes(&diagnostics).contains(&"hierarchy_cycle".to_string()));
    assert!(!has_fatal(&diagnostics));
}

#[test]
fn json_round_trip_preserves_model_bytes() {
    let model = minimal_model();
    let json = to_json_str(&model, true).unwrap();
    let imported = from_json_str(&json).unwrap();
    assert_eq!(model, imported);
    let again = to_json_str(&imported, true).unwrap();
    assert_eq!(json, again);
}

#[test]
fn json_export_refuses_fatal_models() {
    let mut model = minimal_model();
    model.parameters[0].maximum = f32::NAN;
    let error = to_json_str(&model, true).unwrap_err();
    assert_eq!(error.code, "ValidationFailed");
}

#[test]
fn json_import_rejects_schema_mismatch() {
    let model = minimal_model();
    let json = to_json_str(&model, false).unwrap();
    let tampered = json.replace("\"live2d-ir/1\"", "\"live2d-ir/2\"");
    let error = from_json_str(&tampered).unwrap_err();
    assert_eq!(error.code, "SchemaMismatch");
}

#[test]
fn json_import_rejects_syntax_errors() {
    let error = from_json_str("{\"schema\": \"live2d-ir/1\", ").unwrap_err();
    assert_eq!(error.code, "JsonSyntax");
    assert!(error.line.is_some());
}

#[test]
fn diagnostics_serialize_entity_refs() {
    let mut model = minimal_model();
    model.parts[0].parent = Some(PartId::new("Gone"));
    let diagnostics = validate_ir(&model);
    let with_entity = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.entity.is_some())
        .expect("expected an entity-scoped diagnostic");
    let value = serde_json::to_value(with_entity).unwrap();
    assert!(value.get("entity").is_some());
    assert_eq!(
        with_entity.entity,
        Some(EntityRef::Part(PartId::new("PartHead")))
    );
}

#[test]
fn blend_mode_diagnostics_cover_unknown_values() {
    // Sanity: our enum serialization stays snake_case and stable.
    let value = serde_json::to_value(BlendMode::MultiplyCompatible).unwrap();
    assert_eq!(value, serde_json::json!("multiply_compatible"));
    assert_eq!(
        serde_json::to_value(Severity::Recoverable).unwrap(),
        serde_json::json!("recoverable")
    );
    // Texture helper keeps the JSON shape stable too.
    let texture = Texture {
        id: TextureId::new("texture:000000"),
        page_index: 0,
        width: None,
        height: None,
        source_path: None,
        provenance: live2d_ir::Provenance::exact("test".to_string()),
    };
    let value = serde_json::to_value(&texture).unwrap();
    assert!(value.get("width").is_none());
}
