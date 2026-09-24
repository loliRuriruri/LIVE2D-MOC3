//! Structured, panic-free error type for MOC3 ingestion.
//!
//! Every error variant maps to a stable `code()` string so that CLI output and
//! machine consumers can rely on it. Error messages carry three things the
//! master spec asks for: the offending section/field, an offset when known,
//! and a recovery suggestion.

use std::fmt;

/// Machine-readable error category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// File does not start with `MOC3` or header fields are not usable.
    InvalidHeader {
        /// Human-readable reason.
        reason: &'static str,
    },
    /// Version byte outside the supported range (see [`crate::MocVersion`]).
    UnsupportedVersion {
        /// Version byte found in the file.
        version: u8,
        /// Highest version byte this parser understands.
        max_supported: u8,
    },
    /// A read would pass the end of the file.
    UnexpectedEof {
        /// File offset where the read started.
        offset: u64,
        /// Number of bytes the read needed.
        needed: u64,
        /// Number of bytes actually available from `offset`.
        available: u64,
    },
    /// A section offset table entry points outside the file.
    SectionTableOutOfRange {
        /// Slot index inside the offset table.
        slot: usize,
        /// Raw value stored in the slot.
        value: u32,
        /// Total file length.
        file_len: u64,
    },
    /// A section whose element type requires alignment is not aligned.
    MisalignedSection {
        /// Canonical section name.
        section: &'static str,
        /// Offset found in the table.
        offset: u64,
        /// Required alignment in bytes.
        alignment: u64,
    },
    /// A section (including its declared element count) does not fit the file.
    SectionOutOfBounds {
        /// Canonical section name.
        section: &'static str,
        /// Offset found in the table.
        offset: u64,
        /// Byte size implied by the element count.
        size: u64,
        /// Total file length.
        file_len: u64,
    },
    /// Section data offsets are not monotonically increasing.
    SectionsNotMonotonic {
        /// Canonical section name that broke monotonicity.
        section: &'static str,
        /// Offset found in the table.
        offset: u64,
        /// End of the previously seen section.
        previous_end: u64,
    },
    /// A count info field is negative or otherwise impossible.
    InvalidCount {
        /// Count field name.
        field: &'static str,
        /// Raw value.
        value: i64,
    },
    /// `warp + rotation != deformers` in the count info table.
    DeformerCountMismatch {
        /// Warp deformer count.
        warps: i64,
        /// Rotation deformer count.
        rotations: i64,
        /// Total deformer count.
        deformers: i64,
    },
    /// A count exceeds the configured safety limit.
    LimitExceeded {
        /// Count field name.
        field: &'static str,
        /// Value found.
        count: u64,
        /// Configured maximum.
        limit: u64,
    },
    /// An index value points outside its target array.
    InvalidReference {
        /// Field name (for example `part.parent_part`).
        field: &'static str,
        /// Element index inside the field array.
        index: usize,
        /// Value found.
        value: i64,
        /// Exclusive upper bound (or cardinality) of the target array.
        bound: u64,
    },
    /// A `(begin, count)` range does not fit its target array.
    InvalidRange {
        /// Field name (for example `part.keyform`).
        field: &'static str,
        /// Element index inside the field array.
        index: usize,
        /// Begin value.
        begin: i64,
        /// Count value.
        count: i64,
        /// Total size of the target array.
        total: u64,
    },
    /// A scalar value is invalid for structural reasons.
    InvalidValue {
        /// Field name.
        field: &'static str,
        /// Element index inside the field array.
        index: usize,
        /// Value found.
        value: i64,
        /// Short reason string.
        reason: &'static str,
    },
    /// A pre-limit check passed but the allocation could not be reserved.
    AllocationFailed {
        /// Field name being allocated.
        field: &'static str,
        /// Element count requested.
        count: u64,
    },
    /// The file uses a documented feature this parser does not support yet.
    UnsupportedFeature {
        /// Feature description.
        feature: &'static str,
    },
    /// Internal invariant violation; never expected in practice.
    Internal {
        /// What went wrong.
        what: &'static str,
    },
}

impl ErrorKind {
    /// Stable machine-readable code.
    pub fn code(&self) -> &'static str {
        match self {
            ErrorKind::InvalidHeader { .. } => "InvalidHeader",
            ErrorKind::UnsupportedVersion { .. } => "UnsupportedVersion",
            ErrorKind::UnexpectedEof { .. } => "UnexpectedEOF",
            ErrorKind::SectionTableOutOfRange { .. } => "SectionTableOutOfRange",
            ErrorKind::MisalignedSection { .. } => "MisalignedSection",
            ErrorKind::SectionOutOfBounds { .. } => "SectionOutOfBounds",
            ErrorKind::SectionsNotMonotonic { .. } => "SectionsNotMonotonic",
            ErrorKind::InvalidCount { .. } => "InvalidCount",
            ErrorKind::DeformerCountMismatch { .. } => "DeformerCountMismatch",
            ErrorKind::LimitExceeded { .. } => "LimitExceeded",
            ErrorKind::InvalidReference { .. } => "InvalidReference",
            ErrorKind::InvalidRange { .. } => "InvalidRange",
            ErrorKind::InvalidValue { .. } => "InvalidValue",
            ErrorKind::AllocationFailed { .. } => "AllocationFailed",
            ErrorKind::UnsupportedFeature { .. } => "UnsupportedFeature",
            ErrorKind::Internal { .. } => "Internal",
        }
    }

    /// Recovery suggestion shown next to the error.
    pub fn suggestion(&self) -> &'static str {
        match self {
            ErrorKind::InvalidHeader { .. } => {
                "verify the file is an unmodified Live2D Cubism .moc3 export"
            }
            ErrorKind::UnsupportedVersion { .. } => {
                "re-export with a supported Cubism Editor version or extend MocVersion"
            }
            ErrorKind::UnexpectedEof { .. } => {
                "the file is truncated or damaged; re-export from the authoring tool"
            }
            ErrorKind::SectionTableOutOfRange { .. } => {
                "the section offset table is damaged; compare against a pristine copy"
            }
            ErrorKind::MisalignedSection { .. } => {
                "the file is not a well-formed export; inspect the section at the given offset"
            }
            ErrorKind::SectionOutOfBounds { .. } => {
                "a section claims more data than the file contains; re-export the model"
            }
            ErrorKind::SectionsNotMonotonic { .. } => {
                "section order is inconsistent; the file layout was likely rewritten"
            }
            ErrorKind::InvalidCount { .. } => {
                "count table entries must be non-negative; the file is corrupt"
            }
            ErrorKind::DeformerCountMismatch { .. } => {
                "warp + rotation deformer counts must equal the deformer count"
            }
            ErrorKind::LimitExceeded { .. } => {
                "raise the limit explicitly only if you trust the input file"
            }
            ErrorKind::InvalidReference { .. } => {
                "an index points outside its array; the file is structurally corrupt"
            }
            ErrorKind::InvalidRange { .. } => {
                "a begin/count range escapes its array; the file is structurally corrupt"
            }
            ErrorKind::InvalidValue { .. } => "inspect the referenced field for corruption",
            ErrorKind::AllocationFailed { .. } => {
                "reduce the input size or lower the configured limits"
            }
            ErrorKind::UnsupportedFeature { .. } => {
                "this file uses a feature planned for a later recovery phase"
            }
            ErrorKind::Internal { .. } => "please report this as a parser bug with the input file",
        }
    }
}

/// Error returned by every fallible `moc3-ingest` operation.
#[derive(Debug, Clone)]
pub struct Moc3Error {
    /// Machine-readable category.
    pub kind: ErrorKind,
    /// File offset related to the failure, when known.
    pub offset: Option<u64>,
    /// Optional section or field context.
    pub context: Option<&'static str>,
}

impl Moc3Error {
    /// Create an error without location information.
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            offset: None,
            context: None,
        }
    }

    /// Create an error located at `offset`.
    pub fn at(kind: ErrorKind, offset: u64) -> Self {
        Self {
            kind,
            offset: Some(offset),
            context: None,
        }
    }

    /// Create an error located at `offset` inside `context`.
    pub fn in_context(kind: ErrorKind, offset: u64, context: &'static str) -> Self {
        Self {
            kind,
            offset: Some(offset),
            context: Some(context),
        }
    }

    /// Machine-readable error code (see [`ErrorKind::code`]).
    pub fn code(&self) -> &'static str {
        self.kind.code()
    }

    /// Recovery suggestion for this error.
    pub fn suggestion(&self) -> &'static str {
        self.kind.suggestion()
    }
}

impl fmt::Display for Moc3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::InvalidHeader { reason } => write!(f, "invalid header: {reason}"),
            ErrorKind::UnsupportedVersion {
                version,
                max_supported,
            } => write!(
                f,
                "unsupported MOC3 version byte {version} (supported: 1..={max_supported})"
            ),
            ErrorKind::UnexpectedEof {
                offset,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of file: needed {needed} bytes at offset {offset} but only {available} are available"
            ),
            ErrorKind::SectionTableOutOfRange {
                slot,
                value,
                file_len,
            } => write!(
                f,
                "section offset table slot {slot} = {value} points outside the file (length {file_len})"
            ),
            ErrorKind::MisalignedSection {
                section,
                offset,
                alignment,
            } => write!(
                f,
                "section {section} at offset {offset} is not aligned to {alignment} bytes"
            ),
            ErrorKind::SectionOutOfBounds {
                section,
                offset,
                size,
                file_len,
            } => write!(
                f,
                "section {section} needs {size} bytes at offset {offset} but the file is {file_len} bytes"
            ),
            ErrorKind::SectionsNotMonotonic {
                section,
                offset,
                previous_end,
            } => write!(
                f,
                "section {section} at offset {offset} overlaps or precedes previous data ending at {previous_end}"
            ),
            ErrorKind::InvalidCount { field, value } => {
                write!(f, "count field {field} has invalid value {value}")
            }
            ErrorKind::DeformerCountMismatch {
                warps,
                rotations,
                deformers,
            } => write!(
                f,
                "deformer counts inconsistent: warps({warps}) + rotations({rotations}) != deformers({deformers})"
            ),
            ErrorKind::LimitExceeded {
                field,
                count,
                limit,
            } => write!(f, "limit exceeded: {field} count {count} > limit {limit}"),
            ErrorKind::InvalidReference {
                field,
                index,
                value,
                bound,
            } => write!(
                f,
                "invalid reference: {field}[{index}] = {value} outside 0..{bound}"
            ),
            ErrorKind::InvalidRange {
                field,
                index,
                begin,
                count,
                total,
            } => write!(
                f,
                "invalid range: {field}[{index}] begin={begin} count={count} escapes array of {total}"
            ),
            ErrorKind::InvalidValue {
                field,
                index,
                value,
                reason,
            } => write!(f, "invalid value: {field}[{index}] = {value} ({reason})"),
            ErrorKind::AllocationFailed { field, count } => {
                write!(f, "could not reserve memory for {field} ({count} elements)")
            }
            ErrorKind::UnsupportedFeature { feature } => {
                write!(f, "unsupported feature: {feature}")
            }
            ErrorKind::Internal { what } => write!(f, "internal parser error: {what}"),
        }
    }
}

impl std::error::Error for Moc3Error {}

/// Convenience alias used across the crate.
pub type Moc3Result<T> = Result<T, Moc3Error>;
