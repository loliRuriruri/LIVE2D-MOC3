//! Workspace-level smoke tests: the checked-in fixtures must parse through
//! the full recovery-core pipeline deterministically.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use recovery_core::{inspect_file, InspectOptions};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic")
}

#[test]
fn every_fixture_inspects_cleanly() {
    for name in fixture_gen::FIXTURE_NAMES {
        let path = fixtures_dir().join(name);
        let report = inspect_file(&path, &InspectOptions::default())
            .unwrap_or_else(|error| panic!("{name} failed: {error}"));
        assert_eq!(report.source.file_name.as_deref(), Some(*name));
        assert_eq!(
            report.summary.counts.parameters as usize,
            report.model.parameters.len()
        );
        assert_eq!(
            report.summary.counts.parts as usize,
            report.model.parts.len()
        );
        assert_eq!(
            report.summary.counts.art_meshes as usize,
            report.model.art_meshes.len()
        );
        assert_eq!(
            report.summary.counts.deformers as usize,
            report.model.deformers.len()
        );
        assert_eq!(
            report.summary.anomaly_count,
            report.model.anomalies.len() + report.hierarchy.anomalies.len()
        );
    }
}

#[test]
fn inspection_is_deterministic_across_runs() {
    for name in fixture_gen::FIXTURE_NAMES {
        let path = fixtures_dir().join(name);
        let first = inspect_file(&path, &InspectOptions::default()).unwrap();
        let second = inspect_file(&path, &InspectOptions::default()).unwrap();
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap(),
            "non-deterministic report for {name}"
        );
    }
}

#[test]
fn checked_in_fixtures_match_the_generator() {
    for name in fixture_gen::FIXTURE_NAMES {
        let on_disk = std::fs::read(fixtures_dir().join(name)).unwrap();
        let generated = fixture_gen::build_named(name).unwrap();
        assert_eq!(on_disk, generated, "fixture {name} is stale");
    }
}

#[test]
fn hierarchy_recovery_is_independent_from_the_binary_parser() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest =
        std::fs::read_to_string(root.join("crates/hierarchy-recovery/Cargo.toml")).unwrap();
    assert!(
        !manifest.contains("moc3-ingest") && !manifest.contains("recovery-core"),
        "hierarchy-recovery/Cargo.toml must not reference the parser or orchestration crates"
    );

    let output = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(&root)
        .output()
        .expect("failed to run cargo metadata");
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let packages = value
        .get("packages")
        .and_then(|packages| packages.as_array())
        .expect("metadata packages");
    let package = packages
        .iter()
        .find(|package| {
            package.get("name").and_then(|name| name.as_str()) == Some("hierarchy-recovery")
        })
        .expect("hierarchy-recovery package in metadata");
    let dependencies: Vec<&str> = package
        .get("dependencies")
        .and_then(|dependencies| dependencies.as_array())
        .map(|dependencies| {
            dependencies
                .iter()
                .filter_map(|dependency| dependency.get("name").and_then(|name| name.as_str()))
                .collect()
        })
        .unwrap_or_default();
    for forbidden in ["moc3-ingest", "recovery-core", "fixture-gen"] {
        assert!(
            !dependencies.contains(&forbidden),
            "hierarchy-recovery must not depend on {forbidden} (declared: {dependencies:?})"
        );
    }
}

#[test]
fn live2d_ir_is_independent_from_the_binary_parser() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    // Static manifest check (fast and hermetic).
    let manifest = std::fs::read_to_string(root.join("crates/live2d-ir/Cargo.toml")).unwrap();
    assert!(
        !manifest.contains("moc3-ingest"),
        "live2d-ir/Cargo.toml must not reference moc3-ingest"
    );

    // Dependency-graph check via cargo metadata (no build, workspace only).
    let output = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(&root)
        .output()
        .expect("failed to run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let packages = value
        .get("packages")
        .and_then(|packages| packages.as_array())
        .expect("metadata packages");
    let live2d_ir = packages
        .iter()
        .find(|package| package.get("name").and_then(|name| name.as_str()) == Some("live2d-ir"))
        .expect("live2d-ir package in metadata");
    let dependencies: Vec<&str> = live2d_ir
        .get("dependencies")
        .and_then(|dependencies| dependencies.as_array())
        .map(|dependencies| {
            dependencies
                .iter()
                .filter_map(|dependency| dependency.get("name").and_then(|name| name.as_str()))
                .collect()
        })
        .unwrap_or_default();
    for forbidden in ["moc3-ingest", "recovery-core", "fixture-gen"] {
        assert!(
            !dependencies.contains(&forbidden),
            "live2d-ir must not depend on {forbidden} (declared: {dependencies:?})"
        );
    }
}
