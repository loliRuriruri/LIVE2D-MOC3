//! Hierarchy validator: structural checks over the recovered project.
//!
//! Independent from resolution: it re-checks parent/children consistency,
//! typed relations, synthetic structure and statistics, so a corrupted or
//! hand-built project cannot be consumed silently (work order section 36).

use std::collections::{BTreeMap, BTreeSet};

use live2d_ir::{Diagnostic, Severity};

use crate::graph::{NodeId, CYCLES_ID, ROOT_ID, SYNTHETIC_PREFIX, UNRESOLVED_ID};
use crate::project::{compute_statistics, is_synthetic_id, ContainerKind, RecoveredProject};
use crate::resolve::ResolutionState;

/// Validate a recovered project. Fatal findings mean the project must not be
/// consumed or exported.
pub fn validate_hierarchy(project: &RecoveredProject) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = Vec::new();

    // --- node ids and order -------------------------------------------------
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for node in &project.nodes {
        if !seen.insert(node.id.as_str()) {
            out.push(fatal(
                "duplicate_node",
                format!("node '{}' appears more than once", node.id),
            ));
        }
        if is_synthetic_id(node.id.as_str()) {
            out.push(fatal(
                "synthetic_id_collision",
                format!(
                    "model node '{}' uses the reserved synthetic prefix '{SYNTHETIC_PREFIX}'",
                    node.id
                ),
            ));
        }
    }
    for pair in project.nodes.windows(2) {
        if let [left, right] = pair {
            let left_key = (left.kind.rank(), left.source_index);
            let right_key = (right.kind.rank(), right.source_index);
            if left_key >= right_key {
                out.push(fatal(
                    "non_canonical_order",
                    format!(
                        "nodes '{}' ({:?}) and '{}' ({:?}) are not in canonical order",
                        left.id, left_key, right.id, right_key
                    ),
                ));
            }
        }
    }

    // --- indexes ------------------------------------------------------------
    let node_by_id: BTreeMap<&str, &crate::project::RecoveredNode> = project
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();

    // --- per-node consistency ----------------------------------------------
    let mut child_listers: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let children_sets: BTreeMap<&str, BTreeSet<&str>> = project
        .nodes
        .iter()
        .map(|node| {
            (
                node.id.as_str(),
                node.children
                    .iter()
                    .map(|child| child.as_str())
                    .collect::<BTreeSet<&str>>(),
            )
        })
        .collect();
    for node in &project.nodes {
        match node.status {
            ResolutionState::Resolved | ResolutionState::CycleDetected => {
                if node.parent.is_none() || node.confidence.is_none() || node.rule.is_none() {
                    out.push(fatal(
                        "status_inconsistent",
                        format!(
                            "node '{}' is {:?} but lacks parent/confidence/rule",
                            node.id, node.status
                        ),
                    ));
                }
            }
            ResolutionState::TopLevel
            | ResolutionState::Unresolved
            | ResolutionState::Ambiguous => {
                if node.parent.is_some() {
                    out.push(fatal(
                        "status_inconsistent",
                        format!("node '{}' is {:?} but has a parent", node.id, node.status),
                    ));
                }
            }
        }
        if let Some(parent) = &node.parent {
            if parent.as_str() == node.id.as_str() {
                out.push(fatal(
                    "self_parent",
                    format!("node '{}' is its own parent", node.id),
                ));
            }
            match node_by_id.get(parent.as_str()) {
                None => out.push(fatal(
                    "dangling_parent",
                    format!("node '{}' references unknown parent '{parent}'", node.id),
                )),
                Some(parent_node) => {
                    if !parent_node.kind.accepts_child(node.kind) {
                        out.push(fatal(
                            "wrong_type_relation",
                            format!(
                                "{} cannot be the parent of {} (node '{}')",
                                parent_node.kind.label(),
                                node.kind.label(),
                                node.id
                            ),
                        ));
                    }
                    let listed = children_sets
                        .get(parent.as_str())
                        .map(|children| children.contains(node.id.as_str()))
                        .unwrap_or(false);
                    if !listed {
                        out.push(fatal(
                            "parent_child_mismatch",
                            format!(
                                "node '{}' declares parent '{parent}' that does not list it as a child",
                                node.id
                            ),
                        ));
                    }
                }
            }
        }
        let mut child_seen: BTreeSet<&str> = BTreeSet::new();
        for child in &node.children {
            if !child_seen.insert(child.as_str()) {
                out.push(fatal(
                    "multiple_parent",
                    format!("child '{child}' is listed twice under '{}'", node.id),
                ));
            }
            child_listers
                .entry(child.as_str())
                .or_default()
                .insert(node.id.as_str());
            match node_by_id.get(child.as_str()) {
                None => out.push(fatal(
                    "dangling_child",
                    format!("node '{}' lists unknown child '{child}'", node.id),
                )),
                Some(child_node) => {
                    if child_node.parent.as_ref().map(|parent| parent.as_str())
                        != Some(node.id.as_str())
                    {
                        out.push(fatal(
                            "parent_child_mismatch",
                            format!(
                                "node '{}' lists '{child}' as child but the link is not reciprocal",
                                node.id
                            ),
                        ));
                    }
                }
            }
        }
    }
    // A node with a declared parent must not also be listed by another node.
    for node in &project.nodes {
        let Some(parent) = &node.parent else {
            continue;
        };
        if let Some(listers) = child_listers.get(node.id.as_str()) {
            for lister in listers {
                if *lister != parent.as_str() {
                    out.push(fatal(
                        "multiple_parent",
                        format!(
                            "node '{}' is also listed under '{lister}' while its parent is '{parent}'",
                            node.id
                        ),
                    ));
                }
            }
        }
    }

    // --- synthetic structure ------------------------------------------------
    if project.root.id != ROOT_ID || !project.root.synthetic {
        out.push(fatal(
            "root_inconsistent",
            "the synthetic root must use the reserved id and be marked synthetic".to_string(),
        ));
    }
    let mut root_children_seen: BTreeSet<&str> = BTreeSet::new();
    for child in &project.root.children {
        if !root_children_seen.insert(child.as_str()) {
            out.push(fatal(
                "root_inconsistent",
                format!("root lists '{child}' more than once"),
            ));
        }
    }
    let top_level: Vec<&str> = project
        .nodes
        .iter()
        .filter(|node| node.status == ResolutionState::TopLevel)
        .map(|node| node.id.as_str())
        .collect();
    let container_ids: Vec<&str> = project
        .containers
        .iter()
        .map(|container| container.id.as_str())
        .collect();
    let mut expected_root_children: Vec<&str> = top_level.clone();
    expected_root_children.extend(container_ids.iter().copied());
    let actual_root_children: Vec<&str> = project
        .root
        .children
        .iter()
        .map(|child| child.as_str())
        .collect();
    if expected_root_children != actual_root_children {
        out.push(fatal(
            "root_inconsistent",
            "root children must be the top-level nodes followed by the synthetic containers"
                .to_string(),
        ));
    }

    let mut container_seen: BTreeSet<&str> = BTreeSet::new();
    for container in &project.containers {
        if !container_seen.insert(container.id.as_str()) {
            out.push(fatal(
                "container_inconsistent",
                format!("container '{}' appears more than once", container.id),
            ));
        }
        if !container.synthetic || !is_synthetic_id(container.id.as_str()) {
            out.push(fatal(
                "container_inconsistent",
                format!("container '{}' must be synthetic", container.id),
            ));
        }
        let expected_id = match container.kind {
            ContainerKind::Unresolved => UNRESOLVED_ID,
            ContainerKind::Cycles => CYCLES_ID,
        };
        if container.id != expected_id {
            out.push(fatal(
                "container_inconsistent",
                format!(
                    "container '{}' does not match its kind ({:?})",
                    container.id, container.kind
                ),
            ));
        }
        for member in &container.members {
            if !node_by_id.contains_key(member.as_str()) {
                out.push(fatal(
                    "container_inconsistent",
                    format!(
                        "container '{}' lists unknown member '{member}'",
                        container.id
                    ),
                ));
            }
        }
    }
    for container in &project.containers {
        let actual: BTreeSet<&str> = container
            .members
            .iter()
            .map(|member| member.as_str())
            .collect();
        let expected: BTreeSet<&str> = match container.kind {
            ContainerKind::Unresolved => project
                .nodes
                .iter()
                .filter(|node| {
                    matches!(
                        node.status,
                        ResolutionState::Unresolved | ResolutionState::Ambiguous
                    )
                })
                .map(|node| node.id.as_str())
                .collect(),
            ContainerKind::Cycles => project
                .cycles
                .iter()
                .flat_map(|cycle| cycle.nodes.iter().map(|node| node.as_str()))
                .collect(),
        };
        if actual != expected {
            out.push(fatal(
                "container_inconsistent",
                format!(
                    "container '{}' members do not match the node states",
                    container.id
                ),
            ));
        }
    }

    // --- cycles --------------------------------------------------------------
    let cycle_members: BTreeSet<&str> = project
        .cycles
        .iter()
        .flat_map(|cycle| cycle.nodes.iter().map(|node| node.as_str()))
        .collect();
    for (index, cycle) in project.cycles.iter().enumerate() {
        if cycle.index != index {
            out.push(fatal(
                "cycle_inconsistent",
                format!("cycle group {index} has index {}", cycle.index),
            ));
        }
        if cycle.nodes.is_empty() {
            out.push(fatal(
                "cycle_inconsistent",
                format!("cycle group {index} has no members"),
            ));
            continue;
        }
        for member in &cycle.nodes {
            match node_by_id.get(member.as_str()) {
                None => out.push(fatal(
                    "cycle_inconsistent",
                    format!("cycle group {index} lists unknown node '{member}'"),
                )),
                Some(node) => {
                    if node.status != ResolutionState::CycleDetected {
                        out.push(fatal(
                            "cycle_inconsistent",
                            format!("cycle member '{}' has status {:?}", node.id, node.status),
                        ));
                    }
                }
            }
        }
        // Members must form a closed parent chain in traversal order.
        let mut closed = true;
        for (position, member) in cycle.nodes.iter().enumerate() {
            let expected_parent = cycle
                .nodes
                .get((position + 1) % cycle.nodes.len())
                .map(NodeId::as_str);
            let actual_parent = node_by_id
                .get(member.as_str())
                .and_then(|node| node.parent.as_ref())
                .map(NodeId::as_str);
            if actual_parent != expected_parent {
                closed = false;
            }
        }
        if !closed {
            out.push(fatal(
                "cycle_inconsistent",
                format!("cycle group {index} is not a closed parent chain"),
            ));
        }
    }
    for node in &project.nodes {
        if node.status == ResolutionState::CycleDetected
            && !cycle_members.contains(node.id.as_str())
        {
            out.push(fatal(
                "cycle_inconsistent",
                format!("node '{}' is in a cycle but no group declares it", node.id),
            ));
        }
    }

    // --- statistics ----------------------------------------------------------
    let recomputed = compute_statistics(&project.nodes, &project.cycles, project.diagnostics.len());
    if recomputed != project.statistics {
        out.push(fatal(
            "statistics_mismatch",
            "statistics do not match the project contents".to_string(),
        ));
    }

    cap_diagnostics(out)
}

fn fatal(code: &'static str, message: String) -> Diagnostic {
    Diagnostic::new(Severity::Fatal, code, message)
}

/// Bound the number of returned findings per code (same policy as the
/// resolution stages); a crafted project cannot amplify validator output
/// beyond ~256 per code plus one summary entry each.
fn cap_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    const CAP: usize = 256;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut truncated: BTreeMap<String, usize> = BTreeMap::new();
    let mut out: Vec<Diagnostic> = Vec::with_capacity(diagnostics.len().min(CAP));
    for diagnostic in diagnostics {
        let count = counts.entry(diagnostic.code.clone()).or_insert(0);
        if *count < CAP {
            out.push(diagnostic);
        } else {
            *truncated.entry(diagnostic.code.clone()).or_insert(0) += 1;
        }
        *count += 1;
    }
    for (code, count) in truncated {
        out.push(Diagnostic::new(
            Severity::Info,
            "diagnostics_truncated",
            format!("{count} additional '{code}' diagnostic(s) were suppressed by the cap"),
        ));
    }
    out
}
