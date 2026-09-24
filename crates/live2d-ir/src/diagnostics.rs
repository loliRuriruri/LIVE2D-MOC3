//! Structured diagnostics: parser anomalies, mapper notes and validator
//! findings all end up here (never as bare stdout text).

use serde::{Deserialize, Serialize};

use crate::ids::{
    ArtMeshId, BindingId, DeformerId, GlueId, MaskGroupId, ParameterId, PartId, TextureId,
};

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Purely informational.
    Info,
    /// Suspicious but harmless.
    Warning,
    /// Data had to be adjusted (for example a fallback id was used).
    Recoverable,
    /// The IR is invalid and must not be exported or consumed.
    Fatal,
}

/// Reference to the entity a diagnostic is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum EntityRef {
    /// The model as a whole.
    Model(ModelMarker),
    /// A parameter.
    Parameter(ParameterId),
    /// A part.
    Part(PartId),
    /// A deformer.
    Deformer(DeformerId),
    /// An art mesh.
    ArtMesh(ArtMeshId),
    /// A texture page.
    Texture(TextureId),
    /// A mask group.
    MaskGroup(MaskGroupId),
    /// A glue entry.
    Glue(GlueId),
    /// A keyform binding.
    Binding(BindingId),
}

/// Unit marker so `EntityRef::Model` serializes as `{"kind":"model","id":null}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ModelMarker;

/// One structured diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity.
    pub severity: Severity,
    /// Stable machine-readable code (see `docs/IR_SPEC.md`).
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Entity the diagnostic belongs to, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<EntityRef>,
}

impl Diagnostic {
    /// Build a diagnostic without an entity reference.
    pub fn new(severity: Severity, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            entity: None,
        }
    }

    /// Build a diagnostic attached to an entity.
    pub fn for_entity(
        severity: Severity,
        code: impl Into<String>,
        message: impl Into<String>,
        entity: EntityRef,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            entity: Some(entity),
        }
    }
}

/// True when any diagnostic is fatal.
pub fn has_fatal(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Fatal)
}

/// Number of diagnostics with the given severity.
pub fn count_severity(diagnostics: &[Diagnostic], severity: Severity) -> usize {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == severity)
        .count()
}
