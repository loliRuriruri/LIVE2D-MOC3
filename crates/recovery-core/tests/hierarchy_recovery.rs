//! Fixture-based hierarchy reconstruction tests (AGENT.3).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use hierarchy_recovery::{
    reconstruct, validate_hierarchy, RecoveredNode, RecoveredProject, RecoveryPolicy,
    ResolutionState,
};
use live2d_ir::Confidence;
use recovery_core::{build_ir_from_file, InspectOptions};

fn project_for(name: &str, policy: &RecoveryPolicy) -> RecoveredProject {
    let path = support::fixtures_dir().join(name);
    let model = build_ir_from_file(&path, &InspectOptions::default())
        .unwrap_or_else(|error| panic!("{name} failed to map: {error}"));
    reconstruct(&model, policy)
}

fn conservative(name: &str) -> RecoveredProject {
    project_for(name, &RecoveryPolicy::default())
}

fn node<'a>(project: &'a RecoveredProject, id: &str) -> &'a RecoveredNode {
    project
        .nodes
        .iter()
        .find(|node| node.id.as_str() == id)
        .unwrap_or_else(|| panic!("node {id} not found in project"))
}

fn has_code(project: &RecoveredProject, code: &str) -> bool {
    project
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}

#[test]
fn every_fixture_reconstructs_and_validates() {
    for name in fixture_gen::FIXTURE_NAMES {
        let project = conservative(name);
        let validation = validate_hierarchy(&project);
        assert!(
            validation.is_empty(),
            "{name} produced validation findings: {validation:?}"
        );
        assert!(
            !live2d_ir::has_fatal(&project.diagnostics),
            "{name} produced fatal diagnostics"
        );
    }
}

#[test]
fn hierarchy_001_part_artmesh() {
    let project = conservative("hierarchy-001-part-artmesh.moc3");
    let part = node(&project, "Part_Synthetic_00");
    let mesh = node(&project, "ArtMesh_Synthetic_00");
    assert_eq!(part.status, ResolutionState::TopLevel);
    assert_eq!(mesh.status, ResolutionState::Resolved);
    assert_eq!(mesh.confidence, Some(Confidence::Exact));
    assert_eq!(
        mesh.parent.as_ref().map(|id| id.as_str()),
        Some("Part_Synthetic_00")
    );
    assert_eq!(part.children.len(), 1);
}

#[test]
fn hierarchy_002_and_003_deformer_chains() {
    let project = conservative("hierarchy-002-part-warp-artmesh.moc3");
    let warp = node(&project, "WarpDeformer_Synthetic_00");
    let mesh = node(&project, "ArtMesh_Synthetic_00");
    assert_eq!(
        warp.parent.as_ref().map(|id| id.as_str()),
        Some("Part_Synthetic_00")
    );
    assert_eq!(
        mesh.parent.as_ref().map(|id| id.as_str()),
        Some("WarpDeformer_Synthetic_00")
    );
    assert_eq!(
        mesh.part_association.as_ref().map(|id| id.as_str()),
        Some("Part_Synthetic_00")
    );
    assert!(!has_code(&project, "exact_relation_conflict"));

    let project = conservative("hierarchy-003-part-rotation-artmesh.moc3");
    let mesh = node(&project, "ArtMesh_Synthetic_00");
    assert_eq!(
        mesh.parent.as_ref().map(|id| id.as_str()),
        Some("RotationDeformer_Synthetic_00")
    );
}

#[test]
fn hierarchy_004_and_005_nested_deformers() {
    let project = conservative("hierarchy-004-nested-warp.moc3");
    let second = node(&project, "WarpDeformer_Synthetic_01");
    assert_eq!(
        second.parent.as_ref().map(|id| id.as_str()),
        Some("WarpDeformer_Synthetic_00")
    );

    let project = conservative("hierarchy-005-warp-rotation-artmesh.moc3");
    let rotation = node(&project, "RotationDeformer_Synthetic_00");
    assert_eq!(
        rotation.parent.as_ref().map(|id| id.as_str()),
        Some("WarpDeformer_Synthetic_00")
    );
}

#[test]
fn hierarchy_006_multiple_parts() {
    let project = conservative("hierarchy-006-multiple-parts.moc3");
    assert_eq!(
        node(&project, "Part_Synthetic_00").status,
        ResolutionState::TopLevel
    );
    assert_eq!(
        node(&project, "Part_Synthetic_01")
            .parent
            .as_ref()
            .map(|id| id.as_str()),
        Some("Part_Synthetic_00")
    );
    assert_eq!(
        node(&project, "Part_Synthetic_02")
            .parent
            .as_ref()
            .map(|id| id.as_str()),
        Some("Part_Synthetic_01")
    );
    assert_eq!(project.statistics.top_level, 1);
    assert_eq!(project.statistics.resolved, 5);
}

#[test]
fn hierarchy_007_orphan_is_preserved() {
    let project = conservative("hierarchy-007-orphan-artmesh.moc3");
    let orphan = node(&project, "ArtMesh_Orphan");
    assert_eq!(orphan.status, ResolutionState::Unresolved);
    assert!(orphan.parent.is_none());
    assert!(project.containers.iter().any(|container| container
        .members
        .iter()
        .any(|member| member.as_str() == "ArtMesh_Orphan")));
    assert!(has_code(&project, "orphan_node"));
}

#[test]
fn hierarchy_008_ambiguity_requires_heuristics() {
    let conservative_run = conservative("hierarchy-008-ambiguous-parent.moc3");
    let orphan = node(&conservative_run, "ArtMesh_Orphan");
    assert_eq!(orphan.status, ResolutionState::Unresolved);
    assert!(has_code(&conservative_run, "heuristic_parenting_disabled"));

    let heuristic_run = project_for(
        "hierarchy-008-ambiguous-parent.moc3",
        &RecoveryPolicy::with_heuristics(),
    );
    let orphan = node(&heuristic_run, "ArtMesh_Orphan");
    assert_eq!(orphan.status, ResolutionState::Ambiguous);
    assert_eq!(orphan.trace.tied_candidates.len(), 2);
    assert!(has_code(&heuristic_run, "ambiguous_parent"));
}

#[test]
fn hierarchy_009_cycle_is_reported_not_broken() {
    let project = conservative("hierarchy-009-cycle.moc3");
    assert_eq!(
        node(&project, "WarpDeformer_Synthetic_00").status,
        ResolutionState::CycleDetected
    );
    assert_eq!(
        node(&project, "WarpDeformer_Synthetic_01").status,
        ResolutionState::CycleDetected
    );
    assert_eq!(project.cycles.len(), 1);
    assert!(has_code(&project, "hierarchy_cycle"));
    assert!(!live2d_ir::has_fatal(&project.diagnostics));
}

#[test]
fn hierarchy_010_part_mismatch_is_reported() {
    let project = conservative("hierarchy-010-part-mismatch.moc3");
    let mesh = node(&project, "ArtMesh_Synthetic_00");
    assert_eq!(
        mesh.parent.as_ref().map(|id| id.as_str()),
        Some("WarpDeformer_Synthetic_00")
    );
    assert_eq!(
        mesh.part_association.as_ref().map(|id| id.as_str()),
        Some("Part_Synthetic_01")
    );
    assert!(has_code(&project, "exact_relation_conflict"));
}

#[test]
fn hierarchy_014_self_parent_is_rejected() {
    let project = conservative("hierarchy-014-self-parent.moc3");
    let warp = node(&project, "WarpDeformer_Synthetic_00");
    // The stored self relation is rejected; the legal part association is
    // used instead (fallback, not a guess).
    assert_eq!(warp.status, ResolutionState::Resolved);
    assert_eq!(
        warp.parent.as_ref().map(|id| id.as_str()),
        Some("Part_Synthetic_00")
    );
    assert!(warp
        .trace
        .rejected
        .iter()
        .any(|rejected| rejected.reason == hierarchy_recovery::RejectionReason::IllegalSelfParent));
    assert!(has_code(&project, "illegal_relation"));
}

#[test]
fn hierarchy_011_deep_chain_is_iterative() {
    let project = conservative("hierarchy-011-deep.moc3");
    assert!(!live2d_ir::has_fatal(&project.diagnostics));
    assert_eq!(project.statistics.unresolved, 0);
    assert_eq!(project.statistics.ambiguous, 0);
    // 1500 deformers + 1 part + 1 mesh.
    assert_eq!(project.statistics.nodes, 1502);
    assert_eq!(project.statistics.resolved, 1501);
    // Tree and JSON rendering terminate on the deep chain.
    let tree = hierarchy_recovery::text::render_tree(&project, 32);
    assert!(tree.contains("depth limit reached"));
    let again = conservative("hierarchy-011-deep.moc3");
    assert_eq!(
        serde_json::to_string(&project).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
}

#[test]
fn hierarchy_012_and_013_scale() {
    let project = conservative("hierarchy-012-large-flat.moc3");
    assert_eq!(project.statistics.nodes, 1501);
    assert_eq!(project.statistics.resolved, 1500);
    assert_eq!(project.statistics.top_level, 1);

    let project = conservative("hierarchy-013-scale.moc3");
    assert_eq!(project.statistics.nodes, 6001);
    assert_eq!(project.statistics.resolved, 6000);
    assert_eq!(project.statistics.top_level, 1);
    assert_eq!(project.statistics.cycles, 0);
    assert!(!live2d_ir::has_fatal(&project.diagnostics));
    let again = conservative("hierarchy-013-scale.moc3");
    assert_eq!(
        serde_json::to_string(&project).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
}

#[test]
fn recovered_project_does_not_copy_geometry() {
    let project = conservative("hierarchy-002-part-warp-artmesh.moc3");
    let json = serde_json::to_string(&project).unwrap();
    for forbidden in ["\"uvs\"", "\"positions\"", "\"vertices\"", "\"indices\""] {
        assert!(
            !json.contains(forbidden),
            "recovered project JSON contains geometry key {forbidden}"
        );
    }
}

#[test]
fn golden_recovered_projects_match() {
    let update = std::env::var("UPDATE_GOLDEN_HIERARCHY").is_ok();
    std::fs::create_dir_all(support::expected_hierarchy_dir()).unwrap();
    for name in fixture_gen::FIXTURE_NAMES {
        if fixture_gen::GOLDEN_SKIP.contains(name) {
            continue;
        }
        let project = conservative(name);
        let actual = format!("{}\n", serde_json::to_string_pretty(&project).unwrap());
        let golden = support::expected_hierarchy_dir().join(format!("{name}.project.json"));
        if update {
            std::fs::write(&golden, actual.as_bytes()).unwrap();
        } else {
            let expected = std::fs::read_to_string(&golden).unwrap_or_else(|error| {
                panic!(
                    "missing golden {}: {error} (run with UPDATE_GOLDEN_HIERARCHY=1)",
                    golden.display()
                )
            });
            assert_eq!(expected, actual, "golden mismatch for {name}");
        }
    }
}
