//! Differential comparison engine.
//!
//! Every scalar field is flattened to a canonical path and compared across
//! providers. Order-only differences are reported separately from semantic
//! mismatches, and multi-provider consensus is classified per field.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::providers::ReferenceMetadata;
use crate::snapshot::{floats_equal, DifferentialSnapshot, InputInfo};

/// Status of one compared field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FieldStatus {
    /// Values agree.
    Match,
    /// Values disagree.
    Mismatch,
    /// Same set, different order only.
    OrderOnlyDifference,
    /// Ours has no value.
    MissingOurs,
    /// Every reference lacks the value while ours has it.
    MissingReference,
    /// Every reference marks the group unsupported.
    UnsupportedReference,
    /// Provider failed for this comparison.
    NotComparable,
}

/// Consensus verdict across providers for one field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConsensusVerdict {
    /// All supporting references agree with ours.
    Consensus,
    /// At least one supporting reference disagrees with ours.
    Disputed,
    /// Ours has the value but no reference supports the comparison.
    Insufficient,
    /// No reference could supply the field.
    Unsupported,
    /// Ours is missing entirely.
    MissingOurs,
}

/// One compared field.
#[derive(Debug, Clone, Serialize)]
pub struct FieldComparison {
    /// Canonical field path (for example `parts.Part_A.parent`).
    pub field: String,
    /// Aggregate status.
    pub status: FieldStatus,
    /// Our value (rendered).
    pub ours: Option<String>,
    /// Reference values by provider (rendered; `None` = missing).
    pub reference: BTreeMap<String, Option<String>>,
    /// Consensus verdict.
    pub consensus: ConsensusVerdict,
    /// Extra detail lines (for example mismatch excerpts).
    pub details: Vec<String>,
}

/// Provider outcome summary.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderSummary {
    /// Provider name.
    pub name: String,
    /// Pinned metadata.
    pub metadata: ReferenceMetadata,
    /// `ok` or the provider error kind.
    pub outcome: String,
    /// Error detail when the provider failed.
    pub error: Option<String>,
}

/// Aggregate statistics (never called accuracy).
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct AgreementStats {
    /// Comparable field count.
    pub comparable_fields: u64,
    /// Agreement count.
    pub agreement: u64,
    /// Disagreement count.
    pub disagreement: u64,
    /// Order-only differences.
    pub order_only: u64,
    /// Fields missing on our side.
    pub missing_ours: u64,
    /// Fields no reference could supply.
    pub missing_reference: u64,
    /// Fields marked unsupported by all references.
    pub unsupported: u64,
    /// Fields not comparable because every provider failed.
    pub not_comparable: u64,
}

/// Full comparison report.
#[derive(Debug, Clone, Serialize)]
pub struct ComparisonReport {
    /// Input identity.
    pub input: InputInfo,
    /// Our commit (best effort).
    pub our_commit: String,
    /// Provider summaries (ours first).
    pub providers: Vec<ProviderSummary>,
    /// Compared fields (sorted by path).
    pub fields: Vec<FieldComparison>,
    /// Statistics.
    pub statistics: AgreementStats,
    /// Fields whose consensus is DISPUTED.
    pub disputed: Vec<String>,
    /// Notes about caps or provider issues.
    pub notes: Vec<String>,
    /// Real-world validation status marker (never an accuracy claim).
    pub validation_status: String,
    /// Fixed non-accuracy disclaimer.
    pub disclaimer: String,
}

/// Hard cap on compared fields (keeps hostile inputs from exploding output).
pub const MAX_COMPARED_FIELDS: usize = 20_000;

/// Maximum detail lines per comparison.
const MAX_DETAILS: usize = 8;

/// Real-world validation status carried by every report.
pub const VALIDATION_STATUS: &str =
    "UNVALIDATED (no owned real .moc3; cross-implementation comparison only)";

/// Fixed disclaimer; agreement is never an accuracy measurement.
pub const NON_ACCURACY_DISCLAIMER: &str =
    "These numbers are a cross-implementation comparison, not a recovery accuracy measurement.";

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Int(i64),
    Float(f64),
    Text(String),
    List(Vec<String>),
}

impl Value {
    fn render(&self) -> String {
        match self {
            Value::Int(value) => value.to_string(),
            Value::Float(value) => format!("{value}"),
            Value::Text(value) => value.clone(),
            Value::List(values) => format!("[{}]", values.join(", ")),
        }
    }

    fn equivalent(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(left), Value::Int(right)) => left == right,
            (Value::Float(left), Value::Float(right)) => floats_equal(*left, *right),
            (Value::Text(left), Value::Text(right)) => left == right,
            (Value::List(left), Value::List(right)) => {
                let left_set: BTreeSet<&String> = left.iter().collect();
                let right_set: BTreeSet<&String> = right.iter().collect();
                left_set == right_set
            }
            _ => false,
        }
    }

    /// Ordered equality: scalar equivalences plus exact list order.
    fn ordered_equivalent(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::List(left), Value::List(right)) => left == right,
            _ => self.equivalent(other),
        }
    }
}

fn flatten(snapshot: &DifferentialSnapshot) -> BTreeMap<String, Value> {
    let mut fields: BTreeMap<String, Value> = BTreeMap::new();
    if let Some(version) = snapshot.moc3_version {
        fields.insert("moc3_version".to_string(), Value::Int(i64::from(version)));
    }
    if let Some(endian) = &snapshot.endian {
        fields.insert("endian".to_string(), Value::Text(endian.clone()));
    }
    if let Some(canvas) = &snapshot.canvas {
        fields.insert("canvas.width".to_string(), Value::Float(canvas.width));
        fields.insert("canvas.height".to_string(), Value::Float(canvas.height));
        fields.insert(
            "canvas.pixels_per_unit".to_string(),
            Value::Float(canvas.pixels_per_unit),
        );
        fields.insert("canvas.origin_x".to_string(), Value::Float(canvas.origin_x));
        fields.insert("canvas.origin_y".to_string(), Value::Float(canvas.origin_y));
    }
    for (name, value) in &snapshot.counts {
        fields.insert(format!("counts.{name}"), Value::Int(*value as i64));
    }
    fields.insert(
        "parameters.ids".to_string(),
        Value::List(snapshot.parameters.iter().map(|p| p.id.clone()).collect()),
    );
    for parameter in &snapshot.parameters {
        fields.insert(
            format!("parameters.{}.minimum", parameter.id),
            Value::Float(parameter.minimum),
        );
        fields.insert(
            format!("parameters.{}.maximum", parameter.id),
            Value::Float(parameter.maximum),
        );
        fields.insert(
            format!("parameters.{}.default", parameter.id),
            Value::Float(parameter.default),
        );
    }
    fields.insert(
        "parts.ids".to_string(),
        Value::List(snapshot.parts.iter().map(|part| part.id.clone()).collect()),
    );
    for part in &snapshot.parts {
        fields.insert(
            format!("parts.{}.parent", part.id),
            Value::Text(part.parent.clone().unwrap_or_else(|| "-".to_string())),
        );
    }
    fields.insert(
        "deformers.ids".to_string(),
        Value::List(snapshot.deformers.iter().map(|d| d.id.clone()).collect()),
    );
    for deformer in &snapshot.deformers {
        fields.insert(
            format!("deformers.{}.kind", deformer.id),
            Value::Text(deformer.kind.clone()),
        );
        fields.insert(
            format!("deformers.{}.parent_deformer", deformer.id),
            Value::Text(
                deformer
                    .parent_deformer
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
        );
        fields.insert(
            format!("deformers.{}.parent_part", deformer.id),
            Value::Text(
                deformer
                    .parent_part
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
        );
    }
    fields.insert(
        "art_meshes.ids".to_string(),
        Value::List(snapshot.art_meshes.iter().map(|m| m.id.clone()).collect()),
    );
    for mesh in &snapshot.art_meshes {
        fields.insert(
            format!("art_meshes.{}.parent_part", mesh.id),
            Value::Text(mesh.parent_part.clone().unwrap_or_else(|| "-".to_string())),
        );
        fields.insert(
            format!("art_meshes.{}.parent_deformer", mesh.id),
            Value::Text(
                mesh.parent_deformer
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ),
        );
        fields.insert(
            format!("art_meshes.{}.texture", mesh.id),
            Value::Int(mesh.texture.unwrap_or(-1)),
        );
        fields.insert(
            format!("art_meshes.{}.vertex_count", mesh.id),
            Value::Int(mesh.vertex_count),
        );
        fields.insert(
            format!("art_meshes.{}.uv_count", mesh.id),
            Value::Int(mesh.uv_count),
        );
        fields.insert(
            format!("art_meshes.{}.index_count", mesh.id),
            Value::Int(mesh.index_count),
        );
        fields.insert(
            format!("art_meshes.{}.mask_refs", mesh.id),
            Value::List(
                mesh.mask_refs
                    .iter()
                    .map(|value| value.to_string())
                    .collect(),
            ),
        );
    }
    fields.insert(
        "hierarchy_edges".to_string(),
        Value::List(
            snapshot
                .hierarchy_edges
                .iter()
                .map(|edge| format!("{}->{}({})", edge.child, edge.parent, edge.relation))
                .collect(),
        ),
    );
    fields
}

fn group_of(field: &str) -> &str {
    field.split('.').next().unwrap_or(field)
}

fn is_unsupported(provider: &DifferentialSnapshot, field: &str) -> bool {
    let group = group_of(field);
    provider
        .unsupported
        .keys()
        .any(|key| key == group || field == key || field.starts_with(&format!("{key}.")))
}

/// Compare ours against reference providers.
///
/// `providers` lists every provider (ours first) for the report header while
/// `references` contains only the external snapshots to compare against
/// (name, snapshot); ours must never be part of `references`.
pub fn compare(
    input: &InputInfo,
    our_commit: &str,
    ours: Option<&DifferentialSnapshot>,
    providers: &[ProviderSummary],
    references: &[(&str, Option<&DifferentialSnapshot>)],
) -> ComparisonReport {
    let mut notes: Vec<String> = Vec::new();
    let our_fields = ours.map(flatten).unwrap_or_default();
    let mut reference_fields: Vec<BTreeMap<String, Value>> = Vec::new();
    for (_, snapshot) in references {
        if let Some(snapshot) = snapshot {
            reference_fields.push(flatten(snapshot));
        }
    }

    // Union of field paths across all snapshots.
    let mut all_fields: BTreeSet<String> = our_fields.keys().cloned().collect();
    for fields in &reference_fields {
        all_fields.extend(fields.keys().cloned());
    }
    if all_fields.len() > MAX_COMPARED_FIELDS {
        notes.push(format!(
            "comparison capped at {MAX_COMPARED_FIELDS} fields (input produced {})",
            all_fields.len()
        ));
        all_fields = all_fields.into_iter().take(MAX_COMPARED_FIELDS).collect();
    }

    let mut fields: Vec<FieldComparison> = Vec::with_capacity(all_fields.len());
    let mut statistics = AgreementStats::default();
    let mut reference_index: usize;
    for field in &all_fields {
        let ours_value = our_fields.get(field);
        let mut reference_values: BTreeMap<String, Option<String>> = BTreeMap::new();
        let mut supported_values: Vec<Value> = Vec::new();
        let mut unsupported_count = 0usize;
        let mut failed_count = 0usize;
        reference_index = 0;
        for (name, snapshot) in references {
            match snapshot {
                None => {
                    failed_count += 1;
                    reference_values.insert((*name).to_string(), None);
                }
                Some(snapshot) => {
                    if is_unsupported(snapshot, field) {
                        unsupported_count += 1;
                        reference_values.insert((*name).to_string(), None);
                        reference_index += 1;
                        continue;
                    }
                    let value = reference_fields
                        .get(reference_index)
                        .and_then(|fields| fields.get(field))
                        .cloned();
                    reference_values.insert((*name).to_string(), value.as_ref().map(Value::render));
                    if let Some(value) = value {
                        supported_values.push(value);
                    }
                    reference_index += 1;
                }
            }
        }

        let ours_unsupported = ours.is_some_and(|snapshot| is_unsupported(snapshot, field));
        let status = match ours_value {
            None => {
                if ours_unsupported && !supported_values.is_empty() {
                    FieldStatus::UnsupportedReference
                } else if supported_values.is_empty() {
                    FieldStatus::NotComparable
                } else {
                    FieldStatus::MissingOurs
                }
            }
            Some(ours_value) => {
                if supported_values.is_empty() {
                    if failed_count > 0 && unsupported_count == 0 {
                        FieldStatus::NotComparable
                    } else if unsupported_count > 0 {
                        FieldStatus::UnsupportedReference
                    } else {
                        FieldStatus::MissingReference
                    }
                } else if !supported_values
                    .iter()
                    .all(|value| value.equivalent(ours_value))
                {
                    FieldStatus::Mismatch
                } else if supported_values
                    .iter()
                    .all(|value| value.ordered_equivalent(ours_value))
                {
                    FieldStatus::Match
                } else {
                    FieldStatus::OrderOnlyDifference
                }
            }
        };

        let consensus = if ours_unsupported && ours_value.is_none() {
            ConsensusVerdict::Unsupported
        } else if ours_value.is_none() {
            ConsensusVerdict::MissingOurs
        } else if supported_values.is_empty() {
            if unsupported_count > 0 && failed_count == 0 {
                ConsensusVerdict::Unsupported
            } else {
                ConsensusVerdict::Insufficient
            }
        } else if supported_values
            .iter()
            .all(|value| value.equivalent(ours_value.unwrap_or(&Value::Int(0))))
        {
            ConsensusVerdict::Consensus
        } else {
            ConsensusVerdict::Disputed
        };

        let mut details: Vec<String> = Vec::new();
        if status == FieldStatus::Mismatch {
            let mut lines = 0usize;
            for (name, rendered) in &reference_values {
                if lines >= MAX_DETAILS {
                    break;
                }
                if let Some(rendered) = rendered {
                    let our_render = ours_value.map(Value::render).unwrap_or_default();
                    if rendered != &our_render {
                        details.push(format!("{name}: {rendered}"));
                        lines += 1;
                    }
                }
            }
        }

        match status {
            FieldStatus::Match | FieldStatus::OrderOnlyDifference => statistics.agreement += 1,
            FieldStatus::Mismatch => statistics.disagreement += 1,
            FieldStatus::MissingOurs => statistics.missing_ours += 1,
            FieldStatus::MissingReference => statistics.missing_reference += 1,
            FieldStatus::UnsupportedReference => statistics.unsupported += 1,
            FieldStatus::NotComparable => statistics.not_comparable += 1,
        }
        if status == FieldStatus::OrderOnlyDifference {
            statistics.order_only += 1;
        }
        if status != FieldStatus::NotComparable {
            statistics.comparable_fields += 1;
        }

        fields.push(FieldComparison {
            field: field.clone(),
            status,
            ours: ours_value.map(Value::render),
            reference: reference_values,
            consensus,
            details,
        });
    }

    let disputed: Vec<String> = fields
        .iter()
        .filter(|field| field.consensus == ConsensusVerdict::Disputed)
        .map(|field| field.field.clone())
        .collect();

    ComparisonReport {
        input: input.clone(),
        our_commit: our_commit.to_string(),
        providers: providers.to_vec(),
        fields,
        statistics,
        disputed,
        notes,
        validation_status: VALIDATION_STATUS.to_string(),
        disclaimer: NON_ACCURACY_DISCLAIMER.to_string(),
    }
}

/// Group-level rollup used by the human report.
pub fn group_rollup(report: &ComparisonReport) -> Vec<(String, FieldStatus, usize, usize)> {
    fn severity(status: FieldStatus) -> u8 {
        match status {
            FieldStatus::Match => 0,
            FieldStatus::OrderOnlyDifference => 1,
            FieldStatus::UnsupportedReference => 2,
            FieldStatus::NotComparable => 3,
            FieldStatus::MissingReference => 4,
            FieldStatus::MissingOurs => 5,
            FieldStatus::Mismatch => 6,
        }
    }
    let mut groups: BTreeMap<String, (FieldStatus, usize, usize)> = BTreeMap::new();
    for field in &report.fields {
        let group = group_of(&field.field).to_string();
        let entry = groups.entry(group).or_insert((FieldStatus::Match, 0, 0));
        entry.1 += 1;
        if matches!(
            field.status,
            FieldStatus::Match | FieldStatus::OrderOnlyDifference
        ) {
            entry.2 += 1;
        }
        if severity(field.status) > severity(entry.0) {
            entry.0 = field.status;
        }
    }
    groups
        .into_iter()
        .map(|(group, (status, total, matched))| (group, status, matched, total))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::is_unsupported;
    use crate::snapshot::{DifferentialSnapshot, InputInfo};
    use std::collections::BTreeMap;

    fn snapshot_with_unsupported(key: &str) -> DifferentialSnapshot {
        let mut unsupported = BTreeMap::new();
        unsupported.insert(key.to_string(), "test".to_string());
        DifferentialSnapshot {
            schema: "test".to_string(),
            provider: "test".to_string(),
            input: InputInfo {
                file_name: "t".to_string(),
                size: 0,
                fnv1a64: "0".to_string(),
            },
            moc3_version: None,
            endian: None,
            canvas: None,
            counts: BTreeMap::new(),
            parameters: Vec::new(),
            parts: Vec::new(),
            deformers: Vec::new(),
            art_meshes: Vec::new(),
            hierarchy_edges: Vec::new(),
            diagnostics: Vec::new(),
            unsupported,
        }
    }

    #[test]
    fn unsupported_keys_require_a_segment_boundary() {
        let exact = snapshot_with_unsupported("counts.parameter");
        assert!(is_unsupported(&exact, "counts.parameter"));
        assert!(is_unsupported(&exact, "counts.parameter.values"));
        assert!(!is_unsupported(&exact, "counts.parameter_bindings"));
        let group = snapshot_with_unsupported("counts");
        assert!(is_unsupported(&group, "counts.anything.else"));
    }
}
