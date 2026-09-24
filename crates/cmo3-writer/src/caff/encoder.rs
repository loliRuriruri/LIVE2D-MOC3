//! CAFF archive encoder (placeholder-patch strategy, work order section 15).

use super::entry::{CaffEntry, Compression, MAX_ENTRIES, MAX_ENTRY_SIZE};
use super::error::{CaffError, CaffResult};
use super::obfuscation::{xor_byte, xor_i32_bytes, xor_i64_bytes};

/// Default obfuscation key used by both pinned CMO3 writers.
pub const DEFAULT_KEY: i32 = 0x2A;

/// Maximum total archive size this writer will produce.
pub const MAX_ARCHIVE_SIZE: u64 = 1024 * 1024 * 1024;

/// Encode entries into a CAFF archive.
///
/// Offsets are written as placeholders first and patched with the final start
/// positions after the payloads are laid out; the guard bytes are appended
/// raw and never obfuscated.
pub fn encode(key: i32, entries: &[CaffEntry]) -> CaffResult<Vec<u8>> {
    if entries.len() > MAX_ENTRIES {
        return Err(CaffError::Overflow {
            field: "entry count exceeds MAX_ENTRIES",
        });
    }
    // This build only writes RAW entries; compressed payloads would be
    // written unencoded, so they are rejected instead of guessing.
    for entry in entries {
        if !supported_modes().contains(&entry.compression) {
            return Err(CaffError::UnsupportedCompression {
                path: entry.path.clone(),
                mode: entry.compression.mode_byte(),
            });
        }
    }
    let mut out: Vec<u8> = Vec::new();

    // Header.
    out.extend_from_slice(b"CAFF");
    out.extend_from_slice(&[0, 0, 0]);
    out.extend_from_slice(b"----");
    out.extend_from_slice(&[0, 0, 0]);
    out.extend_from_slice(&key.to_be_bytes());
    out.extend_from_slice(&[0u8; 8]);

    // Preview block (none).
    out.push(127);
    out.push(127);
    out.extend_from_slice(&[0u8; 2]);
    out.extend_from_slice(&0i16.to_be_bytes());
    out.extend_from_slice(&0i16.to_be_bytes());
    out.extend_from_slice(&0i64.to_be_bytes());
    out.extend_from_slice(&0i32.to_be_bytes());
    out.extend_from_slice(&[0u8; 8]);

    let count = i32::try_from(entries.len()).map_err(|_| CaffError::Overflow {
        field: "entry count",
    })?;
    out.extend_from_slice(&xor_i32_bytes(count, key));

    // File table with placeholder offsets.
    let mut patch_offsets: Vec<usize> = Vec::with_capacity(entries.len());
    for entry in entries {
        write_string(&mut out, &entry.path, key)?;
        write_string(&mut out, &entry.tag, key)?;
        patch_offsets.push(out.len());
        out.extend_from_slice(&xor_i64_bytes(0, key));
        let size = u32::try_from(entry.bytes.len()).map_err(|_| CaffError::Overflow {
            field: "entry size",
        })?;
        if u64::from(size) > MAX_ENTRY_SIZE {
            return Err(CaffError::Overflow {
                field: "entry size exceeds MAX_ENTRY_SIZE",
            });
        }
        out.extend_from_slice(&xor_i32_bytes(size as i32, key));
        out.push(xor_byte(u8::from(entry.obfuscated), key));
        out.push(xor_byte(entry.compression.mode_byte(), key));
        out.extend_from_slice(&[0u8; 8]);
    }

    // Payloads; patch each start position as it is known.
    for (entry, patch) in entries.iter().zip(patch_offsets.iter()) {
        let start = out.len() as u64;
        let encoded = xor_i64_bytes(start as i64, key);
        if let Some(slot) = out.get_mut(*patch..patch + 8) {
            slot.copy_from_slice(&encoded);
        }
        if entry.obfuscated {
            super::obfuscation::xor_bytes_into(&mut out, &entry.bytes, key);
        } else {
            out.extend_from_slice(&entry.bytes);
        }
        if out.len() as u64 > MAX_ARCHIVE_SIZE {
            return Err(CaffError::Overflow {
                field: "archive size exceeds MAX_ARCHIVE_SIZE",
            });
        }
    }

    // Guard bytes (raw).
    out.push(98);
    out.push(99);
    Ok(out)
}

fn write_string(out: &mut Vec<u8>, value: &str, key: i32) -> CaffResult<()> {
    write_number(out, value.len(), key)?;
    if key == 0 {
        out.extend_from_slice(value.as_bytes());
    } else {
        out.extend(value.as_bytes().iter().map(|byte| xor_byte(*byte, key)));
    }
    Ok(())
}

fn write_number(out: &mut Vec<u8>, value: usize, key: i32) -> CaffResult<()> {
    if value >= 1 << 28 {
        return Err(CaffError::Overflow {
            field: "variable-length integer",
        });
    }
    if value < 128 {
        out.push(xor_byte(value as u8, key));
    } else if value < 16_384 {
        out.push(xor_byte(((value >> 7) as u8 & 127) | 128, key));
        out.push(xor_byte((value as u8) & 127, key));
    } else if value < 2_097_152 {
        out.push(xor_byte(((value >> 14) as u8 & 127) | 128, key));
        out.push(xor_byte(((value >> 7) as u8 & 127) | 128, key));
        out.push(xor_byte((value as u8) & 127, key));
    } else {
        out.push(xor_byte(((value >> 21) as u8 & 127) | 128, key));
        out.push(xor_byte(((value >> 14) as u8 & 127) | 128, key));
        out.push(xor_byte(((value >> 7) as u8 & 127) | 128, key));
        out.push(xor_byte((value as u8) & 127, key));
    }
    Ok(())
}

/// Encode a supplied payload as a FAST (ZIP) entry is not implemented in
/// AGENT.5; RAW (mode 16) is the writer's supported mode.
pub fn supported_modes() -> [Compression; 1] {
    [Compression::Raw]
}
