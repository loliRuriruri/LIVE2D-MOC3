//! Normalized Live2D Intermediate Representation.
//!
//! This crate is the semantic layer between the binary parser and every
//! downstream recovery stage. **It has no dependency on `moc3-ingest` (or any
//! other binary-format crate)** and must never gain one: the mapper that
//! converts `RawMoc3` into these types lives in `recovery-core`.
//!
//! Guarantees:
//!
//! - typed, stable entity identifiers (source name when unique, otherwise a
//!   deterministic `kind:NNNNNN` fallback),
//! - references are typed ids, never raw indices or file offsets,
//! - recovery confidence (`provenance`) travels with the data,
//! - unknown information is preserved in a separate [`model::Unknowns`]
//!   section, never dropped,
//! - canonical JSON export/import with round-trip support,
//! - a deterministic [`validate::validate_ir`] that flags broken models.
//!
//! Schema status: **experimental** (`live2d-ir/1`); no real-file validation
//! has happened yet, so the schema may still change.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::indexing_slicing)]

pub mod diagnostics;
pub mod geometry;
pub mod ids;
pub mod json;
pub mod model;
pub mod provenance;
pub mod validate;

pub use diagnostics::{count_severity, has_fatal, Diagnostic, EntityRef, Severity};
pub use geometry::{Uv, Vec2};
pub use ids::{
    ArtMeshId, BindingId, DeformerId, DrawOrderGroupId, GlueId, IdAssigner, IdAssignment,
    IdOutcome, MaskGroupId, ParameterId, PartId, TextureId,
};
pub use json::{from_json_str, to_json_str, IrExportError, IrImportError};
pub use model::{
    ArtMesh, ArtMeshKeyform, Binding, BindingParameter, BindingTarget, BlendMode, Canvas, Deformer,
    DeformerCommon, DrawOrderGroup, DrawOrderItem, DrawOrderTarget, DrawableFlags, Endianness,
    EntityCounts, Glue, GlueInfoEntry, Live2DModel, MaskGroup, Metadata, Parameter, ParameterKind,
    Part, RotationDeformer, RotationKeyform, RuntimeSectionInfo, SchemaStatus, SourceFormat,
    Texture, UndefinedSlot, UnknownCountField, Unknowns, WarpDeformer, WarpKeyform, SCHEMA_ID,
};
pub use provenance::{Confidence, FieldProvenance, Provenance};
pub use validate::validate_ir;
