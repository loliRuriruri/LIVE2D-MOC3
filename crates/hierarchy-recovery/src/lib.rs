//! Hierarchy reconstruction (RecoveryGraph).
//!
//! **Status: reserved for AGENT.3.** Planned content: graph nodes/edges,
//! `ParentCandidate` scoring, cycle/orphan/multi-parent detection, and
//! deterministic ordering. Not implemented in AGENT.1 by specification.

#![forbid(unsafe_code)]

/// Phase that will implement this crate.
pub const PLANNED_PHASE: &str = "AGENT.3";
