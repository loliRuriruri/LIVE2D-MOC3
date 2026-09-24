//! CMO3 serializer.
//!
//! **Status: reserved for AGENT.5 and intentionally empty.** The master spec
//! requires that no CMO3 bytes are produced before the inspector, IR, and
//! hierarchy stages are complete and reviewed. The writer will be structured
//! as `Project IR -> Serializer -> Cmo3VersionProfile -> CMO3`, never as
//! hardcoded byte patterns.

#![forbid(unsafe_code)]

/// Phase that will implement this crate.
pub const PLANNED_PHASE: &str = "AGENT.5";
