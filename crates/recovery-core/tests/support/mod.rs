//! Shared helpers for IR integration tests.
#![allow(dead_code)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use moc3_ingest::counts::parse_count_info;
use moc3_ingest::table;
use moc3_ingest::{ByteOrder, ByteView, MocVersion};

/// Directory containing the checked-in synthetic fixtures.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic")
}

/// Directory containing the golden IR documents.
pub fn expected_ir_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/expected-ir")
}

/// Directory containing the golden recovered-project documents.
pub fn expected_hierarchy_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/expected-hierarchy")
}

/// Read a fixture by name.
pub fn read_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixtures_dir().join(name))
        .unwrap_or_else(|error| panic!("could not read fixture {name}: {error}"))
}

/// Resolve a named section offset inside a (valid) fixture.
pub fn section_offset(bytes: &[u8], target: &str) -> u64 {
    let view = ByteView::new(bytes, ByteOrder::Little);
    let version = MocVersion::from_byte(*bytes.get(4).unwrap()).unwrap();
    let file_len = bytes.len() as u64;
    let count_info_offset = u64::from(view.u32_at(0x40, "count_info").unwrap());
    let _ = parse_count_info(&view, version, count_info_offset, file_len).unwrap();
    let offsets = view
        .u32_vec(0x40, version.offset_slots() as u64, "table")
        .unwrap();
    for (index, def) in table::defs_for(version).iter().enumerate() {
        if def.name == target {
            return u64::from(*offsets.get(index).unwrap());
        }
    }
    panic!("section {target} not found");
}

/// Write a little-endian `u32` at `offset`.
pub fn write_u32(bytes: &mut [u8], offset: u64, value: u32) {
    let at = usize::try_from(offset).unwrap();
    bytes
        .get_mut(at..at + 4)
        .unwrap()
        .copy_from_slice(&value.to_le_bytes());
}

/// Write a little-endian `f32` at `offset`.
pub fn write_f32(bytes: &mut [u8], offset: u64, value: f32) {
    write_u32(bytes, offset, value.to_bits());
}

/// Deterministic pseudo-random bytes (LCG) for fuzz-style sweeps.
pub fn lcg_bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        out.push((state >> 33) as u8);
    }
    out
}

/// Directory containing the golden keyform documents (AGENT.4).
pub fn expected_keyforms_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/expected-keyforms")
}

/// Relocate the body of a v1-5 `.moc3` by `shift` bytes (the py-moc3
/// convention places the body at 1984; the native layout is 0x2C0 read from
/// the offset table). Used by the body-placement regression tests.
pub fn relocate_body(bytes: &[u8], shift: u32) -> Vec<u8> {
    let version = MocVersion::from_byte(*bytes.get(4).unwrap()).unwrap();
    let slots = version.offset_slots();
    let table_end = 0x40 + slots * 4;
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() + shift as usize);
    out.extend_from_slice(bytes.get(..table_end).unwrap());
    out.resize(out.len() + shift as usize, 0);
    out.extend_from_slice(bytes.get(table_end..).unwrap());
    for slot in 0..slots {
        let at = 0x40 + slot * 4;
        let value = read_u32(bytes, at as u64);
        if value > 0 && u64::from(value) <= bytes.len() as u64 {
            write_u32(&mut out, at as u64, value + shift);
        }
    }
    out
}

/// Read a little-endian `u32` at `offset`.
pub fn read_u32(bytes: &[u8], offset: u64) -> u32 {
    let at = usize::try_from(offset).unwrap();
    let raw = bytes.get(at..at + 4).unwrap();
    u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]])
}
