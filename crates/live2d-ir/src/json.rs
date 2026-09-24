//! Canonical JSON export and import.
//!
//! Export refuses to serialize a model that fails validation, so non-finite
//! floats can never silently become `null` in the output. Import only parses
//! and checks the schema id; callers run [`crate::validate::validate_ir`] on
//! the result.

use std::fmt;

use crate::diagnostics::{has_fatal, Severity};
use crate::model::{Live2DModel, SCHEMA_ID};
use crate::validate::validate_ir;

/// Error returned when exporting IR to JSON fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrExportError {
    /// Stable error code (`ValidationFailed`, `SerializationFailed`).
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
}

impl fmt::Display for IrExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for IrExportError {}

/// Error returned when importing IR from JSON fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrImportError {
    /// Stable error code (`JsonSyntax`, `SchemaMismatch`).
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Line number for syntax errors.
    pub line: Option<usize>,
    /// Column number for syntax errors.
    pub column: Option<usize>,
}

impl fmt::Display for IrImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)?;
        if let (Some(line), Some(column)) = (self.line, self.column) {
            write!(formatter, " (line {line}, column {column})")?;
        }
        Ok(())
    }
}

impl std::error::Error for IrImportError {}

/// Serialize a model to canonical JSON.
///
/// Fails with `ValidationFailed` when the model produces any fatal
/// diagnostic; this is what keeps `NaN`/`Inf` out of the JSON (serde_json
/// would otherwise emit `null`).
pub fn to_json_str(model: &Live2DModel, pretty: bool) -> Result<String, IrExportError> {
    let diagnostics = validate_ir(model);
    if has_fatal(&diagnostics) {
        let first = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == Severity::Fatal)
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .unwrap_or_else(|| "unknown fatal diagnostic".to_string());
        return Err(IrExportError {
            code: "ValidationFailed",
            message: format!("IR failed validation before export ({first})"),
        });
    }
    let result = if pretty {
        serde_json::to_string_pretty(model)
    } else {
        serde_json::to_string(model)
    };
    result.map_err(|error| IrExportError {
        code: "SerializationFailed",
        message: error.to_string(),
    })
}

/// Parse a model from canonical JSON and check the schema id.
pub fn from_json_str(text: &str) -> Result<Live2DModel, IrImportError> {
    let model: Live2DModel = serde_json::from_str(text).map_err(|error| IrImportError {
        code: "JsonSyntax",
        message: error.to_string(),
        line: error.line().checked_sub(0),
        column: error.column().checked_sub(0),
    })?;
    if model.schema != SCHEMA_ID {
        return Err(IrImportError {
            code: "SchemaMismatch",
            message: format!("expected schema '{SCHEMA_ID}', found '{}'", model.schema),
            line: None,
            column: None,
        });
    }
    Ok(model)
}
