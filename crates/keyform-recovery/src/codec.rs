//! Canonical JSON codec for recovered keyform documents.
//!
//! Serialization is deterministic (struct field order, no map iteration).
//! Export refuses documents whose diagnostics contain a Fatal finding;
//! import checks the schema id but does not validate (call
//! [`crate::validate_recovered_keyforms`] afterwards).

use live2d_ir::has_fatal;

use crate::model::{RecoveredKeyformModel, SCHEMA_ID};

/// Export failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyformExportError {
    /// Stable error code.
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
}

impl std::fmt::Display for KeyformExportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for KeyformExportError {}

/// Import failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyformImportError {
    /// Stable error code.
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
}

impl std::fmt::Display for KeyformImportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for KeyformImportError {}

/// Serialize a recovered keyform document to canonical JSON.
pub fn to_json_str(
    document: &RecoveredKeyformModel,
    pretty: bool,
) -> Result<String, KeyformExportError> {
    if has_fatal(&document.diagnostics) {
        return Err(KeyformExportError {
            code: "KeyformValidationFailed",
            message: "document has fatal diagnostics and must not be exported".to_string(),
        });
    }
    let result = if pretty {
        serde_json::to_string_pretty(document)
    } else {
        serde_json::to_string(document)
    };
    result.map_err(|error| KeyformExportError {
        code: "SerializationFailed",
        message: error.to_string(),
    })
}

/// Parse a recovered keyform document from canonical JSON.
pub fn from_json_str(text: &str) -> Result<RecoveredKeyformModel, KeyformImportError> {
    let document: RecoveredKeyformModel =
        serde_json::from_str(text).map_err(|error| KeyformImportError {
            code: "InvalidJson",
            message: error.to_string(),
        })?;
    if document.schema != SCHEMA_ID {
        return Err(KeyformImportError {
            code: "SchemaMismatch",
            message: format!("document schema '{}' is not '{SCHEMA_ID}'", document.schema),
        });
    }
    Ok(document)
}
