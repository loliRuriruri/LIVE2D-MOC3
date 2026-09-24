//! Shared fixtures for the minimal-CMO3 integration tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cmo3_writer::model::Cmo3Project;
use cmo3_writer::textures::{TextureAsset, TextureAssets};

use live2d_ir::model::{
    ArtMesh, ArtMeshKeyform, Binding, BindingParameter, BindingTarget, BlendMode, Canvas,
    DrawableFlags, EntityCounts, Live2DModel, Metadata, Parameter, ParameterKind, Part,
    SourceFormat, Texture,
};
use live2d_ir::{
    ArtMeshId, BindingId, Endianness, ParameterId, PartId, Provenance, SchemaStatus, TextureId, Uv,
    Vec2,
};

/// Build a canvas-sized solid-color PNG (stored-deflate zlib, deterministic).
pub fn solid_png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut raw: Vec<u8> = Vec::with_capacity((height as usize) * (1 + width as usize * 4));
    let mut row: Vec<u8> = Vec::with_capacity(1 + width as usize * 4);
    row.push(0); // filter: none
    for _ in 0..width {
        row.extend_from_slice(&rgba);
    }
    for _ in 0..height {
        raw.extend_from_slice(&row);
    }
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut ihdr: Vec<u8> = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    out.extend_from_slice(&chunk(b"IHDR", &ihdr));
    out.extend_from_slice(&chunk(b"IDAT", &zlib_stored(&raw)));
    out.extend_from_slice(&chunk(b"IEND", &[]));
    out
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input: Vec<u8> = Vec::new();
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    out
}

fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = vec![0x78, 0x01];
    let mut offset = 0usize;
    while offset < raw.len() {
        let take = (raw.len() - offset).min(65_535);
        let last = offset + take == raw.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(take as u16).to_le_bytes());
        out.extend_from_slice(&(!(take as u16)).to_le_bytes());
        out.extend_from_slice(&raw[offset..offset + take]);
        offset += take;
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for byte in bytes {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

/// Canvas-sized assets for `count` pages with distinct colors.
pub fn assets(count: usize) -> TextureAssets {
    let mut assets = TextureAssets::new();
    for page in 0..count {
        let color = [(page as u8).wrapping_mul(60), 64, 128, 255];
        assets.push(TextureAsset {
            page: page as u32,
            bytes: solid_png(512, 512, color),
            source_path: Some(format!("textures/page{page}.png")),
            width: Some(512),
            height: Some(512),
        });
    }
    assets
}

/// A 512x512 model with `mesh_count` textured quads sharing one binding.
pub fn base_model(mesh_count: usize) -> Live2DModel {
    let mut art_meshes: Vec<ArtMesh> = Vec::new();
    let mut drawables: Vec<ArtMeshId> = Vec::new();
    let mut bindings_used_by: Vec<BindingTarget> =
        vec![BindingTarget::Part(PartId::new("Part_00"))];
    for index in 0..mesh_count {
        let id = format!("Mesh_{index:02}");
        art_meshes.push(ArtMesh {
            id: ArtMeshId::new(id.clone()),
            source_name: Some(id.clone()),
            parent_part: Some(PartId::new("Part_00")),
            parent_deformer: None,
            texture: Some(TextureId::new(format!("texture:{index:06}"))),
            vertex_count: 4,
            uvs: vec![Uv { u: 0.0, v: 0.0 }; 4],
            indices: vec![0, 1, 2, 0, 2, 3],
            keyforms: (0..2)
                .map(|form| ArtMeshKeyform {
                    index: form,
                    opacity: 1.0,
                    draw_order: form as f32,
                    positions: vec![
                        Vec2 { x: 0.0, y: 0.0 },
                        Vec2 { x: 512.0, y: 0.0 },
                        Vec2 { x: 512.0, y: 512.0 },
                        Vec2 { x: 0.0, y: 512.0 },
                    ],
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
        });
        drawables.push(ArtMeshId::new(id.clone()));
        bindings_used_by.push(BindingTarget::ArtMesh(ArtMeshId::new(id)));
    }
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
            generator_phase: "AGENT.5.1-test".to_string(),
            entity_counts: EntityCounts {
                parameters: 1,
                parts: 1,
                deformers: 0,
                warp_deformers: 0,
                rotation_deformers: 0,
                art_meshes: mesh_count,
                draw_order_groups: 0,
                mask_groups: 0,
                textures: mesh_count,
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
        art_meshes,
        drawables,
        draw_order_groups: Vec::new(),
        mask_groups: Vec::new(),
        textures: (0..mesh_count)
            .map(|index| Texture {
                id: TextureId::new(format!("texture:{index:06}")),
                page_index: index as u32,
                width: None,
                height: None,
                source_path: None,
                provenance: Provenance::exact("moc3:texture"),
            })
            .collect(),
        glue: Vec::new(),
        bindings: vec![Binding {
            id: BindingId::new("binding:000000"),
            keyform_grid_size: 2,
            keyform_grid: vec![2],
            parameters: vec![BindingParameter {
                parameter: ParameterId::new("ParamAngleX"),
                key_values: vec![-30.0, 30.0],
            }],
            used_by: bindings_used_by,
            provenance: Provenance::derived("moc3:binding"),
        }],
        unknowns: live2d_ir::model::Unknowns::default(),
        diagnostics: Vec::new(),
    }
}

/// Normalized image-pipeline snapshot (work order section 45).
pub fn pipeline_snapshot(project: &Cmo3Project) -> serde_json::Value {
    let layered = project.layered_images.first();
    serde_json::json!({
        "layered_images": project.layered_images.len(),
        "layers": layered.map(|layered| layered.layers.len()).unwrap_or(0),
        "model_images": project.model_images.len(),
        "filter_sets": project.model_images.len(),
        "image_resources": project.image_resources.len(),
        "texture_inputs": project.meshes.iter().filter(|mesh| mesh.model_image.is_some()).count(),
        "artmesh_to_modelimage": project.meshes.iter().enumerate()
            .filter_map(|(index, mesh)| mesh.model_image.map(|model_image| (index, model_image)))
            .collect::<Vec<_>>(),
        "layer_selector_relations": project.model_images.iter()
            .map(|model_image| (model_image.layer, model_image.resource))
            .collect::<Vec<_>>(),
        "layers_detail": layered.map(|layered| layered.layers.iter()
            .map(|layer| serde_json::json!({
                "name": layer.name,
                "layer_id": layer.layer_id,
                "resource": layer.resource,
            }))
            .collect::<Vec<_>>()).unwrap_or_default(),
    })
}
