//! Canonical differential snapshot.
//!
//! External implementations never get compared directly; each provider first
//! normalizes its view of the input into this small schema. Values are stored
//! in provider order and the comparison engine handles ordering separately
//! (see `ORDER_ONLY_DIFFERENCE` in compare.rs).
//!
//! Determinism: the snapshot is plain ordered collections only; the embedded
//! input fingerprint uses FNV-1a (stable, documented, non-cryptographic).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Snapshot schema identifier.
pub const SNAPSHOT_SCHEMA: &str = "live2d-recovery/differential-snapshot/1";

/// Absolute tolerance for float comparison (documented policy; integers and
/// ids are always exact).
pub const FLOAT_ABS_EPSILON: f64 = 1e-6;

/// Relative tolerance for float comparison.
pub const FLOAT_REL_EPSILON: f64 = 1e-5;

/// Input identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputInfo {
    /// File name only.
    pub file_name: String,
    /// File size in bytes.
    pub size: u64,
    /// Stable non-cryptographic fingerprint (FNV-1a 64, hex).
    pub fnv1a64: String,
}

/// Canvas block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasSnapshot {
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
    /// Pixels per unit.
    pub pixels_per_unit: f64,
    /// Origin X.
    pub origin_x: f64,
    /// Origin Y.
    pub origin_y: f64,
}

/// One parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterSnapshot {
    /// Stored identifier.
    pub id: String,
    /// Minimum value.
    pub minimum: f64,
    /// Maximum value.
    pub maximum: f64,
    /// Default value.
    pub default: f64,
}

/// One part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartSnapshot {
    /// Stored identifier.
    pub id: String,
    /// Parent part identifier.
    pub parent: Option<String>,
}

/// One deformer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeformerSnapshot {
    /// Stored identifier.
    pub id: String,
    /// `warp` or `rotation`.
    pub kind: String,
    /// Parent deformer identifier.
    pub parent_deformer: Option<String>,
    /// Parent part identifier.
    pub parent_part: Option<String>,
}

/// One art mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshSnapshot {
    /// Stored identifier.
    pub id: String,
    /// Parent part identifier.
    pub parent_part: Option<String>,
    /// Parent deformer identifier.
    pub parent_deformer: Option<String>,
    /// Texture page number.
    pub texture: Option<i64>,
    /// Vertex count.
    pub vertex_count: i64,
    /// UV float count (2 per vertex).
    pub uv_count: i64,
    /// Triangle index count.
    pub index_count: i64,
    /// Mask references (art mesh indices, source order).
    pub mask_refs: Vec<i64>,
}

/// One hierarchy edge derived from stored relations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EdgeSnapshot {
    /// Child identifier.
    pub child: String,
    /// Parent identifier.
    pub parent: String,
    /// `part` or `deformer`.
    pub relation: String,
}

/// Normalized provider view of one input file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DifferentialSnapshot {
    /// Schema id.
    pub schema: String,
    /// Provider name that produced this snapshot.
    pub provider: String,
    /// Input identity.
    pub input: InputInfo,
    /// MOC3 version byte when known.
    pub moc3_version: Option<u8>,
    /// `little` or `big` when known.
    pub endian: Option<String>,
    /// Canvas block when known.
    pub canvas: Option<CanvasSnapshot>,
    /// Counts by canonical name.
    pub counts: BTreeMap<String, u64>,
    /// Parameters in provider order.
    pub parameters: Vec<ParameterSnapshot>,
    /// Parts in provider order.
    pub parts: Vec<PartSnapshot>,
    /// Deformers in provider order.
    pub deformers: Vec<DeformerSnapshot>,
    /// Art meshes in provider order.
    pub art_meshes: Vec<MeshSnapshot>,
    /// Hierarchy edges (sorted; comparison is set-based).
    pub hierarchy_edges: Vec<EdgeSnapshot>,
    /// Diagnostic codes (sorted).
    pub diagnostics: Vec<String>,
    /// Field groups this provider cannot supply, with reasons.
    pub unsupported: BTreeMap<String, String>,
}

impl DifferentialSnapshot {
    /// Create an empty snapshot for a provider.
    pub fn empty(provider: &str, input: &InputInfo) -> Self {
        Self {
            schema: SNAPSHOT_SCHEMA.to_string(),
            provider: provider.to_string(),
            input: input.clone(),
            moc3_version: None,
            endian: None,
            canvas: None,
            counts: BTreeMap::new(),
            parameters: Vec::new(),
            parts: Vec::new(),
            deformers: Vec::new(),
            art_meshes: Vec::new(),
            hierarchy_edges: Vec::new(),
            diagnostics: Vec::new(),
            unsupported: BTreeMap::new(),
        }
    }

    /// Canonicalize: sort sets and mark provider order only where the source
    /// order is meaningful (arrays stay as produced; edges/diagnostics sort).
    pub fn canonicalize(&mut self) {
        self.hierarchy_edges.sort();
        self.hierarchy_edges.dedup();
        self.diagnostics.sort();
        self.diagnostics.dedup();
    }
}

/// Stable non-cryptographic fingerprint (FNV-1a 64 bit).
pub fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Build the input identity block.
pub fn input_info(path: &std::path::Path, bytes: &[u8]) -> InputInfo {
    InputInfo {
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "(unknown)".to_string()),
        size: bytes.len() as u64,
        fnv1a64: fnv1a64(bytes),
    }
}

/// Float comparison under the documented tolerance policy.
pub fn floats_equal(left: f64, right: f64) -> bool {
    if left == right {
        return true;
    }
    let difference = (left - right).abs();
    if difference <= FLOAT_ABS_EPSILON {
        return true;
    }
    let scale = left.abs().max(right.abs());
    scale > 0.0 && difference / scale <= FLOAT_REL_EPSILON
}
