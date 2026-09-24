//! Read-only parser for Live2D Cubism `.moc3` model files.
//!
//! This crate is the "MOC3 Ingest" stage of the Live2D Recovery pipeline:
//!
//! ```text
//! MOC3 bytes -> [moc3-ingest] -> Raw Model (RawMoc3)
//! ```
//!
//! Design rules (master spec v0.1):
//! - never panic on malformed input: every read is bounds checked and every
//!   count is validated against an explicit limit before allocation,
//! - structured errors that carry an offset and a recovery suggestion,
//! - deterministic output: no timestamps, no randomness, no map iteration
//!   order leaks into the serialized report,
//! - unknown data is preserved and reported, never silently dropped.
//!
//! The parser intentionally exposes a *raw* view of the file (arrays parallel
//! to the on-disk tables). Normalization into the Live2D IR happens in the
//! `live2d-ir` crate (AGENT.2), and recovery heuristics in
//! `hierarchy-recovery` (AGENT.3). Keeping those stages separate is a hard
//! requirement of the master spec.

#![deny(clippy::indexing_slicing)]
#![deny(missing_docs)]

pub mod counts;
pub mod error;
pub mod limits;
pub mod parse;
pub mod raw;
pub mod reader;
pub mod report;
pub mod table;
pub mod text;
pub mod version;

pub use counts::CountInfo;
pub use error::{ErrorKind, Moc3Error};
pub use limits::Limits;
pub use parse::parse;
pub use raw::{FieldConfidence, RawMoc3};
pub use reader::ByteView;
pub use report::{build_inspection_report, HierarchyEdge, InspectionReport, SourceInfo};
pub use version::{ByteOrder, MocVersion};
