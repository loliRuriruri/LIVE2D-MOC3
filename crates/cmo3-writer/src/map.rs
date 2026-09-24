//! Semantic -> writer-model mapping (serialization only, no inference).
//!
//! Every required semantic that is unresolved fails in strict mode (default)
//! and is reported as unsupported in best-effort mode (work order sections
//! 37-38, 81-82). Nothing is fabricated: writer-required defaults are
//! recorded in [`Cmo3Project::defaults`]. Deformer/part references are
//! resolved into a typed [`ParentRef`], so warp and rotation index spaces can
//! never be mixed.

use std::collections::BTreeMap;

use hierarchy_recovery::RecoveredProject;
use keyform_recovery::{GridLayout, RecoveredKeyformModel, TargetKeyforms};
use live2d_ir::{Deformer, Live2DModel};

use crate::model::{
    ArtMeshFormOut, ArtMeshOut, BindingOut, CanvasOut, ChildRef, Cmo3Project, GridFormOut, GridOut,
    ImageResourceOut, LayerOut, LayeredImageOut, ModelImageOut, ParameterOut, ParentRef, PartOut,
    RotationFormOut, RotationOut, TargetOut, TextureOut, UnsupportedNote, WarpFormOut, WarpOut,
    WriterDefault,
};
use crate::textures::TextureAssets;

/// Mapping options.
#[derive(Debug, Clone, Default)]
pub struct MapOptions {
    /// Allow unresolved required semantics (output is labelled BEST_EFFORT).
    pub best_effort: bool,
    /// Model name override (defaults to the synthetic `RecoveredModel`).
    pub model_name: Option<String>,
}

/// Structured mapping failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteError {
    /// Stable code.
    pub code: &'static str,
    /// Detail.
    pub message: String,
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for WriteError {}

fn error(code: &'static str, message: impl Into<String>) -> WriteError {
    WriteError {
        code,
        message: message.into(),
    }
}

fn finite(value: f32, semantic: &str, field: &str) -> Result<f32, WriteError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(error(
            "NonFiniteValue",
            format!("'{semantic}' field '{field}' is not finite"),
        ))
    }
}

fn finite_all(values: &[f32], semantic: &str, field: &str) -> Result<(), WriteError> {
    for value in values {
        finite(*value, semantic, field)?;
    }
    Ok(())
}

/// Build the writer project model from the semantic layers.
pub fn build_project(
    model: &Live2DModel,
    project: &RecoveredProject,
    keyforms: &RecoveredKeyformModel,
    textures: &TextureAssets,
    options: &MapOptions,
) -> Result<Cmo3Project, WriteError> {
    let mut defaults: Vec<WriterDefault> = Vec::new();
    let mut unsupported: Vec<UnsupportedNote> = Vec::new();
    let best_effort = options.best_effort;

    // ---- parameters ------------------------------------------------------
    let mut parameters: Vec<ParameterOut> = Vec::with_capacity(model.parameters.len());
    for (index, parameter) in model.parameters.iter().enumerate() {
        let (name, synthetic_name) = match &parameter.source_name {
            Some(name) if !name.is_empty() => (name.clone(), false),
            _ => (format!("Param_{:06}", index + 1), true),
        };
        if synthetic_name {
            defaults.push(WriterDefault {
                semantic: parameter.id.as_str().to_string(),
                field: "parameter.name",
                reason: "no stored parameter name; deterministic placeholder generated",
            });
        }
        let semantic = parameter.id.as_str();
        finite(parameter.minimum, semantic, "minimum")?;
        finite(parameter.maximum, semantic, "maximum")?;
        finite(parameter.default, semantic, "default")?;
        finite_all(&parameter.key_values, semantic, "key_values")?;
        finite_all(
            &parameter.extension_key_values,
            semantic,
            "extension_key_values",
        )?;
        parameters.push(ParameterOut {
            semantic: semantic.to_string(),
            name,
            minimum: parameter.minimum,
            maximum: parameter.maximum,
            default: parameter.default,
            decimals: parameter.decimal_places,
            repeat: parameter.repeat,
            keys: parameter.key_values.clone(),
            synthetic_name,
        });
    }
    let parameter_index: BTreeMap<&str, usize> = parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| (parameter.semantic.as_str(), index))
        .collect();

    // ---- hierarchy helpers ----------------------------------------------
    let node_parent: BTreeMap<&str, &str> = project
        .nodes
        .iter()
        .map(|node| {
            (
                node.id.as_str(),
                node.parent
                    .as_ref()
                    .map(|id| id.as_str())
                    .unwrap_or("$root"),
            )
        })
        .collect();
    let part_by_id: BTreeMap<&str, usize> = model
        .parts
        .iter()
        .enumerate()
        .map(|(index, part)| (part.id.as_str(), index + 1))
        .collect();
    let warp_ids: Vec<&str> = model
        .deformers
        .iter()
        .filter_map(|deformer| match deformer {
            Deformer::Warp(warp) => Some(warp.common.id.as_str()),
            Deformer::Rotation(_) => None,
        })
        .collect();
    let rotation_ids: Vec<&str> = model
        .deformers
        .iter()
        .filter_map(|deformer| match deformer {
            Deformer::Rotation(rotation) => Some(rotation.common.id.as_str()),
            Deformer::Warp(_) => None,
        })
        .collect();
    let warp_by_id: BTreeMap<&str, usize> = warp_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();
    let rotation_by_id: BTreeMap<&str, usize> = rotation_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();
    let mesh_by_id: BTreeMap<&str, usize> = model
        .art_meshes
        .iter()
        .enumerate()
        .map(|(index, mesh)| (mesh.id.as_str(), index))
        .collect();

    let classify_parent = |id: &str| -> ParentRef {
        if let Some(index) = part_by_id.get(id).copied() {
            return ParentRef::Part(index);
        }
        if warp_by_id.contains_key(id) {
            return ParentRef::Warp(id.to_string());
        }
        if rotation_by_id.contains_key(id) {
            return ParentRef::Rotation(id.to_string());
        }
        ParentRef::Root
    };
    let parent_of = |id: &str| -> ParentRef {
        classify_parent(node_parent.get(id).copied().unwrap_or("$root"))
    };

    // ---- parts -----------------------------------------------------------
    let mut parts: Vec<PartOut> = Vec::with_capacity(model.parts.len() + 1);
    parts.push(PartOut {
        semantic: "$root".to_string(),
        name: "Root".to_string(),
        parent: ParentRef::Root,
        children: Vec::new(),
        draw_orders: Vec::new(),
        grid: None,
        visible: true,
        enabled: true,
        is_root: true,
        synthetic_name: true,
    });
    for (index, part) in model.parts.iter().enumerate() {
        let (name, synthetic_name) = match &part.source_name {
            Some(name) if !name.is_empty() => (name.clone(), false),
            _ => (format!("Part_{:06}", index + 1), true),
        };
        if synthetic_name {
            defaults.push(WriterDefault {
                semantic: part.id.as_str().to_string(),
                field: "part.name",
                reason: "no stored part name; deterministic placeholder generated",
            });
        }
        finite_all(&part.keyform_draw_orders, part.id.as_str(), "draw_orders")?;
        parts.push(PartOut {
            semantic: part.id.as_str().to_string(),
            name,
            parent: parent_of(part.id.as_str()),
            children: Vec::new(),
            draw_orders: part.keyform_draw_orders.clone(),
            grid: None,
            visible: part.visible,
            enabled: part.enabled,
            is_root: false,
            synthetic_name,
        });
    }

    // ---- deformers -------------------------------------------------------
    let mut warps: Vec<WarpOut> = Vec::new();
    let mut rotations: Vec<RotationOut> = Vec::new();
    for deformer in &model.deformers {
        match deformer {
            Deformer::Warp(warp) => {
                let index = warps.len();
                let (name, synthetic_name) = match &warp.common.source_name {
                    Some(name) if !name.is_empty() => (name.clone(), false),
                    _ => (format!("WarpDeformer_{:06}", index + 1), true),
                };
                if synthetic_name {
                    defaults.push(WriterDefault {
                        semantic: warp.common.id.as_str().to_string(),
                        field: "warp.name",
                        reason: "no stored deformer name; deterministic placeholder generated",
                    });
                }
                let forms: Result<Vec<WarpFormOut>, WriteError> = warp
                    .keyforms
                    .iter()
                    .map(|form| {
                        finite(form.opacity, warp.common.id.as_str(), "opacity")?;
                        Ok(WarpFormOut {
                            opacity: form.opacity,
                            positions: to_positions(&form.positions)?,
                        })
                    })
                    .collect();
                let parent = deformer_parent(model, &parent_of, warp.common.id.as_str());
                warps.push(WarpOut {
                    semantic: warp.common.id.as_str().to_string(),
                    name,
                    parent,
                    children: Vec::new(),
                    columns: warp.columns,
                    rows: warp.rows,
                    quad_transform: warp.quad_transform,
                    forms: forms?,
                    grid: None,
                    synthetic_name,
                });
            }
            Deformer::Rotation(rotation) => {
                let index = rotations.len();
                let (name, synthetic_name) = match &rotation.common.source_name {
                    Some(name) if !name.is_empty() => (name.clone(), false),
                    _ => (format!("RotationDeformer_{:06}", index + 1), true),
                };
                if synthetic_name {
                    defaults.push(WriterDefault {
                        semantic: rotation.common.id.as_str().to_string(),
                        field: "rotation.name",
                        reason: "no stored deformer name; deterministic placeholder generated",
                    });
                }
                finite(
                    rotation.base_angle,
                    rotation.common.id.as_str(),
                    "base_angle",
                )?;
                rotations.push(RotationOut {
                    semantic: rotation.common.id.as_str().to_string(),
                    name,
                    parent: deformer_parent(model, &parent_of, rotation.common.id.as_str()),
                    children: Vec::new(),
                    base_angle: rotation.base_angle,
                    forms: rotation
                        .keyforms
                        .iter()
                        .map(|form| {
                            Ok(RotationFormOut {
                                opacity: finite(
                                    form.opacity,
                                    rotation.common.id.as_str(),
                                    "opacity",
                                )?,
                                angle: finite(form.angle, rotation.common.id.as_str(), "angle")?,
                                origin: [
                                    finite(form.origin.x, rotation.common.id.as_str(), "origin_x")?,
                                    finite(form.origin.y, rotation.common.id.as_str(), "origin_y")?,
                                ],
                                scale: finite(form.scale, rotation.common.id.as_str(), "scale")?,
                                reflect_x: form.reflect_x,
                                reflect_y: form.reflect_y,
                            })
                        })
                        .collect::<Result<Vec<_>, WriteError>>()?,
                    grid: None,
                    synthetic_name,
                });
            }
        }
    }

    // ---- art meshes ------------------------------------------------------
    let mut meshes: Vec<ArtMeshOut> = Vec::with_capacity(model.art_meshes.len());
    let texture_page_to_index: BTreeMap<u32, usize> = textures
        .assets
        .iter()
        .enumerate()
        .map(|(index, asset)| (asset.page, index))
        .collect();
    for (index, mesh) in model.art_meshes.iter().enumerate() {
        let semantic = mesh.id.as_str();
        let (name, synthetic_name) = match &mesh.source_name {
            Some(name) if !name.is_empty() => (name.clone(), false),
            _ => (format!("ArtMesh_{:06}", index + 1), true),
        };
        if synthetic_name {
            defaults.push(WriterDefault {
                semantic: semantic.to_string(),
                field: "art_mesh.name",
                reason: "no stored mesh name; deterministic placeholder generated",
            });
        }
        let uvs = to_uvs(mesh)?;
        let base_positions = to_positions(&collect_base_positions(mesh))?;
        if base_positions.len() != mesh.vertex_count {
            return Err(error(
                "InvalidGeometry",
                format!(
                    "art mesh '{semantic}' has {} base positions for {} vertices",
                    base_positions.len(),
                    mesh.vertex_count
                ),
            ));
        }
        if !mesh.keyforms.is_empty() {
            defaults.push(WriterDefault {
                semantic: semantic.to_string(),
                field: "art_mesh.base_positions",
                reason: "the IR stores no explicit base geometry; the first stored form is used",
            });
        } else {
            defaults.push(WriterDefault {
                semantic: semantic.to_string(),
                field: "art_mesh.base_positions",
                reason: "no stored form exists; a zeroed base geometry placeholder is written",
            });
        }
        if mesh
            .indices
            .iter()
            .any(|value| *value as usize >= mesh.vertex_count)
        {
            return Err(error(
                "InvalidGeometry",
                format!("art mesh '{semantic}' has out-of-range indices"),
            ));
        }
        let forms: Result<Vec<ArtMeshFormOut>, WriteError> = mesh
            .keyforms
            .iter()
            .map(|form| {
                finite(form.opacity, semantic, "opacity")?;
                finite(form.draw_order, semantic, "draw_order")?;
                Ok(ArtMeshFormOut {
                    draw_order: form.draw_order,
                    opacity: form.opacity,
                    positions: to_positions(&form.positions)?,
                })
            })
            .collect();
        let texture = match mesh.texture.as_ref() {
            None => None,
            Some(texture) => match texture_page_of(model, texture.as_str()) {
                None => {
                    if best_effort {
                        unsupported.push(UnsupportedNote {
                            subject: semantic.to_string(),
                            reason: "texture reference does not resolve to a texture entity"
                                .to_string(),
                        });
                        None
                    } else {
                        return Err(error(
                            "UnresolvedTexture",
                            format!(
                                "art mesh '{semantic}' references texture '{}' which does not exist",
                                texture.as_str()
                            ),
                        ));
                    }
                }
                Some(page) => match texture_page_to_index.get(&page).copied() {
                    Some(index) => Some(index),
                    None => {
                        if best_effort {
                            unsupported.push(UnsupportedNote {
                                subject: semantic.to_string(),
                                reason: format!("texture page {page} has no supplied asset"),
                            });
                            None
                        } else {
                            return Err(error(
                                "MissingTextureAsset",
                                format!(
                                    "art mesh '{semantic}' references texture page {page} but no asset was supplied"
                                ),
                            ));
                        }
                    }
                },
            },
        };
        let mask_refs = mesh
            .mask_groups
            .iter()
            .filter_map(|group| {
                model
                    .mask_groups
                    .iter()
                    .find(|candidate| &candidate.id == group)
            })
            .flat_map(|group| group.sources.iter())
            .filter_map(|source| mesh_by_id.get(source.as_str()).copied())
            .collect();
        meshes.push(ArtMeshOut {
            semantic: semantic.to_string(),
            name,
            parent: mesh_parent(model, &parent_of, mesh),
            texture,
            model_image: None,
            positions: base_positions,
            uvs,
            indices: mesh.indices.clone(),
            forms: forms?,
            visible: mesh.visible,
            additive: mesh.flags.additive,
            multiplicative: mesh.flags.multiplicative,
            double_sided: mesh.flags.double_sided,
            inverted_mask: mesh.flags.inverted_mask,
            mask_refs,
            grid: None,
            synthetic_name,
        });
    }

    // ---- image pipeline (MODEL_IMAGE mode, one layered image) ------------
    let canvas_w = round_i32(model.canvas.width, "$canvas", "width")?;
    let canvas_h = round_i32(model.canvas.height, "$canvas", "height")?;
    let mut image_resources: Vec<ImageResourceOut> = Vec::new();
    let mut layered_images: Vec<LayeredImageOut> = Vec::new();
    let mut model_images: Vec<ModelImageOut> = Vec::new();
    let mut resource_by_page: BTreeMap<u32, usize> = BTreeMap::new();
    let mut layers: Vec<LayerOut> = Vec::new();
    for mesh in meshes.iter_mut() {
        let Some(asset_index) = mesh.texture else {
            continue;
        };
        let Some(asset) = textures.assets.get(asset_index) else {
            continue;
        };
        if let (Some(width), Some(height)) = (asset.width, asset.height) {
            if width != canvas_w as u32 || height != canvas_h as u32 {
                if !best_effort {
                    return Err(error(
                        "LayerImageSizeMismatch",
                        format!(
                            "texture page {} is {width}x{height} but canvas-sized layers are required ({canvas_w}x{canvas_h})",
                            asset.page
                        ),
                    ));
                }
                unsupported.push(UnsupportedNote {
                    subject: format!("texture:{}", asset.page),
                    reason: format!(
                        "asset is {width}x{height}, not canvas-sized; the synthetic layered image reuses it verbatim"
                    ),
                });
            }
        } else {
            defaults.push(WriterDefault {
                semantic: format!("texture:{}", asset.page),
                field: "image_resource.dimensions",
                reason: "asset dimensions unknown; canvas size is assumed for the resource",
            });
        }
        let resource_index = *resource_by_page.entry(asset.page).or_insert_with(|| {
            let index = image_resources.len();
            image_resources.push(ImageResourceOut {
                semantic: format!("image_resource:{index:06}"),
                width: canvas_w,
                height: canvas_h,
                byte_len: asset.bytes.len(),
                archive_name: TextureAssets::archive_name(asset.page),
            });
            index
        });
        let layer_index = layers.len();
        layers.push(LayerOut {
            semantic: format!("layer:{layer_index:06}"),
            name: mesh.name.clone(),
            layer_id: format!("00-00-{layer_index:02}-01"),
            layer_id_value: layer_index as i32 + 1,
            width: canvas_w,
            height: canvas_h,
            resource: resource_index,
            synthetic_name: mesh.synthetic_name,
        });
        let model_image_index = model_images.len();
        model_images.push(ModelImageOut {
            semantic: format!("model_image:{model_image_index:06}"),
            name: mesh.name.clone(),
            layer: layer_index,
            resource: resource_index,
        });
        mesh.model_image = Some(model_image_index);
    }
    if !layers.is_empty() {
        layered_images.push(LayeredImageOut {
            semantic: "layered_image:000000".to_string(),
            psd_name: "RecoveredLayeredImage.psd".to_string(),
            width: canvas_w,
            height: canvas_h,
            layers,
        });
        defaults.push(WriterDefault {
            semantic: "layered_image:000000".to_string(),
            field: "layered_image.psd_name",
            reason: "no original PSD exists; a synthetic PSD file name is used",
        });
        defaults.push(WriterDefault {
            semantic: "layered_image:000000".to_string(),
            field: "layered_image.group_name",
            reason: "no original image name exists; `RecoveredLayeredImage` is used",
        });
        defaults.push(WriterDefault {
            semantic: "layered_image:000000".to_string(),
            field: "texture.mipmap_level",
            reason: "independent sources disagree (1 vs 64); the pure MODEL_IMAGE reference uses 1",
        });
    }

    // ---- children (second pass, typed refs, no mixed index spaces) -------
    let part_parents: Vec<ParentRef> = parts.iter().map(|part| part.parent.clone()).collect();
    for index in 1..parts.len() {
        let child = ChildRef::Part(index);
        let parent = part_parents.get(index).cloned().unwrap_or(ParentRef::Root);
        attach_child(&mut parts, &mut warps, &mut rotations, &parent, child);
    }
    for index in 0..warps.len() {
        let reference = ChildRef::Warp(warps[index].semantic.clone());
        let parent = warps[index].parent.clone();
        attach_child(&mut parts, &mut warps, &mut rotations, &parent, reference);
    }
    for index in 0..rotations.len() {
        let reference = ChildRef::Rotation(rotations[index].semantic.clone());
        let parent = rotations[index].parent.clone();
        attach_child(&mut parts, &mut warps, &mut rotations, &parent, reference);
    }
    let mesh_parents: Vec<ParentRef> = meshes.iter().map(|mesh| mesh.parent.clone()).collect();
    for (index, parent) in mesh_parents.into_iter().enumerate() {
        attach_child(
            &mut parts,
            &mut warps,
            &mut rotations,
            &parent,
            ChildRef::Mesh(index),
        );
    }

    // ---- keyform grids and bindings -------------------------------------
    let mut bindings: Vec<BindingOut> = Vec::new();
    let mut grids: Vec<GridOut> = Vec::new();
    for entry in &keyforms.target_keyforms {
        let target_kind = match entry {
            TargetKeyforms::Part(_) => "part",
            TargetKeyforms::WarpDeformer(_) => "warp",
            TargetKeyforms::RotationDeformer(_) => "rotation",
            TargetKeyforms::ArtMesh(_) => "art_mesh",
        };
        let target_id = entry.target().id_text().to_string();
        let grid_id = entry.grid_id().to_string();
        let Some(grid) = keyforms.grid(&grid_id) else {
            if best_effort {
                unsupported.push(UnsupportedNote {
                    subject: grid_id.clone(),
                    reason: "keyform grid is missing from the recovered model".to_string(),
                });
                continue;
            }
            return Err(error(
                "UnresolvedKeyformGrid",
                format!("target '{target_id}' references missing grid '{grid_id}'"),
            ));
        };
        let Some(band) = keyforms.band(&grid.band) else {
            if best_effort {
                unsupported.push(UnsupportedNote {
                    subject: grid_id.clone(),
                    reason: "keyform band is missing from the recovered model".to_string(),
                });
                continue;
            }
            return Err(error(
                "UnresolvedKeyformGrid",
                format!("grid '{grid_id}' references missing band '{}'", grid.band),
            ));
        };
        let target = match entry {
            TargetKeyforms::Part(part) => part_by_id
                .get(part.target.as_str())
                .copied()
                .map(TargetOut::Part),
            TargetKeyforms::WarpDeformer(warp) => warp_by_id
                .get(warp.target.as_str())
                .copied()
                .map(TargetOut::Warp),
            TargetKeyforms::RotationDeformer(rotation) => rotation_by_id
                .get(rotation.target.as_str())
                .copied()
                .map(TargetOut::Rotation),
            TargetKeyforms::ArtMesh(mesh) => mesh_by_id
                .get(mesh.target.as_str())
                .copied()
                .map(TargetOut::ArtMesh),
        };
        let Some(target) = target else {
            unsupported.push(UnsupportedNote {
                subject: target_id,
                reason: format!("{target_kind} target is missing from the mapped model"),
            });
            continue;
        };
        if grid.layout != GridLayout::Dense {
            if !best_effort {
                return Err(error(
                    "UnresolvedKeyformGrid",
                    format!(
                        "grid '{grid_id}' has unresolved layout ({:?}); resolve it or use best-effort mode",
                        grid.layout
                    ),
                ));
            }
            unsupported.push(UnsupportedNote {
                subject: grid_id.clone(),
                reason: format!("grid layout is {:?}; forms are not serialized", grid.layout),
            });
            continue;
        }
        if band.axes.is_empty() {
            attach_grid(
                &mut parts,
                &mut warps,
                &mut rotations,
                &mut meshes,
                &target,
                &grid_id,
            );
            grids.push(GridOut {
                semantic: grid_id.clone(),
                target,
                binding_indices: Vec::new(),
                forms: Vec::new(),
                dense: true,
            });
            continue;
        }

        let bindings_start = bindings.len();
        let mut binding_indices: Vec<usize> = Vec::new();
        let mut axis_keys: Vec<Vec<f32>> = Vec::new();
        let mut unresolved_axis = false;
        for axis in &band.axes {
            let Some(parameter_index) = parameter_index.get(axis.parameter.as_str()).copied()
            else {
                unresolved_axis = true;
                continue;
            };
            let binding_index = bindings.len();
            let ordering_default = band.axes.len() > 1;
            bindings.push(BindingOut {
                semantic: format!("{}:{}", band.id, axis.parameter.as_str()),
                grid: grid_id.clone(),
                parameter_index,
                keys: axis.keys.clone(),
                ordering_default,
            });
            binding_indices.push(binding_index);
            axis_keys.push(axis.keys.clone());
        }
        if unresolved_axis {
            // Roll the partially built bindings back: an orphan binding that
            // no grid references must never reach the serializer.
            bindings.truncate(bindings_start);
            if !best_effort {
                return Err(error(
                    "UnresolvedParameter",
                    format!("grid '{grid_id}' has a binding axis without an IR parameter"),
                ));
            }
            unsupported.push(UnsupportedNote {
                subject: grid_id.clone(),
                reason: "a binding axis parameter is unresolved; forms are not serialized"
                    .to_string(),
            });
            continue;
        }
        if band.axes.len() > 1 {
            defaults.push(WriterDefault {
                semantic: grid_id.clone(),
                field: "grid.ordering",
                reason: "multi-axis grid ordering is unknown; writer default: first axis varies fastest",
            });
        }

        let strides = fastest_first_strides(&axis_keys);
        let forms: Vec<GridFormOut> = (0..grid.stored_form_count as usize)
            .map(|form_index| GridFormOut {
                form_index,
                key_indices: strides
                    .iter()
                    .map(|stride| (form_index / stride) % axis_keys.len().max(1))
                    .collect(),
            })
            .collect();
        attach_grid(
            &mut parts,
            &mut warps,
            &mut rotations,
            &mut meshes,
            &target,
            &grid_id,
        );
        grids.push(GridOut {
            semantic: grid_id,
            target,
            binding_indices,
            forms,
            dense: true,
        });
    }

    // ---- textures --------------------------------------------------------
    let textures_out: Vec<TextureOut> = textures
        .assets
        .iter()
        .map(|asset| TextureOut {
            page: asset.page,
            archive_name: TextureAssets::archive_name(asset.page),
            source_path: asset.source_path.clone(),
            width: asset.width,
            height: asset.height,
            byte_len: asset.bytes.len(),
            hash: TextureAssets::hash_of(asset),
        })
        .collect();

    let model_name = match options.model_name.clone() {
        Some(name) => name,
        None => {
            defaults.push(WriterDefault {
                semantic: "$model".to_string(),
                field: "model.name",
                reason: "no stored model name exists; synthetic name 'RecoveredModel' used",
            });
            "RecoveredModel".to_string()
        }
    };

    if best_effort {
        unsupported.push(UnsupportedNote {
            subject: "$output".to_string(),
            reason: "BEST_EFFORT: unresolved required semantics were omitted (never repaired)"
                .to_string(),
        });
    }

    Ok(Cmo3Project {
        profile_id: crate::profile::EDITOR_5_PROFILE.id.to_string(),
        model_name,
        canvas: CanvasOut {
            width: canvas_w,
            height: canvas_h,
            pixels_per_unit: finite(model.canvas.pixels_per_unit, "$canvas", "pixels_per_unit")?,
            origin_x: round_i32(model.canvas.origin.x, "$canvas", "origin_x")?,
            origin_y: round_i32(model.canvas.origin.y, "$canvas", "origin_y")?,
        },
        parameters,
        bindings,
        grids,
        parts,
        meshes,
        warps,
        rotations,
        textures: textures_out,
        image_resources,
        layered_images,
        model_images,
        defaults,
        unsupported,
        best_effort,
    })
}

fn round_i32(value: f32, semantic: &str, field: &str) -> Result<i32, WriteError> {
    let value = finite(value, semantic, field)?;
    let upper = 2_147_483_648.0_f32; // 2^31, exclusive
    if value < -upper || value >= upper {
        return Err(error(
            "NumericOverflow",
            format!("'{semantic}' field '{field}' does not fit an i32"),
        ));
    }
    Ok(value.round() as i32)
}

fn deformer_parent(
    model: &Live2DModel,
    parent_of: &impl Fn(&str) -> ParentRef,
    id: &str,
) -> ParentRef {
    let parent = parent_of(id);
    if !matches!(parent, ParentRef::Root) {
        return parent;
    }
    // Element-level fallback for hand-built documents without a project node.
    match model.deformer(&live2d_ir::DeformerId::new(id)) {
        Some(Deformer::Warp(warp)) => warp
            .common
            .parent_deformer
            .as_ref()
            .map(|parent| parent_kind(model, parent.as_str()))
            .unwrap_or_else(|| {
                warp.common
                    .parent_part
                    .as_ref()
                    .map(|part| {
                        model
                            .parts
                            .iter()
                            .position(|candidate| candidate.id == *part)
                            .map(|index| ParentRef::Part(index + 1))
                            .unwrap_or(ParentRef::Root)
                    })
                    .unwrap_or(ParentRef::Root)
            }),
        Some(Deformer::Rotation(rotation)) => rotation
            .common
            .parent_deformer
            .as_ref()
            .map(|parent| parent_kind(model, parent.as_str()))
            .unwrap_or_else(|| {
                rotation
                    .common
                    .parent_part
                    .as_ref()
                    .map(|part| {
                        model
                            .parts
                            .iter()
                            .position(|candidate| candidate.id == *part)
                            .map(|index| ParentRef::Part(index + 1))
                            .unwrap_or(ParentRef::Root)
                    })
                    .unwrap_or(ParentRef::Root)
            }),
        None => ParentRef::Root,
    }
}

fn parent_kind(model: &Live2DModel, id: &str) -> ParentRef {
    match model.deformer(&live2d_ir::DeformerId::new(id)) {
        Some(Deformer::Warp(_)) => ParentRef::Warp(id.to_string()),
        Some(Deformer::Rotation(_)) => ParentRef::Rotation(id.to_string()),
        None => ParentRef::Root,
    }
}

fn mesh_parent(
    model: &Live2DModel,
    parent_of: &impl Fn(&str) -> ParentRef,
    mesh: &live2d_ir::ArtMesh,
) -> ParentRef {
    let parent = parent_of(mesh.id.as_str());
    if !matches!(parent, ParentRef::Root) {
        return parent;
    }
    if let Some(deformer) = &mesh.parent_deformer {
        let parent = parent_kind(model, deformer.as_str());
        if !matches!(parent, ParentRef::Root) {
            return parent;
        }
    }
    if let Some(part) = &mesh.parent_part {
        if let Some(index) = model
            .parts
            .iter()
            .position(|candidate| candidate.id == *part)
        {
            return ParentRef::Part(index + 1);
        }
    }
    ParentRef::Root
}

fn attach_child(
    parts: &mut [PartOut],
    warps: &mut [WarpOut],
    rotations: &mut [RotationOut],
    parent: &ParentRef,
    child: ChildRef,
) {
    match parent {
        ParentRef::Root => {
            if let Some(root) = parts.first_mut() {
                root.children.push(child);
            }
        }
        ParentRef::Part(index) => {
            if let Some(part) = parts.get_mut(*index) {
                part.children.push(child);
            }
        }
        ParentRef::Warp(id) => {
            if let Some(index) = warps.iter().position(|warp| warp.semantic == *id) {
                if let Some(warp) = warps.get_mut(index) {
                    warp.children.push(child);
                }
            }
        }
        ParentRef::Rotation(id) => {
            if let Some(index) = rotations
                .iter()
                .position(|rotation| rotation.semantic == *id)
            {
                if let Some(rotation) = rotations.get_mut(index) {
                    rotation.children.push(child);
                }
            }
        }
    }
}

fn attach_grid(
    parts: &mut [PartOut],
    warps: &mut [WarpOut],
    rotations: &mut [RotationOut],
    meshes: &mut [ArtMeshOut],
    target: &TargetOut,
    grid_id: &str,
) {
    match target {
        TargetOut::Part(index) => {
            if let Some(part) = parts.get_mut(*index) {
                part.grid = Some(grid_id.to_string());
            }
        }
        TargetOut::Warp(index) => {
            if let Some(warp) = warps.get_mut(*index) {
                warp.grid = Some(grid_id.to_string());
            }
        }
        TargetOut::Rotation(index) => {
            if let Some(rotation) = rotations.get_mut(*index) {
                rotation.grid = Some(grid_id.to_string());
            }
        }
        TargetOut::ArtMesh(index) => {
            if let Some(mesh) = meshes.get_mut(*index) {
                mesh.grid = Some(grid_id.to_string());
            }
        }
    }
}

fn fastest_first_strides(axis_keys: &[Vec<f32>]) -> Vec<usize> {
    let mut strides: Vec<usize> = Vec::with_capacity(axis_keys.len());
    let mut stride = 1usize;
    for keys in axis_keys {
        strides.push(stride);
        stride = stride.saturating_mul(keys.len().max(1));
    }
    strides
}

fn to_positions(values: &[live2d_ir::Vec2]) -> Result<Vec<[f32; 2]>, WriteError> {
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        if !value.x.is_finite() || !value.y.is_finite() {
            return Err(error(
                "NonFiniteGeometry",
                "geometry contains non-finite coordinates",
            ));
        }
        out.push([value.x, value.y]);
    }
    Ok(out)
}

fn to_uvs(mesh: &live2d_ir::ArtMesh) -> Result<Vec<[f32; 2]>, WriteError> {
    let mut out = Vec::with_capacity(mesh.uvs.len());
    for uv in &mesh.uvs {
        if !uv.u.is_finite() || !uv.v.is_finite() {
            return Err(error("NonFiniteGeometry", "UVs contain non-finite values"));
        }
        out.push([uv.u, uv.v]);
    }
    if out.len() != mesh.vertex_count {
        return Err(error(
            "InvalidGeometry",
            format!(
                "art mesh '{}' has {} UVs for {} vertices",
                mesh.id.as_str(),
                out.len(),
                mesh.vertex_count
            ),
        ));
    }
    Ok(out)
}

fn collect_base_positions(mesh: &live2d_ir::ArtMesh) -> Vec<live2d_ir::Vec2> {
    mesh.keyforms
        .first()
        .map(|form| form.positions.clone())
        .unwrap_or_else(|| vec![live2d_ir::Vec2 { x: 0.0, y: 0.0 }; mesh.vertex_count])
}

fn texture_page_of(model: &Live2DModel, texture_id: &str) -> Option<u32> {
    model
        .textures
        .iter()
        .find(|texture| texture.id.as_str() == texture_id)
        .map(|texture| texture.page_index)
}
