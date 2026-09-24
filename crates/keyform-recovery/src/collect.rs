//! Stage 1: collect stored binding evidence from the Live2D IR.
//!
//! Nothing is inferred here: every axis and target comes from a stored
//! reference (work order sections 4, 23, 31). The output is a plain evidence
//! collection; normalization happens in `axes.rs`.

use live2d_ir::{BindingId, BindingTarget, Deformer, Live2DModel, ParameterId, ParameterKind};

use crate::model::KeyformTarget;

/// One target object and the forms it stores.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetEvidence {
    /// Target object.
    pub target: KeyformTarget,
    /// Stored binding reference when present.
    pub binding: Option<BindingId>,
    /// Stored form indices in stored order.
    pub form_indices: Vec<usize>,
}

/// One binding parameter before normalization.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisDraft {
    /// Parameter identifier.
    pub parameter: ParameterId,
    /// Raw stored key values (may contain non-finite values).
    pub keys: Vec<f32>,
    /// Whether the parameter resolves to an IR entity.
    pub resolved: bool,
    /// Whether the parameter is a blend-shape parameter (experimental).
    pub experimental: bool,
}

/// One binding before normalization.
#[derive(Debug, Clone, PartialEq)]
pub struct BindingDraft {
    /// Binding id.
    pub id: BindingId,
    /// Raw axes in binding order.
    pub axes: Vec<AxisDraft>,
    /// Supported targets referencing this binding (parts, deformers, art
    /// meshes only; glue is preserved separately).
    pub supported_targets: Vec<KeyformTarget>,
    /// Whether a glue entry references this binding (deferred, KB-006).
    pub glue_used: bool,
}

/// All collected binding evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct BindingEvidence {
    /// Targets in deterministic order: parts (source order), deformers
    /// (source order), art meshes (source order).
    pub targets: Vec<TargetEvidence>,
    /// Bindings in IR source order.
    pub bindings: Vec<BindingDraft>,
}

/// Collect binding evidence from a model.
///
/// This function performs lookups only; it never repairs or interprets the
/// stored data (rule KB-001/KB-002).
pub fn collect_binding_evidence(model: &Live2DModel) -> BindingEvidence {
    let targets = collect_targets(model);
    let bindings = collect_bindings(model);
    BindingEvidence { targets, bindings }
}

fn collect_targets(model: &Live2DModel) -> Vec<TargetEvidence> {
    let mut targets: Vec<TargetEvidence> = Vec::new();

    for part in &model.parts {
        targets.push(TargetEvidence {
            target: KeyformTarget::Part(part.id.clone()),
            binding: part.binding.clone(),
            form_indices: (0..part.keyform_draw_orders.len()).collect(),
        });
    }

    for deformer in &model.deformers {
        match deformer {
            Deformer::Warp(warp) => targets.push(TargetEvidence {
                target: KeyformTarget::WarpDeformer(warp.common.id.clone()),
                binding: warp.common.binding.clone(),
                form_indices: warp.keyforms.iter().map(|form| form.index).collect(),
            }),
            Deformer::Rotation(rotation) => targets.push(TargetEvidence {
                target: KeyformTarget::RotationDeformer(rotation.common.id.clone()),
                binding: rotation.common.binding.clone(),
                form_indices: rotation.keyforms.iter().map(|form| form.index).collect(),
            }),
        }
    }

    for mesh in &model.art_meshes {
        targets.push(TargetEvidence {
            target: KeyformTarget::ArtMesh(mesh.id.clone()),
            binding: mesh.binding.clone(),
            form_indices: mesh.keyforms.iter().map(|form| form.index).collect(),
        });
    }

    targets
}

fn collect_bindings(model: &Live2DModel) -> Vec<BindingDraft> {
    model
        .bindings
        .iter()
        .map(|binding| {
            let axes = binding
                .parameters
                .iter()
                .map(|parameter| {
                    let resolved = model
                        .parameters
                        .iter()
                        .find(|entity| entity.id == parameter.parameter);
                    AxisDraft {
                        parameter: parameter.parameter.clone(),
                        keys: parameter.key_values.clone(),
                        resolved: resolved.is_some(),
                        experimental: resolved
                            .map(|entity| {
                                entity.kind == ParameterKind::BlendShape
                                    || !entity.extension_key_values.is_empty()
                            })
                            .unwrap_or(false),
                    }
                })
                .collect();

            let mut supported_targets = Vec::new();
            let mut glue_used = false;
            for used_by in &binding.used_by {
                match used_by {
                    BindingTarget::Part(id) => {
                        supported_targets.push(KeyformTarget::Part(id.clone()));
                    }
                    BindingTarget::Deformer(id) => {
                        match model.deformer(id) {
                            Some(Deformer::Warp(warp)) => supported_targets
                                .push(KeyformTarget::WarpDeformer(warp.common.id.clone())),
                            Some(Deformer::Rotation(rotation)) => supported_targets
                                .push(KeyformTarget::RotationDeformer(rotation.common.id.clone())),
                            // A dangling deformer reference keeps its id; the
                            // kind is unknown and is reported downstream.
                            None => supported_targets.push(KeyformTarget::WarpDeformer(id.clone())),
                        }
                    }
                    BindingTarget::ArtMesh(id) => {
                        supported_targets.push(KeyformTarget::ArtMesh(id.clone()));
                    }
                    BindingTarget::Glue(_) => glue_used = true,
                }
            }

            BindingDraft {
                id: binding.id.clone(),
                axes,
                supported_targets,
                glue_used,
            }
        })
        .collect()
}
