//! CAFF (Cubism Archive File Format) layer.
//!
//! Deliberately separate from the CMO3 semantic serializer (work order
//! section 12). Binary layout facts come from two independent MIT-licensed
//! sources that agree (`Stretchy Studio` `caff_packer.py`, pinned commit
//! `5fd958def9ed`; `moc2cmo` `src/caff/writer.rs`, pinned commit `2527e24e93`):
//!
//! - big-endian throughout, strings are varint length + UTF-8,
//! - integer-level XOR obfuscation with an int32 key (default `0x2A`),
//! - `CAFF` magic, `----` format id, 3-byte archive/format versions,
//! - preview block with format byte `127` (none),
//! - file table: path, tag, int64 start, int32 size, obfuscated flag,
//!   compression byte, 8 reserved bytes,
//! - trailing guard bytes `[98, 99]`,
//! - compression modes: `16` raw, `33` fast (ZIP `contents`), `37` small.
//!
//! This writer emits RAW entries only; modes 33/37 are recognised but their
//! payload decode is a documented later-phase item.

pub mod decoder;
mod encoder;
mod entry;
mod error;
mod obfuscation;
mod validate;

pub use decoder::{decode, decode_strict_raw, DecodedArchive};
pub use encoder::{encode, supported_modes, DEFAULT_KEY, MAX_ARCHIVE_SIZE};
pub use entry::{ArchiveHeader, CaffEntry, Compression, DecodedEntry, MAX_ENTRIES, MAX_ENTRY_SIZE};
pub use error::{CaffError, CaffResult};
pub use obfuscation::int64_mask;
pub use validate::{
    describe_error, validate_archive, ArchiveFinding, ArchiveValidation, FindingSeverity,
};

/// Guard bytes required at the end of every archive.
pub const GUARD: [u8; 2] = [98, 99];
/// Fixed archive magic.
pub const MAGIC: [u8; 4] = *b"CAFF";
/// Format id used by `.cmo3` archives.
pub const FORMAT_ID: [u8; 4] = *b"----";
/// Archive entry path of the XML document.
pub const MAIN_XML_PATH: &str = "main.xml";
/// Archive entry tag of the XML document.
pub const MAIN_XML_TAG: &str = "main_xml";

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn sample_entries() -> Vec<CaffEntry> {
        vec![
            CaffEntry::raw(MAIN_XML_PATH, MAIN_XML_TAG, b"<root/>".to_vec()),
            CaffEntry::raw("imageFileBuf_0.png", "", vec![0x89, 0x50, 0x4E, 0x47]),
        ]
    }

    #[test]
    fn round_trip_with_default_key() {
        let bytes = encode(DEFAULT_KEY, &sample_entries()).expect("encode");
        let decoded = decode_strict_raw(&bytes).expect("decode");
        assert!(decoded.guard_ok);
        assert_eq!(decoded.header.key, DEFAULT_KEY);
        assert_eq!(decoded.header.entry_count, 2);
        assert_eq!(decoded.payload(MAIN_XML_PATH), Some(&b"<root/>"[..]));
        assert_eq!(
            decoded.payload("imageFileBuf_0.png"),
            Some(&[0x89, 0x50, 0x4E, 0x47][..])
        );
        let validation = validate_archive(&bytes).expect("validate");
        assert!(validation.is_valid(), "{:?}", validation.findings);
        assert_eq!(validation.decoded_payloads, 2);
    }

    #[test]
    fn round_trip_with_zero_key_and_plain_entries() {
        let entries = vec![
            CaffEntry::raw_plain(MAIN_XML_PATH, MAIN_XML_TAG, b"<root/>".to_vec()),
            CaffEntry::raw_plain("imageFileBuf_0.png", "", vec![1, 2, 3]),
        ];
        let bytes = encode(0, &entries).expect("encode");
        let decoded = decode_strict_raw(&bytes).expect("decode");
        assert_eq!(decoded.header.key, 0);
        assert_eq!(decoded.payload("imageFileBuf_0.png"), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn negative_key_uses_sign_extended_mask() {
        let bytes = encode(-816_980_164, &sample_entries()).expect("encode");
        let decoded = decode_strict_raw(&bytes).expect("decode");
        assert_eq!(decoded.header.key, -816_980_164);
        assert_eq!(decoded.payload(MAIN_XML_PATH), Some(&b"<root/>"[..]));
    }

    #[test]
    fn guard_bytes_are_present_and_raw() {
        let bytes = encode(DEFAULT_KEY, &sample_entries()).expect("encode");
        assert_eq!(bytes.get(bytes.len() - 2..), Some(&GUARD[..]));
        assert_eq!(bytes.get(..4), Some(&MAGIC[..]));
        assert_eq!(bytes.get(7..11), Some(&FORMAT_ID[..]));
    }

    #[test]
    fn corrupted_guard_is_detected() {
        let mut bytes = encode(DEFAULT_KEY, &sample_entries()).expect("encode");
        let last = bytes.len() - 1;
        bytes[last] = 0;
        let validation = validate_archive(&bytes).expect("validate");
        assert!(!validation.is_valid());
        assert!(validation
            .findings
            .iter()
            .any(|finding| finding.code == "caff_bad_guard"));
    }

    #[test]
    fn truncated_and_bad_magic_are_structured_errors() {
        assert!(matches!(decode(&[]), Err(CaffError::Truncated { .. })));
        let mut bytes = encode(DEFAULT_KEY, &sample_entries()).expect("encode");
        bytes[0] = b'X';
        assert_eq!(decode(&bytes).err(), Some(CaffError::BadMagic));
        let short = &bytes[..30];
        assert!(matches!(decode(short), Err(CaffError::Truncated { .. })));
    }

    #[test]
    fn corrupted_payload_bytes_are_visible_to_the_decoder() {
        let bytes = encode(DEFAULT_KEY, &sample_entries()).expect("encode");
        let decoded = decode_strict_raw(&bytes).expect("decode");
        let start = decoded.entries[0].start as usize;
        let mut corrupted = bytes.clone();
        corrupted[start + 1] = 0xFF;
        let decoded = decode_strict_raw(&corrupted).expect("decode");
        assert_ne!(
            decoded
                .payload(MAIN_XML_PATH)
                .map(|payload| payload.to_vec()),
            Some(b"<root/>".to_vec())
        );
    }

    #[test]
    fn out_of_range_offsets_are_rejected() {
        // Patch the first entry's start to point past the archive end.
        let mut bytes = encode(DEFAULT_KEY, &sample_entries()).expect("encode");
        let decoded = decode_strict_raw(&bytes).expect("decode");
        let first_start = decoded.entries[0].start;
        let _ = first_start;
        // Find the patched start by recomputing the table position: encode
        // the same archive with a shifted payload by corrupting the size
        // instead (the offset field is XORed, so corruption is verified via
        // the size path which is deterministic to locate).
        let size_offset = {
            // header(26) + preview(28) + count(4) + path(9) + tag(9) = 76
            26 + 28 + 4 + (1 + MAIN_XML_PATH.len()) + (1 + MAIN_XML_TAG.len())
        };
        let huge = (super::entry::MAX_ENTRY_SIZE as u32) + 1;
        let encoded = super::obfuscation::xor_i32_bytes(huge as i32, DEFAULT_KEY);
        bytes[size_offset..size_offset + 4].copy_from_slice(&encoded);
        assert!(matches!(decode(&bytes), Err(CaffError::BadOffset { .. })));
    }

    #[test]
    fn unsafe_paths_are_fatal() {
        let entries = vec![
            CaffEntry::raw(MAIN_XML_PATH, MAIN_XML_TAG, b"<root/>".to_vec()),
            CaffEntry::raw("../evil.png", "", vec![1]),
        ];
        let bytes = encode(DEFAULT_KEY, &entries).expect("encode");
        let validation = validate_archive(&bytes).expect("validate");
        assert!(!validation.is_valid());
        assert!(validation
            .findings
            .iter()
            .any(|finding| finding.code == "caff_unsafe_path"));
    }

    #[test]
    fn duplicate_paths_are_fatal() {
        let entries = vec![
            CaffEntry::raw(MAIN_XML_PATH, MAIN_XML_TAG, b"<root/>".to_vec()),
            CaffEntry::raw(MAIN_XML_PATH, MAIN_XML_TAG, b"<root/>".to_vec()),
        ];
        let bytes = encode(DEFAULT_KEY, &entries).expect("encode");
        let validation = validate_archive(&bytes).expect("validate");
        assert!(validation
            .findings
            .iter()
            .any(|finding| finding.code == "caff_duplicate_path"));
    }

    #[test]
    fn entry_count_cap_is_enforced() {
        let mut entries = vec![CaffEntry::raw(MAIN_XML_PATH, MAIN_XML_TAG, vec![1])];
        for index in 0..MAX_ENTRIES {
            entries.push(CaffEntry::raw(format!("f{index}.png"), "", vec![1]));
        }
        assert!(encode(DEFAULT_KEY, &entries).is_err());
    }

    #[test]
    fn varint_lengths_cover_all_encodings() {
        for length in [1usize, 127, 128, 200, 16_383, 16_384, 100_000] {
            let path = "p".repeat(length);
            let entries = vec![CaffEntry::raw(
                MAIN_XML_PATH,
                MAIN_XML_TAG,
                b"<root/>".to_vec(),
            )];
            let _ = entries;
            let entry = CaffEntry::raw(path.clone(), "", vec![7]);
            let bytes = encode(DEFAULT_KEY, &[entry]).expect("encode");
            let decoded = decode_strict_raw(&bytes).expect("decode");
            assert_eq!(decoded.entries[0].path.len(), length);
            assert_eq!(decoded.payload(&path), Some(&[7][..]));
        }
    }

    #[test]
    fn compressed_mode_is_recognised_but_undecoded() {
        let mut entry = CaffEntry::raw("imageFileBuf_0.png", "", vec![1, 2, 3]);
        entry.compression = Compression::Fast;
        let bytes = encode(DEFAULT_KEY, &[entry]).expect("encode");
        let decoded = decode(&bytes).expect("decode");
        assert_eq!(decoded.entries[0].compression, 33);
        assert!(decoded.payloads[0].is_none());
        assert!(matches!(
            decode_strict_raw(&bytes),
            Err(CaffError::UnsupportedCompression { .. })
        ));
    }
}
