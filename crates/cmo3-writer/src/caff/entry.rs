//! CAFF entry model and compression modes.

use super::error::{CaffError, CaffResult};

/// Entry compression mode byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    /// Mode 16: payload stored as-is.
    Raw,
    /// Mode 33: payload is a ZIP container with one `contents` entry.
    Fast,
    /// Mode 37: ZIP container, smaller (levels unconfirmed).
    Small,
}

impl Compression {
    /// The on-disk mode byte.
    pub fn mode_byte(self) -> u8 {
        match self {
            Compression::Raw => 16,
            Compression::Fast => 33,
            Compression::Small => 37,
        }
    }

    /// Parse a mode byte.
    pub fn from_byte(value: u8) -> CaffResult<Self> {
        match value {
            16 => Ok(Compression::Raw),
            33 => Ok(Compression::Fast),
            37 => Ok(Compression::Small),
            other => Err(CaffError::BadCompressionMode { mode: other }),
        }
    }
}

/// One archive entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaffEntry {
    /// Internal path (`main.xml`, `imageFileBuf_0.png`, ...).
    pub path: String,
    /// Entry tag (`main_xml` for the document, empty for images).
    pub tag: String,
    /// Stored payload (already transformed for the chosen compression).
    pub bytes: Vec<u8>,
    /// Whether the payload is XOR-obfuscated with the archive key.
    pub obfuscated: bool,
    /// Compression mode.
    pub compression: Compression,
}

impl CaffEntry {
    /// Raw uncompressed, obfuscated entry.
    pub fn raw(path: impl Into<String>, tag: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            tag: tag.into(),
            bytes,
            obfuscated: true,
            compression: Compression::Raw,
        }
    }

    /// Raw uncompressed, non-obfuscated entry (test vectors).
    pub fn raw_plain(path: impl Into<String>, tag: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            obfuscated: false,
            ..Self::raw(path, tag, bytes)
        }
    }
}

/// Accepted limits for hostile inputs.
pub const MAX_ENTRIES: usize = 4_096;
/// Maximum single entry size accepted by the decoder.
pub const MAX_ENTRY_SIZE: u64 = 256 * 1024 * 1024;

/// Decoded archive entry with resolved layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedEntry {
    /// Entry path.
    pub path: String,
    /// Entry tag.
    pub tag: String,
    /// Absolute start offset in the archive.
    pub start: u64,
    /// Stored size in bytes.
    pub size: u32,
    /// Obfuscation flag.
    pub obfuscated: bool,
    /// Compression mode byte.
    pub compression: u8,
}

/// Archive header facts recovered by the decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveHeader {
    /// Archive version bytes.
    pub archive_version: [u8; 3],
    /// Format id (`----`).
    pub format_id: [u8; 4],
    /// Format version bytes.
    pub format_version: [u8; 3],
    /// Obfuscation key.
    pub key: i32,
    /// Entry count.
    pub entry_count: u32,
}
