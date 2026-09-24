//! End-to-end CLI tests for `recover` and `inspect-cmo3` (AGENT.5.2 Gate 5I).
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

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("liver2d-recover-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_textures(dir: &Path, pages: usize, width: u32, height: u32) {
    for page in 0..pages {
        let bytes = cmo3_writer::png::solid_png(width, height, [(page as u8) * 50, 80, 140, 255]);
        std::fs::write(dir.join(format!("page{page}.png")), bytes).unwrap();
    }
}

#[test]
fn recover_writes_cmo3_and_report() {
    let dir = temp_dir("basic");
    write_textures(&dir, 1, 800, 600);
    let output = dir.join("recovered.cmo3");
    let report = dir.join("recovered.report.json");
    let result = run(&[
        "recover",
        fixtures_dir()
            .join("fixture-002-artmesh-param.moc3")
            .to_str()
            .unwrap(),
        "--textures",
        dir.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--report",
        report.to_str().unwrap(),
    ]);
    assert!(result.status.success(), "{result:?}");
    assert!(output.exists());
    assert!(report.exists());
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("Structural validation:"));
    assert!(text.contains("PASS"));
    assert!(text.contains("Cubism validation:"));
    assert!(text.contains("NOT TESTED"));
    assert!(!text.to_lowercase().contains("cubism compatible"));
    let report_value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert_eq!(
        report_value
            .pointer("/validation/structural")
            .and_then(|v| v.as_str()),
        Some("PASS")
    );
    assert_eq!(
        report_value
            .pointer("/validation/cubism")
            .and_then(|v| v.as_str()),
        Some("NOT_TESTED")
    );
}

#[test]
fn recover_refuses_overwrite_without_force_and_is_deterministic() {
    let dir = temp_dir("force");
    write_textures(&dir, 1, 800, 600);
    let output = dir.join("out.cmo3");
    let moc = fixtures_dir().join("fixture-002-artmesh-param.moc3");
    let args = [
        "recover",
        moc.to_str().unwrap(),
        "--textures",
        dir.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ];
    let first = run(&args);
    assert!(first.status.success());
    let first_bytes = std::fs::read(&output).unwrap();
    let second = run(&args);
    assert!(!second.status.success());
    let stderr = String::from_utf8(second.stderr).unwrap();
    assert!(stderr.contains("already exists"), "{stderr}");

    let mut forced = args.to_vec();
    forced.push("--force");
    let third = run(&forced);
    assert!(third.status.success());
    assert_eq!(std::fs::read(&output).unwrap(), first_bytes);
}

#[test]
fn inspect_cmo3_json_reports_entities() {
    let dir = temp_dir("inspect");
    write_textures(&dir, 1, 800, 600);
    let output = dir.join("out.cmo3");
    let result = run(&[
        "recover",
        fixtures_dir()
            .join("fixture-002-artmesh-param.moc3")
            .to_str()
            .unwrap(),
        "--textures",
        dir.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(result.status.success());
    let inspected = run(&["inspect-cmo3", output.to_str().unwrap(), "--json"]);
    assert!(inspected.status.success());
    let value: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert_eq!(
        value.pointer("/caff/valid").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        value.pointer("/xml/dangling_refs").and_then(|v| v.as_u64()),
        Some(0)
    );
    assert_eq!(
        value
            .pointer("/entities/art_meshes")
            .and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(
        value
            .pointer("/entities/parameters")
            .and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(
        value
            .pointer("/textures")
            .and_then(|v| v.as_array())
            .map(|list| list.len()),
        Some(1)
    );
}

#[test]
fn recover_best_effort_labels_unresolved_grids() {
    let dir = temp_dir("best-effort");
    write_textures(&dir, 1, 800, 600);
    let output = dir.join("be.cmo3");
    let result = run(&[
        "recover",
        fixtures_dir()
            .join("keyform-014-cardinality-mismatch.moc3")
            .to_str()
            .unwrap(),
        "--textures",
        dir.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--best-effort",
    ]);
    assert!(result.status.success(), "{result:?}");
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("BEST_EFFORT_OUTPUT"), "{text}");

    let strict = run(&[
        "recover",
        fixtures_dir()
            .join("keyform-014-cardinality-mismatch.moc3")
            .to_str()
            .unwrap(),
        "--textures",
        dir.to_str().unwrap(),
        "--output",
        dir.join("strict.cmo3").to_str().unwrap(),
    ]);
    assert!(!strict.status.success());
}

#[test]
fn inputs_are_never_modified() {
    let dir = temp_dir("immutable");
    write_textures(&dir, 1, 800, 600);
    let moc = fixtures_dir().join("fixture-002-artmesh-param.moc3");
    let moc_before = std::fs::read(&moc).unwrap();
    let texture_before = std::fs::read(dir.join("page0.png")).unwrap();
    let result = run(&[
        "recover",
        moc.to_str().unwrap(),
        "--textures",
        dir.to_str().unwrap(),
        "--output",
        dir.join("out.cmo3").to_str().unwrap(),
    ]);
    assert!(result.status.success());
    assert_eq!(std::fs::read(&moc).unwrap(), moc_before);
    assert_eq!(
        std::fs::read(dir.join("page0.png")).unwrap(),
        texture_before
    );
}

#[test]
fn version_and_self_test_work() {
    let version = run(&["--version"]);
    assert!(version.status.success());
    let text = String::from_utf8(version.stdout).unwrap();
    assert!(text.contains("0.1.0-alpha"), "{text}");
    assert!(text.contains("commit"), "long version expected: {text}");

    let self_test = run(&["self-test"]);
    assert!(self_test.status.success(), "{self_test:?}");
    let text = String::from_utf8(self_test.stdout).unwrap();
    assert!(text.contains("SELF TEST PASS"), "{text}");
}

#[test]
fn unicode_and_space_paths_are_supported() {
    let dir = std::env::temp_dir().join("Live2D 복구 (test) 1.0");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("textures")).unwrap();
    write_textures(&dir.join("textures"), 1, 800, 600);
    let moc = dir.join("my model (v1.0.0).moc3");
    std::fs::copy(fixtures_dir().join("fixture-002-artmesh-param.moc3"), &moc).unwrap();
    let output = dir.join("recovered (draft).cmo3");
    let result = run(&[
        "recover",
        moc.to_str().unwrap(),
        "--textures",
        dir.join("textures").to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(result.status.success(), "{result:?}");
    assert!(output.exists());
}

#[test]
#[allow(clippy::permissions_set_readonly_false)]
fn read_only_source_files_are_supported() {
    let dir = temp_dir("readonly");
    write_textures(&dir, 1, 800, 600);
    let moc = dir.join("model.moc3");
    std::fs::copy(fixtures_dir().join("fixture-002-artmesh-param.moc3"), &moc).unwrap();
    let mut permissions = std::fs::metadata(&moc).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&moc, permissions).unwrap();
    let before = std::fs::read(&moc).unwrap();
    let output = dir.join("out.cmo3");
    let result = run(&[
        "recover",
        moc.to_str().unwrap(),
        "--textures",
        dir.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(std::fs::read(&moc).unwrap(), before);
    let mut permissions = std::fs::metadata(&moc).unwrap().permissions();
    permissions.set_readonly(false);
    let _ = std::fs::set_permissions(&moc, permissions);
}
