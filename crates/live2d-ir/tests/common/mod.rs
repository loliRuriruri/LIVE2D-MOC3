//! Hand-built IR models for validator tests.
//!
//! Deliberately constructed without any parser: this is also what keeps the
//! crate's test surface independent from `moc3-ingest`.
#![allow(dead_code)]

use live2d_ir::{
    ArtMesh, ArtMeshId, ArtMeshKeyform, BlendMode, Canvas, Confidence, Deformer, DeformerCommon,
    DeformerId, DrawableFlags, Endianness, EntityCounts, Live2DModel, Metadata, Parameter,
    ParameterId, ParameterKind, Part, PartId, Provenance, SchemaStatus, SourceFormat, Texture,
    TextureId, Uv, Vec2, WarpDeformer, WarpKeyform, SCHEMA_ID,
};

fn exact(source: &str) -> Provenance {
    Provenance::exact(source.to_string())
}

/// A structurally valid minimal model: 1 parameter, 1 part, 1 warp-less art
/// mesh, 1 texture.
pub fn minimal_model() -> Live2DModel {
    let parameter = Parameter {
        id: ParameterId::new("ParamAngleX"),
        source_name: Some("ParamAngleX".to_string()),
        minimum: -30.0,
        maximum: 30.0,
        default: 0.0,
        current: None,
        repeat: false,
        decimal_places: 2,
        kind: ParameterKind::Normal,
        key_values: vec![-30.0, 30.0],
        extension_key_values: Vec::new(),
        field_provenance: Vec::new(),
        provenance: exact("test:parameter"),
    };
    let part = Part {
        id: PartId::new("PartHead"),
        source_name: Some("PartHead".to_string()),
        parent: None,
        children: Vec::new(),
        opacity: None,
        draw_order: None,
        keyform_draw_orders: Vec::new(),
        binding: None,
        visible: true,
        enabled: true,
        offscreen_surface_index: None,
        field_provenance: vec![
            live2d_ir::FieldProvenance {
                field: "opacity".to_string(),
                provenance: Provenance::unknown("test:part", "not stored"),
            },
            live2d_ir::FieldProvenance {
                field: "draw_order".to_string(),
                provenance: Provenance::unknown("test:part", "not stored"),
            },
        ],
        provenance: exact("test:part"),
    };
    let mesh = ArtMesh {
        id: ArtMeshId::new("ArtMeshFace"),
        source_name: Some("ArtMeshFace".to_string()),
        parent_part: Some(PartId::new("PartHead")),
        parent_deformer: None,
        texture: Some(TextureId::new("texture:000000")),
        vertex_count: 4,
        uvs: vec![
            Uv::new(0.0, 0.0),
            Uv::new(1.0, 0.0),
            Uv::new(1.0, 1.0),
            Uv::new(0.0, 1.0),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        keyforms: vec![ArtMeshKeyform {
            index: 0,
            opacity: 1.0,
            draw_order: 0.0,
            positions: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
        }],
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
        binding: None,
        mask_groups: Vec::new(),
        field_provenance: vec![
            live2d_ir::FieldProvenance {
                field: "opacity".to_string(),
                provenance: Provenance::unknown("test:art_mesh", "not stored"),
            },
            live2d_ir::FieldProvenance {
                field: "draw_order".to_string(),
                provenance: Provenance::unknown("test:art_mesh", "not stored"),
            },
        ],
        provenance: exact("test:art_mesh"),
    };
    let texture = Texture {
        id: TextureId::new("texture:000000"),
        page_index: 0,
        width: None,
        height: None,
        source_path: None,
        provenance: Provenance::new(Confidence::Derived, "test:texture".to_string()),
    };

    let mut model = Live2DModel {
        schema: SCHEMA_ID.to_string(),
        schema_status: SchemaStatus::Experimental,
        source: SourceFormat {
            format: "moc3".to_string(),
            version_byte: 2,
            version_label: "2 (3.3.00-3.3.03)".to_string(),
            byte_order: Endianness::Little,
        },
        metadata: Metadata {
            generator: "test".to_string(),
            generator_phase: "test".to_string(),
            entity_counts: EntityCounts {
                parameters: 0,
                parts: 0,
                deformers: 0,
                warp_deformers: 0,
                rotation_deformers: 0,
                art_meshes: 0,
                mask_groups: 0,
                textures: 0,
                glue: 0,
                bindings: 0,
            },
        },
        canvas: Canvas {
            width: 800.0,
            height: 600.0,
            origin: Vec2::new(400.0, 300.0),
            pixels_per_unit: 1000.0,
            y_axis_reversed: false,
            field_provenance: Vec::new(),
            provenance: exact("test:canvas"),
        },
        parameters: vec![parameter],
        parts: vec![part],
        deformers: Vec::new(),
        art_meshes: vec![mesh],
        drawables: vec![ArtMeshId::new("ArtMeshFace")],
        mask_groups: Vec::new(),
        textures: vec![texture],
        glue: Vec::new(),
        bindings: Vec::new(),
        unknowns: Default::default(),
        diagnostics: Vec::new(),
    };
    model.metadata.entity_counts = model.entity_counts();
    model
}

/// Add a valid 2x2 warp deformer (childless) to the model.
pub fn push_warp(model: &mut Live2DModel) {
    let common = DeformerCommon {
        id: DeformerId::new("warp:000000"),
        source_name: None,
        parent_part: Some(PartId::new("PartHead")),
        parent_deformer: None,
        children_deformers: Vec::new(),
        children_art_meshes: Vec::new(),
        binding: None,
        visible: true,
        enabled: true,
        field_provenance: Vec::new(),
        provenance: exact("test:deformer"),
    };
    model.deformers.push(Deformer::Warp(WarpDeformer {
        common,
        rows: 1,
        columns: 1,
        vertex_count: 4,
        quad_transform: false,
        keyforms: vec![WarpKeyform {
            index: 0,
            opacity: 1.0,
            positions: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(1.0, 1.0),
            ],
        }],
    }));
    model.metadata.entity_counts = model.entity_counts();
}
