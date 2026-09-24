//! Compact hand-built IR models for hierarchy tests.
//!
//! Deliberately built without the parser: this crate must never depend on
//! `moc3-ingest`, and its tests prove that by using `live2d-ir` types only.
#![allow(dead_code)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use live2d_ir::{
    ArtMesh, ArtMeshId, Binding, BindingId, BindingParameter, BindingTarget, BlendMode, Canvas,
    Confidence, Deformer, DeformerCommon, DeformerId, DrawableFlags, Endianness, EntityCounts,
    Live2DModel, Metadata, ParameterId, Part, PartId, Provenance, RotationDeformer,
    RotationKeyform, SchemaStatus, SourceFormat, Vec2, WarpDeformer, WarpKeyform, SCHEMA_ID,
};

fn exact(source: &str) -> Provenance {
    Provenance::exact(source.to_string())
}

/// Build a part with an optional stored parent.
pub fn part(name: &str, parent: Option<&str>) -> Part {
    Part {
        id: PartId::new(name),
        source_name: Some(name.to_string()),
        parent: parent.map(PartId::new),
        children: Vec::new(),
        opacity: None,
        draw_order: None,
        keyform_draw_orders: Vec::new(),
        binding: None,
        visible: true,
        enabled: true,
        field_provenance: Vec::new(),
        provenance: exact("test:part"),
    }
}

fn common(
    name: &str,
    parent_deformer: Option<&str>,
    parent_part: Option<&str>,
    binding: Option<&str>,
) -> DeformerCommon {
    DeformerCommon {
        id: DeformerId::new(name),
        source_name: Some(name.to_string()),
        parent_part: parent_part.map(PartId::new),
        parent_deformer: parent_deformer.map(DeformerId::new),
        children_deformers: Vec::new(),
        children_art_meshes: Vec::new(),
        binding: binding.map(BindingId::new),
        visible: true,
        enabled: true,
        field_provenance: Vec::new(),
        provenance: exact("test:deformer"),
    }
}

/// Build a warp deformer (4 control vertices).
pub fn warp(name: &str, parent_deformer: Option<&str>, parent_part: Option<&str>) -> Deformer {
    Deformer::Warp(WarpDeformer {
        common: common(name, parent_deformer, parent_part, None),
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
    })
}

/// Build a rotation deformer.
pub fn rotation(name: &str, parent_deformer: Option<&str>, parent_part: Option<&str>) -> Deformer {
    Deformer::Rotation(RotationDeformer {
        common: common(name, parent_deformer, parent_part, None),
        base_angle: 0.0,
        keyforms: vec![RotationKeyform {
            index: 0,
            opacity: 1.0,
            angle: 0.0,
            origin: Vec2::new(0.0, 0.0),
            scale: 1.0,
            reflect_x: false,
            reflect_y: false,
        }],
    })
}

/// Build an art mesh.
pub fn mesh(name: &str, parent_deformer: Option<&str>, parent_part: Option<&str>) -> ArtMesh {
    ArtMesh {
        id: ArtMeshId::new(name),
        source_name: Some(name.to_string()),
        parent_part: parent_part.map(PartId::new),
        parent_deformer: parent_deformer.map(DeformerId::new),
        texture: None,
        vertex_count: 0,
        uvs: Vec::new(),
        indices: Vec::new(),
        keyforms: Vec::new(),
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
        visible: true,
        enabled: true,
        mask_groups: Vec::new(),
        field_provenance: Vec::new(),
        provenance: exact("test:art_mesh"),
    }
}

/// Attach a binding to an art mesh.
pub fn with_binding(mut mesh: ArtMesh, binding: &str) -> ArtMesh {
    mesh.binding = Some(BindingId::new(binding));
    mesh
}

/// Attach a binding to a part.
pub fn with_part_binding(mut part: Part, binding: &str) -> Part {
    part.binding = Some(BindingId::new(binding));
    part
}

/// Build a binding used by the given targets (deterministic order).
pub fn binding(id: &str, used_by: Vec<BindingTarget>) -> Binding {
    Binding {
        id: BindingId::new(id),
        keyform_grid_size: 1,
        keyform_grid: Vec::new(),
        parameters: Vec::<BindingParameter>::new(),
        used_by,
        provenance: Provenance::new(Confidence::Derived, "test:binding".to_string()),
    }
}

/// Build a complete valid model from parts/deformers/meshes/bindings.
pub fn model(
    parts: Vec<Part>,
    deformers: Vec<Deformer>,
    art_meshes: Vec<ArtMesh>,
    bindings: Vec<Binding>,
) -> Live2DModel {
    let drawables = art_meshes.iter().map(|mesh| mesh.id.clone()).collect();
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
                draw_order_groups: 0,
                mask_groups: 0,
                textures: 0,
                glue: 0,
                bindings: 0,
            },
        },
        canvas: Canvas {
            width: 1.0,
            height: 1.0,
            origin: Vec2::new(0.0, 0.0),
            pixels_per_unit: 1.0,
            y_axis_reversed: false,
            field_provenance: Vec::new(),
            provenance: exact("test:canvas"),
        },
        parameters: Vec::<live2d_ir::Parameter>::new(),
        parts,
        deformers,
        art_meshes,
        drawables,
        draw_order_groups: Vec::new(),
        mask_groups: Vec::new(),
        textures: Vec::new(),
        glue: Vec::new(),
        bindings,
        unknowns: Default::default(),
        diagnostics: Vec::new(),
    };
    model.metadata.entity_counts = model.entity_counts();
    model
}

/// Part-target binding helper.
pub fn target_part(name: &str) -> BindingTarget {
    BindingTarget::Part(PartId::new(name))
}

/// Parameter id helper (unused parameter list entries would need real data).
pub fn parameter_id(name: &str) -> ParameterId {
    ParameterId::new(name)
}
