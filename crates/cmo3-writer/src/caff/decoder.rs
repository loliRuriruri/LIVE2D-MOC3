//! Minimal CAFF test decoder (work order section 14).
//!
//! The encoder must be able to read its own output: this decoder resolves the
//! header, file table, offsets, guard bytes and RAW payloads. ZIP-compressed
//! payloads (modes 33/37) are recognised structurally but the payload decode
//! reports `UnsupportedCompression` instead of guessing.

use super::entry::{ArchiveHeader, Compression, DecodedEntry, MAX_ENTRIES, MAX_ENTRY_SIZE};
use super::error::{CaffError, CaffResult};
use super::obfuscation::{int64_mask, xor_byte};

/// Preview block size: format, color, 2 pad, width, height, start, size,
/// 8 reserved = 28 bytes.
const PREVIEW_BLOCK_SIZE: usize = 28;
/// Offset where the preview block starts: magic(4) + archive version(3) +
/// format id(4) + format version(3) + key(4) + reserved(8) = 26.
const PREVIEW_OFFSET: usize = 26;

/// Decoded archive: header, entries and extracted RAW payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedArchive {
    /// Header facts.
    pub header: ArchiveHeader,
    /// Entries in table order.
    pub entries: Vec<DecodedEntry>,
    /// Extracted payloads, aligned with `entries` (`None` when the mode is
    /// not RAW).
    pub payloads: Vec<Option<Vec<u8>>>,
    /// Whether the guard bytes are correct.
    pub guard_ok: bool,
}

impl DecodedArchive {
    /// Payload of the entry with the given path, when RAW.
    pub fn payload(&self, path: &str) -> Option<&[u8]> {
        self.entries
            .iter()
            .position(|entry| entry.path == path)
            .and_then(|index| self.payloads.get(index))
            .and_then(|payload| payload.as_deref())
    }

    /// Entry with the given path.
    pub fn entry(&self, path: &str) -> Option<&DecodedEntry> {
        self.entries.iter().find(|entry| entry.path == path)
    }
}

/// Decode a CAFF archive without decompressing ZIP payloads.
pub fn decode(bytes: &[u8]) -> CaffResult<DecodedArchive> {
    if bytes.len() < PREVIEW_OFFSET + PREVIEW_BLOCK_SIZE + 4 + 2 {
        return Err(CaffError::Truncated {
            needed: PREVIEW_OFFSET + PREVIEW_BLOCK_SIZE + 6,
            actual: bytes.len(),
        });
    }
    if bytes.get(..4) != Some(b"CAFF") {
        return Err(CaffError::BadMagic);
    }
    let archive_version = read3(bytes, 4)?;
    let format_id = read4(bytes, 7)?;
    let format_version = read3(bytes, 11)?;
    let key = i32::from_be_bytes(read4(bytes, 14)?);

    let mut cursor = PREVIEW_OFFSET + PREVIEW_BLOCK_SIZE;
    if cursor > bytes.len() {
        return Err(CaffError::Truncated {
            needed: cursor,
            actual: bytes.len(),
        });
    }

    let entry_count_raw = read_i32(bytes, &mut cursor, key)?;
    if entry_count_raw < 0 || entry_count_raw as usize > MAX_ENTRIES {
        return Err(CaffError::BadEntryCount {
            count: entry_count_raw,
        });
    }
    let entry_count = entry_count_raw as u32;

    let mut entries: Vec<DecodedEntry> = Vec::with_capacity(entry_count as usize);
    for _ in 0..entry_count {
        let path = read_string(bytes, &mut cursor, key)?;
        let tag = read_string(bytes, &mut cursor, key)?;
        let start = read_i64(bytes, &mut cursor, key)?;
        let size = read_i32(bytes, &mut cursor, key)?;
        let obfuscated = read_byte(bytes, &mut cursor, key)? != 0;
        let compression = read_byte(bytes, &mut cursor, key)?;
        skip(bytes, &mut cursor, 8)?;
        if size < 0 || u64::from(size as u32) > MAX_ENTRY_SIZE {
            return Err(CaffError::BadOffset {
                path,
                start: start.max(0) as u64,
                size: size.max(0) as u64,
                archive_len: bytes.len(),
            });
        }
        let end = start.saturating_add(i64::from(size));
        let data_limit = (bytes.len() as i64) - 2; // guard bytes are not data
        if start < 0 || end > data_limit {
            return Err(CaffError::BadOffset {
                path,
                start: start.max(0) as u64,
                size: size as u64,
                archive_len: bytes.len(),
            });
        }
        entries.push(DecodedEntry {
            path,
            tag,
            start: start as u64,
            size: size as u32,
            obfuscated,
            compression,
        });
    }

    let guard = bytes.get(bytes.len().saturating_sub(2)..).unwrap_or(&[]);
    let guard_ok = guard == [98, 99];

    let mut payloads: Vec<Option<Vec<u8>>> = Vec::with_capacity(entries.len());
    for entry in &entries {
        let start = entry.start as usize;
        let end = start.saturating_add(entry.size as usize);
        let raw = bytes.get(start..end).unwrap_or(&[]);
        let mode = Compression::from_byte(entry.compression)?;
        match mode {
            Compression::Raw => {
                let mut payload = raw.to_vec();
                if entry.obfuscated {
                    for byte in &mut payload {
                        *byte = xor_byte(*byte, key);
                    }
                }
                payloads.push(Some(payload));
            }
            Compression::Fast | Compression::Small => {
                // Recognised but not decoded; a dedicated zip decode is a
                // later-phase item (documented limitation).
                payloads.push(None);
            }
        }
    }

    Ok(DecodedArchive {
        header: ArchiveHeader {
            archive_version,
            format_id,
            format_version,
            key,
            entry_count,
        },
        entries,
        payloads,
        guard_ok,
    })
}

/// Decode and additionally require every entry payload to be RAW and
/// extracted (the writer's own round-trip contract).
pub fn decode_strict_raw(bytes: &[u8]) -> CaffResult<DecodedArchive> {
    let archive = decode(bytes)?;
    for (entry, payload) in archive.entries.iter().zip(archive.payloads.iter()) {
        if payload.is_none() {
            return Err(CaffError::UnsupportedCompression {
                path: entry.path.clone(),
                mode: entry.compression,
            });
        }
    }
    Ok(archive)
}

fn read3(bytes: &[u8], offset: usize) -> CaffResult<[u8; 3]> {
    let slice = bytes.get(offset..offset + 3).ok_or(CaffError::Truncated {
        needed: offset + 3,
        actual: bytes.len(),
    })?;
    Ok([slice[0], slice[1], slice[2]])
}

fn read4(bytes: &[u8], offset: usize) -> CaffResult<[u8; 4]> {
    let slice = bytes.get(offset..offset + 4).ok_or(CaffError::Truncated {
        needed: offset + 4,
        actual: bytes.len(),
    })?;
    Ok([slice[0], slice[1], slice[2], slice[3]])
}

fn read_byte(bytes: &[u8], cursor: &mut usize, key: i32) -> CaffResult<u8> {
    let value = *bytes.get(*cursor).ok_or(CaffError::Truncated {
        needed: *cursor + 1,
        actual: bytes.len(),
    })?;
    *cursor += 1;
    Ok(xor_byte(value, key))
}

fn read_number(bytes: &[u8], cursor: &mut usize, key: i32) -> CaffResult<u32> {
    let start = *cursor;
    let mut value: u32 = 0;
    for _ in 0..4 {
        let byte = read_byte(bytes, cursor, key)?;
        value = (value << 7) | u32::from(byte & 127);
        if byte & 128 == 0 {
            return Ok(value);
        }
    }
    Err(CaffError::BadVarInt { offset: start })
}

fn read_i32(bytes: &[u8], cursor: &mut usize, key: i32) -> CaffResult<i32> {
    let raw = read4(bytes, *cursor)?;
    *cursor += 4;
    Ok(i32::from_be_bytes(raw) ^ key)
}

fn read_i64(bytes: &[u8], cursor: &mut usize, key: i32) -> CaffResult<i64> {
    let raw: [u8; 8] = bytes
        .get(*cursor..*cursor + 8)
        .ok_or(CaffError::Truncated {
            needed: *cursor + 8,
            actual: bytes.len(),
        })?
        .try_into()
        .map_err(|_| CaffError::Truncated {
            needed: *cursor + 8,
            actual: bytes.len(),
        })?;
    *cursor += 8;
    Ok((u64::from_be_bytes(raw) ^ int64_mask(key)) as i64)
}

fn read_string(bytes: &[u8], cursor: &mut usize, key: i32) -> CaffResult<String> {
    let start = *cursor;
    let length = read_number(bytes, cursor, key)? as usize;
    let raw = bytes
        .get(*cursor..*cursor + length)
        .ok_or(CaffError::Truncated {
            needed: *cursor + length,
            actual: bytes.len(),
        })?;
    *cursor += length;
    let decoded: Vec<u8> = raw.iter().map(|byte| xor_byte(*byte, key)).collect();
    String::from_utf8(decoded).map_err(|error| CaffError::BadString {
        offset: start,
        detail: error.to_string(),
    })
}

fn skip(bytes: &[u8], cursor: &mut usize, count: usize) -> CaffResult<()> {
    if *cursor + count > bytes.len() {
        return Err(CaffError::Truncated {
            needed: *cursor + count,
            actual: bytes.len(),
        });
    }
    *cursor += count;
    Ok(())
}
