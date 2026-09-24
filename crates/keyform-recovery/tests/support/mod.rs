//! Hand-built Live2D IR models for keyform-recovery tests.
//!
//! These are IR-level fixtures (work order section 53): cases that cannot be
//! expressed in a valid `.moc3` file (sparse grids, dangling references,
//! cardinality limits) are constructed here instead of as binary fixtures.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use live2d_ir::model::BlendMode;
use live2d_ir::model::{
    ArtMesh, ArtMeshKeyform, Binding, BindingParameter, BindingTarget, Canvas, Deformer,
    DeformerCommon, DrawableFlags, EntityCounts, Glue, Live2DModel, MaskGroup, Metadata, Parameter,
    ParameterKind, Part, RotationDeformer, RotationKeyform, SourceFormat, Texture, Unknowns,
    WarpDeformer, WarpKeyform,
};
use live2d_ir::{
    ArtMeshId, BindingId, DeformerId, Endianness, GlueId, ParameterId, PartId, Provenance,
    SchemaStatus, TextureId, Uv, Vec2,
};

/// Build an empty model skeleton.
pub fn base_model() -> Live2DModel {
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
            generator: "keyform-test".to_string(),
            generator_phase: "AGENT.4-test".to_string(),
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
            width: 800.0,
            height: 600.0,
            origin: Vec2 { x: 400.0, y: 300.0 },
            pixels_per_unit: 1000.0,
            y_axis_reversed: false,
            field_provenance: Vec::new(),
            provenance: Provenance::exact("moc3:canvas"),
        },
        parameters: Vec::new(),
        parts: Vec::new(),
        deformers: Vec::new(),
        art_meshes: Vec::new(),
        drawables: Vec::new(),
        draw_order_groups: Vec::new(),
        mask_groups: Vec::<MaskGroup>::new(),
        textures: Vec::<Texture>::new(),
        glue: Vec::new(),
        bindings: Vec::new(),
        unknowns: Unknowns::default(),
        diagnostics: Vec::new(),
    }
}

/// Add a parameter with key values.
pub fn add_parameter(
    model: &mut Live2DModel,
    id: &str,
    keys: &[f32],
    kind: ParameterKind,
) -> ParameterId {
    let parameter = Parameter {
        id: ParameterId::new(id),
        source_name: Some(id.to_string()),
        minimum: -30.0,
        maximum: 30.0,
        default: 0.0,
        current: None,
        repeat: false,
        decimal_places: 2,
        kind,
        key_values: keys.to_vec(),
        extension_key_values: Vec::new(),
        field_provenance: Vec::new(),
        provenance: Provenance::exact("moc3:parameter"),
    };
    model.parameters.push(parameter);
    ParameterId::new(id)
}

/// Add a binding with the given axes (`(parameter id, keys)`).
pub fn add_binding<T: AsRef<[f32]>>(
    model: &mut Live2DModel,
    id: &str,
    axes: &[(&str, T)],
) -> BindingId {
    let parameters: Vec<BindingParameter> = axes
        .iter()
        .map(|(parameter, keys)| BindingParameter {
            parameter: ParameterId::new(*parameter),
            key_values: keys.as_ref().to_vec(),
        })
        .collect();
    let grid: Vec<usize> = parameters
        .iter()
        .map(|parameter| parameter.key_values.len())
        .collect();
    let grid_size = grid
        .iter()
        .fold(1usize, |acc, value| acc.saturating_mul(*value));
    model.bindings.push(Binding {
        id: BindingId::new(id),
        keyform_grid_size: grid_size,
        keyform_grid: grid,
        parameters,
        used_by: Vec::new(),
        provenance: Provenance::derived("moc3:binding"),
    });
    BindingId::new(id)
}

/// Register a used_by target on a binding.
pub fn bind_used_by(model: &mut Live2DModel, binding: &str, target: BindingTarget) {
    let binding = model
        .bindings
        .iter_mut()
        .find(|entry| entry.id.as_str() == binding)
        .expect("binding exists");
    binding.used_by.push(target);
    binding.used_by.sort();
    binding.used_by.dedup();
}

/// Add a part.
pub fn add_part(
    model: &mut Live2DModel,
    id: &str,
    binding: Option<&str>,
    draw_orders: &[f32],
) -> PartId {
    model.parts.push(Part {
        id: PartId::new(id),
        source_name: Some(id.to_string()),
        parent: None,
        children: Vec::new(),
        opacity: None,
        draw_order: None,
        keyform_draw_orders: draw_orders.to_vec(),
        binding: binding.map(BindingId::new),
        visible: true,
        enabled: true,
        field_provenance: Vec::new(),
        provenance: Provenance::exact("moc3:part"),
    });
    PartId::new(id)
}

fn deformer_common(id: &str, binding: Option<&str>) -> DeformerCommon {
    DeformerCommon {
        id: DeformerId::new(id),
        source_name: Some(id.to_string()),
        parent_part: None,
        parent_deformer: None,
        children_deformers: Vec::new(),
        children_art_meshes: Vec::new(),
        binding: binding.map(BindingId::new),
        visible: true,
        enabled: true,
        field_provenance: Vec::new(),
        provenance: Provenance::exact("moc3:deformer"),
    }
}

/// Add a warp deformer with `forms` as `(opacity, vertex count)`.
pub fn add_warp(
    model: &mut Live2DModel,
    id: &str,
    binding: Option<&str>,
    forms: &[(f32, usize)],
) -> DeformerId {
    model.deformers.push(Deformer::Warp(WarpDeformer {
        common: deformer_common(id, binding),
        rows: 1,
        columns: 1,
        vertex_count: 4,
        quad_transform: false,
        keyforms: forms
            .iter()
            .enumerate()
            .map(|(index, (opacity, positions))| WarpKeyform {
                index,
                opacity: *opacity,
                positions: vec![Vec2 { x: 0.0, y: 0.0 }; *positions],
            })
            .collect(),
    }));
    DeformerId::new(id)
}

/// Add a rotation deformer with the given form count.
pub fn add_rotation(
    model: &mut Live2DModel,
    id: &str,
    binding: Option<&str>,
    form_count: usize,
) -> DeformerId {
    model.deformers.push(Deformer::Rotation(RotationDeformer {
        common: deformer_common(id, binding),
        base_angle: 0.0,
        keyforms: (0..form_count)
            .map(|index| RotationKeyform {
                index,
                opacity: 1.0,
                angle: index as f32,
                origin: Vec2 { x: 0.0, y: 0.0 },
                scale: 1.0,
                reflect_x: false,
                reflect_y: false,
            })
            .collect(),
    }));
    DeformerId::new(id)
}

/// Add an art mesh with `forms` as `(opacity, draw_order, vertex count)`.
pub fn add_mesh(
    model: &mut Live2DModel,
    id: &str,
    binding: Option<&str>,
    forms: &[(f32, f32, usize)],
) -> ArtMeshId {
    let vertex_count = forms.first().map(|(_, _, count)| *count).unwrap_or(4);
    model.art_meshes.push(ArtMesh {
        id: ArtMeshId::new(id),
        source_name: Some(id.to_string()),
        parent_part: None,
        parent_deformer: None,
        texture: Some(TextureId::new("texture:000000")),
        vertex_count,
        uvs: vec![Uv { u: 0.5, v: 0.5 }; vertex_count],
        indices: vec![0, 1, 2, 0, 2, 3],
        keyforms: forms
            .iter()
            .enumerate()
            .map(|(index, (opacity, draw_order, positions))| ArtMeshKeyform {
                index,
                opacity: *opacity,
                draw_order: *draw_order,
                positions: vec![Vec2 { x: 1.0, y: 1.0 }; *positions],
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
        binding: binding.map(BindingId::new),
        visible: true,
        enabled: true,
        mask_groups: Vec::new(),
        field_provenance: Vec::new(),
        provenance: Provenance::exact("moc3:art_mesh"),
    });
    model.drawables.push(ArtMeshId::new(id));
    ArtMeshId::new(id)
}

/// Add a glue entry referencing a binding (for KB-006 deferral tests).
pub fn add_glue(model: &mut Live2DModel, id: &str, a: &str, b: &str, binding: &str) -> GlueId {
    model.glue.push(Glue {
        id: GlueId::new(id),
        source_name: Some(id.to_string()),
        art_mesh_a: ArtMeshId::new(a),
        art_mesh_b: ArtMeshId::new(b),
        binding: Some(BindingId::new(binding)),
        keyform_intensities: Vec::new(),
        info: Vec::new(),
        field_provenance: Vec::new(),
        provenance: Provenance::exact("moc3:glue"),
    });
    GlueId::new(id)
}
