//! Layout interoperability: body offset conventions.
//!
//! Differential work against py-moc3 (commit 2fb112e11a) showed that its
//! writer/reader always places the body (count info onward) at offset 1984,
//! while our generated files and the public format references place it
//! directly after the section offset table (704 for versions 1-5, 1984 for
//! version 6). Our parser follows the offset table pointers, so it must
//! accept both conventions with identical semantics.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use moc3_ingest::{build_inspection_report, parse, Limits, SourceInfo};

/// Serialize the inspection report with layout-dependent keys removed, so two
/// files that carry identical semantics compare equal regardless of where the
/// body sits in the file.
fn semantic_projection(name: &str, bytes: &[u8]) -> serde_json::Value {
    let model = parse(bytes, &Limits::default()).unwrap();
    let report = build_inspection_report(
        model,
        SourceInfo {
            file_name: Some(name.to_string()),
            file_size: bytes.len() as u64,
        },
    );
    let mut value = serde_json::to_value(report).unwrap();
    strip_layout_keys(&mut value);
    value
}

fn strip_layout_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.remove("offset");
            map.remove("file_name");
            map.remove("file_size");
            for child in map.values_mut() {
                strip_layout_keys(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_layout_keys(item);
            }
        }
        _ => {}
    }
}

/// Shift the body by inserting `shift` zero bytes at the end of the offset
/// table and patching every non-zero offset (the py-moc3 convention).
fn relocate_body(bytes: &[u8], shift: u64) -> Vec<u8> {
    const TABLE_END: usize = 0x2C0;
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() + shift as usize);
    out.extend_from_slice(bytes.get(..TABLE_END).unwrap());
    out.resize(out.len() + shift as usize, 0);
    out.extend_from_slice(bytes.get(TABLE_END..).unwrap());
    for slot in 0..160usize {
        let at = 0x40 + slot * 4;
        let value = support::read_u32(bytes, at as u64);
        if value > 0 && u64::from(value) <= bytes.len() as u64 {
            support::write_u32(&mut out, at as u64, value + shift as u32);
        }
    }
    out
}

#[test]
fn relocated_body_parses_with_identical_semantics() {
    for name in [
        "fixture-002-artmesh-param.moc3",
        "fixture-006-clipping-mask.moc3",
        "hierarchy-002-part-warp-artmesh.moc3",
    ] {
        let original = support::read_fixture(name);
        // 1280 bytes moves the v3.3 body from 704 to the py-moc3 offset 1984.
        let relocated = relocate_body(&original, 1280);
        assert_eq!(
            semantic_projection(name, &original),
            semantic_projection(name, &relocated),
            "{name}: relocated body changed parsed semantics"
        );
    }
}
