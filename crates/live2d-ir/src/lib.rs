//! Live2D Intermediate Representation (IR).
//!
//! **Status: reserved for AGENT.2.** The master spec forbids implementing the
//! IR before the read-only inspector passes its acceptance criteria. This
//! crate intentionally contains no logic yet; it exists so the workspace
//! layout matches the specification from the start.
//!
//! Planned content (master spec section 6):
//!
//! ```text
//! ModelProject
//! +- Metadata
//! +- Canvas
//! +- Parameters[]
//! +- Parts[]
//! +- ArtMeshes[]
//! +- Deformers[]        (WarpDeformer | RotationDeformer)
//! +- Drawables[]
//! +- Textures[]
//! +- Masks[]
//! +- Bindings[]
//! ```
//!
//! The IR must be a *normalized* representation: binary parser internals
//! (raw offsets, parallel arrays) are not exposed.

#![forbid(unsafe_code)]

/// Phase that will implement this crate.
pub const PLANNED_PHASE: &str = "AGENT.2";
