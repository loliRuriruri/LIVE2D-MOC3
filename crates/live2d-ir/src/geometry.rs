//! Shared geometry primitives.
//!
//! Non-finite values are not rejected here; they are preserved so the IR
//! validator can report exactly where they came from (see `validate.rs` and
//! `docs/IR_SPEC.md` for the policy).

use serde::{Deserialize, Serialize};

/// A 2D vector in canvas units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
}

impl Vec2 {
    /// Construct a vector.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// True when both components are finite.
    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// A texture coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Uv {
    /// U component.
    pub u: f32,
    /// V component.
    pub v: f32,
}

impl Uv {
    /// Construct a texture coordinate.
    pub fn new(u: f32, v: f32) -> Self {
        Self { u, v }
    }

    /// True when both components are finite.
    pub fn is_finite(&self) -> bool {
        self.u.is_finite() && self.v.is_finite()
    }
}
