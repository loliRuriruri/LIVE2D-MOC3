//! Provenance (recovery confidence) attached to entities and derived fields.
//!
//! Provenance is not attached to every primitive; it lives at entity level
//! and on the meaningful fields listed in `field_provenance` (AGENT.2 work
//! order section 7).

use serde::{Deserialize, Serialize};

/// Recovery confidence of an IR value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Copied from a stored file value.
    Exact,
    /// Computed from stored values without guessing.
    Derived,
    /// Produced by an inference rule that may be wrong.
    Heuristic,
    /// No basis for the value.
    Unknown,
}

/// Where a value came from and how much it can be trusted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Recovery confidence.
    pub confidence: Confidence,
    /// Stable source label, for example `moc3:part.parent_part`.
    pub source: String,
    /// Optional explanation, required when confidence is not `Exact`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Provenance {
    /// Build a provenance record.
    pub fn new(confidence: Confidence, source: impl Into<String>) -> Self {
        Self {
            confidence,
            source: source.into(),
            note: None,
        }
    }

    /// Add an explanatory note.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Shorthand for an exact, stored value.
    pub fn exact(source: impl Into<String>) -> Self {
        Self::new(Confidence::Exact, source)
    }

    /// Shorthand for a value computed from stored values.
    pub fn derived(source: impl Into<String>) -> Self {
        Self::new(Confidence::Derived, source)
    }

    /// Shorthand for a value with no basis.
    pub fn unknown(source: impl Into<String>, note: impl Into<String>) -> Self {
        Self::new(Confidence::Unknown, source).with_note(note)
    }
}

/// Provenance override for one field of an entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldProvenance {
    /// Field name inside the entity, for example `y_axis_reversed`.
    pub field: String,
    /// Provenance of that field.
    pub provenance: Provenance,
}
