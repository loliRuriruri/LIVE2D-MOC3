//! AGENT.4 crate tests: binding normalization, axis reconstruction, grid
//! shapes, target payloads, unresolved handling, determinism and round-trip.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use keyform_recovery::{
    codes, from_json_str, recover, to_json_str, Cardinality, GridLayout, GridOrdering,
    KeyformTarget, TargetKeyforms,
};
use live2d_ir::{BindingTarget, Confidence, ParameterKind, Severity};

fn base() -> live2d_ir::Live2DModel {
    support::base_model()
}

fn has_code(document: &keyform_recovery::RecoveredKeyformModel, code: &str) -> bool {
    document
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}

#[test]
fn one_dimensional_band_with_dense_grid() {
    let mut model = base();
    support::add_parameter(
        &mut model,
        "ParamAngleX",
        &[-30.0, 0.0, 30.0],
        ParameterKind::Normal,
    );
    support::add_binding(
        &mut model,
        "binding:000000",
        &[("ParamAngleX", &[-30.0, 0.0, 30.0])],
    );
    support::add_mesh(
        &mut model,
        "Face",
        Some("binding:000000"),
        &[(1.0, 0.0, 4), (1.0, 0.0, 4), (1.0, 0.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Face")),
    );

    let document = recover(&model, None);
    assert_eq!(document.statistics.targets, 1);
    assert_eq!(document.statistics.stored_keyforms, 3);
    assert_eq!(document.statistics.dense_grids, 1);
    assert_eq!(document.unresolved.len(), 0);

    let band = &document.binding_bands[0];
    assert_eq!(band.axes.len(), 1);
    assert_eq!(band.axes[0].keys, vec![-30.0, 0.0, 30.0]);
    assert_eq!(band.expected_cardinality, Cardinality::Exact { value: 3 });
    assert_eq!(band.provenance.confidence, Confidence::Exact);

    let grid = &document.keyform_grids[0];
    assert_eq!(grid.layout, GridLayout::Dense);
    assert_eq!(grid.ordering, GridOrdering::Derived);
    assert_eq!(grid.confidence, Confidence::Derived);
}

#[test]
fn two_dimensional_grid_ordering_stays_unknown() {
    let mut model = base();
    support::add_parameter(&mut model, "X", &[-1.0, 0.0, 1.0], ParameterKind::Normal);
    support::add_parameter(&mut model, "Y", &[-1.0, 0.0, 1.0], ParameterKind::Normal);
    support::add_binding(
        &mut model,
        "binding:000000",
        &[("X", &[-1.0, 0.0, 1.0]), ("Y", &[-1.0, 0.0, 1.0])],
    );
    let forms: Vec<(f32, usize)> = (0..9).map(|_| (1.0, 4)).collect();
    support::add_warp(&mut model, "Warp", Some("binding:000000"), &forms);
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::Deformer(live2d_ir::DeformerId::new("Warp")),
    );

    let document = recover(&model, None);
    let grid = &document.keyform_grids[0];
    assert_eq!(grid.expected_cardinality, Cardinality::Exact { value: 9 });
    assert_eq!(grid.layout, GridLayout::Dense);
    assert_eq!(grid.ordering, GridOrdering::Unknown);
    assert!(grid.rules.iter().any(|rule| rule.as_str() == "KF-005"));
}

#[test]
fn three_dimensional_grid_is_supported() {
    let mut model = base();
    for name in ["A", "B", "C"] {
        support::add_parameter(&mut model, name, &[0.0, 1.0], ParameterKind::Normal);
    }
    support::add_binding(
        &mut model,
        "binding:000000",
        &[("A", &[0.0, 1.0]), ("B", &[0.0, 1.0]), ("C", &[0.0, 1.0])],
    );
    let forms: Vec<(f32, f32, usize)> = (0..8).map(|_| (1.0, 0.0, 4)).collect();
    support::add_mesh(&mut model, "Mesh", Some("binding:000000"), &forms);
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Mesh")),
    );

    let document = recover(&model, None);
    assert_eq!(
        document.binding_bands[0].expected_cardinality,
        Cardinality::Exact { value: 8 }
    );
    assert_eq!(document.keyform_grids[0].layout, GridLayout::Dense);
    assert_eq!(document.target_keyforms[0].form_count(), 8);
}

#[test]
fn zero_dimensional_static_target() {
    let mut model = base();
    support::add_part(&mut model, "Part", None, &[7.5]);
    let document = recover(&model, None);
    let band = &document.binding_bands[0];
    assert!(band.axes.is_empty());
    assert_eq!(band.expected_cardinality, Cardinality::Exact { value: 1 });
    assert_eq!(band.provenance.confidence, Confidence::Derived);
    assert_eq!(document.keyform_grids[0].layout, GridLayout::Dense);
    assert_eq!(document.keyform_grids[0].ordering, GridOrdering::Derived);
}

#[test]
fn duplicate_keys_preserved_with_diagnostic() {
    let mut model = base();
    support::add_parameter(
        &mut model,
        "P",
        &[-1.0, 0.0, 0.0, 1.0],
        ParameterKind::Normal,
    );
    support::add_binding(
        &mut model,
        "binding:000000",
        &[("P", &[-1.0, 0.0, 0.0, 1.0])],
    );
    let document = recover(&model, None);
    let axis = &document.binding_bands[0].axes[0];
    assert_eq!(axis.keys, vec![-1.0, 0.0, 0.0, 1.0]);
    assert!(!axis.usable);
    assert!(has_code(&document, codes::DUPLICATE_PARAMETER_KEY));
}

#[test]
fn unsorted_keys_preserved_in_stored_order() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, -1.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, -1.0, 1.0])]);
    let document = recover(&model, None);
    let axis = &document.binding_bands[0].axes[0];
    assert_eq!(axis.keys, vec![0.0, -1.0, 1.0]);
    assert!(axis.usable);
    assert!(has_code(&document, codes::UNSORTED_PARAMETER_KEYS));
}

#[test]
fn out_of_range_keys_are_reported_but_preserved() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[-30.0, 45.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[-30.0, 45.0])]);
    let document = recover(&model, None);
    assert!(has_code(&document, codes::OUT_OF_RANGE_PARAMETER_KEY));
    assert_eq!(document.binding_bands[0].axes[0].keys, vec![-30.0, 45.0]);
}

#[test]
fn non_finite_keys_are_reported_positionally_and_serialize() {
    let mut model = base();
    support::add_parameter(
        &mut model,
        "P",
        &[0.0, f32::NAN, 1.0],
        ParameterKind::Normal,
    );
    support::add_binding(
        &mut model,
        "binding:000000",
        &[("P", &[0.0, f32::NAN, 1.0])],
    );
    let document = recover(&model, None);
    let axis = &document.binding_bands[0].axes[0];
    assert_eq!(axis.non_finite_key_indices, vec![1]);
    assert_eq!(axis.keys, vec![0.0, 1.0]);
    assert_eq!(axis.stored_key_count, 3);
    assert!(!axis.usable);
    assert!(has_code(&document, codes::NON_FINITE_PARAMETER_KEY));
    // Canonical JSON must serialize (no NaN leaks into the document).
    let json = to_json_str(&document, true).unwrap();
    assert!(json.contains("non_finite_key_indices"));
}

#[test]
fn dangling_parameter_is_unresolved() {
    let mut model = base();
    support::add_binding(&mut model, "binding:000000", &[("Missing", &[0.0, 1.0])]);
    let document = recover(&model, None);
    let band = &document.binding_bands[0];
    assert_eq!(band.axes.len(), 1);
    assert!(!band.axes[0].resolved);
    assert_eq!(band.provenance.confidence, Confidence::Unknown);
    assert_eq!(band.expected_cardinality, Cardinality::Exact { value: 2 });
    assert!(has_code(&document, codes::DANGLING_PARAMETER));
    assert!(document
        .unresolved
        .iter()
        .any(|entry| entry.code == codes::DANGLING_PARAMETER));
}

#[test]
fn dangling_binding_reference_is_reported() {
    let mut model = base();
    support::add_part(&mut model, "Part", Some("binding:999999"), &[1.0]);
    let document = recover(&model, None);
    assert!(has_code(&document, codes::DANGLING_BINDING_REFERENCE));
    assert_eq!(document.target_keyforms.len(), 0);
}

#[test]
fn dangling_target_reference_is_reported() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("artmesh:999999")),
    );
    let document = recover(&model, None);
    assert!(has_code(&document, codes::DANGLING_KEYFORM_TARGET));
    assert_eq!(document.keyform_grids.len(), 0);
}

#[test]
fn extra_forms_are_unknown_layout_not_repaired() {
    let mut model = base();
    support::add_parameter(&mut model, "X", &[0.0, 1.0], ParameterKind::Normal);
    support::add_parameter(&mut model, "Y", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(
        &mut model,
        "binding:000000",
        &[("X", &[0.0, 1.0]), ("Y", &[0.0, 1.0])],
    );
    let forms: Vec<(f32, f32, usize)> = (0..5).map(|_| (1.0, 0.0, 4)).collect();
    support::add_mesh(&mut model, "Mesh", Some("binding:000000"), &forms);
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Mesh")),
    );

    let document = recover(&model, None);
    let grid = &document.keyform_grids[0];
    assert_eq!(grid.stored_form_count, 5);
    assert_eq!(grid.layout, GridLayout::UnknownLayout);
    assert_eq!(grid.confidence, Confidence::Unknown);
    assert!(has_code(
        &document,
        codes::KEYFORM_GRID_CARDINALITY_MISMATCH
    ));
    assert_eq!(document.target_keyforms[0].form_count(), 5);
}

#[test]
fn cardinality_limit_is_unresolved_without_materialization() {
    let mut model = base();
    let keys: Vec<f32> = (0..2000).map(|index| index as f32).collect();
    support::add_parameter(&mut model, "A", &keys, ParameterKind::Normal);
    support::add_parameter(&mut model, "B", &keys, ParameterKind::Normal);
    support::add_parameter(&mut model, "C", &keys, ParameterKind::Normal);
    support::add_binding(
        &mut model,
        "binding:000000",
        &[("A", &keys), ("B", &keys), ("C", &keys)],
    );
    let document = recover(&model, None);
    assert_eq!(
        document.binding_bands[0].expected_cardinality,
        Cardinality::LimitExceeded
    );
    assert!(has_code(
        &document,
        codes::KEYFORM_CARDINALITY_LIMIT_EXCEEDED
    ));
    assert!(document
        .unresolved
        .iter()
        .any(|entry| { entry.code == codes::KEYFORM_CARDINALITY_LIMIT_EXCEEDED }));
}

#[test]
fn cardinality_overflow_is_unresolved() {
    let mut model = base();
    let keys: Vec<f32> = (0..65536).map(|index| index as f32).collect();
    for name in ["A", "B", "C", "D", "E"] {
        support::add_parameter(&mut model, name, &keys, ParameterKind::Normal);
    }
    support::add_binding(
        &mut model,
        "binding:000000",
        &[
            ("A", &keys),
            ("B", &keys),
            ("C", &keys),
            ("D", &keys),
            ("E", &keys),
        ],
    );
    let document = recover(&model, None);
    assert_eq!(
        document.binding_bands[0].expected_cardinality,
        Cardinality::Overflow
    );
    assert!(has_code(&document, codes::KEYFORM_CARDINALITY_OVERFLOW));
}

#[test]
fn axis_count_and_key_limits_are_enforced() {
    let mut model = base();
    let mut axes: Vec<(String, Vec<f32>)> = Vec::new();
    for index in 0..65 {
        let name = format!("P{index}");
        support::add_parameter(&mut model, &name, &[0.0], ParameterKind::Normal);
        axes.push((name, vec![0.0]));
    }
    let axis_refs: Vec<(&str, &[f32])> = axes
        .iter()
        .map(|(name, keys)| (name.as_str(), keys.as_slice()))
        .collect();
    support::add_binding(&mut model, "binding:000000", &axis_refs);
    let document = recover(&model, None);
    assert!(has_code(&document, codes::AXIS_COUNT_LIMIT_EXCEEDED));
    assert_eq!(
        document.binding_bands[0].expected_cardinality,
        Cardinality::LimitExceeded
    );

    let mut model = base();
    let keys: Vec<f32> = (0..65537).map(|index| index as f32).collect();
    support::add_parameter(&mut model, "Big", &keys, ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("Big", &keys)]);
    let document = recover(&model, None);
    assert!(has_code(&document, codes::AXIS_KEY_LIMIT_EXCEEDED));
    assert_eq!(
        document.binding_bands[0].expected_cardinality,
        Cardinality::LimitExceeded
    );
}

#[test]
fn all_target_types_have_typed_payloads() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    support::add_part(&mut model, "Part", Some("binding:000000"), &[1.0, 2.0]);
    support::add_warp(
        &mut model,
        "Warp",
        Some("binding:000000"),
        &[(1.0, 4), (0.5, 4)],
    );
    support::add_rotation(&mut model, "Rot", Some("binding:000000"), 2);
    support::add_mesh(
        &mut model,
        "Mesh",
        Some("binding:000000"),
        &[(1.0, 0.0, 4), (1.0, 1.0, 4)],
    );
    for target in [
        BindingTarget::Part(live2d_ir::PartId::new("Part")),
        BindingTarget::Deformer(live2d_ir::DeformerId::new("Warp")),
        BindingTarget::Deformer(live2d_ir::DeformerId::new("Rot")),
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Mesh")),
    ] {
        support::bind_used_by(&mut model, "binding:000000", target);
    }

    let document = recover(&model, None);
    assert_eq!(document.statistics.targets, 4);
    assert_eq!(document.statistics.stored_keyforms, 8);
    let mut kinds: Vec<&'static str> = document
        .target_keyforms
        .iter()
        .map(|entry| match entry {
            TargetKeyforms::Part(_) => "part",
            TargetKeyforms::WarpDeformer(_) => "warp",
            TargetKeyforms::RotationDeformer(_) => "rotation",
            TargetKeyforms::ArtMesh(_) => "art_mesh",
        })
        .collect();
    kinds.sort_unstable();
    assert_eq!(kinds, vec!["art_mesh", "part", "rotation", "warp"]);

    for entry in &document.target_keyforms {
        match entry {
            TargetKeyforms::Part(part) => {
                assert_eq!(part.keyforms.len(), 2);
                assert_eq!(part.keyforms[0].draw_order, 1.0);
            }
            TargetKeyforms::WarpDeformer(warp) => {
                assert_eq!(warp.keyforms[0].geometry.position_count, 4);
                assert_eq!(warp.keyforms[1].opacity, 0.5);
            }
            TargetKeyforms::RotationDeformer(rotation) => {
                assert_eq!(rotation.keyforms.len(), 2);
                assert_eq!(rotation.keyforms[1].angle, 1.0);
            }
            TargetKeyforms::ArtMesh(mesh) => {
                assert_eq!(mesh.keyforms[0].geometry.position_count, 4);
                assert_eq!(mesh.keyforms[1].draw_order, 1.0);
            }
        }
    }
}

#[test]
fn blend_shape_binding_is_experimental() {
    let mut model = base();
    support::add_parameter(&mut model, "Blend", &[0.0, 1.0], ParameterKind::BlendShape);
    support::add_binding(&mut model, "binding:000000", &[("Blend", &[0.0, 1.0])]);
    support::add_mesh(
        &mut model,
        "Mesh",
        Some("binding:000000"),
        &[(1.0, 0.0, 4), (1.0, 0.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Mesh")),
    );
    let document = recover(&model, None);
    assert!(document.binding_bands[0].experimental);
    assert_eq!(
        document.binding_bands[0].provenance.confidence,
        Confidence::Unknown
    );
    assert_eq!(document.keyform_grids[0].confidence, Confidence::Unknown);
    assert!(has_code(&document, codes::BLEND_SHAPE_BINDING_EXPERIMENTAL));
}

#[test]
fn glue_binding_is_deferred() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    support::add_glue(&mut model, "Glue", "A", "B", "binding:000000");
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::Glue(live2d_ir::GlueId::new("Glue")),
    );
    let document = recover(&model, None);
    assert!(has_code(&document, codes::GLUE_KEYFORM_BINDING_DEFERRED));
    assert!(document.binding_bands[0].targets.is_empty());
    assert_eq!(document.statistics.targets, 0);
}

#[test]
fn output_is_deterministic_and_round_trips_byte_identically() {
    let mut model = base();
    support::add_parameter(&mut model, "X", &[-1.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("X", &[-1.0, 1.0])]);
    support::add_part(&mut model, "Part", Some("binding:000000"), &[1.0, 2.0]);
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::Part(live2d_ir::PartId::new("Part")),
    );

    let first = recover(&model, None);
    let second = recover(&model, None);
    let json_first = to_json_str(&first, true).unwrap();
    let json_second = to_json_str(&second, true).unwrap();
    assert_eq!(json_first, json_second, "recovery must be deterministic");

    let imported = from_json_str(&json_first).unwrap();
    let json_again = to_json_str(&imported, true).unwrap();
    assert_eq!(
        json_first, json_again,
        "JSON round-trip must be byte-identical"
    );
}

#[test]
fn validator_reports_duplicate_grid_ids_as_fatal() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    support::add_part(&mut model, "Part", Some("binding:000000"), &[1.0, 2.0]);
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::Part(live2d_ir::PartId::new("Part")),
    );
    let mut document = recover(&model, None);
    let duplicate = document.keyform_grids[0].clone();
    document.keyform_grids.push(duplicate);
    let validation = keyform_recovery::validate_recovered_keyforms(&document, &model, None);
    assert!(validation.iter().any(|diagnostic| {
        diagnostic.code == codes::DUPLICATE_GRID_ID && diagnostic.severity == Severity::Fatal
    }));
    document.diagnostics.extend(validation);
    assert!(to_json_str(&document, true).is_err());
}

#[test]
fn geometry_is_referenced_not_copied() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    let forms: Vec<(f32, f32, usize)> = (0..300).map(|_| (1.0, 0.0, 32)).collect();
    support::add_mesh(&mut model, "Big", Some("binding:000000"), &forms);
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Big")),
    );

    let document = recover(&model, None);
    assert_eq!(document.statistics.stored_keyforms, 300);
    let keyform_json = to_json_str(&document, false).unwrap();
    let ir_json = serde_json::to_string(&model).unwrap();
    assert!(
        keyform_json.len() * 5 < ir_json.len(),
        "keyform document must not copy geometry (keyform {} bytes vs IR {} bytes)",
        keyform_json.len(),
        ir_json.len()
    );
}

#[test]
fn large_dense_dataset_is_linear_and_exact() {
    let mut model = base();
    let keys: Vec<f32> = (0..500).map(|index| index as f32).collect();
    support::add_parameter(&mut model, "P", &keys, ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &keys)]);
    let forms: Vec<(f32, usize)> = (0..500).map(|_| (1.0, 4)).collect();
    support::add_warp(&mut model, "W", Some("binding:000000"), &forms);
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::Deformer(live2d_ir::DeformerId::new("W")),
    );
    let document = recover(&model, None);
    assert_eq!(document.statistics.stored_keyforms, 500);
    assert_eq!(document.statistics.dense_grids, 1);
    assert_eq!(document.unresolved.len(), 0);
    let grid = &document.keyform_grids[0];
    assert_eq!(grid.stored_form_count, 500);
    assert!(document
        .trace_for(&KeyformTarget::WarpDeformer(live2d_ir::DeformerId::new(
            "W"
        )))
        .is_some());
}

#[test]
fn target_not_in_project_is_a_warning() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    support::add_part(&mut model, "Extra", Some("binding:000000"), &[1.0, 2.0]);
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::Part(live2d_ir::PartId::new("Extra")),
    );
    // Project built from a different model that does not contain "Extra".
    let other = base();
    let project =
        hierarchy_recovery::reconstruct(&other, &hierarchy_recovery::RecoveryPolicy::default());
    let document = recover(&model, Some(&project));
    assert!(has_code(&document, codes::TARGET_NOT_IN_PROJECT));
    assert!(!document
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Fatal));
}

#[test]
fn explain_renders_target_trace() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    support::add_warp(
        &mut model,
        "W",
        Some("binding:000000"),
        &[(1.0, 4), (1.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::Deformer(live2d_ir::DeformerId::new("W")),
    );
    let document = recover(&model, None);
    let text = keyform_recovery::explain(&document, "W").expect("trace exists");
    assert!(text.contains("Target:"));
    assert!(text.contains("Expected Cartesian:"));
    assert!(text.contains("Stored Forms:"));
    assert!(text.contains("derived from the single stored sequence"));
}

#[test]
fn sparse_grid_is_partial_and_never_padded() {
    // IR-level negative: 2x2 expected (4) but only 2 forms stored.
    let mut model = base();
    support::add_parameter(&mut model, "X", &[0.0, 1.0], ParameterKind::Normal);
    support::add_parameter(&mut model, "Y", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(
        &mut model,
        "binding:000000",
        &[("X", &[0.0, 1.0]), ("Y", &[0.0, 1.0])],
    );
    support::add_mesh(
        &mut model,
        "Partial",
        Some("binding:000000"),
        &[(1.0, 0.0, 4), (1.0, 0.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Partial")),
    );

    let document = recover(&model, None);
    let grid = &document.keyform_grids[0];
    assert_eq!(grid.expected_cardinality, Cardinality::Exact { value: 4 });
    assert_eq!(grid.stored_form_count, 2);
    assert_eq!(grid.layout, GridLayout::Sparse);
    assert_eq!(grid.confidence, Confidence::Derived);
    assert_eq!(document.target_keyforms[0].form_count(), 2, "no padding");
    assert!(has_code(
        &document,
        codes::KEYFORM_GRID_CARDINALITY_MISMATCH
    ));
    assert!(keyform_recovery::strict_violations(&document)
        .contains(&codes::KEYFORM_GRID_CARDINALITY_MISMATCH));
}

#[test]
fn form_span_gap_is_preserved_and_validated() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 0.5, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 0.5, 1.0])]);
    support::add_mesh(
        &mut model,
        "Gap",
        Some("binding:000000"),
        &[(1.0, 0.0, 4), (1.0, 0.0, 4), (1.0, 0.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Gap")),
    );
    // Tamper with the stored order: indices [0, 2, 2].
    if let Some(mesh) = model.art_meshes.first_mut() {
        if let Some(form) = mesh.keyforms.get_mut(1) {
            form.index = 2;
        }
        if let Some(form) = mesh.keyforms.get_mut(2) {
            form.index = 2;
        }
    }

    let document = recover(&model, None);
    assert!(has_code(&document, codes::KEYFORM_FORM_SPAN_MISMATCH));
    assert_eq!(document.keyform_grids[0].layout, GridLayout::UnknownLayout);
    let indices = document.target_keyforms[0].form_indices();
    assert_eq!(
        indices,
        vec![0, 2, 2],
        "indices are preserved, not reindexed"
    );

    let validation = keyform_recovery::validate_recovered_keyforms(&document, &model, None);
    assert!(validation
        .iter()
        .any(|diagnostic| diagnostic.code == codes::KEYFORM_FORM_SPAN_MISMATCH));
    assert!(!validation
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Fatal));
}

#[test]
fn validator_rejects_out_of_bounds_form_index() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    support::add_mesh(
        &mut model,
        "Mesh",
        Some("binding:000000"),
        &[(1.0, 0.0, 4), (1.0, 0.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Mesh")),
    );
    let mut document = recover(&model, None);
    if let Some(keyform_recovery::TargetKeyforms::ArtMesh(entry)) =
        document.target_keyforms.first_mut()
    {
        if let Some(form) = entry.keyforms.get_mut(1) {
            form.index = 99;
        }
    }
    let validation = keyform_recovery::validate_recovered_keyforms(&document, &model, None);
    assert!(validation.iter().any(|diagnostic| {
        diagnostic.code == codes::FORM_INDEX_OUT_OF_BOUNDS && diagnostic.severity == Severity::Fatal
    }));
    document.diagnostics.extend(validation);
    assert!(to_json_str(&document, true).is_err());
}

#[test]
fn non_finite_payload_is_reported_and_refused_at_export() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    support::add_mesh(
        &mut model,
        "NaN",
        Some("binding:000000"),
        &[(f32::NAN, 0.0, 4), (1.0, 0.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("NaN")),
    );
    let document = recover(&model, None);
    assert!(has_code(&document, codes::NON_FINITE_PAYLOAD_VALUE));
    assert!(document
        .unresolved
        .iter()
        .any(|entry| entry.code == codes::NON_FINITE_PAYLOAD_VALUE));
    let export = to_json_str(&document, true);
    assert!(export.is_err(), "canonical JSON must refuse NaN payloads");
    assert_eq!(export.err().map(|error| error.code), Some("NonFiniteValue"));
    // Explain must not panic even with non-finite payloads.
    assert!(keyform_recovery::explain(&document, "NaN").is_some());
}

#[test]
fn explain_survives_tampered_non_finite_positions() {
    let mut model = base();
    support::add_parameter(
        &mut model,
        "P",
        &[0.0, f32::NAN, 1.0],
        ParameterKind::Normal,
    );
    support::add_binding(
        &mut model,
        "binding:000000",
        &[("P", &[0.0, f32::NAN, 1.0])],
    );
    support::add_mesh(
        &mut model,
        "Mesh",
        Some("binding:000000"),
        &[(1.0, 0.0, 4), (1.0, 0.0, 4), (1.0, 0.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Mesh")),
    );
    let mut document = recover(&model, None);
    if let Some(axis) = document
        .binding_bands
        .get_mut(0)
        .and_then(|band| band.axes.first_mut())
    {
        axis.non_finite_key_indices = vec![99];
    }
    // The validator flags the tampered position as Fatal...
    let validation = keyform_recovery::validate_recovered_keyforms(&document, &model, None);
    assert!(validation.iter().any(|diagnostic| {
        diagnostic.code == codes::FORM_INDEX_OUT_OF_BOUNDS && diagnostic.severity == Severity::Fatal
    }));
    // ...and rendering never panics.
    let text = keyform_recovery::explain(&document, "Mesh");
    assert!(text.is_some());
    assert!(text.unwrap_or_default().contains("non-finite"));
}

#[test]
fn validator_checks_target_band_and_binding_identity() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    support::add_mesh(
        &mut model,
        "Mesh",
        Some("binding:000000"),
        &[(1.0, 0.0, 4), (1.0, 0.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Mesh")),
    );
    let mut document = recover(&model, None);

    // Band identity mismatch between the target entry and its grid: Fatal.
    if let Some(keyform_recovery::TargetKeyforms::ArtMesh(entry)) =
        document.target_keyforms.first_mut()
    {
        entry.band = "band:missing".to_string();
    }
    let validation = keyform_recovery::validate_recovered_keyforms(&document, &model, None);
    assert!(validation.iter().any(|diagnostic| {
        diagnostic.code == codes::UNRESOLVED_ACCOUNTING_MISMATCH
            && diagnostic.severity == Severity::Fatal
    }));

    // Binding identity mismatch between entry and band: Warning.
    let mut document = recover(&model, None);
    if let Some(keyform_recovery::TargetKeyforms::ArtMesh(entry)) =
        document.target_keyforms.first_mut()
    {
        entry.binding = Some(live2d_ir::BindingId::new("binding:wrong"));
    }
    let validation = keyform_recovery::validate_recovered_keyforms(&document, &model, None);
    assert!(validation.iter().any(|diagnostic| {
        diagnostic.code == codes::CONFIDENCE_PROVENANCE_INCONSISTENT
            && diagnostic.severity == Severity::Warning
    }));
}

#[test]
fn non_contiguous_indices_do_not_trigger_a_false_layout_finding() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 0.5, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 0.5, 1.0])]);
    support::add_mesh(
        &mut model,
        "Gap",
        Some("binding:000000"),
        &[(1.0, 0.0, 4), (1.0, 0.0, 4), (1.0, 0.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Gap")),
    );
    if let Some(mesh) = model.art_meshes.first_mut() {
        if let Some(form) = mesh.keyforms.get_mut(2) {
            form.index = 9;
        }
    }
    let document = recover(&model, None);
    assert!(!document
        .diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.code == codes::LAYOUT_INCONSISTENT }));
}

#[test]
fn non_finite_trace_keys_are_refused_at_export() {
    let mut model = base();
    support::add_parameter(&mut model, "P", &[0.0, 1.0], ParameterKind::Normal);
    support::add_binding(&mut model, "binding:000000", &[("P", &[0.0, 1.0])]);
    support::add_mesh(
        &mut model,
        "Mesh",
        Some("binding:000000"),
        &[(1.0, 0.0, 4), (1.0, 0.0, 4)],
    );
    support::bind_used_by(
        &mut model,
        "binding:000000",
        BindingTarget::ArtMesh(live2d_ir::ArtMeshId::new("Mesh")),
    );
    let mut document = recover(&model, None);
    if let Some(trace) = document.traces.first_mut() {
        if let Some(axis) = trace.axes.first_mut() {
            axis.keys.push(f32::NAN);
        }
    }
    let export = to_json_str(&document, true);
    assert_eq!(export.err().map(|error| error.code), Some("NonFiniteValue"));
}
