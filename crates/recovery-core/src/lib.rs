//! Pipeline services for the Live2D recovery toolkit.
//!
//! This crate owns everything that is *around* the pure parser: reading files
//! from disk in a read-only fashion, size guarding, and assembling the
//! inspection report. Later phases (IR export, reconstruction, validation)
//! also live here so that the CLI layer stays thin.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

use live2d_ir::{IrExportError, IrImportError, Live2DModel};
use moc3_ingest::{
    build_inspection_report, parse, parse_full, InspectionReport, Limits, Moc3Error, ParsedModel,
    SourceInfo,
};

pub mod ir_mapper;

/// Options for [`inspect_file`].
#[derive(Debug, Clone, Default)]
pub struct InspectOptions {
    /// Parser safety limits.
    pub limits: Limits,
}

/// Errors produced by inspection at the filesystem level.
#[derive(Debug)]
pub enum InspectError {
    /// The file could not be read.
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// The file is larger than the configured limit (checked before reading).
    TooLarge {
        /// Path that failed.
        path: PathBuf,
        /// File size in bytes.
        size: u64,
        /// Configured maximum.
        limit: u64,
    },
    /// The moc3 parser rejected the file.
    Parse {
        /// Path that failed.
        path: PathBuf,
        /// Structured parser error.
        source: Moc3Error,
    },
}

impl fmt::Display for InspectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InspectError::Io { path, source } => {
                write!(f, "failed to read '{}': {source}", path.display())
            }
            InspectError::TooLarge { path, size, limit } => write!(
                f,
                "'{}' is {size} bytes which exceeds the configured limit of {limit} bytes",
                path.display()
            ),
            InspectError::Parse { path, source } => {
                write!(f, "failed to parse '{}': {source}", path.display())
            }
        }
    }
}

impl std::error::Error for InspectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InspectError::Io { source, .. } => Some(source),
            InspectError::Parse { source, .. } => Some(source),
            InspectError::TooLarge { .. } => None,
        }
    }
}

impl InspectError {
    /// Machine-readable error code, or the underlying parser code.
    pub fn code(&self) -> &'static str {
        match self {
            InspectError::Io { .. } => "IoError",
            InspectError::TooLarge { .. } => "FileTooLarge",
            InspectError::Parse { source, .. } => source.code(),
        }
    }

    /// Recovery suggestion, or `None` for IO-level failures.
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            InspectError::Io { .. } => None,
            InspectError::TooLarge { .. } => {
                Some("raise --max-file-size only if you trust the input file")
            }
            InspectError::Parse { source, .. } => Some(source.suggestion()),
        }
    }

    /// Related file offset when the parser provides one.
    pub fn offset(&self) -> Option<u64> {
        match self {
            InspectError::Parse { source, .. } => source.offset,
            _ => None,
        }
    }
}

/// Inspect a `.moc3` file read-only and build the inspection report.
///
/// The file is opened read-only and never modified. The file name (not the
/// full path) is embedded in the report to keep output deterministic across
/// machines.
pub fn inspect_file(
    path: &Path,
    options: &InspectOptions,
) -> Result<InspectionReport, InspectError> {
    let bytes = read_file_capped(path, options.limits.max_file_size)?;
    let model = parse(&bytes, &options.limits).map_err(|source| InspectError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    Ok(build_inspection_report(
        model,
        SourceInfo {
            file_name,
            file_size: bytes.len() as u64,
        },
    ))
}

/// Convenience helper: parse bytes directly without filesystem access.
pub fn inspect_bytes(
    bytes: &[u8],
    file_name: Option<String>,
    options: &InspectOptions,
) -> Result<InspectionReport, Moc3Error> {
    let model = parse(bytes, &options.limits)?;
    Ok(build_inspection_report(
        model,
        SourceInfo {
            file_name,
            file_size: bytes.len() as u64,
        },
    ))
}

/// Read a file through a hard cap (`take`) so a file that grows between the
/// size check and the read can never exceed the limit.
fn read_file_capped(path: &Path, limit: u64) -> Result<Vec<u8>, InspectError> {
    let file = std::fs::File::open(path).map_err(|source| InspectError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut bytes = Vec::new();
    let mut reader = file.take(limit.saturating_add(1));
    reader
        .read_to_end(&mut bytes)
        .map_err(|source| InspectError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() as u64 > limit {
        return Err(InspectError::TooLarge {
            path: path.to_path_buf(),
            size: bytes.len() as u64,
            limit,
        });
    }
    Ok(bytes)
}

/// Map an already parsed model (with pools) into the Live2D IR.
pub fn build_ir(parsed: ParsedModel) -> Live2DModel {
    ir_mapper::map_parsed_model(parsed)
}

/// Parse `.moc3` bytes (including bulk pools) and map them to the Live2D IR.
pub fn build_ir_from_bytes(
    bytes: &[u8],
    options: &InspectOptions,
) -> Result<Live2DModel, Moc3Error> {
    let parsed = parse_full(bytes, &options.limits)?;
    Ok(build_ir(parsed))
}

/// Read a `.moc3` file read-only and map it to the Live2D IR.
pub fn build_ir_from_file(
    path: &Path,
    options: &InspectOptions,
) -> Result<Live2DModel, InspectError> {
    let bytes = read_file_capped(path, options.limits.max_file_size)?;
    let parsed = parse_full(&bytes, &options.limits).map_err(|source| InspectError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    // Release the input buffer before mapping so the IR is built with the
    // parser pools as the only large allocation besides the output.
    drop(bytes);
    Ok(build_ir(parsed))
}

/// Serialize an IR model to canonical JSON (validating first).
pub fn export_ir_json(model: &Live2DModel, pretty: bool) -> Result<String, IrExportError> {
    live2d_ir::to_json_str(model, pretty)
}

/// Parse an IR model from canonical JSON and check the schema id.
pub fn import_ir_json(text: &str) -> Result<Live2DModel, IrImportError> {
    live2d_ir::from_json_str(text)
}

/// Recover the semantic keyform model from an IR document (AGENT.4).
///
/// The optional recovered project is used for cross-layer checks only.
pub fn recover_keyforms(
    model: &Live2DModel,
    project: Option<&hierarchy_recovery::RecoveredProject>,
) -> keyform_recovery::RecoveredKeyformModel {
    keyform_recovery::recover(model, project)
}

/// Serialize a recovered keyform document to canonical JSON (refusing fatal
/// diagnostics).
pub fn export_keyforms_json(
    document: &keyform_recovery::RecoveredKeyformModel,
    pretty: bool,
) -> Result<String, keyform_recovery::KeyformExportError> {
    keyform_recovery::to_json_str(document, pretty)
}

/// Parse a recovered keyform document from canonical JSON.
pub fn import_keyforms_json(
    text: &str,
) -> Result<keyform_recovery::RecoveredKeyformModel, keyform_recovery::KeyformImportError> {
    keyform_recovery::from_json_str(text)
}
