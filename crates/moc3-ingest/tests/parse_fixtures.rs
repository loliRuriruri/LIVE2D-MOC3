//! Golden and per-fixture structural tests for the MOC3 inspector.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use moc3_ingest::raw::DeformerSpecific;
use moc3_ingest::{build_inspection_report, parse, Limits, RawMoc3, SourceInfo};

fn parse_fixture(name: &str) -> RawMoc3 {
    let bytes = support::read_fixture(name);
    parse(&bytes, &Limits::default())
        .unwrap_or_else(|error| panic!("{name} failed to parse: {error}"))
}

fn report_for(name: &str) -> moc3_ingest::InspectionReport {
    let bytes = support::read_fixture(name);
    let model = parse(&bytes, &Limits::default()).unwrap();
    build_inspection_report(
        model,
        SourceInfo {
            file_name: Some(name.to_string()),
            file_size: bytes.len() as u64,
        },
    )
}

#[test]
fn fixture_001_single_artmesh() {
    let model = parse_fixture("fixture-001-single-artmesh.moc3");
    assert_eq!(model.header.version_byte, 2);
    assert!(model.canvas.flag_y_reversed);
    assert_eq!(model.canvas.width, 800.0);
    assert_eq!(model.counts.parameters, 0);
    assert_eq!(model.counts.parts, 0);
    assert_eq!(model.counts.art_meshes, 1);
    assert!(model.parameters.is_empty());
    assert!(model.anomalies.is_empty(), "{:?}", model.anomalies);
    let mesh = &model.art_meshes[0];
    assert_eq!(mesh.id.text, "ArtMesh_Synthetic_00");
    assert_eq!(mesh.vertex_count, 4);
    assert_eq!(mesh.index_count, 6);
    assert_eq!(mesh.texture_number, 0);
    assert_eq!(mesh.keyform_count, 1);
    assert!(mesh.parent_part.is_none());
    assert!(mesh.parent_deformer.is_none());
}

#[test]
fn fixture_002_artmesh_param() {
    let model = parse_fixture("fixture-002-artmesh-param.moc3");
    assert_eq!(model.parameters.len(), 1);
    let parameter = &model.parameters[0];
    assert_eq!(parameter.id.text, "ParamSynthetic_AngleX");
    assert_eq!(parameter.minimum, -30.0);
    assert_eq!(parameter.maximum, 30.0);
    assert_eq!(parameter.default, 0.0);
    assert_eq!(parameter.key_table_count, 1);
    assert_eq!(parameter.decimal_places, 2);
    assert!(!parameter.repeat);

    assert_eq!(model.parts.len(), 1);
    assert_eq!(model.parts[0].id.text, "Part_Synthetic_00");
    assert_eq!(model.parts[0].keyform_count, 2);
    assert!(model.parts[0].parent_part.is_none());

    let mesh = &model.art_meshes[0];
    assert_eq!(mesh.parent_part, Some(0));
    assert_eq!(mesh.keyform_count, 2);

    assert_eq!(model.bindings.len(), 1);
    assert_eq!(model.bindings[0].key_tables, vec![0]);
    assert_eq!(model.key_tables[0].key_count, 2);
    assert_eq!(model.counts.keyform_positions, 16);
}

#[test]
fn fixture_003_warp() {
    let model = parse_fixture("fixture-003-warp.moc3");
    assert_eq!(model.deformers.len(), 1);
    let deformer = &model.deformers[0];
    assert_eq!(deformer.id.text, "WarpDeformer_Synthetic_00");
    match &deformer.specific {
        DeformerSpecific::Warp {
            vertex_count,
            rows,
            columns,
            quad_transform,
        } => {
            assert_eq!(*vertex_count, 4);
            assert_eq!(*rows, 1);
            assert_eq!(*columns, 1);
            assert_eq!(*quad_transform, Some(0));
        }
        other => panic!("expected warp deformer, got {other:?}"),
    }
    assert_eq!(model.art_meshes[0].parent_deformer, Some(0));
}

#[test]
fn fixture_004_nested_warp() {
    let model = parse_fixture("fixture-004-nested-warp.moc3");
    assert_eq!(model.deformers.len(), 2);
    assert!(model.deformers[0].parent_deformer.is_none());
    assert_eq!(model.deformers[1].parent_deformer, Some(0));
    assert_eq!(model.art_meshes[0].parent_deformer, Some(1));

    let report = report_for("fixture-004-nested-warp.moc3");
    let edges = &report.hierarchy.edges;
    assert!(
        edges.iter().any(|edge| edge.relation == "parent_deformer"
            && edge.parent.index == 0
            && edge.child.index == 1),
        "deformer parent edge missing"
    );
    assert!(report.hierarchy.anomalies.is_empty());
}

#[test]
fn fixture_005_rotation() {
    let model = parse_fixture("fixture-005-rotation.moc3");
    match &model.deformers[0].specific {
        DeformerSpecific::Rotation { base_angle } => assert_eq!(*base_angle, 30.0),
        other => panic!("expected rotation deformer, got {other:?}"),
    }
}

#[test]
fn fixture_006_clipping_mask() {
    let model = parse_fixture("fixture-006-clipping-mask.moc3");
    assert_eq!(model.art_meshes.len(), 2);
    assert!(model.art_meshes[0].mask_drawables.is_empty());
    assert_eq!(model.art_meshes[1].mask_drawables, vec![0]);
    assert_eq!(model.masks.len(), 1);
    assert_eq!(model.masks[0].art_mesh, 0);

    let report = report_for("fixture-006-clipping-mask.moc3");
    assert!(report
        .hierarchy
        .edges
        .iter()
        .any(|edge| edge.relation == "masked_by"
            && edge.parent.index == 0
            && edge.child.index == 1));
}

#[test]
fn fixture_007_multi_params() {
    let model = parse_fixture("fixture-007-multi-params.moc3");
    assert_eq!(model.header.version_byte, 4);
    assert_eq!(model.counts.parameters, 5);
    assert_eq!(model.counts.parts, 2);
    assert_eq!(model.parts[1].parent_part, Some(0));
    assert_eq!(model.counts.part_keyforms, 12);
    assert_eq!(model.art_meshes[0].keyform_count, 2);
    assert_eq!(model.art_meshes[1].keyform_count, 1);
    assert_eq!(model.counts.keyform_multiply_colors, 3);
    assert!(model.art_meshes[0].keyform_color_begin.is_some());
    assert_eq!(model.bindings.len(), 4);
    assert_eq!(model.bindings[1].key_tables, vec![2, 3, 4]);
    assert!(model.anomalies.is_empty(), "{:?}", model.anomalies);
}

#[test]
fn fixture_008_multi_textures() {
    let model = parse_fixture("fixture-008-multi-textures.moc3");
    assert_eq!(model.art_meshes[0].texture_number, 0);
    assert_eq!(model.art_meshes[1].texture_number, 1);
    let report = report_for("fixture-008-multi-textures.moc3");
    assert_eq!(report.textures.len(), 2);
    assert_eq!(report.textures[0].texture_number, 0);
    assert_eq!(report.textures[1].texture_number, 1);
}

#[test]
fn fixture_009_v50() {
    let model = parse_fixture("fixture-009-v50.moc3");
    assert_eq!(model.header.version_byte, 5);
    assert_eq!(model.counts.raw_int_count, 64);
    assert_eq!(model.counts.parameters, 1);
}

#[test]
fn fixture_010_v53() {
    let model = parse_fixture("fixture-010-v53.moc3");
    assert_eq!(model.header.version_byte, 6);
    assert!(model.parts[0].offscreen_index.is_none());
    assert_eq!(model.art_meshes[0].blend_mode_5_3, Some(0));
    assert_eq!(model.header.version_label, "6 (5.3.00+)");
}

#[test]
fn fixture_011_v30() {
    let model = parse_fixture("fixture-011-v30.moc3");
    assert_eq!(model.header.version_byte, 1);
    assert_eq!(model.counts.parameters, 1);
    assert_eq!(model.counts.raw_int_count, 32);
}

#[test]
fn fixture_012_v40() {
    let model = parse_fixture("fixture-012-v40.moc3");
    assert_eq!(model.header.version_byte, 3);
    match &model.deformers[0].specific {
        DeformerSpecific::Warp { quad_transform, .. } => assert_eq!(*quad_transform, Some(1)),
        other => panic!("expected warp deformer, got {other:?}"),
    }
}

#[test]
fn golden_reports_match() {
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    std::fs::create_dir_all(support::expected_dir()).unwrap();
    for name in fixture_gen::FIXTURE_NAMES {
        if fixture_gen::GOLDEN_SKIP.contains(name) {
            continue;
        }
        let report = report_for(name);
        let actual = format!("{}\n", serde_json::to_string_pretty(&report).unwrap());
        let path = support::golden_path(name);
        if update {
            std::fs::write(&path, actual.as_bytes()).unwrap();
        } else {
            let expected = std::fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!(
                    "missing golden {}: {error} (run with UPDATE_GOLDEN=1)",
                    path.display()
                )
            });
            assert_eq!(expected, actual, "golden mismatch for {name}");
        }
    }
}

#[test]
fn checked_in_fixtures_match_generator() {
    for name in fixture_gen::FIXTURE_NAMES {
        let on_disk = support::read_fixture(name);
        let generated = fixture_gen::build_named(name).unwrap();
        assert_eq!(on_disk, generated, "fixture {name} is stale; regenerate it");
    }
}

#[test]
fn parse_is_deterministic_and_read_only() {
    for name in fixture_gen::FIXTURE_NAMES {
        let bytes = support::read_fixture(name);
        let snapshot = bytes.clone();
        let first = parse(&bytes, &Limits::default()).unwrap();
        let second = parse(&bytes, &Limits::default()).unwrap();
        assert_eq!(bytes, snapshot, "parser modified its input for {name}");
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap(),
            "parse output differs across runs for {name}"
        );
    }
}
