//! End-to-end CLI tests for `reconstruct-hierarchy`.
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
    std::env::temp_dir().join(format!("liver2d-hier-{name}"))
}

fn node_status(json_stdout: &[u8], node_id: &str) -> String {
    let value: serde_json::Value = serde_json::from_slice(json_stdout).unwrap();
    let nodes = value
        .get("nodes")
        .and_then(|nodes| nodes.as_array())
        .unwrap();
    nodes
        .iter()
        .find(|node| node.get("id").and_then(|id| id.as_str()) == Some(node_id))
        .and_then(|node| node.get("status"))
        .and_then(|status| status.as_str())
        .unwrap_or_else(|| panic!("node {node_id} without status"))
        .to_string()
}

#[test]
fn human_tree_output_contains_markers() {
    let path = fixtures_dir().join("hierarchy-002-part-warp-artmesh.moc3");
    let output = run(&["reconstruct-hierarchy", path.to_str().unwrap()]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for needle in [
        "RecoveredRoot (synthetic)",
        "[Exact]",
        "Hierarchy Resolution Statistics",
        "Note:",
    ] {
        assert!(
            text.contains(needle),
            "missing '{needle}' in report:\n{text}"
        );
    }
}

#[test]
fn json_output_is_deterministic() {
    let path = fixtures_dir().join("hierarchy-009-cycle.moc3");
    let first = run(&["reconstruct-hierarchy", path.to_str().unwrap(), "--json"]);
    let second = run(&["reconstruct-hierarchy", path.to_str().unwrap(), "--json"]);
    assert!(first.status.success());
    assert_eq!(
        first.stdout, second.stdout,
        "hierarchy JSON is not deterministic"
    );
    let value: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(
        value.get("schema").and_then(|schema| schema.as_str()),
        Some("live2d-recovery/recovered-project/1")
    );
    assert_eq!(
        node_status(&first.stdout, "WarpDeformer_Synthetic_00"),
        "cycle_detected"
    );
}

#[test]
fn output_file_matches_json_stdout() {
    let path = fixtures_dir().join("hierarchy-006-multiple-parts.moc3");
    let stdout_run = run(&["reconstruct-hierarchy", path.to_str().unwrap(), "--json"]);
    assert!(stdout_run.status.success());
    let target = temp_path("output.json");
    let file_run = run(&[
        "reconstruct-hierarchy",
        path.to_str().unwrap(),
        "--output",
        target.to_str().unwrap(),
    ]);
    assert!(file_run.status.success());
    let file_bytes = std::fs::read(&target).unwrap();
    let _ = std::fs::remove_file(&target);
    assert_eq!(file_bytes, stdout_run.stdout);
}

#[test]
fn strict_mode_fails_on_cycle() {
    let cycle = fixtures_dir().join("hierarchy-009-cycle.moc3");
    let output = run(&["reconstruct-hierarchy", cycle.to_str().unwrap(), "--strict"]);
    assert_eq!(output.status.code(), Some(1));

    let clean = fixtures_dir().join("hierarchy-001-part-artmesh.moc3");
    let output = run(&["reconstruct-hierarchy", clean.to_str().unwrap(), "--strict"]);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn heuristic_flag_changes_resolution() {
    let path = fixtures_dir().join("hierarchy-008-ambiguous-parent.moc3");
    let conservative = run(&["reconstruct-hierarchy", path.to_str().unwrap(), "--json"]);
    assert!(conservative.status.success());
    assert_eq!(
        node_status(&conservative.stdout, "ArtMesh_Orphan"),
        "unresolved"
    );

    let heuristic = run(&[
        "reconstruct-hierarchy",
        path.to_str().unwrap(),
        "--json",
        "--allow-heuristic",
    ]);
    assert!(heuristic.status.success());
    assert_eq!(
        node_status(&heuristic.stdout, "ArtMesh_Orphan"),
        "ambiguous"
    );
    let value: serde_json::Value = serde_json::from_slice(&heuristic.stdout).unwrap();
    assert!(value
        .get("diagnostics")
        .and_then(|diagnostics| diagnostics.as_array())
        .unwrap()
        .iter()
        .any(
            |diagnostic| diagnostic.get("code").and_then(|code| code.as_str())
                == Some("ambiguous_parent")
        ));

    let strict = run(&[
        "reconstruct-hierarchy",
        path.to_str().unwrap(),
        "--allow-heuristic",
        "--strict",
    ]);
    assert_eq!(strict.status.code(), Some(1));
}

#[test]
fn explain_mode_reports_rules() {
    let path = fixtures_dir().join("hierarchy-002-part-warp-artmesh.moc3");
    let output = run(&[
        "reconstruct-hierarchy",
        path.to_str().unwrap(),
        "--explain",
        "ArtMesh_Synthetic_00",
    ]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Selected parent:"), "{text}");
    assert!(text.contains("HR-005"), "{text}");

    let missing = run(&[
        "reconstruct-hierarchy",
        path.to_str().unwrap(),
        "--explain",
        "does-not-exist",
        "--json",
    ]);
    assert_eq!(missing.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(
        value
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(|code| code.as_str()),
        Some("UnknownNode")
    );
}

#[test]
fn accepts_ir_json_input() {
    let moc3 = fixtures_dir().join("hierarchy-004-nested-warp.moc3");
    let ir_target = temp_path("input.ir.json");
    let export = run(&[
        "export-ir",
        moc3.to_str().unwrap(),
        "--output",
        ir_target.to_str().unwrap(),
    ]);
    assert!(export.status.success());

    let from_moc3 = run(&["reconstruct-hierarchy", moc3.to_str().unwrap(), "--json"]);
    let from_ir = run(&[
        "reconstruct-hierarchy",
        ir_target.to_str().unwrap(),
        "--json",
    ]);
    let _ = std::fs::remove_file(&ir_target);
    assert!(
        from_ir.status.success(),
        "{}",
        String::from_utf8_lossy(&from_ir.stderr)
    );
    assert_eq!(
        from_moc3.stdout, from_ir.stdout,
        "moc3 and IR inputs must agree"
    );
}

#[test]
fn fatal_hierarchy_findings_block_export() {
    // Duplicate a node id in a canonical IR document: the project then has
    // fatal validator findings and must not be exported.
    let moc3 = fixtures_dir().join("hierarchy-001-part-artmesh.moc3");
    let ir_target = temp_path("tampered.ir.json");
    let export = run(&[
        "export-ir",
        moc3.to_str().unwrap(),
        "--output",
        ir_target.to_str().unwrap(),
    ]);
    assert!(export.status.success());
    let text = std::fs::read_to_string(&ir_target).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    if let Some(meshes) = value.get_mut("art_meshes").and_then(|v| v.as_array_mut()) {
        if let Some(first) = meshes.first().cloned() {
            meshes.push(first);
        }
    }
    let tampered = temp_path("tampered2.ir.json");
    std::fs::write(&tampered, serde_json::to_string(&value).unwrap()).unwrap();

    let out_target = temp_path("should-not-exist.project.json");
    let _ = std::fs::remove_file(&out_target);
    let output = run(&[
        "reconstruct-hierarchy",
        tampered.to_str().unwrap(),
        "--output",
        out_target.to_str().unwrap(),
        "--json",
    ]);
    let _ = std::fs::remove_file(&ir_target);
    let _ = std::fs::remove_file(&tampered);
    assert_eq!(output.status.code(), Some(1), "fatal project must fail");
    assert!(!out_target.exists(), "rejected project must not be written");
    let _ = std::fs::remove_file(&out_target);
}

#[test]
fn strict_mode_does_not_write_rejected_output() {
    let cycle = fixtures_dir().join("hierarchy-009-cycle.moc3");
    let target = temp_path("strict-should-not-exist.project.json");
    let _ = std::fs::remove_file(&target);
    let output = run(&[
        "reconstruct-hierarchy",
        cycle.to_str().unwrap(),
        "--strict",
        "--output",
        target.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        !target.exists(),
        "strict-rejected output must not be written"
    );
}

#[test]
fn ir_json_with_utf8_bom_is_accepted() {
    let moc3 = fixtures_dir().join("hierarchy-003-part-rotation-artmesh.moc3");
    let ir_target = temp_path("bom.ir.json");
    let export = run(&[
        "export-ir",
        moc3.to_str().unwrap(),
        "--output",
        ir_target.to_str().unwrap(),
    ]);
    assert!(export.status.success());
    let mut bytes = Vec::from([0xEFu8, 0xBB, 0xBF]);
    bytes.extend_from_slice(&std::fs::read(&ir_target).unwrap());
    let bom_target = temp_path("bom-with-bom.ir.json");
    std::fs::write(&bom_target, &bytes).unwrap();
    let output = run(&["reconstruct-hierarchy", bom_target.to_str().unwrap()]);
    let _ = std::fs::remove_file(&ir_target);
    let _ = std::fs::remove_file(&bom_target);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn input_is_not_modified_and_corrupt_input_is_rejected() {
    let path = fixtures_dir().join("hierarchy-012-large-flat.moc3");
    let before = std::fs::read(&path).unwrap();
    let output = run(&["reconstruct-hierarchy", path.to_str().unwrap()]);
    let after = std::fs::read(&path).unwrap();
    assert!(output.status.success());
    assert_eq!(before, after);

    let mut bytes = std::fs::read(fixtures_dir().join("hierarchy-001-part-artmesh.moc3")).unwrap();
    bytes.truncate(bytes.len() / 2);
    let corrupt = temp_path("corrupt.moc3");
    std::fs::write(&corrupt, &bytes).unwrap();
    let output = run(&["reconstruct-hierarchy", corrupt.to_str().unwrap(), "--json"]);
    let _ = std::fs::remove_file(&corrupt);
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value.get("error").is_some());
}
