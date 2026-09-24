//! End-to-end CLI tests for `export-ir` and `validate-ir`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use moc3_ingest::counts::parse_count_info;
use moc3_ingest::table;
use moc3_ingest::{ByteOrder, ByteView, MocVersion};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic")
}

fn expected_ir_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/expected-ir")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_recovery"))
        .args(args)
        .output()
        .expect("failed to run recovery binary")
}

fn section_offset(bytes: &[u8], target: &str) -> u64 {
    let view = ByteView::new(bytes, ByteOrder::Little);
    let version = MocVersion::from_byte(*bytes.get(4).unwrap()).unwrap();
    let count_info_offset = u64::from(view.u32_at(0x40, "count_info").unwrap());
    let _ = parse_count_info(&view, version, count_info_offset, bytes.len() as u64).unwrap();
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

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("liver2d-{name}"))
}

#[test]
fn export_ir_stdout_is_deterministic_and_valid_json() {
    let path = fixtures_dir().join("fixture-007-multi-params.moc3");
    let first = run(&["export-ir", path.to_str().unwrap()]);
    let second = run(&["export-ir", path.to_str().unwrap()]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(
        first.stdout, second.stdout,
        "export output is not deterministic"
    );
    let value: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(
        value.get("schema").and_then(|schema| schema.as_str()),
        Some("live2d-ir/1")
    );
    assert_eq!(
        value
            .get("schema_status")
            .and_then(|status| status.as_str()),
        Some("experimental")
    );
}

#[test]
fn export_ir_output_flag_matches_stdout() {
    let path = fixtures_dir().join("fixture-002-artmesh-param.moc3");
    let stdout_run = run(&["export-ir", path.to_str().unwrap()]);
    assert!(stdout_run.status.success());
    let target = temp_path("export-ir-output.json");
    let file_run = run(&[
        "export-ir",
        path.to_str().unwrap(),
        "--output",
        target.to_str().unwrap(),
    ]);
    assert!(file_run.status.success());
    let file_bytes = std::fs::read(&target).unwrap();
    let _ = std::fs::remove_file(&target);
    assert_eq!(file_bytes, stdout_run.stdout);
    assert!(file_bytes.ends_with(b"\n"));
}

#[test]
fn export_ir_compact_is_single_line() {
    let path = fixtures_dir().join("fixture-001-single-artmesh.moc3");
    let output = run(&["export-ir", path.to_str().unwrap(), "--compact"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        !text.trim_end().contains('\n'),
        "compact output has newlines"
    );
    let _: serde_json::Value = serde_json::from_str(&text).unwrap();
}

#[test]
fn export_ir_matches_golden_document() {
    for name in fixture_gen::FIXTURE_NAMES {
        if fixture_gen::GOLDEN_SKIP.contains(name) {
            continue;
        }
        let path = fixtures_dir().join(name);
        let output = run(&["export-ir", path.to_str().unwrap()]);
        assert!(output.status.success());
        let golden = std::fs::read(expected_ir_dir().join(format!("{name}.ir.json"))).unwrap();
        assert_eq!(
            output.stdout, golden,
            "CLI export differs from golden for {name}"
        );
    }
}

#[test]
fn export_ir_rejects_corrupt_input() {
    let mut bytes = std::fs::read(fixtures_dir().join("fixture-001-single-artmesh.moc3")).unwrap();
    bytes.truncate(bytes.len() / 2);
    let path = temp_path("export-ir-corrupt.moc3");
    std::fs::write(&path, &bytes).unwrap();
    let output = run(&["export-ir", path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(output.status.code(), Some(1));
    // Machine mode (stdout) emits a structured JSON error.
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value
        .get("error")
        .and_then(|error| error.get("code"))
        .and_then(|code| code.as_str())
        .is_some());
}

#[test]
fn export_ir_fails_structured_on_ir_validation_errors() {
    let mut bytes = std::fs::read(fixtures_dir().join("fixture-002-artmesh-param.moc3")).unwrap();
    let minimum_offset = section_offset(&bytes, "parameter.minimum");
    let at = usize::try_from(minimum_offset).unwrap();
    bytes
        .get_mut(at..at + 4)
        .unwrap()
        .copy_from_slice(&100.0f32.to_le_bytes());
    let path = temp_path("export-ir-invalid-range.moc3");
    std::fs::write(&path, &bytes).unwrap();
    let output = run(&["export-ir", path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(|code| code.as_str()),
        Some("IrValidationFailed")
    );
    assert!(value
        .get("error")
        .and_then(|error| error.get("diagnostics"))
        .and_then(|diagnostics| diagnostics.as_array())
        .map(|diagnostics| !diagnostics.is_empty())
        .unwrap_or(false));
}

#[test]
fn validate_ir_accepts_golden_documents() {
    let golden = expected_ir_dir().join("fixture-003-warp.moc3.ir.json");
    let output = run(&["validate-ir", golden.to_str().unwrap(), "--json"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value.get("valid").and_then(|valid| valid.as_bool()),
        Some(true)
    );
}

#[test]
fn validate_ir_rejects_malformed_documents() {
    let bad_syntax = temp_path("bad-syntax.ir.json");
    std::fs::write(&bad_syntax, "{\"schema\": \"live2d-ir/1\", ").unwrap();
    let output = run(&["validate-ir", bad_syntax.to_str().unwrap(), "--json"]);
    let _ = std::fs::remove_file(&bad_syntax);
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(|code| code.as_str()),
        Some("JsonSyntax")
    );

    let bad_schema = temp_path("bad-schema.ir.json");
    std::fs::write(&bad_schema, "{\"schema\": \"live2d-ir/99\"}").unwrap();
    let output = run(&["validate-ir", bad_schema.to_str().unwrap(), "--json"]);
    let _ = std::fs::remove_file(&bad_schema);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn export_ir_does_not_modify_input() {
    let path = fixtures_dir().join("fixture-005-rotation.moc3");
    let before = std::fs::read(&path).unwrap();
    let output = run(&["export-ir", path.to_str().unwrap()]);
    let after = std::fs::read(&path).unwrap();
    assert!(output.status.success());
    assert_eq!(before, after);
}
