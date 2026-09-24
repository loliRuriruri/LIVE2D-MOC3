//! Structured CAFF errors (never panics).

use std::fmt;

/// CAFF archive failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaffError {
    /// The archive is shorter than the fixed header.
    Truncated {
        /// Minimum required size.
        needed: usize,
        /// Actual size.
        actual: usize,
    },
    /// Magic bytes are not `CAFF`.
    BadMagic,
    /// A variable-length integer is malformed or too long.
    BadVarInt {
        /// Offset where the varint started.
        offset: usize,
    },
    /// A string is not valid UTF-8 or exceeds the cap.
    BadString {
        /// Offset where the string started.
        offset: usize,
        /// Detail.
        detail: String,
    },
    /// The entry count is outside the accepted range.
    BadEntryCount {
        /// Declared count.
        count: i32,
    },
    /// An entry offset/size is outside the file or overlaps the guard.
    BadOffset {
        /// Entry path (best effort).
        path: String,
        /// Declared start.
        start: u64,
        /// Declared size.
        size: u64,
        /// Archive length.
        archive_len: usize,
    },
    /// The guard bytes are not `[98, 99]`.
    BadGuard {
        /// Found bytes.
        found: [u8; 2],
    },
    /// The entry uses a compression this build cannot decode.
    UnsupportedCompression {
        /// Entry path.
        path: String,
        /// Mode byte.
        mode: u8,
    },
    /// The compression mode byte is not a known CAFF mode.
    BadCompressionMode {
        /// Mode byte.
        mode: u8,
    },
    /// The entry payload claims obfuscation but the layout is inconsistent.
    ObscuredEntry {
        /// Entry path.
        path: String,
        /// Detail.
        detail: String,
    },
    /// A numeric value does not fit the target width.
    Overflow {
        /// Field name.
        field: &'static str,
    },
}

impl fmt::Display for CaffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaffError::Truncated { needed, actual } => {
                write!(
                    formatter,
                    "archive truncated: need {needed} bytes, have {actual}"
                )
            }
            CaffError::BadMagic => write!(formatter, "CAFF magic missing"),
            CaffError::BadVarInt { offset } => {
                write!(formatter, "malformed variable-length integer at {offset}")
            }
            CaffError::BadString { offset, detail } => {
                write!(formatter, "bad string at {offset}: {detail}")
            }
            CaffError::BadEntryCount { count } => {
                write!(formatter, "invalid entry count {count}")
            }
            CaffError::BadOffset {
                path,
                start,
                size,
                archive_len,
            } => write!(
                formatter,
                "entry '{path}' range {start}..{} is outside the archive ({archive_len} bytes)",
                start.saturating_add(*size)
            ),
            CaffError::BadGuard { found } => {
                write!(formatter, "guard bytes are {found:?}, expected [98, 99]")
            }
            CaffError::UnsupportedCompression { path, mode } => write!(
                formatter,
                "entry '{path}' uses compression mode {mode} which this build does not decode"
            ),
            CaffError::BadCompressionMode { mode } => {
                write!(formatter, "unknown compression mode byte {mode}")
            }
            CaffError::ObscuredEntry { path, detail } => {
                write!(formatter, "entry '{path}' is inconsistent: {detail}")
            }
            CaffError::Overflow { field } => write!(formatter, "{field} does not fit"),
        }
    }
}

impl std::error::Error for CaffError {}

/// Result alias for CAFF operations.
pub type CaffResult<T> = Result<T, CaffError>;
