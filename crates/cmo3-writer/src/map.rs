//! Semantic -> writer-model mapping (serialization only, no inference).
//!
//! Every required semantic that is unresolved fails in strict mode (default)
//! and is reported as unsupported in best-effort mode (work order sections
//! 37-38, 81-82). Nothing is fabricated: writer-required defaults are
//! recorded in [`Cmo3Project::defaults`].

use std::collections::BTreeMap;

use hierarchy_recovery::RecoveredProject;
use keyform_recovery::{GridLayout, RecoveredKeyformModel, TargetKeyforms};
use live2d_ir::{Deformer, Live2DModel};

use crate::model::{
    ArtMeshFormOut, ArtMeshOut, BindingOut, CanvasOut, Cmo3Project, GridFormOut, GridOut,
    ParameterOut, PartOut, RotationFormOut, RotationOut, TargetOut, TextureOut, UnsupportedNote,
    WarpFormOut, WarpOut, WriterDefault,
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
        parameters.push(ParameterOut {
            semantic: parameter.id.as_str().to_string(),
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
    let deformer_by_id: BTreeMap<&str, (bool, usize)> = model
        .deformers
        .iter()
        .enumerate()
        .map(|(index, deformer)| match deformer {
            Deformer::Warp(warp) => (warp.common.id.as_str(), (true, index)),
            Deformer::Rotation(rotation) => (rotation.common.id.as_str(), (false, index)),
        })
        .collect();
    let mesh_by_id: BTreeMap<&str, usize> = model
        .art_meshes
        .iter()
        .enumerate()
        .map(|(index, mesh)| (mesh.id.as_str(), index))
        .collect();

    // ---- parts -----------------------------------------------------------
    let mut parts: Vec<PartOut> = Vec::with_capacity(model.parts.len() + 1);
    parts.push(PartOut {
        semantic: "$root".to_string(),
        name: "Root".to_string(),
        parent: None,
        children_parts: Vec::new(),
        children_deformers: Vec::new(),
        children_meshes: Vec::new(),
        target_deformer: None,
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
        // Hierarchy parent: resolved parent part, else the synthetic root.
        let parent = node_parent
            .get(part.id.as_str())
            .and_then(|parent| part_by_id.get(parent).copied());
        parts.push(PartOut {
            semantic: part.id.as_str().to_string(),
            name,
            parent,
            children_parts: Vec::new(),
            children_deformers: Vec::new(),
            children_meshes: Vec::new(),
            target_deformer: None,
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
                let parent_part = node_parent
                    .get(warp.common.id.as_str())
                    .and_then(|parent| part_by_id.get(parent).copied())
                    .or_else(|| {
                        warp.common
                            .parent_part
                            .as_ref()
                            .and_then(|part| part_by_id.get(part.as_str()).copied())
                    });
                let forms: Result<Vec<WarpFormOut>, WriteError> = warp
                    .keyforms
                    .iter()
                    .map(|form| {
                        let positions = to_positions(&form.positions)?;
                        Ok(WarpFormOut {
                            opacity: form.opacity,
                            positions,
                        })
                    })
                    .collect();
                warps.push(WarpOut {
                    semantic: warp.common.id.as_str().to_string(),
                    name,
                    parent_part,
                    parent_deformer: None,
                    children_deformers: Vec::new(),
                    children_meshes: Vec::new(),
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
                let parent_part = node_parent
                    .get(rotation.common.id.as_str())
                    .and_then(|parent| part_by_id.get(parent).copied())
                    .or_else(|| {
                        rotation
                            .common
                            .parent_part
                            .as_ref()
                            .and_then(|part| part_by_id.get(part.as_str()).copied())
                    });
                rotations.push(RotationOut {
                    semantic: rotation.common.id.as_str().to_string(),
                    name,
                    parent_part,
                    parent_deformer: None,
                    children_deformers: Vec::new(),
                    children_meshes: Vec::new(),
                    base_angle: rotation.base_angle,
                    forms: rotation
                        .keyforms
                        .iter()
                        .map(|form| RotationFormOut {
                            opacity: form.opacity,
                            angle: form.angle,
                            origin: [form.origin.x, form.origin.y],
                            scale: form.scale,
                            reflect_x: form.reflect_x,
                            reflect_y: form.reflect_y,
                        })
                        .collect(),
                    grid: None,
                    synthetic_name,
                });
            }
        }
    }
    // Deformer parents may be deformers; resolve now.
    for (index, deformer) in model.deformers.iter().enumerate() {
        let id = match deformer {
            Deformer::Warp(warp) => warp.common.id.as_str(),
            Deformer::Rotation(rotation) => rotation.common.id.as_str(),
        };
        let parent_id = node_parent.get(id).copied().unwrap_or("$root");
        if let Some((is_warp, parent_index)) = deformer_by_id.get(parent_id).copied() {
            if parent_index == index {
                continue;
            }
            if is_warp {
                if let Some(entry) = warps.get_mut(index) {
                    entry.parent_deformer = Some(parent_index);
                    entry.parent_part = None;
                }
            } else if let Some(entry) = rotations.get_mut(index) {
                entry.parent_deformer = Some(parent_index);
                entry.parent_part = None;
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
        let (name, synthetic_name) = match &mesh.source_name {
            Some(name) if !name.is_empty() => (name.clone(), false),
            _ => (format!("ArtMesh_{:06}", index + 1), true),
        };
        if synthetic_name {
            defaults.push(WriterDefault {
                semantic: mesh.id.as_str().to_string(),
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
                    "art mesh '{}' has {} base positions for {} vertices",
                    mesh.id.as_str(),
                    base_positions.len(),
                    mesh.vertex_count
                ),
            ));
        }
        if !mesh.keyforms.is_empty() {
            defaults.push(WriterDefault {
                semantic: mesh.id.as_str().to_string(),
                field: "art_mesh.base_positions",
                reason: "the IR stores no explicit base geometry; the first stored form is used",
            });
        }
        if mesh
            .indices
            .iter()
            .any(|value| *value as usize >= mesh.vertex_count)
        {
            return Err(error(
                "InvalidGeometry",
                format!("art mesh '{}' has out-of-range indices", mesh.id.as_str()),
            ));
        }
        let forms: Result<Vec<ArtMeshFormOut>, WriteError> = mesh
            .keyforms
            .iter()
            .map(|form| {
                let positions = to_positions(&form.positions)?;
                Ok(ArtMeshFormOut {
                    draw_order: form.draw_order,
                    opacity: form.opacity,
                    positions,
                })
            })
            .collect();
        let texture = match mesh
            .texture
            .as_ref()
            .map(|texture| texture_page_of(model, texture.as_str()))
        {
            Some(Some(page)) => match texture_page_to_index.get(&page).copied() {
                Some(index) => Some(index),
                None => {
                    if best_effort {
                        unsupported.push(UnsupportedNote {
                            subject: mesh.id.as_str().to_string(),
                            reason: format!("texture page {page} has no supplied asset"),
                        });
                        None
                    } else {
                        return Err(error(
                            "MissingTextureAsset",
                            format!(
                                "art mesh '{}' references texture page {page} but no asset was supplied",
                                mesh.id.as_str()
                            ),
                        ));
                    }
                }
            },
            _ => None,
        };
        let parent_part = node_parent
            .get(mesh.id.as_str())
            .and_then(|parent| part_by_id.get(parent).copied())
            .or_else(|| {
                mesh.parent_part
                    .as_ref()
                    .and_then(|part| part_by_id.get(part.as_str()).copied())
            });
        let target_deformer = node_parent
            .get(mesh.id.as_str())
            .and_then(|parent| deformer_by_id.get(parent).map(|(_, index)| *index))
            .or_else(|| {
                mesh.parent_deformer.as_ref().and_then(|deformer| {
                    deformer_by_id
                        .get(deformer.as_str())
                        .map(|(_, index)| *index)
                })
            });
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
            semantic: mesh.id.as_str().to_string(),
            name,
            parent_part,
            target_deformer,
            texture,
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
            continue;
        };
        let Some(band) = keyforms.band(&grid.band) else {
            continue;
        };
        let target = match entry {
            TargetKeyforms::Part(part) => part_by_id
                .get(part.target.as_str())
                .copied()
                .map(TargetOut::Part),
            TargetKeyforms::WarpDeformer(warp) => warps
                .iter()
                .position(|item| item.semantic == warp.target.as_str())
                .map(TargetOut::Warp),
            TargetKeyforms::RotationDeformer(rotation) => rotations
                .iter()
                .position(|item| item.semantic == rotation.target.as_str())
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
            // Static target: no bindings, no forms mapping needed.
            let grid_out = GridOut {
                semantic: grid_id.clone(),
                target: target.clone(),
                binding_indices: Vec::new(),
                forms: Vec::new(),
                dense: true,
            };
            attach_grid(
                &mut parts,
                &mut warps,
                &mut rotations,
                &mut meshes,
                &target,
                &grid_id,
            );
            grids.push(grid_out);
            continue;
        }

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
        if bindings
            .iter()
            .any(|binding| binding.grid == grid_id && binding.ordering_default)
        {
            defaults.push(WriterDefault {
                semantic: grid_id.clone(),
                field: "grid.ordering",
                reason: "multi-axis grid ordering is unknown; writer default: first axis varies fastest",
            });
        }

        // Form coordinates: stored order mapped to the writer default
        // ordering (fastest-first) for dense grids only.
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

    if best_effort {
        unsupported.push(UnsupportedNote {
            subject: "$output".to_string(),
            reason: "BEST_EFFORT: unresolved required semantics were omitted (never repaired)"
                .to_string(),
        });
    }

    Ok(Cmo3Project {
        profile_id: crate::profile::EDITOR_5_PROFILE.id.to_string(),
        model_name: options
            .model_name
            .clone()
            .unwrap_or_else(|| "RecoveredModel".to_string()),
        canvas: CanvasOut {
            width: model.canvas.width.round() as i32,
            height: model.canvas.height.round() as i32,
            pixels_per_unit: model.canvas.pixels_per_unit,
            origin_x: model.canvas.origin.x.round() as i32,
            origin_y: model.canvas.origin.y.round() as i32,
        },
        parameters,
        bindings,
        grids,
        parts,
        meshes,
        warps,
        rotations,
        textures: textures_out,
        defaults,
        unsupported,
        best_effort,
    })
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
    // The IR stores base geometry implicitly through the first form; when no
    // form exists the origin is used. This is a documented writer default
    // (trace entry added by the caller path below).
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
