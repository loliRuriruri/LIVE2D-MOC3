//! Corruption tests through the full IR pipeline (parse -> map -> validate ->
//! serialize). Corrupt input may stop at the parser or at IR validation, but
//! must never panic, hang or allocate without bound.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::panic::{catch_unwind, AssertUnwindSafe};

use live2d_ir::has_fatal;
use moc3_ingest::Limits;
use recovery_core::{build_ir_from_bytes, export_ir_json, import_ir_json, InspectOptions};

fn run_pipeline(bytes: &[u8]) {
    if let Ok(model) = build_ir_from_bytes(bytes, &InspectOptions::default()) {
        let _ = export_ir_json(&model, true);
        let _ = export_ir_json(&model, false);
        let _ = live2d_ir::validate_ir(&model);
    }
}

#[test]
fn truncations_and_bad_magic_fail_without_panics() {
    for name in fixture_gen::FIXTURE_NAMES {
        let bytes = support::read_fixture(name);
        let step = (bytes.len() / 37).max(1);
        let mut cut = step;
        while cut < bytes.len() {
            let truncated = bytes.get(..cut).unwrap();
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                build_ir_from_bytes(truncated, &InspectOptions::default()).is_err()
            }));
            match outcome {
                Ok(is_err) => assert!(is_err, "truncated {name} at {cut} unexpectedly mapped"),
                Err(_) => panic!("panic while mapping truncated {name} at {cut}"),
            }
            cut += step;
        }
    }
    let mut bad_magic = support::read_fixture("fixture-001-single-artmesh.moc3");
    bad_magic[0] = b'X';
    assert!(build_ir_from_bytes(&bad_magic, &InspectOptions::default()).is_err());
}

#[test]
fn byte_flips_never_panic_through_the_ir_pipeline() {
    for name in fixture_gen::FIXTURE_NAMES {
        let bytes = support::read_fixture(name);
        let stride = if name.starts_with("fixture-001") {
            1
        } else {
            (bytes.len() / 120).max(1)
        };
        let mut offset = 0usize;
        while offset < bytes.len() {
            let mut mutated = bytes.clone();
            if let Some(byte) = mutated.get_mut(offset) {
                *byte ^= 0xFF;
            }
            let outcome = catch_unwind(AssertUnwindSafe(|| run_pipeline(&mutated)));
            if outcome.is_err() {
                panic!("panic in IR pipeline for {name} with byte {offset} flipped");
            }
            offset += stride;
        }
    }
}

#[test]
fn random_inputs_never_panic() {
    for (seed, len) in [(0u64, 1024usize), (1, 4096), (2, 65536)] {
        let bytes = support::lcg_bytes(len, 0xDEAD_BEEF_0000_0000 + seed);
        let outcome = catch_unwind(AssertUnwindSafe(|| run_pipeline(&bytes)));
        assert!(outcome.is_ok(), "panic on random input of length {len}");
    }
}

#[test]
fn ir_validation_surfaces_blank_corruption() {
    // Non-finite parameter default: parses, then fails IR validation.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let default_offset = support::section_offset(&bytes, "parameter.default");
    support::write_f32(&mut bytes, default_offset, f32::NAN);
    let model = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    assert!(has_fatal(&model.diagnostics));
    assert!(export_ir_json(&model, true).is_err());

    // Reversed parameter range: parses, then fails IR validation.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let minimum_offset = support::section_offset(&bytes, "parameter.minimum");
    support::write_f32(&mut bytes, minimum_offset, 100.0);
    let model = build_ir_from_bytes(&bytes, &InspectOptions::default()).unwrap();
    assert!(has_fatal(&model.diagnostics));
}

#[test]
fn malformed_ir_documents_are_rejected_structurally() {
    // Syntax error.
    let error = import_ir_json("{\"schema\":").unwrap_err();
    assert_eq!(error.code, "JsonSyntax");

    // Schema mismatch.
    let fixture = support::fixtures_dir().join("fixture-002-artmesh-param.moc3");
    let model = recovery_core::build_ir_from_file(&fixture, &InspectOptions::default()).unwrap();
    let json = export_ir_json(&model, false).unwrap();
    let tampered = json.replace("\"live2d-ir/1\"", "\"live2d-ir/7\"");
    let error = import_ir_json(&tampered).unwrap_err();
    assert_eq!(error.code, "SchemaMismatch");

    // Dangling reference: import succeeds, validation reports it as fatal.
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    if let Some(mesh) = value
        .get_mut("art_meshes")
        .and_then(|meshes| meshes.get_mut(0))
    {
        mesh["parent_part"] = serde_json::Value::String("DoesNotExist".to_string());
    }
    let dangling = serde_json::to_string(&value).unwrap();
    let imported = import_ir_json(&dangling).unwrap();
    let diagnostics = live2d_ir::validate_ir(&imported);
    assert!(has_fatal(&diagnostics));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "dangling_reference"));
}

#[test]
fn limit_violations_are_enforced_through_the_ir_path() {
    let bytes = support::read_fixture("fixture-007-multi-params.moc3");
    let limits = Limits {
        max_parts: 0,
        ..Limits::default()
    };
    let error = moc3_ingest::parse_full(&bytes, &limits).unwrap_err();
    assert_eq!(error.code(), "LimitExceeded");
}
