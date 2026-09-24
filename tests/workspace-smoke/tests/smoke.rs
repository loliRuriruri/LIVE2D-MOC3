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
