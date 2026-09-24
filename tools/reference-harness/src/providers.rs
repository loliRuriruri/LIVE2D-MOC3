//! Reference providers: our pipeline plus isolated external adapters.
//!
//! External tools are always executed as separate processes with a hard
//! timeout and output caps (`REFERENCE_TIMEOUT`, `REFERENCE_ERROR`); they are
//! never linked into production crates. Providers that are documented but not
//! wired up return `NotConfigured`/`Unsupported` instead of guessing.

use std::fmt;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use hierarchy_recovery::{NodeKind, RecoveredProject, RecoveryPolicy};
use live2d_ir::{Deformer, Live2DModel};
use serde::{Deserialize, Serialize};

use crate::snapshot::{
    CanvasSnapshot, DeformerSnapshot, DifferentialSnapshot, EdgeSnapshot, MeshSnapshot,
    ParameterSnapshot, PartSnapshot, SNAPSHOT_SCHEMA,
};

/// Pinned metadata for every audited reference (see
/// `docs/EXTERNAL_REFERENCE_MATRIX.md`; never "latest" at runtime).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceMetadata {
    /// Provider name.
    pub name: String,
    /// Provider kind (`in_process`, `subprocess`, `planned`).
    pub kind: String,
    /// Version string (pinned).
    pub version: String,
    /// Commit hash (pinned when available).
    pub commit: String,
    /// License identifier.
    pub license: String,
    /// Repository URL.
    pub repository: String,
    /// Must always be `false` for the audit harness.
    pub production_dependency: bool,
}

fn reference(
    name: &str,
    kind: &str,
    version: &str,
    commit: &str,
    license: &str,
    repository: &str,
) -> ReferenceMetadata {
    ReferenceMetadata {
        name: name.to_string(),
        kind: kind.to_string(),
        version: version.to_string(),
        commit: commit.to_string(),
        license: license.to_string(),
        repository: repository.to_string(),
        production_dependency: false,
    }
}

/// All references audited in AGENT.3.5, pinned at audit time.
pub fn pinned_references() -> Vec<ReferenceMetadata> {
    vec![
        reference(
            "ours",
            "in_process",
            env!("CARGO_PKG_VERSION"),
            "worktree",
            "UNLICENSED (project decision pending)",
            "local",
        ),
        reference(
            "moc2cmo",
            "planned",
            "1.1.18",
            "2527e24e93",
            "MIT",
            "https://github.com/Eatgrapes/moc2cmo",
        ),
        reference(
            "py-moc3",
            "subprocess",
            "0.2.x (pip `py-moc3`)",
            "2fb112e11a",
            "MIT",
            "https://github.com/Ludentes/py-moc3",
        ),
        reference(
            "PurismCore",
            "planned",
            "v1.1.0",
            "1069334965",
            "MIT",
            "https://github.com/SakuraMotion/PurismCore",
        ),
        reference(
            "ayagami",
            "planned",
            "0.1.x (head)",
            "0d1d7aa3ef",
            "Apache-2.0",
            "https://github.com/AyagamiDev/ayagami",
        ),
        reference(
            "Quadrism",
            "behavioral-only",
            "single initial commit",
            "cb3f8557bd",
            "LGPL-3.0 (copyleft; no code use)",
            "https://codeberg.org/Podimium/Quadrism",
        ),
        reference(
            "caff-archive",
            "reference-only",
            "0.1.0",
            "76f208cf01",
            "MIT (per repository README; GitHub detection empty)",
            "https://github.com/vtubing/caff-archive",
        ),
        reference(
            "StretchyStudio",
            "reference-only",
            "head",
            "pushed 2026-06-19",
            "MIT",
            "https://github.com/pelmentor/stretchystudio",
        ),
    ]
}

/// Look up pinned metadata by provider name.
pub fn pinned_metadata(name: &str) -> Option<ReferenceMetadata> {
    pinned_references()
        .into_iter()
        .find(|reference| reference.name == name)
}

/// Timeout/output configuration for subprocess providers.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Hard timeout per external process.
    pub timeout: Duration,
    /// Maximum accepted stdout size (bytes).
    pub output_limit: usize,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            output_limit: 8 * 1024 * 1024,
        }
    }
}

/// Structured provider failure (never treated as our parser failing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderError {
    /// Provider exists but is not configured in this environment.
    NotConfigured {
        /// Explanation and configuration hint.
        reason: String,
    },
    /// Provider cannot supply the requested comparison at all.
    Unsupported {
        /// Explanation.
        reason: String,
    },
    /// External process failed (non-zero exit, spawn failure, bad output).
    ReferenceError {
        /// Details.
        reason: String,
    },
    /// External process exceeded the timeout and was killed.
    ReferenceTimeout {
        /// Configured timeout in seconds.
        seconds: u64,
    },
    /// External process produced more output than allowed.
    OutputLimitExceeded {
        /// Configured limit in bytes.
        limit: usize,
    },
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderError::NotConfigured { reason } => {
                write!(formatter, "not configured: {reason}")
            }
            ProviderError::Unsupported { reason } => write!(formatter, "unsupported: {reason}"),
            ProviderError::ReferenceError { reason } => {
                write!(formatter, "reference error: {reason}")
            }
            ProviderError::ReferenceTimeout { seconds } => {
                write!(formatter, "reference timeout after {seconds}s")
            }
            ProviderError::OutputLimitExceeded { limit } => {
                write!(formatter, "reference output exceeded {limit} bytes")
            }
        }
    }
}

/// Result alias for provider operations.
pub type ProviderResult<T> = Result<T, ProviderError>;

/// A reference implementation adapter.
pub trait ReferenceProvider {
    /// Pinned metadata.
    fn metadata(&self) -> ReferenceMetadata;
    /// Produce the normalized snapshot for one input.
    fn snapshot(
        &self,
        input: &Path,
        bytes: &[u8],
        config: &ProviderConfig,
    ) -> ProviderResult<DifferentialSnapshot>;
}

/// Construct a provider by name.
pub fn provider_by_name(name: &str) -> Option<Box<dyn ReferenceProvider>> {
    match name {
        "ours" => Some(Box::new(OursProvider)),
        "py-moc3" | "pymoc3" => Some(Box::new(PyMoc3Provider)),
        "moc2cmo" => Some(Box::new(PlannedProvider::new(
            "moc2cmo",
            "library adapter is designed but not enabled in this environment; \
             install with `cargo add --dev moc2cmo@1.1.18` under the optional feature and see docs/EXTERNAL_REFERENCE_MATRIX.md",
        ))),
        "purismcore" => Some(Box::new(PlannedProvider::new(
            "PurismCore",
            "runtime adapter is planned (see docs/RUNTIME_ORACLE_PLAN.md); PurismCore is a C runtime, not a snapshot provider",
        ))),
        "ayagami" => Some(Box::new(PlannedProvider::new(
            "ayagami",
            "runtime oracle is planned for AGENT.4 (see docs/RUNTIME_ORACLE_PLAN.md)",
        ))),
        "quadrism" => Some(Box::new(PlannedProvider::new(
            "Quadrism",
            "behavioral reference only (LGPL-3.0 copyleft; no code or process use)",
        ))),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ours (in-process)
// ---------------------------------------------------------------------------

/// Our own pipeline as a provider (IR + recovered hierarchy).
pub struct OursProvider;

impl ReferenceProvider for OursProvider {
    fn metadata(&self) -> ReferenceMetadata {
        pinned_metadata("ours").unwrap_or_else(|| ReferenceMetadata {
            name: "ours".to_string(),
            kind: "in_process".to_string(),
            version: "unknown".to_string(),
            commit: "worktree".to_string(),
            license: "unknown".to_string(),
            repository: "local".to_string(),
            production_dependency: false,
        })
    }

    fn snapshot(
        &self,
        input: &Path,
        bytes: &[u8],
        _config: &ProviderConfig,
    ) -> ProviderResult<DifferentialSnapshot> {
        let options = recovery_core::InspectOptions::default();
        let model = recovery_core::build_ir_from_file(input, &options).map_err(|error| {
            ProviderError::ReferenceError {
                reason: error.to_string(),
            }
        })?;
        let project = hierarchy_recovery::reconstruct(&model, &RecoveryPolicy::default());
        Ok(snapshot_from_model(&model, Some(&project), input, bytes))
    }
}

/// Build a snapshot from our IR (and optionally the recovered project).
pub fn snapshot_from_model(
    model: &Live2DModel,
    project: Option<&RecoveredProject>,
    input: &Path,
    bytes: &[u8],
) -> DifferentialSnapshot {
    let input_info = crate::snapshot::input_info(input, bytes);
    let mut snapshot = DifferentialSnapshot::empty("ours", &input_info);
    snapshot.moc3_version = Some(model.source.version_byte);
    snapshot.endian = Some(format!("{:?}", model.source.byte_order).to_lowercase());
    snapshot.canvas = Some(CanvasSnapshot {
        width: f64::from(model.canvas.width),
        height: f64::from(model.canvas.height),
        pixels_per_unit: f64::from(model.canvas.pixels_per_unit),
        origin_x: f64::from(model.canvas.origin.x),
        origin_y: f64::from(model.canvas.origin.y),
    });

    let display = |id: &str| -> String {
        project
            .and_then(|project| project.nodes.iter().find(|node| node.id.as_str() == id))
            .and_then(|node| node.source_name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let display_opt = |id: &live2d_ir::PartId| -> Option<String> { Some(display(id.as_str())) };
    let display_deformer =
        |id: &live2d_ir::DeformerId| -> Option<String> { Some(display(id.as_str())) };

    // Counts (canonical names shared with the py-moc3 adapter).
    let entity = model.entity_counts();
    snapshot
        .counts
        .insert("parts".to_string(), entity.parts as u64);
    snapshot
        .counts
        .insert("deformers".to_string(), entity.deformers as u64);
    snapshot
        .counts
        .insert("warp_deformers".to_string(), entity.warp_deformers as u64);
    snapshot.counts.insert(
        "rotation_deformers".to_string(),
        entity.rotation_deformers as u64,
    );
    snapshot
        .counts
        .insert("art_meshes".to_string(), entity.art_meshes as u64);
    snapshot
        .counts
        .insert("parameters".to_string(), entity.parameters as u64);
    snapshot
        .counts
        .insert("textures".to_string(), entity.textures as u64);
    snapshot
        .counts
        .insert("glue".to_string(), entity.glue as u64);
    snapshot
        .counts
        .insert("keyform_bindings".to_string(), entity.bindings as u64);
    snapshot.counts.insert(
        "drawable_masks".to_string(),
        model
            .mask_groups
            .iter()
            .map(|group| group.sources.len() as u64)
            .sum(),
    );
    snapshot.counts.insert(
        "uvs".to_string(),
        model
            .art_meshes
            .iter()
            .map(|mesh| mesh.uvs.len() as u64 * 2)
            .sum(),
    );
    snapshot.counts.insert(
        "position_indices".to_string(),
        model
            .art_meshes
            .iter()
            .map(|mesh| mesh.indices.len() as u64)
            .sum(),
    );
    snapshot.counts.insert(
        "keys".to_string(),
        model
            .parameters
            .iter()
            .map(|parameter| {
                (parameter.key_values.len() + parameter.extension_key_values.len()) as u64
            })
            .sum(),
    );
    snapshot.counts.insert(
        "keyform_positions".to_string(),
        model
            .art_meshes
            .iter()
            .map(|mesh| {
                mesh.keyforms
                    .iter()
                    .map(|keyform| keyform.positions.len() as u64 * 2)
                    .sum::<u64>()
            })
            .sum::<u64>()
            + model
                .deformers
                .iter()
                .map(|deformer| match deformer {
                    Deformer::Warp(warp) => warp
                        .keyforms
                        .iter()
                        .map(|keyform| keyform.positions.len() as u64 * 2)
                        .sum::<u64>(),
                    Deformer::Rotation(_) => 0,
                })
                .sum::<u64>(),
    );
    snapshot.unsupported.insert(
        "counts.parameter_bindings".to_string(),
        "key table identity is merged into parameter key values in the IR".to_string(),
    );

    // Parameters.
    for parameter in &model.parameters {
        snapshot.parameters.push(ParameterSnapshot {
            id: parameter
                .source_name
                .clone()
                .unwrap_or_else(|| parameter.id.as_str().to_string()),
            minimum: f64::from(parameter.minimum),
            maximum: f64::from(parameter.maximum),
            default: f64::from(parameter.default),
        });
    }

    // Parts.
    for part in &model.parts {
        snapshot.parts.push(PartSnapshot {
            id: part
                .source_name
                .clone()
                .unwrap_or_else(|| part.id.as_str().to_string()),
            parent: part.parent.as_ref().map(|id| display(id.as_str())),
        });
    }

    // Deformers.
    for deformer in &model.deformers {
        match deformer {
            Deformer::Warp(warp) => snapshot.deformers.push(DeformerSnapshot {
                id: warp
                    .common
                    .source_name
                    .clone()
                    .unwrap_or_else(|| warp.common.id.as_str().to_string()),
                kind: "warp".to_string(),
                parent_deformer: warp
                    .common
                    .parent_deformer
                    .as_ref()
                    .and_then(display_deformer),
                parent_part: warp.common.parent_part.as_ref().and_then(display_opt),
            }),
            Deformer::Rotation(rotation) => snapshot.deformers.push(DeformerSnapshot {
                id: rotation
                    .common
                    .source_name
                    .clone()
                    .unwrap_or_else(|| rotation.common.id.as_str().to_string()),
                kind: "rotation".to_string(),
                parent_deformer: rotation
                    .common
                    .parent_deformer
                    .as_ref()
                    .and_then(display_deformer),
                parent_part: rotation.common.parent_part.as_ref().and_then(display_opt),
            }),
        }
    }

    // Art meshes.
    let mesh_index_of = |id: &live2d_ir::ArtMeshId| -> Option<i64> {
        model
            .art_meshes
            .iter()
            .position(|mesh| &mesh.id == id)
            .map(|index| index as i64)
    };
    for mesh in &model.art_meshes {
        let mask_refs: Vec<i64> = model
            .mask_groups
            .iter()
            .filter(|group| group.target.as_str() == mesh.id.as_str())
            .flat_map(|group| group.sources.iter())
            .filter_map(&mesh_index_of)
            .collect();
        snapshot.art_meshes.push(MeshSnapshot {
            id: mesh
                .source_name
                .clone()
                .unwrap_or_else(|| mesh.id.as_str().to_string()),
            parent_part: mesh.parent_part.as_ref().and_then(display_opt),
            parent_deformer: mesh.parent_deformer.as_ref().and_then(display_deformer),
            texture: mesh.texture.as_ref().map(|_| {
                // Texture ids are fallback text; recover the page number from
                // the IR texture list.
                model
                    .textures
                    .iter()
                    .find(|texture| Some(&texture.id) == mesh.texture.as_ref())
                    .map(|texture| i64::from(texture.page_index))
                    .unwrap_or(-1)
            }),
            vertex_count: mesh.vertex_count as i64,
            uv_count: mesh.uvs.len() as i64 * 2,
            index_count: mesh.indices.len() as i64,
            mask_refs,
        });
    }

    // Hierarchy edges from the recovered project (display names only).
    snapshot.hierarchy_edges = project.map(edges_from_project).unwrap_or_default();

    // AGENT.4: binding/keyform evidence.
    snapshot.keyforms = keyform_summary(model, project);

    if let Some(project) = project {
        snapshot.diagnostics = project
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.clone())
            .collect();
    }
    snapshot.unsupported.insert(
        "bindings.targets".to_string(),
        "binding target lists are represented through hierarchy edges instead".to_string(),
    );
    snapshot.canonicalize();
    snapshot
}

// ---------------------------------------------------------------------------
// py-moc3 (subprocess, env gated)
// ---------------------------------------------------------------------------

/// py-moc3 adapter (dev-only, pinned commit; see scripts/py_moc3_snapshot.py).
pub struct PyMoc3Provider;

impl ReferenceProvider for PyMoc3Provider {
    fn metadata(&self) -> ReferenceMetadata {
        pinned_metadata("py-moc3").unwrap_or_else(|| ReferenceMetadata {
            name: "py-moc3".to_string(),
            kind: "subprocess".to_string(),
            version: "unknown".to_string(),
            commit: "unknown".to_string(),
            license: "MIT".to_string(),
            repository: "https://github.com/Ludentes/py-moc3".to_string(),
            production_dependency: false,
        })
    }

    fn snapshot(
        &self,
        input: &Path,
        _bytes: &[u8],
        config: &ProviderConfig,
    ) -> ProviderResult<DifferentialSnapshot> {
        let python = std::env::var("PY_MOC3_PYTHON").unwrap_or_else(|_| "python".to_string());
        let checkout = std::env::var("PY_MOC3_DIR").map_err(|_| ProviderError::NotConfigured {
            reason: "set PY_MOC3_DIR to a py-moc3 checkout at commit 2fb112e11a \
                     (scripts/fetch_py_moc3.ps1) and optionally PY_MOC3_PYTHON"
                .to_string(),
        })?;
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/py_moc3_snapshot.py");
        if !script.exists() {
            return Err(ProviderError::NotConfigured {
                reason: format!("adapter script missing: {}", script.display()),
            });
        }
        let mut command = Command::new(python);
        command
            .arg(script)
            .arg(input)
            .env("PY_MOC3_DIR", checkout)
            .env(
                "PYTHONPATH",
                std::env::var("PYTHONPATH").unwrap_or_default(),
            );
        let stdout = run_command_capped(&mut command, config, "py-moc3")?;
        let mut snapshot: DifferentialSnapshot =
            serde_json::from_slice(&stdout).map_err(|error| ProviderError::ReferenceError {
                reason: format!("py-moc3 produced invalid snapshot JSON: {error}"),
            })?;
        if snapshot.schema != SNAPSHOT_SCHEMA {
            return Err(ProviderError::ReferenceError {
                reason: format!("py-moc3 snapshot schema mismatch: {}", snapshot.schema),
            });
        }
        snapshot.canonicalize();
        Ok(snapshot)
    }
}

// ---------------------------------------------------------------------------
// planned providers
// ---------------------------------------------------------------------------

/// A documented provider that is not wired up in this environment.
pub struct PlannedProvider {
    name: &'static str,
    reason: String,
}

impl PlannedProvider {
    /// Construct a planned provider.
    pub fn new(name: &'static str, reason: &str) -> Self {
        Self {
            name,
            reason: reason.to_string(),
        }
    }
}

impl ReferenceProvider for PlannedProvider {
    fn metadata(&self) -> ReferenceMetadata {
        pinned_metadata(self.name).unwrap_or_else(|| ReferenceMetadata {
            name: self.name.to_string(),
            kind: "planned".to_string(),
            version: "unknown".to_string(),
            commit: "unknown".to_string(),
            license: "unknown".to_string(),
            repository: "unknown".to_string(),
            production_dependency: false,
        })
    }

    fn snapshot(
        &self,
        _input: &Path,
        _bytes: &[u8],
        _config: &ProviderConfig,
    ) -> ProviderResult<DifferentialSnapshot> {
        Err(ProviderError::NotConfigured {
            reason: self.reason.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// subprocess plumbing
// ---------------------------------------------------------------------------

/// Run a command with a hard timeout and capped stdout/stderr.
pub fn run_command_capped(
    command: &mut Command,
    config: &ProviderConfig,
    label: &str,
) -> ProviderResult<Vec<u8>> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ProviderError::ReferenceError {
            reason: format!("failed to spawn {label}: {error}"),
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ProviderError::ReferenceError {
            reason: format!("{label}: stdout pipe unavailable"),
        })?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ProviderError::ReferenceError {
            reason: format!("{label}: stderr pipe unavailable"),
        })?;
    let limit = config.output_limit;
    let stdout_reader = std::thread::spawn(move || read_capped(stdout, limit));
    let stderr_reader = std::thread::spawn(move || read_capped(stderr, 64 * 1024));

    let deadline = Instant::now() + config.timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    // Reader threads are intentionally detached: on Windows a
                    // grandchild can inherit the pipe handles and keep a join
                    // blocked, which would defeat the timeout bound.
                    drop(stdout_reader);
                    drop(stderr_reader);
                    return Err(ProviderError::ReferenceTimeout {
                        seconds: config.timeout.as_secs(),
                    });
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProviderError::ReferenceError {
                    reason: format!("{label}: wait failed: {error}"),
                });
            }
        }
    };

    let (stdout_bytes, stdout_overflow) = join_reader(stdout_reader, label)?;
    let (stderr_bytes, _) = join_reader(stderr_reader, label)?;
    if stdout_overflow {
        return Err(ProviderError::OutputLimitExceeded { limit });
    }
    if !status.success() {
        let stderr_text = String::from_utf8_lossy(&stderr_bytes);
        let mut excerpt: String = stderr_text.chars().take(500).collect();
        if stderr_text.chars().count() > 500 {
            excerpt.push_str("...");
        }
        return Err(ProviderError::ReferenceError {
            reason: format!("{label} exited with {status}: {excerpt}"),
        });
    }
    Ok(stdout_bytes)
}

type ReadResult = (Vec<u8>, bool);

fn join_reader(
    handle: std::thread::JoinHandle<std::io::Result<ReadResult>>,
    label: &str,
) -> ProviderResult<ReadResult> {
    match handle.join() {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(error)) => Err(ProviderError::ReferenceError {
            reason: format!("{label}: read failed: {error}"),
        }),
        Err(_) => Err(ProviderError::ReferenceError {
            reason: format!("{label}: reader thread panicked"),
        }),
    }
}

/// Read a stream, keeping at most `limit` bytes but draining the rest so the
/// child never blocks; reports whether the limit was exceeded.
fn read_capped(mut reader: impl Read, limit: usize) -> std::io::Result<ReadResult> {
    let mut kept: Vec<u8> = Vec::new();
    let mut exceeded = false;
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let room = limit.saturating_sub(kept.len());
        let take = room.min(read);
        if let Some(chunk) = buffer.get(..take) {
            kept.extend_from_slice(chunk);
        }
        if read > take {
            exceeded = true;
        }
    }
    Ok((kept, exceeded))
}

/// Build the AGENT.4 binding/keyform evidence summary for our provider.
pub fn keyform_summary(
    model: &Live2DModel,
    project: Option<&RecoveredProject>,
) -> crate::snapshot::KeyformSummary {
    let document = recovery_core::recover_keyforms(model, project);
    let mut summary = crate::snapshot::KeyformSummary {
        targets: document.statistics.targets as u64,
        stored_forms: document.statistics.stored_keyforms,
        bindings: document.statistics.bindings as u64,
        axes: document.statistics.parameter_axes as u64,
        dense_grids: document.statistics.dense_grids as u64,
        sparse_grids: document.statistics.sparse_grids as u64,
        unknown_layout_grids: document.statistics.unknown_layout_grids as u64,
        unresolved: document.statistics.unresolved_entries as u64,
        ..crate::snapshot::KeyformSummary::default()
    };
    for entry in &document.target_keyforms {
        let (label, count) = match entry {
            keyform_recovery::TargetKeyforms::Part(part) => ("part", part.keyforms.len()),
            keyform_recovery::TargetKeyforms::WarpDeformer(warp) => ("warp", warp.keyforms.len()),
            keyform_recovery::TargetKeyforms::RotationDeformer(rotation) => {
                ("rotation", rotation.keyforms.len())
            }
            keyform_recovery::TargetKeyforms::ArtMesh(mesh) => ("art_mesh", mesh.keyforms.len()),
        };
        let count = count as u64;
        match label {
            "part" => summary.part_forms += count,
            "warp" => summary.warp_forms += count,
            "rotation" => summary.rotation_forms += count,
            _ => summary.art_mesh_forms += count,
        }
        summary
            .target_forms
            .push(format!("{}:{}={count}", label, entry.target().id_text()));
    }
    summary.target_forms.sort();
    for band in &document.binding_bands {
        if band.axes.is_empty() || band.binding.as_str().is_empty() {
            continue;
        }
        // Comparable form: raw stored keys per binding (py-moc3 exposes the
        // same raw key arrays through keyform_binding.keys_begin/counts).
        let values: Vec<String> = band
            .axes
            .iter()
            .flat_map(|axis| axis.keys.iter().map(|key| format!("{key}")))
            .collect();
        summary
            .axis_keys
            .push(format!("{}={}", band.binding.as_str(), values.join(",")));
        let parameters: Vec<String> = band
            .axes
            .iter()
            .map(|axis| format!("{}[{}]", axis.parameter.as_str(), axis.stored_key_count))
            .collect();
        summary.axis_parameters.push(format!(
            "{}={}",
            band.binding.as_str(),
            parameters.join(";")
        ));
    }
    summary.axis_keys.sort();
    summary.axis_parameters.sort();
    summary
}

/// Build hierarchy edges from a recovered project (display names only).
pub fn edges_from_project(project: &RecoveredProject) -> Vec<EdgeSnapshot> {
    let display = |id: &str| -> String {
        project
            .nodes
            .iter()
            .find(|node| node.id.as_str() == id)
            .and_then(|node| node.source_name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let kind_of = |id: &str| -> Option<NodeKind> {
        project
            .nodes
            .iter()
            .find(|node| node.id.as_str() == id)
            .map(|node| node.kind)
    };
    let mut edges: Vec<EdgeSnapshot> = Vec::new();
    for node in &project.nodes {
        let Some(parent) = &node.parent else {
            continue;
        };
        let relation = match kind_of(parent.as_str()) {
            Some(NodeKind::Part) => "part",
            Some(NodeKind::WarpDeformer) | Some(NodeKind::RotationDeformer) => "deformer",
            _ => continue,
        };
        edges.push(EdgeSnapshot {
            child: display(node.id.as_str()),
            parent: display(parent.as_str()),
            relation: relation.to_string(),
        });
    }
    edges.sort();
    edges.dedup();
    edges
}

/// A snapshot used by tests to simulate external providers.
pub fn mock_snapshot(
    ours: &DifferentialSnapshot,
    provider: &str,
    mutation: MockMutation,
) -> DifferentialSnapshot {
    let mut snapshot = ours.clone();
    snapshot.provider = provider.to_string();
    match mutation {
        MockMutation::Agree => {}
        MockMutation::FlipParameterMaximum { index } => {
            if let Some(parameter) = snapshot.parameters.get_mut(index) {
                parameter.maximum += 1.0;
            }
        }
        MockMutation::DropLastPart => {
            snapshot.parts.pop();
            snapshot
                .counts
                .insert("parts".to_string(), snapshot.parts.len() as u64);
        }
        MockMutation::ReverseParameters => {
            snapshot.parameters.reverse();
        }
        MockMutation::TinyFloatDrift { index } => {
            if let Some(parameter) = snapshot.parameters.get_mut(index) {
                parameter.default += 1e-9;
            }
        }
        MockMutation::UnsupportedSection => {
            snapshot
                .unsupported
                .insert("parameters".to_string(), "mock: unsupported".to_string());
        }
        MockMutation::DropEdges => {
            snapshot.hierarchy_edges.clear();
        }
    }
    snapshot.canonicalize();
    snapshot
}

/// Test-only snapshot mutations (used by the hidden `mock-provider` mode and
/// by unit tests; never part of a real provider).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockMutation {
    /// Identical snapshot.
    Agree,
    /// Change one parameter maximum.
    FlipParameterMaximum {
        /// Parameter index.
        index: usize,
    },
    /// Remove the last part.
    DropLastPart,
    /// Reverse the parameter order (order-only difference).
    ReverseParameters,
    /// Float change inside the documented tolerance.
    TinyFloatDrift {
        /// Parameter index.
        index: usize,
    },
    /// Mark a field group as unsupported.
    UnsupportedSection,
    /// Remove hierarchy edges.
    DropEdges,
}

/// Parse a mock mutation name (hidden CLI mode).
pub fn mock_mutation_by_name(name: &str) -> Option<MockMutation> {
    Some(match name {
        "agree" => MockMutation::Agree,
        "mismatch-param" => MockMutation::FlipParameterMaximum { index: 0 },
        "missing-part" => MockMutation::DropLastPart,
        "order-only" => MockMutation::ReverseParameters,
        "float-epsilon" => MockMutation::TinyFloatDrift { index: 0 },
        "unsupported-section" => MockMutation::UnsupportedSection,
        "drop-edges" => MockMutation::DropEdges,
        _ => return None,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::read_capped;
    use std::io::Cursor;

    #[test]
    fn read_capped_accepts_exactly_the_limit() {
        let (kept, exceeded) = read_capped(Cursor::new(vec![7u8; 100]), 100).unwrap();
        assert_eq!(kept.len(), 100);
        assert!(!exceeded, "exactly output_limit bytes must not be flagged");
    }

    #[test]
    fn read_capped_flags_actual_overflow() {
        let (kept, exceeded) = read_capped(Cursor::new(vec![7u8; 101]), 100).unwrap();
        assert_eq!(kept.len(), 100);
        assert!(exceeded, "dropped bytes must be flagged");
    }
}
