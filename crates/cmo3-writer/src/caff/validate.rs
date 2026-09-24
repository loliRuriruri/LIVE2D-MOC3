//! Structural CAFF validation (work order section 13/26).

use std::collections::BTreeSet;

use super::decoder::decode;
use super::entry::{MAX_ENTRIES, MAX_ENTRY_SIZE};
use super::error::{CaffError, CaffResult};

/// Severity of an archive finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    /// The archive is invalid.
    Fatal,
    /// Suspicious but decodable.
    Warning,
}

/// One archive validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveFinding {
    /// Stable code.
    pub code: &'static str,
    /// Severity.
    pub severity: FindingSeverity,
    /// Detail.
    pub detail: String,
}

/// Validation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveValidation {
    /// Archive length.
    pub size: usize,
    /// Entry count.
    pub entry_count: usize,
    /// Guard bytes correct.
    pub guard_ok: bool,
    /// Entries whose RAW payload extracted cleanly.
    pub decoded_payloads: usize,
    /// Findings (empty means structurally valid).
    pub findings: Vec<ArchiveFinding>,
}

impl ArchiveValidation {
    /// True when no Fatal finding exists.
    pub fn is_valid(&self) -> bool {
        !self
            .findings
            .iter()
            .any(|finding| finding.severity == FindingSeverity::Fatal)
    }
}

/// Validate a CAFF archive: header fields, table, offsets, guard bytes and
/// payload extraction for RAW entries.
pub fn validate_archive(bytes: &[u8]) -> CaffResult<ArchiveValidation> {
    let archive = decode(bytes)?;
    let mut findings: Vec<ArchiveFinding> = Vec::new();

    if !archive.guard_ok {
        findings.push(ArchiveFinding {
            code: "caff_bad_guard",
            severity: FindingSeverity::Fatal,
            detail: "guard bytes are not [98, 99]".to_string(),
        });
    }
    if archive.header.format_id != *b"----" {
        findings.push(ArchiveFinding {
            code: "caff_format_id",
            severity: FindingSeverity::Warning,
            detail: format!(
                "format id is {:?}, expected '----' for .cmo3",
                String::from_utf8_lossy(&archive.header.format_id)
            ),
        });
    }
    if archive.header.archive_version != [0, 0, 0] || archive.header.format_version != [0, 0, 0] {
        findings.push(ArchiveFinding {
            code: "caff_unknown_version",
            severity: FindingSeverity::Warning,
            detail: "archive/format version bytes are not all zero".to_string(),
        });
    }
    if archive.header.entry_count as usize != archive.entries.len() {
        findings.push(ArchiveFinding {
            code: "caff_entry_count",
            severity: FindingSeverity::Fatal,
            detail: "declared entry count differs from the table".to_string(),
        });
    }
    if archive.entries.len() > MAX_ENTRIES {
        findings.push(ArchiveFinding {
            code: "caff_too_many_entries",
            severity: FindingSeverity::Fatal,
            detail: format!("more than {MAX_ENTRIES} entries"),
        });
    }

    let mut paths: BTreeSet<&str> = BTreeSet::new();
    let mut decoded_payloads = 0usize;
    for entry in &archive.entries {
        if !paths.insert(entry.path.as_str()) {
            findings.push(ArchiveFinding {
                code: "caff_duplicate_path",
                severity: FindingSeverity::Fatal,
                detail: format!("duplicate entry path '{}'", entry.path),
            });
        }
        if entry.path.is_empty() {
            findings.push(ArchiveFinding {
                code: "caff_empty_path",
                severity: FindingSeverity::Fatal,
                detail: "entry with empty path".to_string(),
            });
        }
        if entry.path.is_empty()
            || entry.path.contains("..")
            || !entry
                .path
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
        {
            findings.push(ArchiveFinding {
                code: "caff_unsafe_path",
                severity: FindingSeverity::Fatal,
                detail: format!("entry path '{}' is not a safe internal name", entry.path),
            });
        }
        if u64::from(entry.size) > MAX_ENTRY_SIZE {
            findings.push(ArchiveFinding {
                code: "caff_entry_too_large",
                severity: FindingSeverity::Fatal,
                detail: format!("entry '{}' exceeds the size cap", entry.path),
            });
        }
    }
    for payload in archive.payloads.iter() {
        if payload.is_some() {
            decoded_payloads += 1;
        }
    }

    if archive.entry("main.xml").is_none() {
        findings.push(ArchiveFinding {
            code: "caff_main_xml_missing",
            severity: FindingSeverity::Fatal,
            detail: "archive has no 'main.xml' entry".to_string(),
        });
    }

    Ok(ArchiveValidation {
        size: bytes.len(),
        entry_count: archive.entries.len(),
        guard_ok: archive.guard_ok,
        decoded_payloads,
        findings,
    })
}

/// Convert decode failures into a structured finding list for reports.
pub fn describe_error(error: &CaffError) -> String {
    error.to_string()
}
