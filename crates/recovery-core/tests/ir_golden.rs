//! Golden IR documents for all 12 synthetic fixtures.
//!
//! The goldens were generated once and reviewed as part of AGENT.2; they are
//! not automatically authoritative just because the generator wrote them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use recovery_core::{build_ir_from_file, export_ir_json, import_ir_json, InspectOptions};

#[test]
fn golden_ir_documents_match() {
    let update = std::env::var("UPDATE_GOLDEN_IR").is_ok();
    std::fs::create_dir_all(support::expected_ir_dir()).unwrap();
    for name in fixture_gen::FIXTURE_NAMES {
        if fixture_gen::GOLDEN_SKIP.contains(name) {
            continue;
        }
        let path = support::fixtures_dir().join(name);
        let model = build_ir_from_file(&path, &InspectOptions::default())
            .unwrap_or_else(|error| panic!("{name} failed to map: {error}"));
        let actual = format!("{}\n", export_ir_json(&model, true).unwrap());
        let golden = support::expected_ir_dir().join(format!("{name}.ir.json"));
        if update {
            std::fs::write(&golden, actual.as_bytes()).unwrap();
        } else {
            let expected = std::fs::read_to_string(&golden).unwrap_or_else(|error| {
                panic!(
                    "missing golden {}: {error} (run with UPDATE_GOLDEN_IR=1)",
                    golden.display()
                )
            });
            assert_eq!(expected, actual, "golden IR mismatch for {name}");
        }
    }
}

#[test]
fn golden_ir_documents_import_cleanly() {
    for name in fixture_gen::FIXTURE_NAMES {
        if fixture_gen::GOLDEN_SKIP.contains(name) {
            continue;
        }
        let golden = support::expected_ir_dir().join(format!("{name}.ir.json"));
        let text = std::fs::read_to_string(&golden)
            .unwrap_or_else(|error| panic!("missing golden {}: {error}", golden.display()));
        let model = import_ir_json(&text).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            !live2d_ir::has_fatal(&model.diagnostics),
            "{name} golden has fatal diagnostics"
        );
    }
}
