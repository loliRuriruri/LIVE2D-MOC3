//! Typed-project and XML validators (work order sections 32-36).
//!
//! The typed validator checks every reference inside `Cmo3Project` before a
//! single byte is serialized. The XML scanner re-checks the produced text for
//! duplicate `xs.id` values, dangling `xs.ref` targets and GUID-type
//! mismatches. A failing validator means no output is produced.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Cmo3Project, ParentRef};

/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    /// Output must not be produced.
    Fatal,
    /// Suspicious but serializable.
    Warning,
}

/// One validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Stable code.
    pub code: &'static str,
    /// Severity.
    pub severity: FindingSeverity,
    /// Detail.
    pub detail: String,
}

impl Finding {
    fn fatal(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            severity: FindingSeverity::Fatal,
            detail: detail.into(),
        }
    }
}

/// Validate the typed project model.
pub fn validate_typed(project: &Cmo3Project) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    let part_count = project.parts.len();
    let warp_count = project.warps.len();
    let rotation_count = project.rotations.len();
    let mesh_count = project.meshes.len();
    let layer_count = project
        .layered_images
        .first()
        .map(|layered| layered.layers.len())
        .unwrap_or(0);
    let resource_count = project.image_resources.len();
    let model_image_count = project.model_images.len();

    if part_count == 0 || !project.parts[0].is_root {
        findings.push(Finding::fatal(
            "missing_root_part",
            "the typed project must start with the synthetic root part",
        ));
    }
    let textured_meshes = project
        .meshes
        .iter()
        .filter(|mesh| mesh.model_image.is_some())
        .count();
    if textured_meshes > 0 && project.layered_images.len() != 1 {
        findings.push(Finding::fatal(
            "missing_layered_image",
            format!(
                "{textured_meshes} textured mesh(es) require exactly one layered image, found {}",
                project.layered_images.len()
            ),
        ));
    }
    if textured_meshes > 0 && layer_count != textured_meshes {
        findings.push(Finding::fatal(
            "layer_count_mismatch",
            format!(
                "{textured_meshes} textured mesh(es) require one layer each, found {layer_count}"
            ),
        ));
    }
    if model_image_count != textured_meshes {
        findings.push(Finding::fatal(
            "model_image_count_mismatch",
            format!(
                "{textured_meshes} textured mesh(es) require one model image each, found {model_image_count}"
            ),
        ));
    }
    for (index, mesh) in project.meshes.iter().enumerate() {
        if mesh.model_image.is_some() && mesh.texture.is_none() {
            findings.push(Finding::fatal(
                "model_image_without_texture",
                format!("mesh {index} has a model image but no texture resource"),
            ));
        }
    }

    for (index, part) in project.parts.iter().enumerate() {
        match &part.parent {
            ParentRef::Root => {}
            ParentRef::Part(parent) => {
                if *parent >= part_count {
                    findings.push(Finding::fatal(
                        "dangling_part_parent",
                        format!("part {index} references missing part {parent}"),
                    ));
                }
            }
            ParentRef::Warp(_) | ParentRef::Rotation(_) => findings.push(Finding::fatal(
                "invalid_part_parent",
                format!("part {index} has a deformer parent; parts must parent to parts"),
            )),
        }
    }
    for (index, mesh) in project.meshes.iter().enumerate() {
        match &mesh.parent {
            ParentRef::Root => {}
            ParentRef::Part(parent) => {
                if *parent >= part_count {
                    findings.push(Finding::fatal(
                        "dangling_mesh_parent",
                        format!("mesh {index} references missing part {parent}"),
                    ));
                }
            }
            ParentRef::Warp(_) | ParentRef::Rotation(_) => {
                // Supported since AGENT.5.2: the deformer source is emitted
                // and `targetDeformerGuid` resolves through `deformer_pools`.
            }
        }
        if mesh.texture.is_some() && mesh.model_image.is_none() {
            findings.push(Finding::fatal(
                "missing_model_image",
                format!("mesh {index} has a texture but no model image"),
            ));
        }
        if let Some(resource) = mesh.texture {
            if resource >= resource_count {
                findings.push(Finding::fatal(
                    "dangling_texture_resource",
                    format!("mesh {index} references missing image resource {resource}"),
                ));
            }
        }
        if let Some(model_image) = mesh.model_image {
            if model_image >= model_image_count {
                findings.push(Finding::fatal(
                    "dangling_model_image",
                    format!("mesh {index} references missing model image {model_image}"),
                ));
            }
        }
        if mesh.positions.len() != mesh.uvs.len() || mesh.positions.is_empty() {
            findings.push(Finding::fatal(
                "invalid_geometry_counts",
                format!(
                    "mesh {index} has {} positions and {} uvs",
                    mesh.positions.len(),
                    mesh.uvs.len()
                ),
            ));
        }
        if mesh
            .indices
            .iter()
            .any(|value| *value as usize >= mesh.uvs.len())
        {
            findings.push(Finding::fatal(
                "invalid_geometry_indices",
                format!("mesh {index} has out-of-range indices"),
            ));
        }
        if let Some(mask) = mesh.mask_refs.first() {
            if *mask >= mesh_count {
                findings.push(Finding::fatal(
                    "dangling_mask_reference",
                    format!("mesh {index} references missing mask mesh {mask}"),
                ));
            }
        }
    }
    for (index, model_image) in project.model_images.iter().enumerate() {
        if model_image.layer >= layer_count {
            findings.push(Finding::fatal(
                "dangling_layer_selection",
                format!(
                    "model image {index} selects missing layer {}",
                    model_image.layer
                ),
            ));
        }
        if model_image.resource >= resource_count {
            findings.push(Finding::fatal(
                "dangling_model_image_resource",
                format!(
                    "model image {index} references missing resource {}",
                    model_image.resource
                ),
            ));
        }
    }
    // Keyform form-count consistency: a target with a grid must carry exactly
    // the grid's stored form count; forms without a grid would be dropped.
    let grid_form_count = |semantic: &str| -> Option<usize> {
        project
            .grids
            .iter()
            .find(|grid| grid.semantic == semantic)
            .map(|grid| grid.forms.len())
    };
    let mut check_forms = |label: &str, semantic: &str, grid: &Option<String>, forms: usize| match (
        grid,
        grid_form_count(grid.as_deref().unwrap_or_default()),
    ) {
        (Some(_), Some(expected)) if expected == forms => {}
        (Some(grid_id), Some(expected)) => findings.push(Finding::fatal(
            "form_count_mismatch",
            format!(
                "{label} '{semantic}' carries {forms} form(s) but grid '{grid_id}' has {expected}"
            ),
        )),
        (Some(grid_id), None) => findings.push(Finding::fatal(
            "dangling_target_grid",
            format!("{label} '{semantic}' references missing grid '{grid_id}'"),
        )),
        (None, _) if forms > 0 => findings.push(Finding::fatal(
            "forms_without_grid",
            format!("{label} '{semantic}' carries {forms} form(s) without a keyform grid"),
        )),
        _ => {}
    };
    for part in &project.parts {
        check_forms("part", &part.semantic, &part.grid, part.draw_orders.len());
    }
    for warp in &project.warps {
        check_forms("warp", &warp.semantic, &warp.grid, warp.forms.len());
    }
    for rotation in &project.rotations {
        check_forms(
            "rotation",
            &rotation.semantic,
            &rotation.grid,
            rotation.forms.len(),
        );
    }
    for mesh in &project.meshes {
        check_forms("art mesh", &mesh.semantic, &mesh.grid, mesh.forms.len());
    }
    // Binding parameter indices must resolve inside the emitted parameters
    // (the serializer emits a synthetic parameter when none exist).
    let parameter_slots = project.parameters.len().max(1);
    for (index, binding) in project.bindings.iter().enumerate() {
        if binding.parameter_index >= parameter_slots {
            findings.push(Finding::fatal(
                "dangling_binding_parameter",
                format!(
                    "binding {index} ('{}') references parameter slot {} outside 0..{parameter_slots}",
                    binding.semantic, binding.parameter_index
                ),
            ));
        }
    }
    // Deformer parent cycles: stored cycles are preserved by AGENT.3, but a
    // CMO3 hierarchy cannot represent them; fail strict mode.
    let deformer_parent = |semantic: &str| -> Option<String> {
        project
            .warps
            .iter()
            .find(|warp| warp.semantic == semantic)
            .and_then(|warp| match &warp.parent {
                ParentRef::Warp(parent) | ParentRef::Rotation(parent) => Some(parent.clone()),
                _ => None,
            })
            .or_else(|| {
                project
                    .rotations
                    .iter()
                    .find(|rotation| rotation.semantic == semantic)
                    .and_then(|rotation| match &rotation.parent {
                        ParentRef::Warp(parent) | ParentRef::Rotation(parent) => {
                            Some(parent.clone())
                        }
                        _ => None,
                    })
            })
    };
    let all_deformers: Vec<&str> = project
        .warps
        .iter()
        .map(|warp| warp.semantic.as_str())
        .chain(
            project
                .rotations
                .iter()
                .map(|rotation| rotation.semantic.as_str()),
        )
        .collect();
    for semantic in &all_deformers {
        let mut current = semantic.to_string();
        let mut depth = 0usize;
        while let Some(parent) = deformer_parent(&current) {
            if parent == *semantic {
                findings.push(Finding::fatal(
                    "deformer_parent_cycle",
                    format!("deformer '{semantic}' is inside a stored parent cycle"),
                ));
                break;
            }
            depth += 1;
            if depth > all_deformers.len() {
                break;
            }
            current = parent;
        }
    }

    let mut layer_ids: BTreeSet<&str> = BTreeSet::new();
    if let Some(layered) = project.layered_images.first() {
        for (index, layer) in layered.layers.iter().enumerate() {
            if !layer_ids.insert(layer.layer_id.as_str()) {
                findings.push(Finding::fatal(
                    "duplicate_layer_identifier",
                    format!("duplicate layerId '{}'", layer.layer_id),
                ));
            }
            if layer.resource >= resource_count {
                findings.push(Finding::fatal(
                    "dangling_layer_resource",
                    format!(
                        "layer {index} references missing resource {}",
                        layer.resource
                    ),
                ));
            }
        }
    }
    let _ = (warp_count, rotation_count);
    findings
}

/// Facts recovered by scanning produced XML text.
#[derive(Debug, Clone, Default)]
pub struct XmlScan {
    /// All `xs.id` values in order of appearance.
    pub ids: Vec<usize>,
    /// `(tag, xs.ref target)` pairs in order of appearance.
    pub refs: Vec<(String, usize)>,
    /// `xs.id` -> element tag.
    pub id_tags: BTreeMap<usize, String>,
    /// Duplicate `xs.id` values.
    pub duplicate_ids: Vec<usize>,
    /// References whose target does not exist.
    pub dangling: Vec<(String, usize)>,
    /// References from GUID-typed elements to non-GUID targets.
    pub guid_type_mismatches: Vec<(String, usize, String)>,
    /// `xs.id`/`xs.ref` values that are not `#<number>` (tag, raw value).
    pub malformed_refs: Vec<(String, String)>,
    /// Number of `<?xml ...?>` declarations found.
    pub xml_decl_count: usize,
}

impl XmlScan {
    /// True when the document is internally consistent and well formed enough
    /// for our own checks (single declaration, no malformed refs).
    pub fn is_valid(&self) -> bool {
        self.duplicate_ids.is_empty()
            && self.dangling.is_empty()
            && self.guid_type_mismatches.is_empty()
            && self.malformed_refs.is_empty()
            && self.xml_decl_count == 1
    }
}

/// Scan produced XML for id/reference integrity (small internal scanner; no
/// DTDs or entities are ever interpreted).
pub fn scan_xml(xml: &str) -> XmlScan {
    let mut scan = XmlScan::default();
    let mut seen: BTreeSet<usize> = BTreeSet::new();
    let bytes = xml.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let Some(open) = xml[cursor..].find('<') else {
            break;
        };
        let start = cursor + open;
        cursor = start + 1;
        // Skip declarations, processing instructions, comments and closers.
        match bytes.get(cursor) {
            Some(b'?') => {
                if xml[start..].starts_with("<?xml ") {
                    scan.xml_decl_count += 1;
                }
                continue;
            }
            Some(b'!') | Some(b'/') => continue,
            None => break,
            _ => {}
        }
        let Some(end_rel) = xml[start..].find('>') else {
            break;
        };
        let end = start + end_rel;
        let inner = &xml[start + 1..end];
        let mut parts = inner.split_whitespace();
        let Some(tag) = parts.next() else {
            cursor = end + 1;
            continue;
        };
        let tag = tag.trim_end_matches('/').to_string();
        let mut id: Option<usize> = None;
        let mut ref_target: Option<usize> = None;
        let mut rest = inner;
        while let Some(attribute_start) = rest.find('=') {
            let name_area = &rest[..attribute_start];
            let name = name_area
                .split_whitespace()
                .last()
                .unwrap_or_default()
                .to_string();
            let after = &rest[attribute_start + 1..];
            let Some(quote) = after.chars().next() else {
                break;
            };
            if quote != '"' && quote != '\'' {
                break;
            }
            let Some(close_rel) = after[1..].find(quote) else {
                break;
            };
            let value = &after[1..1 + close_rel];
            if name == "xs.id" {
                match value.strip_prefix('#').map(str::parse::<usize>) {
                    Some(Ok(parsed)) => id = Some(parsed),
                    Some(Err(_)) => scan.malformed_refs.push((tag.clone(), value.to_string())),
                    None => scan.malformed_refs.push((tag.clone(), value.to_string())),
                }
            } else if name == "xs.ref" {
                match value.strip_prefix('#').map(str::parse::<usize>) {
                    Some(Ok(parsed)) => ref_target = Some(parsed),
                    Some(Err(_)) => scan.malformed_refs.push((tag.clone(), value.to_string())),
                    None => scan.malformed_refs.push((tag.clone(), value.to_string())),
                }
            }
            rest = &after[1 + close_rel + 1..];
        }
        if let Some(id) = id {
            if !seen.insert(id) {
                scan.duplicate_ids.push(id);
            }
            scan.ids.push(id);
            scan.id_tags.insert(id, tag.clone());
        }
        if let Some(target) = ref_target {
            scan.refs.push((tag, target));
        }
        cursor = end + 1;
    }
    for (tag, target) in &scan.refs {
        match scan.id_tags.get(target) {
            None => scan.dangling.push((tag.clone(), *target)),
            Some(target_tag) => {
                if tag.ends_with("Guid") && !target_tag.ends_with("Guid") {
                    scan.guid_type_mismatches
                        .push((tag.clone(), *target, target_tag.clone()));
                }
            }
        }
    }
    scan
}

/// Entity counts for the semantic CMO3 inspector (work order section 45).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cmo3Inspection {
    /// Parts (`CPartSource` objects).
    pub parts: usize,
    /// Art meshes (`CArtMeshSource` objects).
    pub art_meshes: usize,
    /// Warp deformers.
    pub warps: usize,
    /// Rotation deformers.
    pub rotations: usize,
    /// Parameters.
    pub parameters: usize,
    /// Keyform binding sources.
    pub bindings: usize,
    /// Keyform grid sources.
    pub keyform_grids: usize,
    /// Stored forms (all four form kinds).
    pub forms: usize,
    /// Texture image resources (`file imageFileBuf`).
    pub textures: usize,
    /// Mask references across all `clipGuidList` entries.
    pub masks: usize,
    /// Layers.
    pub layers: usize,
    /// Model images.
    pub model_images: usize,
}

/// Count semantic entities in a generated `main.xml`.
pub fn inspect_xml(xml: &str) -> Cmo3Inspection {
    let scan = scan_xml(xml);
    let objects = |tag: &str| {
        scan.id_tags
            .values()
            .filter(|candidate| candidate.as_str() == tag)
            .count()
    };
    let occurrences = |needle: &str| xml.matches(needle).count();
    let mut masks = 0usize;
    let mut cursor = 0usize;
    while let Some(found) = xml[cursor..].find("clipGuidList") {
        let at = cursor + found;
        if let Some(count_at) = xml[at..].find("count=\"") {
            let value_at = at + count_at + "count=\"".len();
            let value: String = xml[value_at..]
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect();
            masks += value.parse::<usize>().unwrap_or(0);
        }
        cursor = at + "clipGuidList".len();
    }
    // `<main>` objects carry no `xs.id`; count their exact opening tags
    // (references always carry attributes, so the bare tag is unambiguous).
    let main_objects = |tag: &str| occurrences(&format!("<{tag}>"));
    Cmo3Inspection {
        parts: main_objects("CPartSource"),
        art_meshes: main_objects("CArtMeshSource"),
        warps: main_objects("CWarpDeformerSource"),
        rotations: main_objects("CRotationDeformerSource"),
        parameters: main_objects("CParameterSource"),
        bindings: objects("KeyformBindingSource"),
        keyform_grids: objects("KeyformGridSource"),
        forms: occurrences("<CArtMeshForm>")
            + occurrences("<CPartForm>")
            + occurrences("<CWarpDeformerForm>")
            + occurrences("<CRotationDeformerForm>"),
        textures: occurrences("xs.n=\"imageFileBuf\""),
        masks,
        layers: objects("CLayer"),
        model_images: occurrences("<CModelImage "),
    }
}

/// One step of the image-pipeline trace (work order section 37).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineStep {
    /// Step label.
    pub label: String,
    /// Whether the link is present and resolvable.
    pub pass: bool,
    /// Detail (referenced id or reason).
    pub detail: String,
}

/// Build the ArtMesh -> PNG pipeline trace for every textured mesh.
pub fn pipeline_trace(project: &Cmo3Project) -> Vec<PipelineStep> {
    let mut steps: Vec<PipelineStep> = Vec::new();
    let resource_count = project.image_resources.len();
    let layer_count = project
        .layered_images
        .first()
        .map(|layered| layered.layers.len())
        .unwrap_or(0);
    for (index, mesh) in project.meshes.iter().enumerate() {
        let label = |suffix: &str| format!("mesh[{index}] {suffix}");
        let has_model_image = mesh.model_image.is_some();
        steps.push(PipelineStep {
            label: label("TextureInputExtension"),
            pass: has_model_image,
            detail: if has_model_image {
                "present".to_string()
            } else {
                "mesh has no texture input (untethered)".to_string()
            },
        });
        let Some(model_image_index) = mesh.model_image else {
            continue;
        };
        steps.push(PipelineStep {
            label: label("TextureInput_ModelImage"),
            pass: true,
            detail: format!("model image {model_image_index}"),
        });
        let model_image = project.model_images.get(model_image_index);
        steps.push(PipelineStep {
            label: label("ModelImageGuid"),
            pass: model_image.is_some(),
            detail: model_image
                .map(|model| model.semantic.clone())
                .unwrap_or_else(|| "missing".to_string()),
        });
        let Some(model_image) = model_image else {
            continue;
        };
        steps.push(PipelineStep {
            label: label("CModelImage"),
            pass: true,
            detail: model_image.name.clone(),
        });
        steps.push(PipelineStep {
            label: label("FilterEnv"),
            pass: true,
            detail: "2 env values (current guid, layer selector)".to_string(),
        });
        let layer = project
            .layered_images
            .first()
            .and_then(|layered| layered.layers.get(model_image.layer));
        steps.push(PipelineStep {
            label: label("LayerSelector"),
            pass: layer.is_some() && model_image.layer < layer_count,
            detail: format!("layer {}", model_image.layer),
        });
        let Some(layer) = layer else {
            continue;
        };
        steps.push(PipelineStep {
            label: label("Layer"),
            pass: true,
            detail: format!("{} ({})", layer.name, layer.layer_id),
        });
        let resource = project.image_resources.get(layer.resource);
        steps.push(PipelineStep {
            label: label("ImageResource"),
            pass: resource.is_some() && layer.resource < resource_count,
            detail: resource
                .map(|resource| resource.semantic.clone())
                .unwrap_or_else(|| "missing".to_string()),
        });
        steps.push(PipelineStep {
            label: label("imageFileBuf"),
            pass: resource.is_some(),
            detail: resource
                .map(|resource| resource.archive_name.clone())
                .unwrap_or_else(|| "missing".to_string()),
        });
    }
    steps
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    const DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

    #[test]
    fn scanner_detects_dangling_and_duplicate_ids() {
        let xml = format!(
            "{DECL}<root><a xs.id=\"#0\" xs.n=\"x\"/><b xs.id=\"#0\"/><c xs.ref=\"#9\"/><DGuid xs.ref=\"#7\"/><DGuid xs.id=\"#7\" xs.n=\"g\"/></root>"
        );
        let scan = scan_xml(&xml);
        assert_eq!(scan.duplicate_ids, vec![0]);
        assert_eq!(scan.dangling, vec![("c".to_string(), 9)]);
        assert!(!scan.is_valid());
    }

    #[test]
    fn scanner_accepts_balanced_references() {
        let xml = format!(
            "{DECL}<root><CLayerGuid xs.id=\"#1\" uuid=\"u\"/><CLayer xs.ref=\"#1\"/></root>"
        );
        let scan = scan_xml(&xml);
        assert!(scan.is_valid(), "{scan:?}");
    }

    #[test]
    fn scanner_flags_guid_type_mismatch() {
        let xml = format!(
            "{DECL}<root><CLayerGuid xs.id=\"#1\" uuid=\"u\"/><CLayerGuid xs.ref=\"#1\"/><CLayer xs.id=\"#2\"/><CLayerGuid xs.ref=\"#2\"/></root>"
        );
        let scan = scan_xml(&xml);
        assert_eq!(scan.guid_type_mismatches.len(), 1);
    }

    #[test]
    fn scanner_flags_malformed_refs_and_duplicate_declarations() {
        let xml = format!(
            "{DECL}{DECL}<root><a xs.ref=\"#abc\"/><b xs.id=\"#99999999999999999999\"/></root>"
        );
        let scan = scan_xml(&xml);
        assert_eq!(scan.malformed_refs.len(), 2);
        assert_eq!(scan.xml_decl_count, 2);
        assert!(!scan.is_valid());
    }
}
