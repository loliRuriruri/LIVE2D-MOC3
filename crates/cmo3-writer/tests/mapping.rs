//! Writer mapping tests: strict unresolved policy, writer defaults,
//! determinism, texture handling. No recovery logic lives here; the semantic
//! layers are built with the same crates the pipeline uses.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cmo3_writer::map::{build_project, MapOptions};
use cmo3_writer::textures::{TextureAsset, TextureAssets};

use hierarchy_recovery::{reconstruct, RecoveryPolicy};
use keyform_recovery::recover;
use live2d_ir::model::{
    ArtMesh, ArtMeshKeyform, Binding, BindingParameter, BindingTarget, BlendMode, Canvas,
    DrawableFlags, EntityCounts, Live2DModel, Metadata, Parameter, ParameterKind, Part,
    SourceFormat,
};
use live2d_ir::{
    ArtMeshId, BindingId, Endianness, ParameterId, PartId, Provenance, SchemaStatus, TextureId, Uv,
    Vec2,
};

fn base_model() -> Live2DModel {
    Live2DModel {
        schema: live2d_ir::model::SCHEMA_ID.to_string(),
        schema_status: SchemaStatus::Experimental,
        source: SourceFormat {
            format: "moc3".to_string(),
            version_byte: 2,
            version_label: "2 (3.3.00-3.3.03)".to_string(),
            byte_order: Endianness::Little,
        },
        metadata: Metadata {
            generator: "cmo3-writer-test".to_string(),
            generator_phase: "AGENT.5-test".to_string(),
            entity_counts: EntityCounts {
                parameters: 1,
                parts: 1,
                deformers: 0,
                warp_deformers: 0,
                rotation_deformers: 0,
                art_meshes: 1,
                draw_order_groups: 0,
                mask_groups: 0,
                textures: 1,
                glue: 0,
                bindings: 1,
            },
        },
        canvas: Canvas {
            width: 512.0,
            height: 512.0,
            origin: Vec2 { x: 256.0, y: 256.0 },
            pixels_per_unit: 1.0,
            y_axis_reversed: false,
            field_provenance: Vec::new(),
            provenance: Provenance::exact("moc3:canvas"),
        },
        parameters: vec![Parameter {
            id: ParameterId::new("ParamAngleX"),
            source_name: Some("ParamAngleX".to_string()),
            minimum: -30.0,
            maximum: 30.0,
            default: 0.0,
            current: None,
            repeat: false,
            decimal_places: 3,
            kind: ParameterKind::Normal,
            key_values: vec![-30.0, 30.0],
            extension_key_values: Vec::new(),
            field_provenance: Vec::new(),
            provenance: Provenance::exact("moc3:parameter"),
        }],
        parts: vec![Part {
            id: PartId::new("Part_00"),
            source_name: Some("Part_00".to_string()),
            parent: None,
            children: Vec::new(),
            opacity: None,
            draw_order: None,
            keyform_draw_orders: vec![0.0, 1.0],
            binding: Some(BindingId::new("binding:000000")),
            visible: true,
            enabled: true,
            field_provenance: Vec::new(),
            provenance: Provenance::exact("moc3:part"),
        }],
        deformers: Vec::new(),
        art_meshes: vec![ArtMesh {
            id: ArtMeshId::new("Mesh_00"),
            source_name: Some("Mesh_00".to_string()),
            parent_part: Some(PartId::new("Part_00")),
            parent_deformer: None,
            texture: Some(TextureId::new("texture:000000")),
            vertex_count: 4,
            uvs: vec![Uv { u: 0.0, v: 0.0 }; 4],
            indices: vec![0, 1, 2, 0, 2, 3],
            keyforms: (0..2)
                .map(|index| ArtMeshKeyform {
                    index,
                    opacity: 1.0,
                    draw_order: index as f32,
                    positions: vec![Vec2 { x: 1.0, y: 1.0 }; 4],
                })
                .collect(),
            flags: DrawableFlags {
                additive: false,
                multiplicative: false,
                double_sided: false,
                inverted_mask: false,
                unknown_bits: 0,
            },
            blend_mode: BlendMode::Normal,
            blend_mode_raw: None,
            opacity: None,
            draw_order: None,
            binding: Some(BindingId::new("binding:000000")),
            visible: true,
            enabled: true,
            mask_groups: Vec::new(),
            field_provenance: Vec::new(),
            provenance: Provenance::exact("moc3:art_mesh"),
        }],
        drawables: vec![ArtMeshId::new("Mesh_00")],
        draw_order_groups: Vec::new(),
        mask_groups: Vec::new(),
        textures: vec![live2d_ir::model::Texture {
            id: TextureId::new("texture:000000"),
            page_index: 0,
            width: None,
            height: None,
            source_path: None,
            provenance: Provenance::exact("moc3:texture"),
        }],
        glue: Vec::new(),
        bindings: vec![Binding {
            id: BindingId::new("binding:000000"),
            keyform_grid_size: 2,
            keyform_grid: vec![2],
            parameters: vec![BindingParameter {
                parameter: ParameterId::new("ParamAngleX"),
                key_values: vec![-30.0, 30.0],
            }],
            used_by: vec![
                BindingTarget::Part(PartId::new("Part_00")),
                BindingTarget::ArtMesh(ArtMeshId::new("Mesh_00")),
            ],
            provenance: Provenance::derived("moc3:binding"),
        }],
        unknowns: live2d_ir::model::Unknowns::default(),
        diagnostics: Vec::new(),
    }
}

fn pipeline(
    model: &Live2DModel,
) -> (
    hierarchy_recovery::RecoveredProject,
    keyform_recovery::RecoveredKeyformModel,
) {
    let project = reconstruct(model, &RecoveryPolicy::default());
    let keyforms = recover(model, Some(&project));
    (project, keyforms)
}

#[test]
fn strict_mapping_fails_without_texture_asset() {
    let model = base_model();
    let (project, keyforms) = pipeline(&model);
    let result = build_project(
        &model,
        &project,
        &keyforms,
        &TextureAssets::new(),
        &MapOptions::default(),
    );
    assert_eq!(
        result.err().map(|error| error.code),
        Some("MissingTextureAsset")
    );
}

#[test]
fn strict_mapping_succeeds_with_assets_and_is_deterministic() {
    let model = base_model();
    let (project, keyforms) = pipeline(&model);
    let mut assets = TextureAssets::new();
    assets.push(TextureAsset {
        page: 0,
        bytes: vec![0x89, 0x50, 0x4E, 0x47],
        source_path: Some("textures/page0.png".to_string()),
        width: Some(512),
        height: Some(512),
    });
    let first = build_project(&model, &project, &keyforms, &assets, &MapOptions::default())
        .expect("strict mapping");
    let second = build_project(&model, &project, &keyforms, &assets, &MapOptions::default())
        .expect("strict mapping");
    assert_eq!(first, second, "mapping must be deterministic");

    assert_eq!(first.parameters.len(), 1);
    assert_eq!(first.parts.len(), 2, "synthetic root + one part");
    assert!(first.parts[0].is_root);
    assert_eq!(first.meshes.len(), 1);
    assert_eq!(first.meshes[0].texture, Some(0));
    assert_eq!(first.textures.len(), 1);
    assert_eq!(first.textures[0].archive_name, "imageFileBuf_0.png");
    assert_eq!(first.textures[0].byte_len, 4);
    // One binding + one grid per target (part and art mesh share the band but
    // each CMO3 grid source is target-owned).
    assert_eq!(first.bindings.len(), 2);
    assert_eq!(first.grids.len(), 2);
    assert!(first.grids.iter().all(|grid| grid.dense));
    assert!(first.grids.iter().all(|grid| grid.forms.len() == 2));
    // The writer default for base geometry is recorded, never claimed as
    // recovered.
    assert!(first
        .defaults
        .iter()
        .any(|entry| entry.field == "art_mesh.base_positions"));
    // 1D ordering is derived, so no ordering default may be claimed.
    assert!(!first
        .defaults
        .iter()
        .any(|entry| entry.field == "grid.ordering"));
    assert!(!first.best_effort);
}

#[test]
fn archive_entry_names_are_internal_and_page_derived() {
    let mut assets = TextureAssets::new();
    assets.push(TextureAsset {
        page: 3,
        bytes: vec![1],
        source_path: Some("C:/evil/../path.png".to_string()),
        width: None,
        height: None,
    });
    assert_eq!(TextureAssets::archive_name(3), "imageFileBuf_3.png");
    let model = base_model();
    let (project, keyforms) = pipeline(&model);
    let mut assets_two = assets.clone();
    assets_two.push(TextureAsset {
        page: 0,
        bytes: vec![2],
        source_path: None,
        width: None,
        height: None,
    });
    let mapped = build_project(
        &model,
        &project,
        &keyforms,
        &assets_two,
        &MapOptions::default(),
    )
    .expect("mapping");
    assert!(mapped
        .textures
        .iter()
        .all(|texture| texture.archive_name.starts_with("imageFileBuf_")));
    assert!(!mapped
        .textures
        .iter()
        .any(|texture| texture.archive_name.contains('/') || texture.archive_name.contains("..")));
}
