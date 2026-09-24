//! End-to-end CLI tests for `recover-keyforms`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_recovery"))
        .args(args)
        .output()
        .expect("failed to run recovery binary")
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("liver2d-keyforms-{name}"))
}

#[test]
fn human_report_contains_exact_counts() {
    let path = fixtures_dir().join("keyform-003-2d-3x3.moc3");
    let output = run(&["recover-keyforms", path.to_str().unwrap()]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for needle in [
        "Keyform Recovery",
        "Targets             2",
        "Parameter axes      2",
        "Stored keyforms     18",
        "not accuracy measurements",
    ] {
        assert!(text.contains(needle), "missing '{needle}' in:\n{text}");
    }
    assert!(!text.to_lowercase().contains("accuracy:"));
}

#[test]
fn json_output_is_deterministic_and_schema_tagged() {
    let path = fixtures_dir().join("keyform-019-deep-nested-targets.moc3");
    let first = run(&["recover-keyforms", path.to_str().unwrap(), "--json"]);
    let second = run(&["recover-keyforms", path.to_str().unwrap(), "--json"]);
    assert!(first.status.success());
    assert_eq!(
        first.stdout, second.stdout,
        "keyform JSON is not deterministic"
    );
    let value: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(
        value.get("schema").and_then(|schema| schema.as_str()),
        Some("live2d-recovery/recovered-keyforms/1")
    );
    assert_eq!(
        value
            .pointer("/statistics/targets")
            .and_then(|targets| targets.as_u64()),
        Some(6)
    );
}

#[test]
fn output_file_matches_json_stdout() {
    let path = fixtures_dir().join("keyform-002-1d-three-keys.moc3");
    let stdout_run = run(&["recover-keyforms", path.to_str().unwrap(), "--json"]);
    let target = temp_path("002.json");
    let _ = std::fs::remove_file(&target);
    let file_run = run(&[
        "recover-keyforms",
        path.to_str().unwrap(),
        "--output",
        target.to_str().unwrap(),
    ]);
    assert!(file_run.status.success());
    let written = std::fs::read(&target).unwrap();
    assert_eq!(
        written, stdout_run.stdout,
        "--output must equal --json stdout"
    );
}

#[test]
fn ir_json_input_matches_moc3_input() {
    let path = fixtures_dir().join("keyform-007-warp-forms.moc3");
    let ir_path = temp_path("007.ir.json");
    let _ = std::fs::remove_file(&ir_path);
    let export = run(&[
        "export-ir",
        path.to_str().unwrap(),
        "--output",
        ir_path.to_str().unwrap(),
    ]);
    assert!(export.status.success());
    let from_moc3 = run(&["recover-keyforms", path.to_str().unwrap(), "--json"]);
    let from_ir = run(&["recover-keyforms", ir_path.to_str().unwrap(), "--json"]);
    assert!(from_ir.status.success());
    assert_eq!(
        from_moc3.stdout, from_ir.stdout,
        "IR JSON and .moc3 inputs must recover identical keyforms"
    );
}

#[test]
fn strict_mode_fails_on_cardinality_mismatch() {
    let path = fixtures_dir().join("keyform-014-cardinality-mismatch.moc3");
    let lax = run(&["recover-keyforms", path.to_str().unwrap(), "--json"]);
    assert!(lax.status.success());
    let strict = run(&["recover-keyforms", path.to_str().unwrap(), "--strict"]);
    assert!(!strict.status.success());
    let text = String::from_utf8(strict.stdout).unwrap();
    assert!(text.contains("StrictModeViolation"), "stdout:\n{text}");
    assert!(text.contains("keyform_grid_cardinality_mismatch"));
}

#[test]
fn strict_mode_passes_on_clean_fixture() {
    let path = fixtures_dir().join("hierarchy-002-part-warp-artmesh.moc3");
    let output = run(&["recover-keyforms", path.to_str().unwrap(), "--strict"]);
    assert!(output.status.success());
}

#[test]
fn non_finite_keys_are_refused_at_the_cli_by_ir_validation() {
    // NaN/Inf are Fatal IR diagnostics (live2d-ir/1 invariant), so the CLI
    // refuses them before keyform recovery. The keyform layer itself reports
    // them positionally when used as a library on unvalidated IR (see the
    // keyform-recovery crate tests).
    let path = fixtures_dir().join("keyform-016-non-finite-key.moc3");
    let output = run(&["recover-keyforms", path.to_str().unwrap(), "--json"]);
    assert!(!output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("IrValidationFailed"), "stdout:\n{text}");
    assert!(text.contains("non_finite_value"));
}

#[test]
fn explain_renders_trace_and_rejects_unknown_query() {
    let path = fixtures_dir().join("keyform-003-2d-3x3.moc3");
    let found = run(&[
        "recover-keyforms",
        path.to_str().unwrap(),
        "--explain",
        "ArtMesh_Keyform_00",
    ]);
    assert!(found.status.success());
    let text = String::from_utf8(found.stdout).unwrap();
    assert!(text.contains("Target:"));
    assert!(text.contains("Expected Cartesian:"));
    assert!(text.contains("unknown (not guessed)"));

    let missing = run(&[
        "recover-keyforms",
        path.to_str().unwrap(),
        "--explain",
        "nope",
    ]);
    assert!(!missing.status.success());
    let stderr = String::from_utf8(missing.stderr).unwrap();
    assert!(
        stderr.contains("node 'nope' was not found")
            || stderr.contains("UnknownKeyformTarget")
            || stderr.contains("was not found in the recovered keyform model"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn input_moc3_is_never_modified() {
    let path = fixtures_dir().join("keyform-005-3d-small-grid.moc3");
    let before = std::fs::read(&path).unwrap();
    let output = run(&["recover-keyforms", path.to_str().unwrap(), "--json"]);
    assert!(output.status.success());
    let after = std::fs::read(&path).unwrap();
    assert_eq!(before, after, "input file must never be modified");
}
