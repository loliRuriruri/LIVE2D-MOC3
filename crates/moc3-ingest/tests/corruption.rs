//! Corruption and robustness tests: damaged input must produce structured
//! errors and never a panic.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::panic::{catch_unwind, AssertUnwindSafe};

use moc3_ingest::{parse, ErrorKind, Limits};

fn try_parse(bytes: &[u8]) -> Result<moc3_ingest::RawMoc3, moc3_ingest::Moc3Error> {
    parse(bytes, &Limits::default())
}

fn expect_code(bytes: &[u8], code: &str) {
    match try_parse(bytes) {
        Ok(_) => panic!("expected error {code}, but parsing succeeded"),
        Err(error) => assert_eq!(error.code(), code, "unexpected error: {error}"),
    }
}

#[test]
fn empty_and_short_inputs_fail_cleanly() {
    for len in 0usize..64 {
        let bytes = vec![0u8; len];
        assert!(try_parse(&bytes).is_err(), "len {len} should fail");
    }
    let mut header_only = vec![0u8; 64];
    header_only[..4].copy_from_slice(b"MOC3");
    header_only[4] = 2;
    assert!(try_parse(&header_only).is_err());
}

#[test]
fn bad_magic_is_rejected() {
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    bytes[0] = b'X';
    expect_code(&bytes, "InvalidHeader");
}

#[test]
fn unsupported_versions_are_rejected() {
    for version in [0u8, 7, 42, 255] {
        let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
        bytes[4] = version;
        expect_code(&bytes, "UnsupportedVersion");
    }
}

#[test]
fn bad_endian_flag_is_rejected() {
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    bytes[5] = 2;
    match try_parse(&bytes) {
        Err(error) => assert_eq!(error.code(), "InvalidHeader"),
        Ok(_) => panic!("endian flag 2 must be rejected"),
    }
}

#[test]
fn truncations_are_rejected_without_panics() {
    for name in fixture_gen::FIXTURE_NAMES {
        if fixture_gen::GOLDEN_SKIP.contains(name) {
            continue; // covered by `large_fixtures_are_sampled`
        }
        let bytes = support::read_fixture(name);
        let step = (bytes.len() / 37).max(1);
        let mut cut = step;
        while cut < bytes.len() {
            let truncated = bytes.get(..cut).unwrap();
            let outcome = catch_unwind(AssertUnwindSafe(|| try_parse(truncated)));
            match outcome {
                Err(_) => panic!("panic while parsing truncated {name} at {cut}"),
                Ok(Ok(_)) => panic!("truncated {name} at {cut} unexpectedly parsed"),
                Ok(Err(_)) => {}
            }
            cut += step;
        }
    }
}

#[test]
fn single_byte_flips_never_panic() {
    // Full sweep on the smallest fixture plus strided sweeps on all fixtures.
    for name in fixture_gen::FIXTURE_NAMES {
        if fixture_gen::GOLDEN_SKIP.contains(name) {
            continue; // covered by `large_fixtures_are_sampled`
        }
        let bytes = support::read_fixture(name);
        let stride = if name.starts_with("fixture-001") {
            1
        } else {
            (bytes.len() / 200).max(1)
        };
        let mut offset = 0usize;
        while offset < bytes.len() {
            let mut mutated = bytes.clone();
            if let Some(byte) = mutated.get_mut(offset) {
                *byte ^= 0xFF;
            }
            let outcome = catch_unwind(AssertUnwindSafe(|| try_parse(&mutated)));
            if outcome.is_err() {
                panic!("panic while parsing {name} with byte {offset} flipped");
            }
            offset += stride;
        }
    }
}

#[test]
fn large_fixtures_are_sampled() {
    // Bounded, deterministic corruption sampling for the multi-megabyte
    // fixtures that are excluded from the full sweeps.
    for name in fixture_gen::GOLDEN_SKIP {
        let bytes = support::read_fixture(name);
        for cut in [bytes.len() / 4, bytes.len() / 2, bytes.len() - 1] {
            let truncated = bytes.get(..cut).unwrap();
            let outcome = catch_unwind(AssertUnwindSafe(|| try_parse(truncated).is_err()));
            match outcome {
                Ok(is_err) => assert!(is_err, "truncated {name} at {cut} unexpectedly parsed"),
                Err(_) => panic!("panic while parsing truncated {name} at {cut}"),
            }
        }
        for offset in [
            0usize,
            4,
            6,
            bytes.len() / 3,
            bytes.len() / 2,
            bytes.len() - 7,
        ] {
            let mut mutated = bytes.clone();
            if let Some(byte) = mutated.get_mut(offset) {
                *byte ^= 0xFF;
            }
            let outcome = catch_unwind(AssertUnwindSafe(|| try_parse(&mutated)));
            assert!(
                outcome.is_ok(),
                "panic while parsing mutated {name} at {offset}"
            );
        }
    }
}

#[test]
fn random_inputs_never_panic() {
    let lengths = [16usize, 64, 320, 1024, 4096, 65536];
    for (seed, len) in lengths.iter().enumerate() {
        let bytes = support::lcg_bytes(*len, 0x1234_5678_0000_0000 + seed as u64);
        let outcome = catch_unwind(AssertUnwindSafe(|| try_parse(&bytes)));
        assert!(outcome.is_ok(), "panic on random input of length {len}");
        if let Ok(Ok(model)) = outcome {
            // Random bytes passing the header is unlikely; if it happens the
            // parse must at least be internally consistent.
            assert_eq!(model.parts.len(), model.counts.parts as usize);
            assert_eq!(model.parameters.len(), model.counts.parameters as usize);
            assert_eq!(model.art_meshes.len(), model.counts.art_meshes as usize);
        }
    }
}

#[test]
fn patched_references_are_rejected() {
    // Mask index out of range.
    let mut bytes = support::read_fixture("fixture-006-clipping-mask.moc3");
    let masks = support::section_offset(&bytes, "drawable_mask.indices");
    support::write_i32(&mut bytes, masks, 99);
    expect_code(&bytes, "InvalidReference");

    // Part binding index out of range.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let bindings = support::section_offset(&bytes, "part.binding_index");
    support::write_i32(&mut bytes, bindings, 99);
    expect_code(&bytes, "InvalidReference");

    // Keyform count escaping the keyform pool.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let keyform_count = support::section_offset(&bytes, "part.keyform_count");
    support::write_i32(&mut bytes, keyform_count, 1_000_000);
    expect_code(&bytes, "InvalidRange");

    // Invalid warp grid.
    let mut bytes = support::read_fixture("fixture-003-warp.moc3");
    let rows = support::section_offset(&bytes, "warp.rows");
    support::write_i32(&mut bytes, rows, 0);
    expect_code(&bytes, "InvalidValue");

    // Invalid deformer type.
    let mut bytes = support::read_fixture("fixture-003-warp.moc3");
    let deformer_type = support::section_offset(&bytes, "deformer.type");
    support::write_i32(&mut bytes, deformer_type, 7);
    expect_code(&bytes, "InvalidValue");
}

#[test]
fn patched_counts_are_rejected() {
    // Negative count.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let counts_offset = support::count_info_offset(&bytes);
    support::write_i32(&mut bytes, counts_offset, -1);
    expect_code(&bytes, "InvalidCount");

    // Count above the safety limit.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let counts_offset = support::count_info_offset(&bytes);
    support::write_i32(&mut bytes, counts_offset, 2_000_000);
    expect_code(&bytes, "LimitExceeded");

    // Deformer count mismatch: warp + rotation != deformers.
    let mut bytes = support::read_fixture("fixture-003-warp.moc3");
    let counts_offset = support::count_info_offset(&bytes);
    support::write_i32(&mut bytes, counts_offset + 4, 5);
    expect_code(&bytes, "DeformerCountMismatch");
}

#[test]
fn patched_offset_table_is_rejected() {
    // Slot 3 (part.id) beyond the end of the file.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let beyond = u32::try_from(bytes.len()).unwrap() + 8;
    support::write_u32(&mut bytes, 0x40 + 3 * 4, beyond);
    expect_code(&bytes, "SectionTableOutOfRange");

    // Slot 3 overlapping the header region.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    support::write_u32(&mut bytes, 0x40 + 3 * 4, 0);
    expect_code(&bytes, "SectionsNotMonotonic");

    // Slot 3 misaligned.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    support::write_u32(&mut bytes, 0x40 + 3 * 4, 3);
    expect_code(&bytes, "MisalignedSection");

    // Slot 1 (static canvas info) pointing into the offset table.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    support::write_u32(&mut bytes, 0x44, 0);
    expect_code(&bytes, "InvalidValue");

    // Slot 0 (count info) pointing into the offset table.
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    support::write_u32(&mut bytes, 0x40, 0x40 + 8);
    expect_code(&bytes, "InvalidValue");
}

#[test]
fn tiny_limits_are_enforced() {
    let cases: [(&str, Limits); 2] = [
        (
            "fixture-002-artmesh-param.moc3",
            Limits {
                max_parts: 0,
                ..Limits::default()
            },
        ),
        (
            "fixture-006-clipping-mask.moc3",
            Limits {
                max_masks: 0,
                ..Limits::default()
            },
        ),
    ];
    for (name, limits) in cases {
        let bytes = support::read_fixture(name);
        match parse(&bytes, &limits) {
            Err(error) => assert_eq!(error.code(), "LimitExceeded"),
            Ok(_) => panic!("expected LimitExceeded for {name}"),
        }
    }
}

#[test]
fn file_size_limit_is_enforced() {
    let bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    let limits = Limits {
        max_file_size: 16,
        ..Limits::default()
    };
    match parse(&bytes, &limits) {
        Err(error) => assert_eq!(error.code(), "LimitExceeded"),
        Ok(_) => panic!("expected LimitExceeded"),
    }
}

#[test]
fn error_details_include_context() {
    let mut bytes = support::read_fixture("fixture-002-artmesh-param.moc3");
    bytes[0] = b'X';
    let error = try_parse(&bytes).unwrap_err();
    assert_eq!(error.code(), "InvalidHeader");
    assert!(error.offset.is_some());
    assert!(!error.suggestion().is_empty());
    match error.kind {
        ErrorKind::InvalidHeader { reason } => assert!(!reason.is_empty()),
        other => panic!("unexpected kind {other:?}"),
    }
}
