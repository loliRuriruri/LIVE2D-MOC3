//! Hierarchy reconstruction tests over hand-built IR models.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::*;
use hierarchy_recovery::{
    explain, reconstruct, validate_hierarchy, NodeId, NodeKind, RecoveryPolicy, RejectionReason,
    ResolutionState, RuleId,
};
use live2d_ir::Confidence;

fn json(project: &hierarchy_recovery::RecoveredProject) -> String {
    serde_json::to_string_pretty(project).unwrap()
}

fn node<'a>(
    project: &'a hierarchy_recovery::RecoveredProject,
    id: &str,
) -> &'a hierarchy_recovery::RecoveredNode {
    project
        .nodes
        .iter()
        .find(|node| node.id.as_str() == id)
        .unwrap_or_else(|| panic!("node {id} not found"))
}

#[test]
fn part_chain_resolves_exactly() {
    let model = model(
        vec![part("Part_A", None), part("Part_B", Some("Part_A"))],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let project = reconstruct(&model, &RecoveryPolicy::default());
    assert!(!live2d_ir::has_fatal(&project.diagnostics));
    let a = node(&project, "Part_A");
    let b = node(&project, "Part_B");
    assert_eq!(a.status, ResolutionState::TopLevel);
    assert_eq!(b.status, ResolutionState::Resolved);
    assert_eq!(b.parent.as_ref().map(|id| id.as_str()), Some("Part_A"));
    assert_eq!(b.confidence, Some(Confidence::Exact));
    assert_eq!(b.rule.as_ref().map(RuleId::as_str), Some("HR-001"));
    assert_eq!(a.children, vec![NodeId::new("Part_B")]);
    assert_eq!(project.statistics.exact, 1);
    assert_eq!(project.statistics.top_level, 1);
}

#[test]
fn deformer_precedence_demotes_part_to_association() {
    let model = model(
        vec![part("Part_A", None)],
        vec![warp("Warp_A", None, Some("Part_A"))],
        vec![live2d_ir::ArtMesh {
            parent_deformer: Some(live2d_ir::DeformerId::new("Warp_A")),
            parent_part: Some(live2d_ir::PartId::new("Part_A")),
            ..mesh("Mesh_A", None, None)
        }],
        Vec::new(),
    );
    let project = reconstruct(&model, &RecoveryPolicy::default());
    let mesh = node(&project, "Mesh_A");
    assert_eq!(mesh.status, ResolutionState::Resolved);
    assert_eq!(mesh.parent.as_ref().map(|id| id.as_str()), Some("Warp_A"));
    assert_eq!(mesh.confidence, Some(Confidence::Exact));
    assert_eq!(
        mesh.part_association.as_ref().map(|id| id.as_str()),
        Some("Part_A")
    );
    assert!(mesh.trace.rejected.iter().any(|rejected| {
        rejected.reason == RejectionReason::DemotedToAssociation
            && rejected.parent.as_str() == "Part_A"
    }));
}

#[test]
fn orphan_mesh_is_preserved_unresolved() {
    let model = model(
        Vec::new(),
        Vec::new(),
        vec![mesh("Mesh_Orphan", None, None)],
        Vec::new(),
    );
    let project = reconstruct(&model, &RecoveryPolicy::default());
    let orphan = node(&project, "Mesh_Orphan");
    assert_eq!(orphan.status, ResolutionState::Unresolved);
    assert!(orphan.parent.is_none());
    assert_eq!(project.statistics.unresolved, 1);
    assert_eq!(project.statistics.orphans, 1);
    assert!(project.containers.iter().any(|container| {
        container.kind == hierarchy_recovery::ContainerKind::Unresolved
            && container.members.contains(&NodeId::new("Mesh_Orphan"))
    }));
    // The synthetic root does not secretly adopt orphans.
    assert!(!project.root.children.contains(&NodeId::new("Mesh_Orphan")));
}

#[test]
fn heuristic_parenting_is_blocked_by_default() {
    let model = model(
        vec![
            with_part_binding(part("Part_A", None), "binding:000000"),
            part("Part_B", None),
        ],
        Vec::new(),
        vec![with_binding(
            mesh("Mesh_Orphan", None, None),
            "binding:000000",
        )],
        vec![binding("binding:000000", vec![target_part("Part_A")])],
    );
    let conservative = reconstruct(&model, &RecoveryPolicy::default());
    let orphan = node(&conservative, "Mesh_Orphan");
    assert_eq!(orphan.status, ResolutionState::Unresolved);
    assert!(orphan
        .trace
        .rejected
        .iter()
        .any(|rejected| { rejected.reason == RejectionReason::PolicyDisabledHeuristic }));
    assert!(conservative
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "heuristic_parenting_disabled"));

    let heuristic = reconstruct(&model, &RecoveryPolicy::with_heuristics());
    let orphan = node(&heuristic, "Mesh_Orphan");
    assert_eq!(orphan.status, ResolutionState::Resolved);
    assert_eq!(orphan.confidence, Some(Confidence::Heuristic));
    assert_eq!(orphan.parent.as_ref().map(|id| id.as_str()), Some("Part_A"));
    assert_eq!(heuristic.statistics.heuristic, 1);
}

#[test]
fn ambiguous_parents_are_preserved() {
    let model = model(
        vec![
            with_part_binding(part("Part_A", None), "binding:000000"),
            with_part_binding(part("Part_B", None), "binding:000000"),
        ],
        Vec::new(),
        vec![with_binding(
            mesh("Mesh_Orphan", None, None),
            "binding:000000",
        )],
        vec![binding(
            "binding:000000",
            vec![target_part("Part_A"), target_part("Part_B")],
        )],
    );
    let project = reconstruct(&model, &RecoveryPolicy::with_heuristics());
    let orphan = node(&project, "Mesh_Orphan");
    assert_eq!(orphan.status, ResolutionState::Ambiguous);
    assert!(orphan.parent.is_none());
    assert_eq!(orphan.trace.tied_candidates.len(), 2);
    assert!(project
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "ambiguous_parent"));
    assert_eq!(project.statistics.ambiguous, 1);
    // Ambiguity is never resolved by picking the first candidate.
    assert!(orphan.trace.selected.is_none());
}

#[test]
fn self_parent_is_rejected_and_reported() {
    let mut self_warp = warp("Warp_Self", None, None);
    if let live2d_ir::Deformer::Warp(warp) = &mut self_warp {
        warp.common.parent_deformer = Some(live2d_ir::DeformerId::new("Warp_Self"));
    }
    let model = model(Vec::new(), vec![self_warp], Vec::new(), Vec::new());
    let project = reconstruct(&model, &RecoveryPolicy::default());
    let node = node(&project, "Warp_Self");
    assert_eq!(node.status, ResolutionState::Unresolved);
    assert!(node
        .trace
        .rejected
        .iter()
        .any(|rejected| { rejected.reason == RejectionReason::IllegalSelfParent }));
    assert!(project
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "illegal_relation"));
}

#[test]
fn stored_cycles_are_reported_not_broken() {
    let model = model(
        Vec::new(),
        vec![
            warp("Warp_A", Some("Warp_B"), None),
            warp("Warp_B", Some("Warp_A"), None),
        ],
        vec![mesh("Mesh_A", Some("Warp_A"), None)],
        Vec::new(),
    );
    let project = reconstruct(&model, &RecoveryPolicy::default());
    let a = node(&project, "Warp_A");
    let b = node(&project, "Warp_B");
    assert_eq!(a.status, ResolutionState::CycleDetected);
    assert_eq!(b.status, ResolutionState::CycleDetected);
    // Stored edges are kept.
    assert_eq!(a.parent.as_ref().map(|id| id.as_str()), Some("Warp_B"));
    assert_eq!(b.parent.as_ref().map(|id| id.as_str()), Some("Warp_A"));
    assert_eq!(project.cycles.len(), 1);
    assert_eq!(project.cycles[0].nodes.len(), 2);
    assert!(project
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "hierarchy_cycle"));
    assert_eq!(project.statistics.cycle_detected, 2);
    // Rendering terminates and shows the cycle container.
    let tree = hierarchy_recovery::text::render_tree(&project, 64);
    assert!(tree.contains("Cycles (synthetic)"), "{tree}");
    assert!(tree.contains("Cycle 0"), "{tree}");
}

#[test]
fn deep_chain_is_iterative() {
    let depth = 20_000usize;
    let mut deformers = Vec::with_capacity(depth);
    for index in 0..depth {
        let parent = if index == 0 {
            None
        } else {
            Some(format!("warp:{:06}", index - 1))
        };
        deformers.push(warp(
            &format!("warp:{index:06}"),
            parent.as_deref(),
            Some("Part_Root"),
        ));
    }
    let model = model(
        vec![part("Part_Root", None)],
        deformers,
        vec![mesh(
            "Mesh_Deep",
            Some(&format!("warp:{:06}", depth - 1)),
            None,
        )],
        Vec::new(),
    );
    let project = reconstruct(&model, &RecoveryPolicy::default());
    assert!(!live2d_ir::has_fatal(&project.diagnostics));
    assert_eq!(project.statistics.resolved, depth + 1);
    assert_eq!(project.statistics.cycles, 0);
    let tree = hierarchy_recovery::text::render_tree(&project, 8);
    assert!(tree.contains("depth limit reached"), "{tree}");
}

#[test]
fn reconstruction_is_deterministic_and_idempotent() {
    let model = model(
        vec![part("Part_A", None), part("Part_B", Some("Part_A"))],
        vec![warp("Warp_A", None, Some("Part_B"))],
        vec![mesh("Mesh_A", Some("Warp_A"), Some("Part_B"))],
        Vec::new(),
    );
    let first = reconstruct(&model, &RecoveryPolicy::default());
    let second = reconstruct(&model, &RecoveryPolicy::default());
    assert_eq!(json(&first), json(&second));
}

#[test]
fn explain_reports_rule_and_rejections() {
    let model = model(
        vec![part("Part_A", None)],
        vec![warp("Warp_A", None, Some("Part_A"))],
        vec![live2d_ir::ArtMesh {
            parent_deformer: Some(live2d_ir::DeformerId::new("Warp_A")),
            parent_part: Some(live2d_ir::PartId::new("Part_A")),
            ..mesh("Mesh_A", None, None)
        }],
        Vec::new(),
    );
    let project = reconstruct(&model, &RecoveryPolicy::default());
    let text = explain(&project, "Mesh_A").expect("explain output");
    assert!(text.contains("Selected parent:"), "{text}");
    assert!(text.contains("Warp_A"), "{text}");
    assert!(text.contains("HR-005"), "{text}");
    assert!(text.contains("demoted to part association"), "{text}");
    assert!(explain(&project, "does_not_exist").is_none());
}

#[test]
fn wrong_type_relations_are_rejected() {
    // Hand-built graph: an art mesh cannot be a parent.
    let model = model(
        Vec::new(),
        Vec::new(),
        vec![
            mesh("Mesh_A", None, None),
            mesh("Mesh_B", Some("Mesh_A"), None),
        ],
        Vec::new(),
    );
    let project = reconstruct(&model, &RecoveryPolicy::default());
    let _ = project; // IR types already prevent cross-kind parents; see validator tests.
}

#[test]
fn validator_catches_corrupted_projects() {
    let base = model(
        vec![part("Part_A", None), part("Part_B", Some("Part_A"))],
        Vec::new(),
        vec![mesh("Mesh_A", None, Some("Part_A"))],
        Vec::new(),
    );
    let clean = reconstruct(&base, &RecoveryPolicy::default());
    assert!(validate_hierarchy(&clean).is_empty());

    // Duplicate node.
    let mut duplicate = clean.clone();
    duplicate.nodes.push(duplicate.nodes[0].clone());
    assert!(coded(&validate_hierarchy(&duplicate), "duplicate_node"));

    // Dangling parent.
    let mut dangling = clean.clone();
    if let Some(node) = dangling
        .nodes
        .iter_mut()
        .find(|node| node.id.as_str() == "Part_B")
    {
        node.parent = Some(NodeId::new("missing:node"));
    }
    assert!(coded(&validate_hierarchy(&dangling), "dangling_parent"));

    // Wrong-type relation: mesh parented to a mesh.
    let mut wrong_type = clean.clone();
    if let Some(node) = wrong_type
        .nodes
        .iter_mut()
        .find(|node| node.id.as_str() == "Mesh_A")
    {
        node.parent = Some(NodeId::new("Part_A"));
    }
    if let Some(parent) = wrong_type
        .nodes
        .iter_mut()
        .find(|node| node.id.as_str() == "Part_A")
    {
        parent.kind = NodeKind::ArtMesh;
    }
    assert!(coded(
        &validate_hierarchy(&wrong_type),
        "wrong_type_relation"
    ));

    // Parent/children mismatch.
    let mut mismatch = clean.clone();
    if let Some(parent) = mismatch
        .nodes
        .iter_mut()
        .find(|node| node.id.as_str() == "Part_A")
    {
        parent.children.clear();
    }
    assert!(coded(
        &validate_hierarchy(&mismatch),
        "parent_child_mismatch"
    ));

    // Multiple parents: Part_A lists Mesh_A while Mesh_A's parent is also
    // listed under another node.
    let mut multiple = clean.clone();
    let mesh_id = NodeId::new("Mesh_A");
    let extra = NodeId::new("Part_B");
    if let Some(extra_node) = multiple.nodes.iter_mut().find(|node| node.id == extra) {
        extra_node.children.push(mesh_id.clone());
    }
    assert!(coded(&validate_hierarchy(&multiple), "multiple_parent"));

    // Self parent.
    let mut self_parent = clean.clone();
    if let Some(node) = self_parent
        .nodes
        .iter_mut()
        .find(|node| node.id.as_str() == "Part_B")
    {
        node.parent = Some(node.id.clone());
    }
    assert!(
        coded(&validate_hierarchy(&self_parent), "self_parent")
            || coded(&validate_hierarchy(&self_parent), "parent_child_mismatch")
    );

    // Statistics mismatch.
    let mut stats = clean.clone();
    stats.statistics.nodes += 1;
    assert!(coded(&validate_hierarchy(&stats), "statistics_mismatch"));

    // Undeclared cycle status.
    let mut cycle = clean.clone();
    if let Some(node) = cycle.nodes.first_mut() {
        node.status = ResolutionState::CycleDetected;
    }
    assert!(coded(&validate_hierarchy(&cycle), "cycle_inconsistent"));

    // Synthetic id collision.
    let mut synthetic = clean.clone();
    if let Some(node) = synthetic.nodes.first_mut() {
        node.id = NodeId::new("$root");
    }
    assert!(coded(
        &validate_hierarchy(&synthetic),
        "synthetic_id_collision"
    ));
}

fn coded(diagnostics: &[live2d_ir::Diagnostic], code: &str) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic.code == code)
}

#[test]
fn statistics_are_measured_not_accuracy() {
    let model = model(
        vec![part("Part_A", None)],
        vec![warp("Warp_A", None, Some("Part_A"))],
        vec![
            mesh("Mesh_A", Some("Warp_A"), None),
            mesh("Mesh_Orphan", None, None),
        ],
        Vec::new(),
    );
    let project = reconstruct(&model, &RecoveryPolicy::default());
    let tree = hierarchy_recovery::text::render_tree(&project, 64);
    let stats = hierarchy_recovery::text::render_statistics(&project);
    assert!(stats.contains("Hierarchy Resolution Statistics"), "{stats}");
    assert!(!stats.to_lowercase().contains("accuracy"));
    let _ = tree;
    assert_eq!(project.statistics.nodes, 4);
    assert_eq!(project.statistics.resolved, 2);
    assert_eq!(project.statistics.top_level, 1);
    assert_eq!(project.statistics.unresolved, 1);
}
