# AGENT.1 COMPLETION REPORT

```text
=== LIVE2D RECOVERY AGENT.1 REPORT ===

Repository:
D:\test\liver2d  (git repository initialized; no commits made - committing
requires an explicit user request, per standing instructions)

Commit:
(none; review performed against the working tree)

Implemented:
- crates/moc3-ingest: read-only MOC3 parser
  * header (magic/version 1-6/endianness), 64-byte layout
  * fixed section offset table walk (160 slots for v1-5, 480 for v6+),
    alignment (8), in-file bounds, monotonic data order, unknown-slot
    preservation
  * count info (32 ints v1-4 / 64 ints v5+) with non-negativity,
    deformer-count consistency, per-field and aggregate limits
  * canvas, parameters, parts, deformers (warp/rotation), art meshes,
    drawable masks, draw order groups/items, glue structure,
    bindings/key tables, keyform-span validation, v4.2/v5.0/v5.3
    blend-shape and offscreen range validation
  * structured errors (code/offset/context/suggestion), anomalies for
    non-fatal oddities, no unwrap/expect/panic/unsafe, try_reserve
    allocations, bounded CPU (post-review), deterministic output
  * JSON inspection report + human text report
- crates/recovery-core: read-only file loading with hard size cap
  (`take(limit+1)`, closing the metadata/read race), report assembly
- apps/recovery-cli: `recovery inspect <file> [--json] [--max-file-size N]`
  with structured human/JSON error output and exit codes
- tools/fixture-gen: deterministic synthetic MOC3 writer (test fixtures
  only; never emits CMO3) + 12 fixtures across six format versions
- fixtures/: synthetic (12 files), expected (12 golden reports),
  owned/ (empty by policy, documented)
- docs/: ARCHITECTURE, FORMAT_NOTES (evidence+confidence per assumption),
  LICENSE_NOTES, RESEARCH_LOG, IR_SPEC (draft), RECOVERY_RULES (draft),
  TEST_PLAN, LIMITATIONS
- tests: 46 total (lib unit 6, fixture-gen 3, corruption 13,
  fixtures/golden 15, CLI end-to-end 6, workspace smoke 3)

MOC3 versions tested:
- 1 (Cubism 3.0-3.2), 2 (3.3), 3 (4.0), 4 (4.2), 5 (5.0), 6 (5.3+)
- via synthetic fixtures only; no real Cubism export available in this
  environment (disclosed in docs/LIMITATIONS.md and FORMAT_NOTES.md)

Extractable structures:
- version/endianness/header, canvas (size, origin, ppu, Y-reversed flag)
- counts for all documented categories
- parameters: id, min/max/default, repeat, decimal places, key table
  ranges, type (4.2+), blend key table ranges (4.2+)
- parts: id, parent, visible/enabled, keyform ranges, offscreen index (5.3+)
- deformers: id, type, parent part/deformer, warp grid (rows/cols/vertex
  count/quad flag) or base angle, binding index
- art meshes: id, parents, texture number, drawable flags (blend bits,
  double-sided, inverted), vertex count, UV/index/mask ranges, resolved
  mask references, keyform ranges
- masks, draw order groups/items, glue structure
- keyform bindings and key tables (ranges), key counts
- texture page references (max+1 pages observed per model)
- hierarchy candidates: part/deformer/mask edges with confidence and the
  stored source field; parent-cycle detection
- runtime scratch section inventory (offsets/sizes, not interpreted)

Unknown structures:
- semantics of reserved header padding bytes (preserved/reported)
- runtime scratch section contents (explicitly not interpreted)
- drawable flag bits 4-7, canvas flag bits 1-7 (preserved/reported)
- blend shape and offscreen window semantics (ranges validated only;
  AGENT.6), glue weight semantics (AGENT.6), draw-order *use* (AGENT.3)
- the `0x40..0x2C0` region interpretation difference vs the OpenL2D
  ImHex pattern (documented; fixed-slot model adopted with evidence)

Fixtures:
- 12 synthetic fixtures (fixtures/synthetic), regenerable and
  staleness-checked; corruption cases generated deterministically in tests
- 0 owned fixtures (none available; fixtures/owned/ is intentionally empty
  and documented)

Tests:
- cargo test: 46 passed / 0 failed
- cargo clippy --workspace --all-targets: clean
- cargo fmt --all --check: clean
- corruption: truncation sweeps on all fixtures, full byte-flip sweep on
  fixture-001 + strided sweeps on all, LCG random inputs, patched
  counts/offsets/references/ranges; zero panics (catch_unwind)
- determinism: repeated CLI runs byte-identical; 12 golden reports matched
- input immutability: before/after byte equality (parser + CLI tests)

Test result:
PASS

Memory safety issues:
- none found by independent review; no unsafe code (forbidden workspace-wide)
- hardening applied after review: O(n^2) art-mesh color array read removed,
  O(n^2) duplicate-id detection replaced with BTreeMap, static-section
  overlap now rejected (canvas/count_info), TOCTOU race closed in
  recovery-core, control characters stripped from human output,
  big-endian reader unit tests added

Format assumptions:
- recorded per-entry with Evidence / Confidence / Open questions in
  docs/FORMAT_NOTES.md; primary layout cross-verified between PurismCore
  (MIT), mocari and moc2cmo slot semantics, py-moc3, and the OpenL2D
  ImHex pattern (with its discrepancy documented)
- no third-party code copied; references and licenses in
  docs/LICENSE_NOTES.md

Open questions:
- real-file validation (offset table base, empty-section conventions,
  runtime scratch sizing) pending an owned .moc3
- big-endian files implemented but never observed
- 5.3 offscreen/blend-shape semantics, draw-order use, glue weights
- CMO3 container research plan for AGENT.5 (separate phase, not started)

Files changed:
- Cargo.toml, Cargo.lock, .gitignore, README.md
- crates/moc3-ingest/** (parser, report, tests)
- crates/recovery-core/** , apps/recovery-cli/**
- crates/{live2d-ir,hierarchy-recovery,cmo3-writer,project-validator,
  texture-tools}/** (reserved placeholders only)
- tools/fixture-gen/**, tests/workspace-smoke/**
- fixtures/** (synthetic + expected + policy), docs/** (9 documents)

Production code:
YES  (read-only inspection tooling is production-grade for its phase;
      CMO3 writer / PSD / GUI intentionally NOT implemented)

Ready for AGENT.2:
YES  (caveat: real-file validation should be scheduled as soon as an owned
      .moc3 is available; the IR spec draft lists the pools AGENT.2 must
      additionally read)

Supervision notes:
- JEV route: AGENT.0 = NORMAL, AGENT.1 = HARD-class, REVIEW phase routed
  REVIEW/high with jev-review subagent (verdict PASS)
- JEV verify: 5/7 claims supported; 2 compound claims flagged at low
  confidence. Deterministic test output and the review transcript are the
  authoritative evidence for those; all review findings were fixed and
  re-verified afterwards.

=== END REPORT ===
```
