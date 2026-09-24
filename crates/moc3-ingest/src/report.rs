//! Inspection report: a deterministic, serializable view over the raw parse.
//!
//! The report separates what the file *stores* (exact) from what the inspector
//! *derives* (hierarchy edges, textures). Hierarchy edges carry a recovery
//! confidence label (master spec section 8) and the field they came from.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::counts::CountInfo;
use crate::raw::{Anomaly, RawCanvas, RawMoc3};
use crate::reader::IdField;

/// Report schema identifier (stable across releases of this phase).
pub const REPORT_SCHEMA: &str = "live2d-recovery/inspect-report/1";

/// Where the inspected bytes came from.
#[derive(Debug, Clone, Serialize)]
pub struct SourceInfo {
    /// File name only (paths are deliberately not embedded, for determinism
    /// across machines).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    /// File size in bytes.
    pub file_size: u64,
}

/// Recovery confidence for derived structure (master spec section 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryConfidence {
    /// Value is stored directly in the file.
    Exact,
    /// Value is computed from stored values without guessing.
    Derived,
    /// Value comes from an inference rule that may be wrong.
    Heuristic,
    /// No basis for the value.
    Unknown,
}

/// A node reference used by hierarchy output.
#[derive(Debug, Clone, Serialize)]
pub struct NodeRef {
    /// Node kind (`part`, `deformer`, `art_mesh`).
    pub kind: &'static str,
    /// Index inside its own array.
    pub index: usize,
    /// Display name (identifier or deterministic fallback).
    pub name: String,
}

/// One hierarchy edge.
#[derive(Debug, Clone, Serialize)]
pub struct HierarchyEdge {
    /// Parent node.
    pub parent: NodeRef,
    /// Child node.
    pub child: NodeRef,
    /// Relationship kind (`parent_part`, `parent_deformer`, `masked_by`).
    pub relation: &'static str,
    /// Stored field the edge was derived from.
    pub source_field: &'static str,
    /// Recovery confidence.
    pub confidence: RecoveryConfidence,
}

/// Derived hierarchy summary.
#[derive(Debug, Clone, Serialize)]
pub struct HierarchyReport {
    /// Edges in deterministic order.
    pub edges: Vec<HierarchyEdge>,
    /// Nodes without any parent reference.
    pub roots: Vec<NodeRef>,
    /// Anomalies found while deriving hierarchy (cycles, dangling links).
    pub anomalies: Vec<Anomaly>,
}

/// Texture page usage.
#[derive(Debug, Clone, Serialize)]
pub struct TextureRef {
    /// Texture page number as stored in art meshes.
    pub texture_number: u32,
    /// Art mesh indices referencing this page (ascending).
    pub art_mesh_indices: Vec<usize>,
}

/// High-level summary block.
#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    /// Raw MOC3 version byte.
    pub moc3_version_byte: u8,
    /// Version label with Cubism range.
    pub moc3_version_label: String,
    /// Byte order label.
    pub byte_order: &'static str,
    /// Canvas block.
    pub canvas: RawCanvas,
    /// Count info table.
    pub counts: CountInfo,
    /// Number of distinct texture pages referenced.
    pub texture_reference_count: usize,
    /// Total keyforms across all object categories.
    pub total_keyforms: u64,
    /// Number of hierarchy edges.
    pub hierarchy_edge_count: usize,
    /// Number of non-fatal anomalies.
    pub anomaly_count: usize,
}

/// Complete inspection report.
#[derive(Debug, Clone, Serialize)]
pub struct InspectionReport {
    /// Schema identifier.
    pub schema: &'static str,
    /// Input source description.
    pub source: SourceInfo,
    /// Summary block.
    pub summary: Summary,
    /// Derived hierarchy.
    pub hierarchy: HierarchyReport,
    /// Texture references.
    pub textures: Vec<TextureRef>,
    /// Raw parse result.
    pub model: RawMoc3,
}

/// Deterministic display name for a node.
///
/// Stored identifiers are used verbatim when present. Empty identifiers fall
/// back to the deterministic naming scheme from the master spec (section 7):
/// `ArtMesh_0001`, `WarpDeformer_0012`, `Parameter_0004`, ...
pub fn node_name(kind: &'static str, id: &IdField, index: usize) -> String {
    if !id.text.is_empty() {
        return id.text.clone();
    }
    match kind {
        "parameter" => format!("Parameter_{index:04}"),
        "part" => format!("Part_{index:04}"),
        "art_mesh" => format!("ArtMesh_{index:04}"),
        "deformer" => format!("Deformer_{index:04}"),
        "glue" => format!("Glue_{index:04}"),
        _ => format!("Node_{index:04}"),
    }
}

fn part_names(model: &RawMoc3) -> Vec<String> {
    model
        .parts
        .iter()
        .map(|part| node_name("part", &part.id, part.index))
        .collect()
}

fn deformer_names(model: &RawMoc3) -> Vec<String> {
    model
        .deformers
        .iter()
        .map(|deformer| node_name("deformer", &deformer.id, deformer.index))
        .collect()
}

fn art_mesh_names(model: &RawMoc3) -> Vec<String> {
    model
        .art_meshes
        .iter()
        .map(|mesh| node_name("art_mesh", &mesh.id, mesh.index))
        .collect()
}

/// Build the inspection report from a raw parse result.
pub fn build_inspection_report(model: RawMoc3, source: SourceInfo) -> InspectionReport {
    let parts = part_names(&model);
    let deformers = deformer_names(&model);
    let art_meshes = art_mesh_names(&model);

    let mut edges: Vec<HierarchyEdge> = Vec::new();
    let mut roots: Vec<NodeRef> = Vec::new();
    let mut hierarchy_anomalies: Vec<Anomaly> = Vec::new();

    let part_ref = |index: usize| NodeRef {
        kind: "part",
        index,
        name: parts
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("Part_{index:04}")),
    };
    let deformer_ref = |index: usize| NodeRef {
        kind: "deformer",
        index,
        name: deformers
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("Deformer_{index:04}")),
    };
    let mesh_ref = |index: usize| NodeRef {
        kind: "art_mesh",
        index,
        name: art_meshes
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("ArtMesh_{index:04}")),
    };

    for part in &model.parts {
        match part.parent_part {
            Some(parent) => edges.push(HierarchyEdge {
                parent: part_ref(parent as usize),
                child: part_ref(part.index),
                relation: "parent_part",
                source_field: "part.parent_part",
                confidence: RecoveryConfidence::Exact,
            }),
            None => roots.push(part_ref(part.index)),
        }
    }

    for deformer in &model.deformers {
        let mut has_parent = false;
        if let Some(parent) = deformer.parent_deformer {
            edges.push(HierarchyEdge {
                parent: deformer_ref(parent as usize),
                child: deformer_ref(deformer.index),
                relation: "parent_deformer",
                source_field: "deformer.parent_deformer",
                confidence: RecoveryConfidence::Exact,
            });
            has_parent = true;
        }
        if let Some(parent) = deformer.parent_part {
            edges.push(HierarchyEdge {
                parent: part_ref(parent as usize),
                child: deformer_ref(deformer.index),
                relation: "parent_part",
                source_field: "deformer.parent_part",
                confidence: RecoveryConfidence::Exact,
            });
            has_parent = true;
        }
        if !has_parent {
            roots.push(deformer_ref(deformer.index));
        }
    }

    for mesh in &model.art_meshes {
        let mut has_parent = false;
        if let Some(parent) = mesh.parent_deformer {
            edges.push(HierarchyEdge {
                parent: deformer_ref(parent as usize),
                child: mesh_ref(mesh.index),
                relation: "parent_deformer",
                source_field: "art_mesh.parent_deformer",
                confidence: RecoveryConfidence::Exact,
            });
            has_parent = true;
        }
        if let Some(parent) = mesh.parent_part {
            edges.push(HierarchyEdge {
                parent: part_ref(parent as usize),
                child: mesh_ref(mesh.index),
                relation: "parent_part",
                source_field: "art_mesh.parent_part",
                confidence: RecoveryConfidence::Exact,
            });
            has_parent = true;
        }
        if !has_parent {
            roots.push(mesh_ref(mesh.index));
        }
        for mask in &mesh.mask_drawables {
            if *mask < 0 {
                continue;
            }
            edges.push(HierarchyEdge {
                parent: mesh_ref(*mask as usize),
                child: mesh_ref(mesh.index),
                relation: "masked_by",
                source_field: "art_mesh.mask / drawable_mask.indices",
                confidence: RecoveryConfidence::Exact,
            });
        }
    }

    // Cycle detection over part->part and deformer->deformer chains.
    for (kind, parents) in [
        (
            "part",
            model
                .parts
                .iter()
                .map(|part| part.parent_part.map(|value| value as usize))
                .collect::<Vec<_>>(),
        ),
        (
            "deformer",
            model
                .deformers
                .iter()
                .map(|deformer| deformer.parent_deformer.map(|value| value as usize))
                .collect::<Vec<_>>(),
        ),
    ] {
        let cyclic = find_cycles(&parents);
        for index in cyclic {
            hierarchy_anomalies.push(Anomaly {
                code: "hierarchy_cycle",
                message: format!("{kind}[{index}] participates in a parent cycle"),
                offset: None,
                index: Some(index),
            });
        }
    }

    let mut texture_map: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for mesh in &model.art_meshes {
        if mesh.texture_number < 0 {
            continue;
        }
        let key = u32::try_from(mesh.texture_number).unwrap_or(0);
        texture_map.entry(key).or_default().push(mesh.index);
    }
    let textures: Vec<TextureRef> = texture_map
        .into_iter()
        .map(|(texture_number, art_mesh_indices)| TextureRef {
            texture_number,
            art_mesh_indices,
        })
        .collect();

    let total_keyforms = [
        model.counts.part_keyforms,
        model.counts.warp_deformer_keyforms,
        model.counts.rotation_deformer_keyforms,
        model.counts.art_mesh_keyforms,
        model.counts.glue_keyforms,
        model.counts.offscreen_keyforms,
    ]
    .iter()
    .map(|value| *value as u64)
    .fold(0u64, |acc, value| acc.saturating_add(value));

    let anomaly_count = model.anomalies.len() + hierarchy_anomalies.len();

    let summary = Summary {
        moc3_version_byte: model.header.version_byte,
        moc3_version_label: model.header.version_label.clone(),
        byte_order: model.header.byte_order.label(),
        canvas: model.canvas.clone(),
        counts: model.counts.clone(),
        texture_reference_count: textures.len(),
        total_keyforms,
        hierarchy_edge_count: edges.len(),
        anomaly_count,
    };

    InspectionReport {
        schema: REPORT_SCHEMA,
        source,
        summary,
        hierarchy: HierarchyReport {
            edges,
            roots,
            anomalies: hierarchy_anomalies,
        },
        textures,
        model,
    }
}

/// Iterative cycle detection over a parent map.
///
/// Returns indices participating in a cycle; deterministic and allocation
/// bounded by the node count.
fn find_cycles(parents: &[Option<usize>]) -> Vec<usize> {
    let n = parents.len();
    let mut state = vec![0u8; n]; // 0 = unknown, 1 = finished, 2 = on stack
    let mut cyclic = vec![false; n];
    for start in 0..n {
        if state.get(start).copied().unwrap_or(1) != 0 {
            continue;
        }
        let mut path: Vec<usize> = Vec::new();
        let mut current = Some(start);
        while let Some(node) = current {
            match state.get(node).copied().unwrap_or(1) {
                0 => {
                    if let Some(slot) = state.get_mut(node) {
                        *slot = 2;
                    }
                    path.push(node);
                    current = parents.get(node).copied().flatten();
                }
                2 => {
                    if let Some(position) = path.iter().position(|value| *value == node) {
                        for marked in path.iter().skip(position) {
                            if let Some(slot) = cyclic.get_mut(*marked) {
                                *slot = true;
                            }
                        }
                    }
                    break;
                }
                _ => break,
            }
        }
        for node in &path {
            if let Some(slot) = state.get_mut(*node) {
                *slot = 1;
            }
        }
    }
    cyclic
        .iter()
        .enumerate()
        .filter_map(|(index, flag)| if *flag { Some(index) } else { None })
        .collect()
}
