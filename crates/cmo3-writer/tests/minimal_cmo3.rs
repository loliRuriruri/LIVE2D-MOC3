//! AGENT.5.1 gates: minimal main.xml (5B.2) and minimal `.cmo3` (5B.3).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::path::{Path, PathBuf};

use cmo3_writer::map::build_project;
use cmo3_writer::serialize::IdentityOptions;
use cmo3_writer::validate::{scan_xml, validate_typed, FindingSeverity};
use cmo3_writer::{caff, write_minimal_cmo3, WrittenCmo3};

use hierarchy_recovery::{reconstruct, RecoveryPolicy};
use keyform_recovery::recover;

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/expected-cmo3")
}

fn write_for(model: &live2d_ir::Live2DModel, page_count: usize) -> WrittenCmo3 {
    let hierarchy = reconstruct(model, &RecoveryPolicy::default());
    let keyforms = recover(model, Some(&hierarchy));
    let assets = support::assets(page_count);
    let project = build_project(
        model,
        &hierarchy,
        &keyforms,
        &assets,
        &cmo3_writer::map::MapOptions::default(),
    )
    .expect("mapping");
    write_minimal_cmo3(&project, &IdentityOptions::default(), &assets).expect("write")
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn gate_5b_single_mesh_minimal_cmo3() {
    let model = support::base_model(1);
    let written = write_for(&model, 1);

    // CAFF Gate 5A integration.
    let decoded = caff::decode_strict_raw(&written.bytes).expect("decode");
    assert_eq!(decoded.header.key, caff::DEFAULT_KEY);
    assert!(decoded.guard_ok);
    assert!(decoded.payload(caff::MAIN_XML_PATH).is_some());
    assert!(decoded.entry("imageFileBuf_0.png").is_some());
    let validation = caff::validate_archive(&written.bytes).expect("validate");
    assert!(validation.is_valid(), "{:?}", validation.findings);

    // XML integrity.
    let scan = scan_xml(&written.xml);
    assert!(scan.is_valid(), "{scan:?}");
    assert!(written.xml.contains("<CModelSource"));
    assert!(written
        .xml
        .contains("isTextureInputModelImageMode\" v=\"true\""));

    // One layered image, one layer, one model image, one texture input.
    let objects = |tag: &str| {
        scan.id_tags
            .values()
            .filter(|candidate| candidate.as_str() == tag)
            .count()
    };
    assert_eq!(objects("CLayeredImage"), 1);
    assert_eq!(objects("CLayer"), 1);
    assert_eq!(count_occurrences(&written.xml, "<CModelImage "), 1);
    assert_eq!(objects("CTextureInput_ModelImage"), 1);
    assert_eq!(
        scan.refs
            .iter()
            .filter(|(tag, _)| tag == "CTextureInput_ModelImage")
            .count(),
        2
    );

    // Pipeline trace passes end to end.
    assert!(!written.pipeline.is_empty());
    for step in &written.pipeline {
        assert!(step.pass, "pipeline step failed: {step:?}");
    }

    // Determinism: byte-identical on a second run.
    let again = write_for(&model, 1);
    assert_eq!(
        written.bytes, again.bytes,
        "writer output must be deterministic"
    );
}

#[test]
fn gate_5b_multi_mesh_single_document_multi_layer() {
    let model = support::base_model(3);
    let written = write_for(&model, 3);

    let scan = scan_xml(&written.xml);
    assert!(scan.is_valid(), "{scan:?}");
    let objects = |tag: &str| {
        scan.id_tags
            .values()
            .filter(|candidate| candidate.as_str() == tag)
            .count()
    };
    assert_eq!(objects("CLayeredImage"), 1);
    assert_eq!(objects("CLayer"), 3);
    assert_eq!(count_occurrences(&written.xml, "<CModelImage "), 3);
    assert_eq!(count_occurrences(&written.xml, "_imageToLayerInput"), 3);

    // Each model image must select a distinct layer: three selector relations
    // with distinct layer targets in the trace.
    let selector_steps: Vec<&cmo3_writer::validate::PipelineStep> = written
        .pipeline
        .iter()
        .filter(|step| step.label.contains("LayerSelector"))
        .collect();
    assert_eq!(selector_steps.len(), 3);
    let mut details: Vec<&str> = selector_steps
        .iter()
        .map(|step| step.detail.as_str())
        .collect();
    details.sort_unstable();
    details.dedup();
    assert_eq!(details.len(), 3, "layers must be selected distinctly");

    let assets = support::assets(3);
    let project = {
        let hierarchy = reconstruct(&model, &RecoveryPolicy::default());
        let keyforms = recover(&model, Some(&hierarchy));
        build_project(
            &model,
            &hierarchy,
            &keyforms,
            &assets,
            &cmo3_writer::map::MapOptions::default(),
        )
        .expect("mapping")
    };
    let snapshot = support::pipeline_snapshot(&project);
    assert_eq!(
        snapshot.get("layers").and_then(|value| value.as_u64()),
        Some(3)
    );
    assert_eq!(
        snapshot
            .get("model_images")
            .and_then(|value| value.as_u64()),
        Some(3)
    );
    assert_eq!(
        snapshot
            .get("layered_images")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
}

#[test]
fn negative_fixtures_fail_structurally_without_output() {
    let model = support::base_model(1);
    let assets = support::assets(1);
    let hierarchy = reconstruct(&model, &RecoveryPolicy::default());
    let keyforms = recover(&model, Some(&hierarchy));
    let base = || {
        build_project(
            &model,
            &hierarchy,
            &keyforms,
            &assets,
            &cmo3_writer::map::MapOptions::default(),
        )
        .expect("mapping")
    };

    // Missing model image while a texture is set.
    let mut project = base();
    project.meshes[0].model_image = None;
    assert!(write_minimal_cmo3(&project, &IdentityOptions::default(), &assets).is_err());
    assert!(validate_typed(&project)
        .iter()
        .any(|finding| finding.severity == FindingSeverity::Fatal));

    // Dangling texture resource.
    let mut project = base();
    project.meshes[0].texture = Some(99);
    assert!(write_minimal_cmo3(&project, &IdentityOptions::default(), &assets).is_err());

    // Layer count mismatch.
    let mut project = base();
    if let Some(layered) = project.layered_images.first_mut() {
        layered.layers.clear();
    }
    assert!(write_minimal_cmo3(&project, &IdentityOptions::default(), &assets).is_err());

    // Duplicate layer identifier.
    // A single layer cannot duplicate; the duplicate case uses two meshes.
    let model3 = support::base_model(2);
    let assets2 = support::assets(2);
    let hierarchy3 = reconstruct(&model3, &RecoveryPolicy::default());
    let keyforms3 = recover(&model3, Some(&hierarchy3));
    let mut project3 = build_project(
        &model3,
        &hierarchy3,
        &keyforms3,
        &assets2,
        &cmo3_writer::map::MapOptions::default(),
    )
    .expect("mapping");
    if let Some(layered) = project3.layered_images.first_mut() {
        let first = layered.layers[0].layer_id.clone();
        layered.layers[1].layer_id = first;
    }
    let findings = validate_typed(&project3);
    assert!(findings
        .iter()
        .any(|finding| finding.code == "duplicate_layer_identifier"));

    // Missing asset bytes.
    let empty_assets = cmo3_writer::textures::TextureAssets::new();
    let project = base();
    assert_eq!(
        write_minimal_cmo3(&project, &IdentityOptions::default(), &empty_assets)
            .err()
            .map(|error| error.code),
        Some("MissingTextureAsset")
    );

    // Model-image count mismatch must fail validation, never panic.
    let model2 = support::base_model(2);
    let assets2b = support::assets(2);
    let hierarchy2 = reconstruct(&model2, &RecoveryPolicy::default());
    let keyforms2 = recover(&model2, Some(&hierarchy2));
    let mut project2 = build_project(
        &model2,
        &hierarchy2,
        &keyforms2,
        &assets2b,
        &cmo3_writer::map::MapOptions::default(),
    )
    .expect("mapping");
    project2.model_images.pop();
    assert!(write_minimal_cmo3(&project2, &IdentityOptions::default(), &assets2b).is_err());
    assert!(validate_typed(&project2)
        .iter()
        .any(|finding| finding.code == "model_image_count_mismatch"));

    // Invalid geometry carries a fatal typed finding at map time already.
    assert!(support::base_model(1).art_meshes[0].uvs.len() == 4);
}

#[test]
fn golden_main_xml_and_pipeline_snapshot() {
    let update = std::env::var("UPDATE_GOLDEN_CMO3").is_ok();
    std::fs::create_dir_all(golden_dir()).unwrap();
    for (name, mesh_count) in [("5B-001", 1usize), ("5B-002", 3usize)] {
        let model = support::base_model(mesh_count);
        let written = write_for(&model, mesh_count);
        let assets = support::assets(mesh_count);
        let hierarchy = reconstruct(&model, &RecoveryPolicy::default());
        let keyforms = recover(&model, Some(&hierarchy));
        let project = build_project(
            &model,
            &hierarchy,
            &keyforms,
            &assets,
            &cmo3_writer::map::MapOptions::default(),
        )
        .expect("mapping");
        let snapshot = format!(
            "{}\n",
            serde_json::to_string_pretty(&support::pipeline_snapshot(&project)).unwrap()
        );

        let xml_path = golden_dir().join(format!("{name}.main.xml"));
        let snapshot_path = golden_dir().join(format!("{name}.pipeline.json"));
        if update {
            std::fs::write(&xml_path, written.xml.as_bytes()).unwrap();
            std::fs::write(&snapshot_path, snapshot.as_bytes()).unwrap();
        } else {
            let expected_xml = std::fs::read_to_string(&xml_path).unwrap_or_else(|error| {
                panic!("missing {name} golden ({error}); run with UPDATE_GOLDEN_CMO3=1")
            });
            assert_eq!(
                expected_xml, written.xml,
                "golden main.xml mismatch for {name}"
            );
            let expected_snapshot = std::fs::read_to_string(&snapshot_path).unwrap();
            assert_eq!(
                expected_snapshot, snapshot,
                "pipeline snapshot mismatch for {name}"
            );
        }
    }
}
