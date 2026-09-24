//! Differential harness tests (offline; external tools are simulated).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use reference_harness::compare::{compare, ConsensusVerdict, FieldStatus, ProviderSummary};
use reference_harness::providers::{
    mock_snapshot, provider_by_name, run_command_capped, MockMutation, ProviderConfig,
    ProviderError,
};
use reference_harness::snapshot::{floats_equal, DifferentialSnapshot};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic")
}

fn fixture(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

fn ours_snapshot(name: &str) -> (DifferentialSnapshot, Vec<u8>, PathBuf) {
    let path = fixture(name);
    let bytes = std::fs::read(&path).unwrap();
    let provider = provider_by_name("ours").unwrap();
    let snapshot = provider
        .snapshot(&path, &bytes, &ProviderConfig::default())
        .unwrap();
    (snapshot, bytes, path)
}

#[test]
fn adapter_parses_our_fixture() {
    let (snapshot, _, _) = ours_snapshot("fixture-002-artmesh-param.moc3");
    assert_eq!(snapshot.moc3_version, Some(2));
    assert_eq!(snapshot.endian.as_deref(), Some("little"));
    assert_eq!(snapshot.parameters.len(), 1);
    assert_eq!(snapshot.parameters[0].id, "ParamSynthetic_AngleX");
    assert_eq!(snapshot.parts.len(), 1);
    assert_eq!(snapshot.art_meshes.len(), 1);
    assert_eq!(
        snapshot.art_meshes[0].parent_part.as_deref(),
        Some("Part_Synthetic_00")
    );
    assert_eq!(snapshot.counts.get("parts"), Some(&1));
    assert_eq!(snapshot.hierarchy_edges.len(), 1);
}

#[test]
fn snapshot_serialization_is_deterministic() {
    let (snapshot, _, _) = ours_snapshot("fixture-007-multi-params.moc3");
    let first = serde_json::to_string(&snapshot).unwrap();
    let second = serde_json::to_string(&snapshot).unwrap();
    assert_eq!(first, second);
    let mut again = snapshot.clone();
    again.canonicalize();
    assert_eq!(snapshot, again, "canonicalize must be idempotent");
}

fn summary(name: &str) -> ProviderSummary {
    match provider_by_name(name) {
        Some(provider) => ProviderSummary {
            name: name.to_string(),
            metadata: provider.metadata(),
            outcome: "ok".to_string(),
            error: None,
        },
        // Mock providers only exist inside tests.
        None => ProviderSummary {
            name: name.to_string(),
            metadata: reference_harness::providers::ReferenceMetadata {
                name: name.to_string(),
                kind: "mock".to_string(),
                version: "test".to_string(),
                commit: "test".to_string(),
                license: "test".to_string(),
                repository: "test".to_string(),
                production_dependency: false,
            },
            outcome: "ok".to_string(),
            error: None,
        },
    }
}

fn compare_with_mocks(
    ours: &DifferentialSnapshot,
    mocks: &[(&str, MockMutation)],
) -> reference_harness::compare::ComparisonReport {
    let snapshots: Vec<DifferentialSnapshot> = mocks
        .iter()
        .map(|(name, mutation)| mock_snapshot(ours, name, *mutation))
        .collect();
    let mut summaries: Vec<ProviderSummary> = vec![summary("ours")];
    summaries.extend(mocks.iter().map(|(name, _)| summary(name)));
    let references: Vec<(&str, Option<&DifferentialSnapshot>)> = mocks
        .iter()
        .zip(snapshots.iter())
        .map(|((name, _), snapshot)| (*name, Some(snapshot)))
        .collect();
    compare(&ours.input, "test", Some(ours), &summaries, &references)
}

fn field<'a>(
    report: &'a reference_harness::compare::ComparisonReport,
    path: &str,
) -> &'a reference_harness::compare::FieldComparison {
    report
        .fields
        .iter()
        .find(|field| field.field == path)
        .unwrap_or_else(|| panic!("field {path} missing"))
}

#[test]
fn diff_match_when_providers_agree() {
    let (ours, _, _) = ours_snapshot("fixture-002-artmesh-param.moc3");
    let report = compare_with_mocks(&ours, &[("mock-agree", MockMutation::Agree)]);
    assert_eq!(report.statistics.disagreement, 0);
    assert!(report.disputed.is_empty());
    assert_eq!(field(&report, "parameters.ids").status, FieldStatus::Match);
}

#[test]
fn diff_mismatch_reports_values() {
    let (ours, _, _) = ours_snapshot("fixture-002-artmesh-param.moc3");
    let report = compare_with_mocks(
        &ours,
        &[(
            "mock-mismatch",
            MockMutation::FlipParameterMaximum { index: 0 },
        )],
    );
    let entry = field(&report, "parameters.ParamSynthetic_AngleX.maximum");
    assert_eq!(entry.status, FieldStatus::Mismatch);
    assert_eq!(entry.consensus, ConsensusVerdict::Disputed);
    assert!(!entry.details.is_empty());
    assert!(report
        .disputed
        .contains(&"parameters.ParamSynthetic_AngleX.maximum".to_string()));
}

#[test]
fn missing_reference_field_is_reported() {
    let (ours, _, _) = ours_snapshot("fixture-002-artmesh-param.moc3");
    let report = compare_with_mocks(&ours, &[("mock-missing", MockMutation::DropLastPart)]);
    assert_eq!(field(&report, "parts.ids").status, FieldStatus::Mismatch);
    let report = compare_with_mocks(&ours, &[("mock-missing", MockMutation::UnsupportedSection)]);
    assert_eq!(
        field(&report, "parameters.ids").status,
        FieldStatus::UnsupportedReference
    );
}

#[test]
fn order_only_difference_is_classified() {
    let (ours, _, _) = ours_snapshot("fixture-007-multi-params.moc3");
    let report = compare_with_mocks(&ours, &[("mock-order", MockMutation::ReverseParameters)]);
    let entry = field(&report, "parameters.ids");
    assert_eq!(entry.status, FieldStatus::OrderOnlyDifference);
    assert_eq!(report.statistics.disagreement, 0);
    assert_eq!(report.statistics.order_only, 1);
    assert_eq!(entry.consensus, ConsensusVerdict::Consensus);
}

#[test]
fn float_tolerance_policy() {
    assert!(floats_equal(1.0, 1.0 + 1e-9));
    assert!(!floats_equal(1.0, 1.1));
    let (ours, _, _) = ours_snapshot("fixture-002-artmesh-param.moc3");
    let report = compare_with_mocks(
        &ours,
        &[("mock-drift", MockMutation::TinyFloatDrift { index: 0 })],
    );
    assert_eq!(
        field(&report, "parameters.ParamSynthetic_AngleX.default").status,
        FieldStatus::Match
    );
}

#[test]
fn multi_reference_consensus_and_dispute() {
    let (ours, _, _) = ours_snapshot("fixture-002-artmesh-param.moc3");
    let consensus = compare_with_mocks(
        &ours,
        &[
            ("mock-a", MockMutation::Agree),
            ("mock-b", MockMutation::Agree),
        ],
    );
    assert_eq!(
        field(&consensus, "parameters.ids").consensus,
        ConsensusVerdict::Consensus
    );
    let disputed = compare_with_mocks(
        &ours,
        &[
            ("mock-a", MockMutation::Agree),
            ("mock-b", MockMutation::FlipParameterMaximum { index: 0 }),
        ],
    );
    assert_eq!(
        field(&disputed, "parameters.ParamSynthetic_AngleX.maximum").consensus,
        ConsensusVerdict::Disputed
    );
    assert!(!disputed.disputed.is_empty());
}

#[test]
fn provider_metadata_is_pinned() {
    let references = reference_harness::pinned_references();
    let moc2cmo = references
        .iter()
        .find(|reference| reference.name == "moc2cmo")
        .unwrap();
    assert_eq!(moc2cmo.version, "1.1.18");
    assert_eq!(moc2cmo.commit, "2527e24e93");
    assert_eq!(moc2cmo.license, "MIT");
    let py = references
        .iter()
        .find(|reference| reference.name == "py-moc3")
        .unwrap();
    assert_eq!(py.commit, "2fb112e11a");
    for reference in &references {
        assert!(
            !reference.production_dependency,
            "harness references must never be production dependencies"
        );
    }
}

#[test]
fn unknown_provider_is_recorded_not_fatal() {
    let path = fixture("fixture-001-single-artmesh.moc3");
    let config = ProviderConfig::default();
    let report = reference_harness::run_compare(
        &path,
        &["ours".to_string(), "does-not-exist".to_string()],
        &config,
        "test",
    )
    .unwrap();
    let unknown = report
        .providers
        .iter()
        .find(|provider| provider.name == "does-not-exist")
        .unwrap();
    assert_eq!(unknown.outcome, "UNKNOWN_PROVIDER");
}

#[test]
fn planned_providers_report_not_configured() {
    let path = fixture("fixture-001-single-artmesh.moc3");
    let bytes = std::fs::read(&path).unwrap();
    for name in ["moc2cmo", "purismcore", "ayagami"] {
        let provider = provider_by_name(name).unwrap();
        let result = provider.snapshot(&path, &bytes, &ProviderConfig::default());
        match result {
            Err(ProviderError::NotConfigured { .. }) => {}
            other => panic!("{name} expected NotConfigured, got {other:?}"),
        }
    }
}

fn harness_binary() -> &'static str {
    env!("CARGO_BIN_EXE_recovery-reference")
}

#[test]
fn subprocess_crash_timeout_and_output_limit_are_structured() {
    let input = fixture("fixture-001-single-artmesh.moc3");
    // Crash: non-zero exit -> ReferenceError.
    let mut command = Command::new(harness_binary());
    command.args(["mock-provider", "crash", input.to_str().unwrap()]);
    let error = run_command_capped(&mut command, &ProviderConfig::default(), "mock").unwrap_err();
    assert!(matches!(error, ProviderError::ReferenceError { .. }));

    // Timeout: hang beyond 1s -> ReferenceTimeout.
    let mut command = Command::new(harness_binary());
    command.args(["mock-provider", "hang", input.to_str().unwrap()]);
    let config = ProviderConfig {
        timeout: Duration::from_secs(1),
        output_limit: 8 * 1024 * 1024,
    };
    let error = run_command_capped(&mut command, &config, "mock").unwrap_err();
    assert_eq!(
        error,
        ProviderError::ReferenceTimeout { seconds: 1 },
        "timeout must be structured"
    );

    // Oversize output -> OutputLimitExceeded.
    let mut command = Command::new(harness_binary());
    command.args(["mock-provider", "oversize", input.to_str().unwrap()]);
    let config = ProviderConfig {
        timeout: Duration::from_secs(30),
        output_limit: 64 * 1024,
    };
    let error = run_command_capped(&mut command, &config, "mock").unwrap_err();
    assert_eq!(
        error,
        ProviderError::OutputLimitExceeded { limit: 64 * 1024 }
    );
}

#[test]
fn subprocess_snapshot_roundtrip_via_mock() {
    let input = fixture("fixture-002-artmesh-param.moc3");
    let mut command = Command::new(harness_binary());
    command.args([
        "mock-provider",
        "agree",
        input.to_str().unwrap(),
        "--name",
        "mock-subprocess",
    ]);
    let stdout = run_command_capped(&mut command, &ProviderConfig::default(), "mock").unwrap();
    let snapshot: DifferentialSnapshot = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(snapshot.provider, "mock-subprocess");
    let (ours, _, _) = ours_snapshot("fixture-002-artmesh-param.moc3");
    assert_eq!(snapshot.parameters, ours.parameters);
}

#[test]
fn ours_unsupported_field_is_not_reported_as_missing_ours() {
    let (mut ours, _, _) = ours_snapshot("fixture-002-artmesh-param.moc3");
    ours.unsupported.insert(
        "counts.parameter_bindings".to_string(),
        "not modeled in the IR".to_string(),
    );
    let mut reference = ours.clone();
    reference.unsupported.clear();
    reference.counts.insert("parameter_bindings".to_string(), 1);
    let report = compare(
        &ours.input,
        "test",
        Some(&ours),
        &[summary("ours"), summary("mock")],
        &[("mock", Some(&reference))],
    );
    let entry = field(&report, "counts.parameter_bindings");
    assert_eq!(entry.status, FieldStatus::UnsupportedReference);
}

#[test]
fn report_rendering_is_deterministic() {
    let (ours, _, _) = ours_snapshot("fixture-006-clipping-mask.moc3");
    let report = compare_with_mocks(&ours, &[("mock", MockMutation::Agree)]);
    let first = reference_harness::report::to_json(&report).unwrap();
    let second = reference_harness::report::to_json(&report).unwrap();
    assert_eq!(first, second);
    let human = reference_harness::report::render_human(&report);
    assert!(human.contains("Cross-Implementation Agreement"));
    // The word may only appear inside the explicit disclaimer.
    assert!(human.contains("not a recovery accuracy measurement"));
    let lowered = human.to_lowercase();
    assert!(!lowered.contains("accuracy:"));
    assert!(!lowered.contains("accurate"));
}
