//! Dev-only differential validation harness (AGENT.3.5).
//!
//! Compares our pipeline against independent external MOC3 implementations by
//! normalizing every provider into a [`snapshot::DifferentialSnapshot`] and
//! diffing the fields. External tools run as subprocesses with a hard timeout
//! and output caps; this crate is a tool and is **never** a production
//! dependency.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod compare;
pub mod providers;
pub mod report;
pub mod snapshot;

use std::path::Path;

use crate::compare::{compare, ComparisonReport, ProviderSummary};
use crate::providers::{pinned_metadata, provider_by_name, ProviderConfig, ReferenceMetadata};
use crate::snapshot::{input_info, DifferentialSnapshot, SNAPSHOT_SCHEMA};

pub use providers::{ProviderError, ReferenceProvider};

/// Run the differential comparison for one input.
pub fn run_compare(
    input: &Path,
    provider_names: &[String],
    config: &ProviderConfig,
    our_commit: &str,
) -> Result<ComparisonReport, String> {
    let metadata = std::fs::metadata(input).map_err(|error| format!("input: {error}"))?;
    let limit = 512u64 * 1024 * 1024;
    if metadata.len() > limit {
        return Err(format!(
            "input exceeds the {limit} byte cap for differential runs"
        ));
    }
    let bytes = std::fs::read(input).map_err(|error| format!("input: {error}"))?;
    let input_info = input_info(input, &bytes);

    let mut names: Vec<String> = provider_names.to_vec();
    if !names.iter().any(|name| name == "ours") {
        names.insert(0, "ours".to_string());
    }

    let mut ours: Option<DifferentialSnapshot> = None;
    let mut notes: Vec<String> = Vec::new();
    let mut summaries: Vec<ProviderSummary> = Vec::new();
    // External snapshots only; ours is never compared against itself.
    let mut external: Vec<(String, Option<DifferentialSnapshot>)> = Vec::new();

    for name in &names {
        let Some(provider) = provider_by_name(name) else {
            summaries.push(ProviderSummary {
                name: name.clone(),
                metadata: ReferenceMetadata {
                    name: name.clone(),
                    kind: "unknown".to_string(),
                    version: "-".to_string(),
                    commit: "-".to_string(),
                    license: "-".to_string(),
                    repository: "-".to_string(),
                    production_dependency: false,
                },
                outcome: "UNKNOWN_PROVIDER".to_string(),
                error: Some(format!("no provider named '{name}'")),
            });
            continue;
        };
        let metadata = provider.metadata();
        match provider.snapshot(input, &bytes, config) {
            Ok(snapshot) => {
                if snapshot.schema != SNAPSHOT_SCHEMA {
                    notes.push(format!(
                        "{name}: snapshot schema '{}' differs from '{SNAPSHOT_SCHEMA}'",
                        snapshot.schema
                    ));
                }
                if name == "ours" {
                    ours = Some(snapshot.clone());
                } else {
                    external.push((name.clone(), Some(snapshot.clone())));
                }
                summaries.push(ProviderSummary {
                    name: name.clone(),
                    metadata,
                    outcome: "ok".to_string(),
                    error: None,
                });
            }
            Err(error) => {
                let outcome = match &error {
                    ProviderError::NotConfigured { .. } => "REFERENCE_NOT_CONFIGURED",
                    ProviderError::Unsupported { .. } => "REFERENCE_UNSUPPORTED",
                    ProviderError::ReferenceError { .. } => "REFERENCE_ERROR",
                    ProviderError::ReferenceTimeout { .. } => "REFERENCE_TIMEOUT",
                    ProviderError::OutputLimitExceeded { .. } => "REFERENCE_OUTPUT_LIMIT",
                };
                notes.push(format!("{name}: {outcome}: {error}"));
                if name != "ours" {
                    external.push((name.clone(), None));
                }
                summaries.push(ProviderSummary {
                    name: name.clone(),
                    metadata,
                    outcome: outcome.to_string(),
                    error: Some(error.to_string()),
                });
            }
        }
    }

    let external_refs: Vec<(&str, Option<&DifferentialSnapshot>)> = external
        .iter()
        .map(|(name, snapshot)| (name.as_str(), snapshot.as_ref()))
        .collect();
    let mut report = compare(
        &input_info,
        our_commit,
        ours.as_ref(),
        &summaries,
        &external_refs,
    );
    report.notes.extend(notes);
    Ok(report)
}

/// Pinned metadata for every audited reference.
pub fn pinned_references() -> Vec<ReferenceMetadata> {
    providers::pinned_references()
}

/// Lookup helper re-exported for tooling.
pub fn metadata_for(name: &str) -> Option<ReferenceMetadata> {
    pinned_metadata(name)
}
