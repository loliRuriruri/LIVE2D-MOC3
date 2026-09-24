//! End-to-end CLI tests for `recovery inspect`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic")
}

fn expected_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/expected")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_recovery"))
        .args(args)
        .output()
        .expect("failed to run recovery binary")
}

#[test]
fn inspect_json_matches_golden_report() {
    for name in fixture_gen::FIXTURE_NAMES {
        if fixture_gen::GOLDEN_SKIP.contains(name) {
            continue;
        }
        let path = fixtures_dir().join(name);
        let output = run(&["inspect", path.to_str().unwrap(), "--json"]);
        assert!(
            output.status.success(),
            "inspect {name} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected = std::fs::read(expected_dir().join(format!("{name}.report.json")))
            .unwrap_or_else(|error| panic!("missing golden for {name}: {error}"));
        assert_eq!(
            output.stdout, expected,
            "CLI output does not match golden for {name}"
        );
        // Output must be valid JSON.
        let _: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    }
}

#[test]
fn inspect_json_is_deterministic() {
    let path = fixtures_dir().join("fixture-007-multi-params.moc3");
    let first = run(&["inspect", path.to_str().unwrap(), "--json"]);
    let second = run(&["inspect", path.to_str().unwrap(), "--json"]);
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout, "repeated runs differ");
}

#[test]
fn human_report_contains_expected_sections() {
    let path = fixtures_dir().join("fixture-003-warp.moc3");
    let output = run(&["inspect", path.to_str().unwrap()]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for needle in [
        "MOC3 Inspection Report",
        "MOC3 version: 2 (3.3.00-3.3.03)",
        "Parameters (0)",
        "Deformers (1)",
        "WarpDeformer_Synthetic_00",
        "Hierarchy candidates",
        "Anomalies (0)",
    ] {
        assert!(text.contains(needle), "missing '{needle}' in human report");
    }
}

#[test]
fn input_file_is_never_modified() {
    let path = fixtures_dir().join("fixture-006-clipping-mask.moc3");
    let before = std::fs::read(&path).unwrap();
    let output = run(&["inspect", path.to_str().unwrap(), "--json"]);
    let after = std::fs::read(&path).unwrap();
    assert!(output.status.success());
    assert_eq!(before, after, "inspect modified its input file");
}

#[test]
fn corrupt_file_reports_structured_error() {
    let mut bytes = std::fs::read(fixtures_dir().join("fixture-001-single-artmesh.moc3")).unwrap();
    bytes.truncate(bytes.len() / 2);
    let path = std::env::temp_dir().join("liver2d-cli-corrupt-test.moc3");
    std::fs::write(&path, &bytes).unwrap();

    let output = run(&["inspect", path.to_str().unwrap(), "--json"]);
    assert_eq!(output.status.code(), Some(1), "corrupt input must exit 1");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let code = value
        .get("error")
        .and_then(|error| error.get("code"))
        .and_then(|code| code.as_str())
        .unwrap_or("");
    assert!(!code.is_empty(), "error JSON must carry a code");
    assert!(
        value
            .get("error")
            .and_then(|error| error.get("suggestion"))
            .and_then(|value| value.as_str())
            .is_some(),
        "error JSON must carry a suggestion"
    );

    // Human mode prints the structured message to stderr.
    let output = run(&["inspect", path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("suggestion:"));
}

#[test]
fn missing_file_reports_io_error() {
    let output = run(&["inspect", "does-not-exist-anywhere.moc3", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(|code| code.as_str()),
        Some("IoError")
    );
}
