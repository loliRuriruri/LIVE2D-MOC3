//! Identity layers (work order sections 8-11).
//!
//! Three distinct identities are kept apart:
//!
//! - **semantic ids** (`artmesh:000042`, `warp:000012`) from the semantic
//!   layers; never rewritten,
//! - **serialization object ids** (`xs.id="#813"`) allocated deterministically
//!   from a fixed traversal order,
//! - **CMO3 GUIDs** (`uuid="..."`), synthetic writer metadata, generated in a
//!   deterministic test mode or a random production mode.
//!
//! The semantic IR is never modified; GUIDs never enter the IR.

use std::collections::BTreeMap;
use std::hash::{BuildHasher, Hasher};

/// Deterministic serialization-object id allocator.
#[derive(Debug, Clone, Default)]
pub struct ObjectPool {
    next_id: usize,
    next_idx: usize,
    trace: Vec<PoolEntry>,
}

/// One allocated pool entry (for traceability, work order section 78).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolEntry {
    /// `xs.id` value without the `#`.
    pub id: usize,
    /// `xs.idx` value.
    pub idx: usize,
    /// Object kind (element name).
    pub kind: String,
    /// Semantic entity this object was generated from, when known.
    pub semantic: Option<String>,
}

impl ObjectPool {
    /// New empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate the next object id with its kind and optional semantic source.
    pub fn allocate(&mut self, kind: impl Into<String>, semantic: Option<&str>) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.trace.push(PoolEntry {
            id,
            idx: self.next_idx,
            kind: kind.into(),
            semantic: semantic.map(str::to_string),
        });
        self.next_idx += 1;
        id
    }

    /// The `xs.idx` counter shared by the whole document.
    pub fn next_idx(&self) -> usize {
        self.next_idx
    }

    /// Number of allocated objects.
    pub fn len(&self) -> usize {
        self.trace.len()
    }

    /// True when nothing was allocated.
    pub fn is_empty(&self) -> bool {
        self.trace.is_empty()
    }

    /// Allocation trace in allocation order.
    pub fn trace(&self) -> &[PoolEntry] {
        &self.trace
    }

    /// Semantic reference for one pool id.
    pub fn semantic_of(&self, id: usize) -> Option<&str> {
        self.trace
            .iter()
            .find(|entry| entry.id == id)
            .and_then(|entry| entry.semantic.as_deref())
    }
}

/// GUID generation policy (work order section 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidMode {
    /// Deterministic UUIDs derived from (namespace, kind, semantic id).
    Deterministic,
    /// Random UUID v4-shaped identifiers (standard project identity).
    Random,
}

/// GUID allocator.
#[derive(Debug, Clone)]
pub struct GuidAllocator {
    mode: GuidMode,
    namespace: u64,
    counters: BTreeMap<String, u64>,
}

impl GuidAllocator {
    /// Build an allocator for one namespace label.
    pub fn new(mode: GuidMode, namespace: &str) -> Self {
        Self {
            mode,
            namespace: fnv1a64(namespace.as_bytes()),
            counters: BTreeMap::new(),
        }
    }

    /// Allocate a GUID for a kind + semantic identity.
    pub fn allocate(&mut self, kind: &str, semantic: &str) -> String {
        match self.mode {
            GuidMode::Deterministic => {
                let mut bytes = [0u8; 16];
                // The occurrence counter keeps repeated (kind, semantic)
                // allocations collision-free while staying deterministic.
                let counter = self
                    .counters
                    .entry(format!("{kind}:{semantic}"))
                    .or_insert(0);
                *counter += 1;
                let seed = self
                    .namespace
                    .rotate_left(13)
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    ^ fnv1a64(kind.as_bytes())
                    ^ fnv1a64(semantic.as_bytes())
                    ^ splitmix64(*counter);
                let mut state = seed;
                for chunk in bytes.chunks_mut(8) {
                    state = splitmix64(state);
                    let value = state.to_le_bytes();
                    chunk.copy_from_slice(&value[..chunk.len()]);
                }
                format_uuid(bytes)
            }
            GuidMode::Random => {
                let counter = self
                    .counters
                    .entry(format!("{kind}:{semantic}"))
                    .or_insert(0);
                *counter += 1;
                let mut bytes = [0u8; 16];
                let seed = std::collections::hash_map::RandomState::new()
                    .build_hasher()
                    .finish();
                let mut state = seed ^ fnv1a64(kind.as_bytes()) ^ (*counter);
                for chunk in bytes.chunks_mut(8) {
                    state = splitmix64(state);
                    chunk.copy_from_slice(&state.to_le_bytes()[..chunk.len()]);
                }
                format_uuid(bytes)
            }
        }
    }
}

fn format_uuid(mut bytes: [u8; 16]) -> String {
    // UUID version 4 shape: version nibble 4, variant bits 10xx.
    if let Some(byte) = bytes.get_mut(6) {
        *byte = (*byte & 0x0F) | 0x40;
    }
    if let Some(byte) = bytes.get_mut(8) {
        *byte = (*byte & 0x3F) | 0x80;
    }
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        hex.get(0..8).unwrap_or_default(),
        hex.get(8..12).unwrap_or_default(),
        hex.get(12..16).unwrap_or_default(),
        hex.get(16..20).unwrap_or_default(),
        hex.get(20..32).unwrap_or_default()
    )
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_ids_are_sequential_and_deterministic() {
        let mut pool = ObjectPool::new();
        assert_eq!(pool.allocate("CModelGuid", Some("model")), 0);
        assert_eq!(pool.allocate("CPartGuid", Some("part:000000")), 1);
        assert_eq!(pool.len(), 2);
        assert_eq!(pool.semantic_of(1), Some("part:000000"));
    }

    #[test]
    fn deterministic_guids_are_stable_and_shaped_like_v4() {
        let mut first = GuidAllocator::new(GuidMode::Deterministic, "liver2d-test");
        let mut second = GuidAllocator::new(GuidMode::Deterministic, "liver2d-test");
        let a = first.allocate("CPartGuid", "part:000001");
        let b = second.allocate("CPartGuid", "part:000001");
        assert_eq!(a, b);
        assert_eq!(a.len(), 36);
        assert_eq!(a.as_bytes().get(14), Some(&b'4'));
        let variant = u8::from_str_radix(&a[19..20], 16).unwrap_or(0);
        assert!(variant >= 8, "variant nibble must be 10xx");
        let different = first.allocate("CPartGuid", "part:000002");
        assert_ne!(a, different);
    }

    #[test]
    fn random_mode_is_still_uuid_shaped() {
        let mut allocator = GuidAllocator::new(GuidMode::Random, "liver2d");
        let a = allocator.allocate("CModelGuid", "model");
        let b = allocator.allocate("CModelGuid", "model");
        assert_ne!(a, b);
        assert_eq!(a.as_bytes().get(14), Some(&b'4'));
    }
}
