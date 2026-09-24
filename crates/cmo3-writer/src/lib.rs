//! Minimal-to-fidelity CMO3 writer (AGENT.5).
//!
//! The writer performs **serialization only**: it consumes the Live2D IR,
//! the recovered project and the recovered keyform model and emits a `.cmo3`
//! container. It never guesses hierarchy, reconstructs keyforms or invents
//! bindings (work order section 0).
//!
//! Pipeline (work order section 6):
//!
//! ```text
//! semantic inputs -> Cmo3Project -> identity allocation -> XML document
//!                 -> CAFF entries -> CAFF encoder -> .cmo3 bytes
//! ```
//!
//! The crate must never depend on `moc3-ingest`: binary layout details of the
//! runtime format are unknown here.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod caff;
pub mod ids;
pub mod map;
pub mod model;
pub mod profile;
pub mod textures;
pub mod xml;
