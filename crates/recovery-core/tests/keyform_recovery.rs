//! AGENT.4 pipeline tests: keyform recovery over the synthetic fixtures.
//!
//! The goldens were generated once and reviewed (fixture definition, rules,
//! expected counts) as part of AGENT.4; they are not automatically
//! authoritative just because the generator wrote them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use hierarchy_recovery::{reconstruct, RecoveryPolicy};
use keyform_recovery::{codes, GridLayout};
use recovery_core::{
    build_ir_from_bytes, build_ir_from_file, export_keyforms_json, import_keyforms_json,
    recover_keyforms, InspectOptions,
};

fn recover_fixture(
    name: &str,
) -> (
    keyform_recovery::RecoveredKeyformModel,
    live2d_ir::Live2DModel,
) {
    let path = support::fixtures_dir().join(name);
    let model = build_ir_from_file(&path, &InspectOptions::default())
        .unwrap_or_else(|error| panic!("{name} failed to map: {error}"));
    let project = reconstruct(&model, &RecoveryPolicy::default());
    let document = recover_keyforms(&model, Some(&project));
    (document, model)
}

#[test]
fn per_fixture_keyform_expectations() {
    for name in fixture_gen::KEYFORM_FIXTURE_NAMES {
        let (document, _) = recover_fixture(name);
        assert!(
            !live2d_ir::has_fatal(&document.diagnostics),
            "{name}: fatal keyform diagnostics"
        );
        match *name {
            "keyform-001-zero-dimensional.moc3" => {
                assert_eq!(document.statistics.targets, 2);
                assert_eq!(document.statistics.stored_keyforms, 2);
                assert_eq!(document.binding_bands[0].axes.len(), 0);
                assert_eq!(document.statistics.dense_grids, 2);
            }
            "keyform-002-1d-three-keys.moc3" => {
                assert_eq!(document.statistics.parameter_axes, 1);
                assert_eq!(document.statistics.stored_keyforms, 6);
                assert_eq!(document.statistics.dense_grids, 2);
            }
            "keyform-003-2d-3x3.moc3" => {
                assert_eq!(document.statistics.parameter_axes, 2);
                assert_eq!(document.statistics.stored_keyforms, 18);
                assert_eq!(
                    document.keyform_grids[0].expected_cardinality,
                    keyform_recovery::Cardinality::Exact { value: 9 }
                );
                assert_eq!(
                    document.keyform_grids[0].ordering,
                    keyform_recovery::GridOrdering::Unknown
                );
            }
            "keyform-004-2d-2x3.moc3" => {
                assert_eq!(document.statistics.stored_keyforms, 12);
                assert_eq!(document.statistics.dense_grids, 2);
            }
            "keyform-005-3d-small-grid.moc3" => {
                assert_eq!(document.statistics.parameter_axes, 3);
                assert_eq!(document.statistics.stored_keyforms, 8);
                assert_eq!(document.statistics.dense_grids, 1);
            }
            "keyform-006-part-draw-order.moc3" => {
                assert_eq!(document.statistics.stored_keyforms, 4);
                assert!(document
                    .target_keyforms
                    .iter()
                    .any(|entry| matches!(entry, keyform_recovery::TargetKeyforms::Part(_))));
            }
            "keyform-007-warp-forms.moc3" => {
                assert_eq!(document.statistics.stored_keyforms, 8);
                assert!(document.target_keyforms.iter().any(|entry| matches!(
                    entry,
                    keyform_recovery::TargetKeyforms::WarpDeformer(_)
                )));
            }
            "keyform-008-rotation-forms.moc3" => {
                assert_eq!(document.statistics.stored_keyforms, 8);
                assert!(document.target_keyforms.iter().any(|entry| matches!(
                    entry,
                    keyform_recovery::TargetKeyforms::RotationDeformer(_)
                )));
            }
            "keyform-009-artmesh-forms.moc3" => {
                assert_eq!(document.statistics.stored_keyforms, 6);
                assert!(document
                    .target_keyforms
                    .iter()
                    .any(|entry| matches!(entry, keyform_recovery::TargetKeyforms::ArtMesh(_))));
            }
            "keyform-010-multiple-bindings.moc3" => {
                assert_eq!(document.statistics.bindings, 2);
                assert_eq!(document.statistics.stored_keyforms, 4);
            }
            "keyform-011-multiple-targets.moc3" => {
                assert_eq!(document.statistics.targets, 2);
                assert_eq!(document.statistics.parameter_axes, 2);
                assert_eq!(document.statistics.stored_keyforms, 8);
            }
            "keyform-012-duplicate-key.moc3" => {
                assert!(document
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == codes::DUPLICATE_PARAMETER_KEY));
                assert_eq!(document.statistics.stored_keyforms, 4);
            }
            "keyform-013-unsorted-keys.moc3" => {
                assert!(document
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == codes::UNSORTED_PARAMETER_KEYS));
                assert_eq!(
                    document.binding_bands[0].axes[0].keys,
                    vec![0.0, -30.0, 30.0]
                );
            }
            "keyform-014-cardinality-mismatch.moc3" => {
                assert_eq!(document.statistics.stored_keyforms, 5);
                assert_eq!(document.statistics.unknown_layout_grids, 1);
                assert_eq!(document.statistics.unresolved_grids, 1);
                assert!(document
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == codes::KEYFORM_GRID_CARDINALITY_MISMATCH));
            }
            "keyform-016-non-finite-key.moc3" => {
                assert_eq!(
                    document.binding_bands[0].axes[0].non_finite_key_indices,
                    vec![1]
                );
                assert!(document
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == codes::NON_FINITE_PARAMETER_KEY));
                // The document must still serialize canonically.
                assert!(export_keyforms_json(&document, false).is_ok());
            }
            "keyform-019-deep-nested-targets.moc3" => {
                assert_eq!(document.statistics.targets, 6);
                assert_eq!(document.statistics.stored_keyforms, 12);
                assert_eq!(document.statistics.dense_grids, 6);
            }
            "keyform-020-large-dataset.moc3" => {
                assert_eq!(document.statistics.stored_keyforms, 64);
                assert_eq!(document.statistics.dense_grids, 1);
            }
            other => panic!("unexpected keyform fixture {other}"),
        }
    }
}

#[test]
fn golden_keyform_documents_match() {
    let update = std::env::var("UPDATE_GOLDEN_KEYFORMS").is_ok();
    std::fs::create_dir_all(support::expected_keyforms_dir()).unwrap();
    for name in fixture_gen::KEYFORM_FIXTURE_NAMES {
        let (document, _) = recover_fixture(name);
        let actual = format!("{}\n", export_keyforms_json(&document, true).unwrap());
        let golden = support::expected_keyforms_dir().join(format!("{name}.keyforms.json"));
        if update {
            std::fs::write(&golden, actual.as_bytes()).unwrap();
        } else {
            let expected = std::fs::read_to_string(&golden).unwrap_or_else(|error| {
                panic!(
                    "missing golden {}: {error} (run with UPDATE_GOLDEN_KEYFORMS=1)",
                    golden.display()
                )
            });
            assert_eq!(expected, actual, "golden keyform mismatch for {name}");
        }
    }
}

#[test]
fn golden_keyform_documents_import_and_validate_cleanly() {
    for name in fixture_gen::KEYFORM_FIXTURE_NAMES {
        let golden = support::expected_keyforms_dir().join(format!("{name}.keyforms.json"));
        let text = std::fs::read_to_string(&golden)
            .unwrap_or_else(|error| panic!("missing golden {}: {error}", golden.display()));
        let document =
            import_keyforms_json(&text).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(document.schema, keyform_recovery::SCHEMA_ID);
        let path = support::fixtures_dir().join(name);
        let model = build_ir_from_file(&path, &InspectOptions::default()).unwrap();
        let validation = keyform_recovery::validate_recovered_keyforms(&document, &model, None);
        assert!(
            !live2d_ir::has_fatal(&validation),
            "{name}: golden keyform document has fatal validation findings"
        );
    }
}

#[test]
fn keyform_output_is_deterministic_and_round_trips() {
    for name in fixture_gen::KEYFORM_FIXTURE_NAMES {
        let (first, _) = recover_fixture(name);
        let (second, _) = recover_fixture(name);
        let json_first = export_keyforms_json(&first, true).unwrap();
        let json_second = export_keyforms_json(&second, true).unwrap();
        assert_eq!(
            json_first, json_second,
            "{name}: recovery is not deterministic"
        );
        let imported = import_keyforms_json(&json_first).unwrap();
        let json_again = export_keyforms_json(&imported, true).unwrap();
        assert_eq!(json_first, json_again, "{name}: JSON round-trip differs");
    }
}

#[test]
fn body_placement_a_and_b_produce_identical_keyforms() {
    // DF-001 regression: both body placements must parse and recover to the
    // same semantic keyform document.
    for name in [
        "fixture-002-artmesh-param.moc3",
        "keyform-003-2d-3x3.moc3",
        "keyform-019-deep-nested-targets.moc3",
    ] {
        let bytes = support::read_fixture(name);
        let relocated = support::relocate_body(&bytes, 1280);
        let options = InspectOptions::default();
        let model_a = build_ir_from_bytes(&bytes, &options)
            .unwrap_or_else(|error| panic!("{name} native failed: {error}"));
        let model_b = build_ir_from_bytes(&relocated, &options)
            .unwrap_or_else(|error| panic!("{name} relocated failed: {error}"));
        let project_a = reconstruct(&model_a, &RecoveryPolicy::default());
        let project_b = reconstruct(&model_b, &RecoveryPolicy::default());
        let document_a = recover_keyforms(&model_a, Some(&project_a));
        let document_b = recover_keyforms(&model_b, Some(&project_b));
        assert_eq!(
            export_keyforms_json(&document_a, true).unwrap(),
            export_keyforms_json(&document_b, true).unwrap(),
            "{name}: body placement changed keyform semantics"
        );
    }
}

#[test]
fn strict_violations_are_reported_for_mismatch_fixture() {
    let (document, _) = recover_fixture("keyform-014-cardinality-mismatch.moc3");
    let violations = keyform_recovery::strict_violations(&document);
    assert!(
        violations.contains(&codes::KEYFORM_GRID_CARDINALITY_MISMATCH),
        "unexpected strict violations: {violations:?}"
    );

    let (clean, _) = recover_fixture("keyform-003-2d-3x3.moc3");
    assert!(keyform_recovery::strict_violations(&clean).is_empty());
}

#[test]
fn archived_differential_evidence_is_preserved() {
    // DF-001..DF-004 regression context (AGENT.3.5 archive must not rot).
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for file in [
        "docs/reports/agent-3.5/differential/relocated-002-b.py-moc3.json",
        "docs/reports/agent-3.5/differential/fixture-010.py-moc3.json",
        "docs/reports/agent-3.5/differential/fixture-002.py-moc3.json",
        "docs/DIFFERENTIAL_FINDINGS.md",
    ] {
        assert!(
            root.join(file).exists(),
            "archived AGENT.3.5 evidence missing: {file}"
        );
    }
    let findings = std::fs::read_to_string(root.join("docs/DIFFERENTIAL_FINDINGS.md")).unwrap();
    for id in ["DF-001", "DF-002", "DF-003", "DF-004"] {
        assert!(findings.contains(id), "finding {id} missing from docs");
    }
    let relocated = std::fs::read_to_string(
        root.join("docs/reports/agent-3.5/differential/relocated-002-b.py-moc3.json"),
    )
    .unwrap();
    assert!(relocated.contains("\"agreement\": 24"));
    assert!(relocated.contains("\"disagreement\": 3"));
    assert!(relocated.contains("UNVALIDATED"));
}

#[test]
fn large_dataset_recovery_is_linear_and_dense() {
    let (document, _) = recover_fixture("keyform-020-large-dataset.moc3");
    assert_eq!(document.statistics.stored_keyforms, 64);
    let grid = &document.keyform_grids[0];
    assert_eq!(grid.layout, GridLayout::Dense);
    assert_eq!(grid.confidence, live2d_ir::Confidence::Derived);
}
