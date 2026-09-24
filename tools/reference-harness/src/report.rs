//! Human and canonical JSON rendering of differential reports.

use std::fmt::Write as _;

use crate::compare::{group_rollup, ComparisonReport, ConsensusVerdict, FieldStatus};

/// Render the §47-style human differential report.
pub fn render_human(report: &ComparisonReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "=== DIFFERENTIAL REPORT ===");
    let _ = writeln!(out);
    let _ = writeln!(out, "Input:      {}", report.input.file_name);
    let _ = writeln!(out, "Size:       {} bytes", report.input.size);
    let _ = writeln!(out, "Fingerprint: fnv1a64:{}", report.input.fnv1a64);
    let _ = writeln!(out, "Our commit: {}", report.our_commit);
    let _ = writeln!(out);
    let _ = writeln!(out, "Providers:");
    for provider in &report.providers {
        let _ = writeln!(
            out,
            "  {} | {} {} | commit {} | license {} | {}",
            provider.name,
            provider.metadata.kind,
            provider.metadata.version,
            provider.metadata.commit,
            provider.metadata.license,
            provider.outcome
        );
        if let Some(error) = &provider.error {
            let _ = writeln!(out, "    {}", error);
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "Field groups:");
    for (group, status, matched, total) in group_rollup(report) {
        let label = match status {
            FieldStatus::Match => "MATCH",
            FieldStatus::OrderOnlyDifference => "MATCH (order-only differences)",
            FieldStatus::Mismatch => "MISMATCH",
            FieldStatus::MissingOurs => "MISSING_OURS",
            FieldStatus::MissingReference => "MISSING_REFERENCE",
            FieldStatus::UnsupportedReference => "UNSUPPORTED_REFERENCE",
            FieldStatus::NotComparable => "NOT_COMPARABLE",
        };
        let _ = writeln!(out, "  {group}: {label} {matched}/{total}");
    }
    let _ = writeln!(out);

    if !report.disputed.is_empty() {
        let _ = writeln!(out, "Disputed fields ({}):", report.disputed.len());
        for field in &report.disputed {
            let Some(entry) = report.fields.iter().find(|entry| &entry.field == field) else {
                continue;
            };
            let _ = writeln!(out, "  {}", entry.field);
            let _ = writeln!(
                out,
                "    ours: {}",
                entry
                    .ours
                    .clone()
                    .unwrap_or_else(|| "(missing)".to_string())
            );
            for (name, value) in &entry.reference {
                let _ = writeln!(
                    out,
                    "    {name}: {}",
                    value
                        .clone()
                        .unwrap_or_else(|| "(unsupported/missing)".to_string())
                );
            }
            for detail in &entry.details {
                let _ = writeln!(out, "    detail: {detail}");
            }
            let _ = writeln!(
                out,
                "    status: {:?} / action: manual evidence investigation",
                entry.consensus
            );
        }
        let _ = writeln!(out);
    }

    let stats = &report.statistics;
    let _ = writeln!(out, "Cross-Implementation Agreement");
    let _ = writeln!(out, "------------------------------");
    let _ = writeln!(out, "Comparable fields  {}", stats.comparable_fields);
    let _ = writeln!(out, "Agreement          {}", stats.agreement);
    let _ = writeln!(out, "Disagreement       {}", stats.disagreement);
    let _ = writeln!(out, "Order-only         {}", stats.order_only);
    let _ = writeln!(out, "Missing (ours)     {}", stats.missing_ours);
    let _ = writeln!(out, "Missing (reference){}", stats.missing_reference);
    let _ = writeln!(out, "Unsupported        {}", stats.unsupported);
    let _ = writeln!(out, "Not comparable     {}", stats.not_comparable);
    let unresolved = report
        .fields
        .iter()
        .filter(|field| field.consensus == ConsensusVerdict::Insufficient)
        .count();
    let _ = writeln!(out, "Unresolved         {unresolved}");
    if !report.notes.is_empty() {
        let _ = writeln!(out);
        for note in &report.notes {
            let _ = writeln!(out, "note: {note}");
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "These numbers are a cross-implementation comparison, not a recovery accuracy measurement."
    );
    out
}

/// Canonical JSON rendering.
pub fn to_json(report: &ComparisonReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}
