//! Human-readable keyform recovery report and explain output.
//!
//! These are exact structural counts, never accuracy measurements (work
//! order sections 50, 51, 72).

use std::fmt::Write as _;

use crate::model::{AxisStatus, Cardinality, GridLayout, GridOrdering, RecoveredKeyformModel};

/// Render the statistics report.
pub fn render_human(document: &RecoveredKeyformModel) -> String {
    let stats = &document.statistics;
    let mut out = String::new();
    let _ = writeln!(out, "Keyform Recovery");
    let _ = writeln!(out);
    let _ = writeln!(out, "Targets             {}", stats.targets);
    let _ = writeln!(out, "Bindings            {}", stats.bindings);
    let _ = writeln!(out, "Parameter axes      {}", stats.parameter_axes);
    let _ = writeln!(out, "Stored keyforms     {}", stats.stored_keyforms);
    let _ = writeln!(out);
    let _ = writeln!(out, "Exact bindings      {}", stats.exact_bindings);
    let _ = writeln!(out, "Derived bindings    {}", stats.derived_bindings);
    let _ = writeln!(out, "Heuristic bindings  {}", stats.heuristic_bindings);
    let _ = writeln!(out, "Unresolved bindings {}", stats.unknown_bindings);
    let _ = writeln!(out);
    let _ = writeln!(out, "Dense grids         {}", stats.dense_grids);
    let _ = writeln!(out, "Sparse grids        {}", stats.sparse_grids);
    let _ = writeln!(out, "Unknown layouts     {}", stats.unknown_layout_grids);
    let _ = writeln!(out, "Unresolved grids    {}", stats.unresolved_grids);
    let _ = writeln!(out);
    let _ = writeln!(out, "Unresolved entries  {}", stats.unresolved_entries);
    let _ = writeln!(out, "Diagnostics         {}", stats.diagnostics);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "These are exact structural counts from stored data, not accuracy measurements."
    );
    out
}

/// Render the explain view for one target, band or grid.
pub fn render_explain(document: &RecoveredKeyformModel, query: &str) -> Option<String> {
    let trace = document.traces.iter().find(|trace| {
        trace.target.id_text() == query
            || trace.band == query
            || trace.grid == query
            || document
                .grid(&trace.grid)
                .map(|grid| grid.id == query)
                .unwrap_or(false)
    })?;

    let mut out = String::new();
    let _ = writeln!(out, "Target:");
    let _ = writeln!(
        out,
        "  {} ({})",
        trace.target.id_text(),
        trace.target.kind_label()
    );
    let _ = writeln!(out, "Band:");
    let _ = writeln!(out, "  {}", trace.band);
    let _ = writeln!(out, "Grid:");
    let _ = writeln!(out, "  {} ({})", trace.grid, layout_label(trace.layout));
    let _ = writeln!(out, "Axes:");
    if trace.axes.is_empty() {
        let _ = writeln!(out, "  (none; static target)");
    }
    for axis in &trace.axes {
        let _ = writeln!(
            out,
            "  {} [{}] ({} key(s), {})",
            axis.parameter.as_str(),
            render_keys(&axis.keys, &axis.non_finite_key_indices),
            axis.key_count,
            status_label(axis.status)
        );
    }
    let _ = writeln!(out, "Expected Cartesian:");
    let _ = writeln!(out, "  {}", render_cardinality(trace.expected_cardinality));
    let _ = writeln!(out, "Stored Forms:");
    let _ = writeln!(out, "  {}", trace.stored_form_count);
    let _ = writeln!(out, "Grid ordering:");
    let _ = writeln!(out, "  {}", ordering_label(trace.ordering));
    let _ = writeln!(out, "Confidence:");
    let _ = writeln!(out, "  {:?}", trace.confidence);
    let _ = writeln!(out, "Rules:");
    let rules: Vec<&str> = trace.rules.iter().map(|rule| rule.as_str()).collect();
    let _ = writeln!(out, "  {}", rules.join(", "));
    if !trace.notes.is_empty() {
        let _ = writeln!(out, "Notes:");
        for note in &trace.notes {
            let _ = writeln!(out, "  {note}");
        }
    }
    Some(out)
}

fn render_keys(keys: &[f32], non_finite: &[usize]) -> String {
    let mut parts: Vec<String> = keys.iter().map(|key| format!("{key}")).collect();
    for position in non_finite {
        parts.insert(*position, "<non-finite>".to_string());
    }
    parts.join(", ")
}

fn render_cardinality(cardinality: Cardinality) -> String {
    match cardinality {
        Cardinality::Exact { value } => value.to_string(),
        Cardinality::Overflow => "overflow (unresolved)".to_string(),
        Cardinality::LimitExceeded => "above hard limit (unresolved)".to_string(),
    }
}

fn layout_label(layout: GridLayout) -> &'static str {
    match layout {
        GridLayout::Dense => "dense",
        GridLayout::Sparse => "sparse",
        GridLayout::UnknownLayout => "unknown layout",
    }
}

fn ordering_label(ordering: GridOrdering) -> &'static str {
    match ordering {
        GridOrdering::Observed => "observed",
        GridOrdering::Derived => "derived from the single stored sequence",
        GridOrdering::Unknown => "unknown (not guessed)",
    }
}

fn status_label(status: AxisStatus) -> &'static str {
    match status {
        AxisStatus::Resolved => "resolved",
        AxisStatus::Dangling => "dangling",
        AxisStatus::NonFiniteKeys => "non-finite keys",
        AxisStatus::Experimental => "experimental",
    }
}
