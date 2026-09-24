//! Human-readable rendering: recovered tree, statistics and explain mode.
//!
//! All renderers are iterative (no recursion) and cycle-safe: a node is
//! shown once; later encounters are marked instead of recursing forever.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use live2d_ir::Confidence;

use crate::graph::NodeId;
use crate::project::{
    ContainerKind, CycleGroup, RecoveredNode, RecoveredProject, SyntheticContainer,
};
use crate::resolve::ResolutionState;
use crate::rule::rule_info;

/// Maximum diagnostics rendered in the human report.
const MAX_RENDERED_DIAGNOSTICS: usize = 50;

fn confidence_label(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Exact => "Exact",
        Confidence::Derived => "Derived",
        Confidence::Heuristic => "Heuristic",
        Confidence::Unknown => "Unknown",
    }
}

fn status_label(node: &RecoveredNode) -> String {
    match node.status {
        ResolutionState::Resolved => format!(
            "[{}]",
            node.confidence.map(confidence_label).unwrap_or("Resolved")
        ),
        ResolutionState::TopLevel => "[TopLevel]".to_string(),
        ResolutionState::Unresolved => "[Unresolved]".to_string(),
        ResolutionState::Ambiguous => "[Ambiguous]".to_string(),
        ResolutionState::CycleDetected => "[CycleDetected]".to_string(),
    }
}

fn node_label(node: &RecoveredNode) -> String {
    let name = node
        .source_name
        .as_deref()
        .filter(|name| !name.is_empty())
        .unwrap_or(node.id.as_str());
    if name == node.id.as_str() {
        format!("{name} {}", status_label(node))
    } else {
        format!("{name} {} ({})", status_label(node), node.id)
    }
}

enum Item<'a> {
    Node(&'a RecoveredNode),
    Container(&'a SyntheticContainer),
    Cycle(&'a CycleGroup),
    CycleMember(&'a NodeId, Option<&'a NodeId>),
}

impl Item<'_> {
    fn label(&self, project: &RecoveredProject) -> String {
        match self {
            Item::Node(node) => node_label(node),
            Item::Container(container) => match container.kind {
                ContainerKind::Unresolved => {
                    format!(
                        "Unresolved (synthetic, {} node(s))",
                        container.members.len()
                    )
                }
                ContainerKind::Cycles => "Cycles (synthetic)".to_string(),
            },
            Item::Cycle(cycle) => format!(
                "Cycle {} (synthetic, {} node(s))",
                cycle.index,
                cycle.nodes.len()
            ),
            Item::CycleMember(node, parent) => {
                let label = project
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == **node)
                    .map(node_label)
                    .unwrap_or_else(|| node.0.clone());
                match parent {
                    Some(parent) => format!("{label} (parent: {parent})"),
                    None => label,
                }
            }
        }
    }
}

/// Render the recovered hierarchy as an indented tree.
pub fn render_tree(project: &RecoveredProject, max_depth: usize) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "RecoveredRoot (synthetic)");
    let node_by_id: BTreeMap<&str, &RecoveredNode> = project
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let container_by_id: BTreeMap<&str, &SyntheticContainer> = project
        .containers
        .iter()
        .map(|container| (container.id.as_str(), container))
        .collect();

    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<(Item<'_>, usize)> = Vec::new();
    for child in project.root.children.iter().rev() {
        if let Some(node) = node_by_id.get(child.as_str()) {
            stack.push((Item::Node(node), 1));
        } else if let Some(container) = container_by_id.get(child.as_str()) {
            stack.push((Item::Container(container), 1));
        }
    }

    while let Some((item, depth)) = stack.pop() {
        let indent = "  ".repeat(depth.saturating_sub(1));
        match &item {
            Item::Node(node) => {
                if !visited.insert(node.id.0.clone()) {
                    let _ = writeln!(out, "{indent}{} (already shown)", node_label(node));
                    continue;
                }
                let _ = writeln!(out, "{indent}{}", node_label(node));
                if depth >= max_depth && !node.children.is_empty() {
                    let _ = writeln!(
                        out,
                        "{indent}  ... ({} more child(ren), depth limit reached)",
                        node.children.len()
                    );
                    continue;
                }
                for child in node.children.iter().rev() {
                    if let Some(child_node) = node_by_id.get(child.as_str()) {
                        stack.push((Item::Node(child_node), depth + 1));
                    }
                }
            }
            Item::Container(container) => {
                let label = item.label(project);
                let _ = writeln!(out, "{indent}{label}");
                if depth >= max_depth {
                    continue;
                }
                match container.kind {
                    ContainerKind::Unresolved => {
                        for member in container.members.iter().rev() {
                            if let Some(node) = node_by_id.get(member.as_str()) {
                                stack.push((Item::Node(node), depth + 1));
                            }
                        }
                    }
                    ContainerKind::Cycles => {
                        for cycle in project.cycles.iter().rev() {
                            stack.push((Item::Cycle(cycle), depth + 1));
                        }
                    }
                }
            }
            Item::Cycle(cycle) => {
                let label = item.label(project);
                let _ = writeln!(out, "{indent}{label}");
                if depth >= max_depth {
                    continue;
                }
                for member in cycle.nodes.iter().rev() {
                    let parent = node_by_id
                        .get(member.as_str())
                        .and_then(|node| node.parent.as_ref());
                    stack.push((Item::CycleMember(member, parent), depth + 1));
                }
            }
            Item::CycleMember(_, _) => {
                let label = item.label(project);
                let _ = writeln!(out, "{indent}{label}");
            }
        }
    }
    out
}

/// Render the hierarchy resolution statistics.
pub fn render_statistics(project: &RecoveredProject) -> String {
    let stats = &project.statistics;
    let mut out = String::new();
    let _ = writeln!(out, "Hierarchy Resolution Statistics");
    let _ = writeln!(out);
    let _ = writeln!(out, "Nodes             {:>6}", stats.nodes);
    let _ = writeln!(out, "Resolved          {:>6}", stats.resolved);
    let _ = writeln!(out, "Top level         {:>6}", stats.top_level);
    let _ = writeln!(out, "Unresolved        {:>6}", stats.unresolved);
    let _ = writeln!(out, "Ambiguous         {:>6}", stats.ambiguous);
    let _ = writeln!(out, "Cycle nodes       {:>6}", stats.cycle_detected);
    let _ = writeln!(out);
    let _ = writeln!(out, "Exact             {:>6}", stats.exact);
    let _ = writeln!(out, "Derived           {:>6}", stats.derived);
    let _ = writeln!(out, "Heuristic         {:>6}", stats.heuristic);
    let _ = writeln!(out);
    let _ = writeln!(out, "Cycles            {:>6}", stats.cycles);
    let _ = writeln!(out, "Orphans           {:>6}", stats.orphans);
    let _ = writeln!(out, "Diagnostics       {:>6}", stats.diagnostics);
    out
}

/// Render the complete human report.
pub fn render_human(project: &RecoveredProject, max_depth: usize) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Live2D Recovery - Hierarchy Reconstruction");
    let _ = writeln!(out, "==========================================");
    let _ = writeln!(out);
    let _ = writeln!(out, "Schema:        {}", project.schema);
    let _ = writeln!(
        out,
        "Source:        {} ({})",
        project.source.version_label, project.source.format
    );
    let _ = writeln!(
        out,
        "Policy:        heuristic_parenting={}",
        project.policy.allow_heuristic_parenting
    );
    let _ = writeln!(out);
    out.push_str(&render_tree(project, max_depth));
    let _ = writeln!(out);
    out.push_str(&render_statistics(project));
    let _ = writeln!(out);
    let _ = writeln!(out, "Diagnostics");
    let _ = writeln!(out, "-----------");
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for diagnostic in &project.diagnostics {
        *counts.entry(diagnostic.code.clone()).or_insert(0) += 1;
    }
    if counts.is_empty() {
        let _ = writeln!(out, "  (none)");
    } else {
        for (code, count) in &counts {
            let _ = writeln!(out, "  {code}: {count}");
        }
        let _ = writeln!(out);
        for diagnostic in project.diagnostics.iter().take(MAX_RENDERED_DIAGNOSTICS) {
            let _ = writeln!(
                out,
                "  [{:?}] {}: {}",
                diagnostic.severity, diagnostic.code, diagnostic.message
            );
        }
        if project.diagnostics.len() > MAX_RENDERED_DIAGNOSTICS {
            let _ = writeln!(
                out,
                "  ... (+{} more)",
                project.diagnostics.len() - MAX_RENDERED_DIAGNOSTICS
            );
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Note: these are structure statistics, not an accuracy measurement."
    );
    out
}

/// Render explain mode for one node (canonical id or stored source name).
pub fn render_explain(project: &RecoveredProject, query: &str) -> Option<String> {
    let node = project
        .nodes
        .iter()
        .find(|node| node.id.as_str() == query)
        .or_else(|| {
            project
                .nodes
                .iter()
                .find(|node| node.source_name.as_deref() == Some(query))
        })?;

    let mut out = String::new();
    let _ = writeln!(out, "Node:");
    let _ = writeln!(out, "{}", node.id);
    let _ = writeln!(out);
    let _ = writeln!(out, "Kind:");
    let _ = writeln!(out, "{}", node.kind.label());
    let _ = writeln!(out);
    let _ = writeln!(out, "Status:");
    let _ = writeln!(out, "{:?}", node.status);
    let _ = writeln!(out);
    match &node.trace.selected {
        Some(selected) => {
            let _ = writeln!(out, "Selected parent:");
            let _ = writeln!(out, "{}", selected.parent);
            let _ = writeln!(out);
            let _ = writeln!(out, "Confidence:");
            let _ = writeln!(out, "{}", confidence_label(selected.confidence));
            let _ = writeln!(out);
            let _ = writeln!(out, "Rule:");
            if let Some(info) = rule_info(&selected.rule) {
                let _ = writeln!(out, "{} - {}", info.id, info.description);
            } else {
                let _ = writeln!(out, "{}", selected.rule);
            }
            let _ = writeln!(out);
            let _ = writeln!(out, "Evidence:");
            if let Some(edge) = project.graph.edges.get(selected.evidence_index) {
                let _ = writeln!(
                    out,
                    "- {:?}: {} ({})",
                    edge.kind,
                    edge.note.as_deref().unwrap_or("stored relation"),
                    edge.rule
                );
            }
        }
        None => {
            let _ = writeln!(out, "Selected parent:");
            let _ = writeln!(out, "(none)");
            let _ = writeln!(out);
        }
    }
    if !node.trace.tied_candidates.is_empty() {
        let _ = writeln!(out, "Tied candidates (ambiguous):");
        for candidate in &node.trace.tied_candidates {
            let _ = writeln!(out, "- {candidate}");
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(out, "Rejected candidates:");
    if node.trace.rejected.is_empty() {
        let _ = writeln!(out, "(none)");
    } else {
        for rejected in &node.trace.rejected {
            let _ = writeln!(
                out,
                "- {} [{}] rule {}: reason: {}",
                rejected.parent,
                confidence_label(rejected.confidence),
                rejected.rule,
                rejected.reason.label()
            );
        }
    }
    if let Some(note) = &node.trace.note {
        let _ = writeln!(out);
        let _ = writeln!(out, "Note:");
        let _ = writeln!(out, "{note}");
    }
    Some(out)
}
