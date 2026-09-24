//! End-to-end CMO3 recovery tests (AGENT.5.2 gates 5D-5I).
//!
//! Each case runs the full pipeline MOC3 -> IR -> project -> keyforms ->
//! CMO3 and asserts structural validity, determinism, entity presence and
//! input immutability.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use cmo3_writer::map::{build_project, MapOptions};
use cmo3_writer::serialize::IdentityOptions;
use cmo3_writer::textures::{TextureAsset, TextureAssets};
use cmo3_writer::{caff, png, validate, write_minimal_cmo3, WrittenCmo3};

use hierarchy_recovery::{reconstruct, RecoveryPolicy};
use keyform_recovery::recover;
use recovery_core::{build_ir_from_file, InspectOptions};

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn assets_for(canvas: (u32, u32), pages: usize) -> TextureAssets {
    let mut assets = TextureAssets::new();
    for page in 0..pages {
        assets.push(TextureAsset {
            page: page as u32,
            bytes: png::solid_png(canvas.0, canvas.1, [(page as u8) * 40, 90, 160, 255]),
            source_path: Some(format!("textures/page{page}.png")),
            width: Some(canvas.0),
            height: Some(canvas.1),
        });
    }
    assets
}

/// Run the full pipeline for a fixture and return the written document.
fn run(name: &str, pages: usize) -> (WrittenCmo3, live2d_ir::Live2DModel, u64) {
    let path = support::fixtures_dir().join(name);
    let before = fnv1a64(&support::read_fixture(name));
    let model = build_ir_from_file(&path, &InspectOptions::default())
        .unwrap_or_else(|error| panic!("{name}: IR failed: {error}"));
    let hierarchy = reconstruct(&model, &RecoveryPolicy::default());
    let keyforms = recover(&model, Some(&hierarchy));
    let canvas = (
        model.canvas.width.round() as u32,
        model.canvas.height.round() as u32,
    );
    let assets = assets_for(canvas, pages);
    let project = build_project(
        &model,
        &hierarchy,
        &keyforms,
        &assets,
        &MapOptions::default(),
    )
    .unwrap_or_else(|error| panic!("{name}: mapping failed: {error}"));
    let written = write_minimal_cmo3(&project, &IdentityOptions::default(), &assets)
        .unwrap_or_else(|error| panic!("{name}: write failed: {error}"));
    let after = fnv1a64(&support::read_fixture(name));
    assert_eq!(before, after, "{name}: input fixture changed");
    (written, model, before)
}

fn assert_structural(name: &str, written: &WrittenCmo3) {
    let decoded = caff::decode_strict_raw(&written.bytes)
        .unwrap_or_else(|error| panic!("{name}: CAFF decode failed: {error}"));
    assert!(
        decoded.payload(caff::MAIN_XML_PATH).is_some(),
        "{name}: no main.xml"
    );
    let validation = caff::validate_archive(&written.bytes).unwrap();
    assert!(validation.is_valid(), "{name}: {:?}", validation.findings);
    let scan = validate::scan_xml(&written.xml);
    assert!(scan.is_valid(), "{name}: {scan:?}");
}

#[test]
fn e2e_001_part_artmesh() {
    let (written, model, _) = run("fixture-002-artmesh-param.moc3", 1);
    assert_structural("e2e-001", &written);
    let inspection = validate::inspect_xml(&written.xml);
    assert_eq!(inspection.art_meshes, model.art_meshes.len());
    assert_eq!(inspection.parameters, model.parameters.len());
    assert_eq!(inspection.layers, model.art_meshes.len());
    assert_eq!(inspection.forms, 4, "2 part + 2 mesh forms");
}

#[test]
fn e2e_002_warp_chain() {
    let (written, model, _) = run("hierarchy-004-nested-warp.moc3", 1);
    assert_structural("e2e-002", &written);
    let inspection = validate::inspect_xml(&written.xml);
    assert_eq!(inspection.warps, 2, "nested warp chain");
    assert_eq!(inspection.rotations, 0);
    assert!(inspection.forms >= 3);
    assert!(!model.deformers.is_empty());
}

#[test]
fn e2e_003_rotation_chain() {
    let (written, model, _) = run("hierarchy-003-part-rotation-artmesh.moc3", 1);
    assert_structural("e2e-003", &written);
    let inspection = validate::inspect_xml(&written.xml);
    assert_eq!(inspection.rotations, 1);
    assert_eq!(inspection.warps, 0);
    assert!(written.xml.contains("originX="));
    assert!(!model.deformers.is_empty());
}

#[test]
fn e2e_004_multi_parameter_bindings() {
    let (written, model, _) = run("fixture-007-multi-params.moc3", 1);
    assert_structural("e2e-004", &written);
    let inspection = validate::inspect_xml(&written.xml);
    assert_eq!(inspection.parameters, model.parameters.len());
    assert!(inspection.bindings >= model.bindings.len());
    assert!(written.xml.contains("keyformBindings"));
}

#[test]
fn e2e_005_2d_keyforms() {
    let (written, _, _) = run("keyform-003-2d-3x3.moc3", 1);
    assert_structural("e2e-005", &written);
    let inspection = validate::inspect_xml(&written.xml);
    assert!(inspection.keyform_grids >= 1);
    // 3x3 grid => 9 forms per target.
    assert!(inspection.forms >= 18);
    assert!(written.xml.contains("_keyOnParameterList"));
}

#[test]
fn e2e_006_multiple_parts() {
    let (written, model, _) = run("hierarchy-006-multiple-parts.moc3", 1);
    assert_structural("e2e-006", &written);
    let inspection = validate::inspect_xml(&written.xml);
    assert_eq!(inspection.parts, model.parts.len() + 1, "root + parts");
    assert!(written.xml.contains("_childGuids"));
}

#[test]
fn e2e_007_multiple_artmeshes_and_008_multi_texture() {
    let (written, model, _) = run("fixture-008-multi-textures.moc3", 2);
    assert_structural("e2e-007/008", &written);
    let inspection = validate::inspect_xml(&written.xml);
    assert_eq!(inspection.art_meshes, model.art_meshes.len());
    assert_eq!(inspection.textures, 2, "two texture pages");
    assert_eq!(inspection.layers, model.art_meshes.len());
    assert_eq!(inspection.model_images, model.art_meshes.len());
}

#[test]
fn e2e_009_mask() {
    let (written, model, _) = run("fixture-006-clipping-mask.moc3", 1);
    assert_structural("e2e-009", &written);
    let inspection = validate::inspect_xml(&written.xml);
    assert!(inspection.masks >= 1, "clipGuidList must reference a mask");
    assert!(!model.mask_groups.is_empty());
}

#[test]
fn e2e_010_large_model_is_deterministic() {
    let (first, model, _) = run("hierarchy-012-large-flat.moc3", 1);
    assert_structural("e2e-010", &first);
    assert!(
        first.xml.len() > 1_000_000,
        "large model XML unexpectedly small: {}",
        first.xml.len()
    );
    let (second, _, _) = run("hierarchy-012-large-flat.moc3", 1);
    assert_eq!(
        first.bytes, second.bytes,
        "large model must be deterministic"
    );
    assert_eq!(model.art_meshes.len(), 1500);
}

#[test]
fn cli_recover_smoke() {
    // Library-level stand-in for the CLI path: same modules the CLI calls.
    let (written, _, _) = run("keyform-002-1d-three-keys.moc3", 1);
    assert_structural("cli-smoke", &written);
    let inspection = validate::inspect_xml(&written.xml);
    assert_eq!(inspection.parameters, 1);
    assert!(inspection.forms >= 6, "part + mesh forms from a 3-key grid");
}

#[test]
fn best_effort_labels_unresolved_grids_instead_of_failing() {
    // keyform-014 stores more forms than the grid expects (unknown layout).
    let path = support::fixtures_dir().join("keyform-014-cardinality-mismatch.moc3");
    let model = build_ir_from_file(&path, &InspectOptions::default()).unwrap();
    let hierarchy = reconstruct(&model, &RecoveryPolicy::default());
    let keyforms = recover(&model, Some(&hierarchy));
    let canvas = (
        model.canvas.width.round() as u32,
        model.canvas.height.round() as u32,
    );
    let assets = assets_for(canvas, 1);

    let strict = build_project(
        &model,
        &hierarchy,
        &keyforms,
        &assets,
        &MapOptions::default(),
    );
    assert!(
        strict.is_err(),
        "strict mode must fail on an unknown layout"
    );

    let best_effort = build_project(
        &model,
        &hierarchy,
        &keyforms,
        &assets,
        &MapOptions {
            best_effort: true,
            model_name: None,
        },
    )
    .expect("best-effort mapping must succeed");
    assert!(best_effort.best_effort);
    assert!(best_effort
        .unsupported
        .iter()
        .any(|note| note.reason.contains("BEST_EFFORT")));
    let written = write_minimal_cmo3(&best_effort, &IdentityOptions::default(), &assets)
        .expect("best-effort write must succeed");
    assert_structural("best-effort", &written);
}
