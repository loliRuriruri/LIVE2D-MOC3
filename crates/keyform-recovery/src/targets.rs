//! Stage 5: map stored forms of one target into typed payloads.
//!
//! Only fields the file actually stores are mapped (`KF-008`); geometry
//! payloads are referenced by index and count, never copied (`KF-009`).

use live2d_ir::{Deformer, Live2DModel};

use crate::collect::TargetEvidence;
use crate::model::{
    ArtMeshKeyformForm, ArtMeshTargetKeyforms, FormGeometryRef, KeyformTarget, PartKeyformForm,
    PartTargetKeyforms, RotationKeyformForm, RotationTargetKeyforms, TargetKeyforms,
    WarpKeyformForm, WarpTargetKeyforms,
};

/// Map the forms of one target. Returns `None` when the referenced entity
/// does not exist in the IR (dangling target reference).
pub fn map_forms(
    model: &Live2DModel,
    evidence: &TargetEvidence,
    band_id: &str,
    grid_id: &str,
) -> Option<(TargetKeyforms, Vec<String>)> {
    match &evidence.target {
        KeyformTarget::Part(id) => {
            let part = model.parts.iter().find(|part| &part.id == id)?;
            let keyforms: Vec<PartKeyformForm> = part
                .keyform_draw_orders
                .iter()
                .enumerate()
                .map(|(index, draw_order)| PartKeyformForm {
                    index,
                    draw_order: *draw_order,
                })
                .collect();
            let notes = vec![
                "part forms store draw order only; base opacity is not stored in .moc3 (KF-008)"
                    .to_string(),
            ];
            Some((
                TargetKeyforms::Part(PartTargetKeyforms {
                    target: part.id.clone(),
                    band: band_id.to_string(),
                    grid: grid_id.to_string(),
                    binding: part.binding.clone(),
                    keyforms,
                }),
                notes,
            ))
        }
        KeyformTarget::WarpDeformer(id) => {
            let Some(Deformer::Warp(warp)) = model.deformer(id) else {
                return None;
            };
            let keyforms: Vec<WarpKeyformForm> = warp
                .keyforms
                .iter()
                .map(|form| WarpKeyformForm {
                    index: form.index,
                    opacity: form.opacity,
                    geometry: FormGeometryRef {
                        position_count: form.positions.len(),
                    },
                })
                .collect();
            let notes = vec![format!(
                "warp control-point geometry ({} positions per form) stays in the Live2D IR (KF-009); the warp grid is {}x{} and is unrelated to the parameter grid",
                warp.vertex_count, warp.rows, warp.columns
            )];
            Some((
                TargetKeyforms::WarpDeformer(WarpTargetKeyforms {
                    target: warp.common.id.clone(),
                    band: band_id.to_string(),
                    grid: grid_id.to_string(),
                    binding: warp.common.binding.clone(),
                    keyforms,
                }),
                notes,
            ))
        }
        KeyformTarget::RotationDeformer(id) => {
            let Some(Deformer::Rotation(rotation)) = model.deformer(id) else {
                return None;
            };
            let keyforms: Vec<RotationKeyformForm> = rotation
                .keyforms
                .iter()
                .map(|form| RotationKeyformForm {
                    index: form.index,
                    opacity: form.opacity,
                    angle: form.angle,
                    origin: [form.origin.x, form.origin.y],
                    scale: form.scale,
                    reflect_x: form.reflect_x,
                    reflect_y: form.reflect_y,
                })
                .collect();
            let notes = vec![
                "rotation forms store opacity, angle, origin, scale and reflect flags; base angle is separate (KF-008)"
                    .to_string(),
            ];
            Some((
                TargetKeyforms::RotationDeformer(RotationTargetKeyforms {
                    target: rotation.common.id.clone(),
                    band: band_id.to_string(),
                    grid: grid_id.to_string(),
                    binding: rotation.common.binding.clone(),
                    keyforms,
                }),
                notes,
            ))
        }
        KeyformTarget::ArtMesh(id) => {
            let mesh = model.art_meshes.iter().find(|mesh| &mesh.id == id)?;
            let keyforms: Vec<ArtMeshKeyformForm> = mesh
                .keyforms
                .iter()
                .map(|form| ArtMeshKeyformForm {
                    index: form.index,
                    opacity: form.opacity,
                    draw_order: form.draw_order,
                    geometry: FormGeometryRef {
                        position_count: form.positions.len(),
                    },
                })
                .collect();
            let notes = vec![format!(
                "art mesh keyed geometry ({} vertices per form) stays in the Live2D IR; static uvs/indices remain in the IR too (KF-009)",
                mesh.vertex_count
            )];
            Some((
                TargetKeyforms::ArtMesh(ArtMeshTargetKeyforms {
                    target: mesh.id.clone(),
                    band: band_id.to_string(),
                    grid: grid_id.to_string(),
                    binding: mesh.binding.clone(),
                    keyforms,
                }),
                notes,
            ))
        }
    }
}

/// Scan mapped forms for non-finite payload floats.
///
/// Canonical JSON cannot carry NaN/Inf; the values are preserved in the
/// library model but reported positionally and refused at export time.
pub fn scan_payload_finiteness(
    entry: &TargetKeyforms,
) -> (
    Vec<live2d_ir::Diagnostic>,
    Vec<crate::model::UnresolvedEntry>,
) {
    use live2d_ir::{Diagnostic, Severity};

    let mut fields: Vec<String> = Vec::new();
    let target = entry.target();
    match entry {
        TargetKeyforms::Part(part) => {
            for form in &part.keyforms {
                if !form.draw_order.is_finite() {
                    fields.push(format!("draw_order[{}]", form.index));
                }
            }
        }
        TargetKeyforms::WarpDeformer(warp) => {
            for form in &warp.keyforms {
                if !form.opacity.is_finite() {
                    fields.push(format!("opacity[{}]", form.index));
                }
            }
        }
        TargetKeyforms::RotationDeformer(rotation) => {
            for form in &rotation.keyforms {
                if !form.opacity.is_finite() {
                    fields.push(format!("opacity[{}]", form.index));
                }
                if !form.angle.is_finite() {
                    fields.push(format!("angle[{}]", form.index));
                }
                if !form.origin[0].is_finite() || !form.origin[1].is_finite() {
                    fields.push(format!("origin[{}]", form.index));
                }
                if !form.scale.is_finite() {
                    fields.push(format!("scale[{}]", form.index));
                }
            }
        }
        TargetKeyforms::ArtMesh(mesh) => {
            for form in &mesh.keyforms {
                if !form.opacity.is_finite() {
                    fields.push(format!("opacity[{}]", form.index));
                }
                if !form.draw_order.is_finite() {
                    fields.push(format!("draw_order[{}]", form.index));
                }
            }
        }
    }
    if fields.is_empty() {
        return (Vec::new(), Vec::new());
    }
    const MAX_REPORTED: usize = 16;
    let mut rendered: Vec<String> = fields.iter().take(MAX_REPORTED).cloned().collect();
    if fields.len() > MAX_REPORTED {
        rendered.push(format!("... ({} more)", fields.len() - MAX_REPORTED));
    }
    let detail = rendered.join(", ");
    (
        vec![Diagnostic::new(
            Severity::Warning,
            crate::codes::NON_FINITE_PAYLOAD_VALUE,
            format!(
                "target '{}' stores non-finite payload value(s): {detail}",
                target.id_text()
            ),
        )],
        vec![crate::model::UnresolvedEntry {
            code: crate::codes::NON_FINITE_PAYLOAD_VALUE.to_string(),
            target: Some(target),
            band: None,
            grid: None,
            detail,
        }],
    )
}

/// True when the target exists in the model with the recorded kind.
pub fn target_exists(model: &Live2DModel, target: &KeyformTarget) -> bool {
    match target {
        KeyformTarget::Part(id) => model.parts.iter().any(|item| &item.id == id),
        KeyformTarget::WarpDeformer(id) => {
            matches!(model.deformer(id), Some(Deformer::Warp(_)))
        }
        KeyformTarget::RotationDeformer(id) => {
            matches!(model.deformer(id), Some(Deformer::Rotation(_)))
        }
        KeyformTarget::ArtMesh(id) => model.art_meshes.iter().any(|item| &item.id == id),
    }
}
