//! Stable, typed entity identifiers.
//!
//! ID policy (AGENT.2 work order section 5):
//!
//! - the stored source identifier is kept verbatim when it exists and is
//!   unique inside its entity kind,
//! - otherwise a deterministic fallback `kind:NNNNNN` is generated from the
//!   source index (for example `artmesh:000004`),
//! - duplicates never collide: the first occurrence keeps the source name,
//!   later occurrences receive their deterministic fallback,
//! - no UUIDs, randomness, timestamps, or hash-order dependence.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Create the identifier from any string-like value.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Borrow the identifier text.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_id!(
    /// Identifier of a parameter entry.
    ParameterId
);
string_id!(
    /// Identifier of a part entry.
    PartId
);
string_id!(
    /// Identifier of a deformer entry (warp or rotation).
    DeformerId
);
string_id!(
    /// Identifier of an art mesh entry.
    ArtMeshId
);
string_id!(
    /// Identifier of a texture page entry.
    TextureId
);
string_id!(
    /// Identifier of a mask group entry.
    MaskGroupId
);
string_id!(
    /// Identifier of a glue entry.
    GlueId
);
string_id!(
    /// Identifier of a keyform binding entry.
    BindingId
);
string_id!(
    /// Identifier of a draw order group entry.
    DrawOrderGroupId
);

/// Why an assignment did not keep the stored source name verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdOutcome {
    /// The stored identifier was unique and kept.
    SourceName,
    /// The stored identifier was empty; a deterministic fallback was used.
    EmptySourceName,
    /// The stored identifier was already used; a deterministic fallback was used.
    DuplicateSourceName,
}

/// Result of one identifier assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdAssignment {
    /// Canonical identifier text.
    pub id: String,
    /// Why this text was chosen.
    pub outcome: IdOutcome,
}

/// Deterministic identifier generator for one entity kind.
#[derive(Debug, Clone)]
pub struct IdAssigner {
    prefix: &'static str,
    used: BTreeSet<String>,
}

impl IdAssigner {
    /// Create an assigner for an entity kind (for example `"artmesh"`).
    pub fn new(prefix: &'static str) -> Self {
        Self {
            prefix,
            used: BTreeSet::new(),
        }
    }

    /// Assign an identifier for the entity at `index`.
    pub fn assign(&mut self, index: usize, source_name: Option<&str>) -> IdAssignment {
        let prefix = self.prefix;
        self.assign_with_prefix(index, source_name, prefix)
    }

    /// Assign an identifier using a per-entity fallback prefix.
    ///
    /// The uniqueness set is shared with all other assignments from this
    /// assigner, so two entity kinds can share one namespace while still
    /// receiving kind-specific fallback names (`warp:000003`,
    /// `rotation:000005`, ...).
    pub fn assign_with_prefix(
        &mut self,
        index: usize,
        source_name: Option<&str>,
        prefix: &'static str,
    ) -> IdAssignment {
        let source = source_name.filter(|name| !name.is_empty());
        match source {
            Some(name) if !self.used.contains(name) => {
                self.used.insert(name.to_string());
                IdAssignment {
                    id: name.to_string(),
                    outcome: IdOutcome::SourceName,
                }
            }
            Some(_) => IdAssignment {
                id: self.fallback(index, prefix),
                outcome: IdOutcome::DuplicateSourceName,
            },
            None => IdAssignment {
                id: self.fallback(index, prefix),
                outcome: IdOutcome::EmptySourceName,
            },
        }
    }

    fn fallback(&mut self, index: usize, prefix: &'static str) -> String {
        let base = format!("{prefix}:{index:06}");
        let mut candidate = base.clone();
        let mut suffix = 2usize;
        while self.used.contains(&candidate) {
            candidate = format!("{base}:{suffix}");
            suffix += 1;
        }
        self.used.insert(candidate.clone());
        candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_unique_source_names() {
        let mut assigner = IdAssigner::new("part");
        let assignment = assigner.assign(0, Some("Part_Head"));
        assert_eq!(assignment.id, "Part_Head");
        assert_eq!(assignment.outcome, IdOutcome::SourceName);
    }

    #[test]
    fn falls_back_deterministically() {
        let mut assigner = IdAssigner::new("part");
        let first = assigner.assign(0, Some("Dup"));
        let second = assigner.assign(1, Some("Dup"));
        let empty = assigner.assign(2, Some(""));
        assert_eq!(first.outcome, IdOutcome::SourceName);
        assert_eq!(second.id, "part:000001");
        assert_eq!(second.outcome, IdOutcome::DuplicateSourceName);
        assert_eq!(empty.id, "part:000002");
        assert_eq!(empty.outcome, IdOutcome::EmptySourceName);
    }

    #[test]
    fn never_collides_with_source_names() {
        let mut assigner = IdAssigner::new("part");
        let _ = assigner.assign(0, Some("part:000001"));
        let second = assigner.assign(1, Some(""));
        assert_eq!(second.id, "part:000001:2");
    }
}
