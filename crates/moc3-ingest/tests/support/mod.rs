//! Shared helpers for moc3-ingest integration tests.
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

/// Directory containing the golden inspection reports.
pub fn expected_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/expected")
}

/// Read a checked-in fixture by file name.
pub fn read_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixtures_dir().join(name)).unwrap_or_else(|error| {
        panic!("could not read fixture {name}: {error} (run: cargo run -p fixture-gen -- fixtures/synthetic)")
    })
}

/// Golden report path for a fixture name.
pub fn golden_path(name: &str) -> PathBuf {
    expected_dir().join(format!("{name}.report.json"))
}

/// Resolve the file offset of a named section in a (valid) fixture.
pub fn section_offset(bytes: &[u8], target: &str) -> u64 {
    let view = ByteView::new(bytes, ByteOrder::Little);
    let version = MocVersion::from_byte(*bytes.get(4).unwrap()).unwrap();
    let file_len = bytes.len() as u64;
    let count_info_offset = u64::from(view.u32_at(0x40, "count_info").unwrap());
    let counts = parse_count_info(&view, version, count_info_offset, file_len).unwrap();
    let _ = &counts;
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

/// Overwrite a little-endian `u32` at `offset`.
pub fn write_u32(bytes: &mut [u8], offset: u64, value: u32) {
    let at = usize::try_from(offset).unwrap();
    bytes
        .get_mut(at..at + 4)
        .unwrap()
        .copy_from_slice(&value.to_le_bytes());
}

/// Overwrite a little-endian `i32` at `offset`.
pub fn write_i32(bytes: &mut [u8], offset: u64, value: i32) {
    write_u32(bytes, offset, value as u32);
}

/// Read a little-endian `u32` at `offset`.
pub fn read_u32(bytes: &[u8], offset: u64) -> u32 {
    let at = usize::try_from(offset).unwrap();
    let slice = bytes.get(at..at + 4).unwrap();
    u32::from_le_bytes(<[u8; 4]>::try_from(slice).unwrap())
}

/// Offset of the count info table in a fixture.
pub fn count_info_offset(bytes: &[u8]) -> u64 {
    u64::from(read_u32(bytes, 0x40))
}

/// Deterministic pseudo-random bytes (LCG) for fuzz smoke tests.
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
