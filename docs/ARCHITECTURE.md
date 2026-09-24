# Architecture

Live2D Project Recovery Tool (master spec v0.1). Current status: **AGENT.0
through AGENT.4 complete** (research/bootstrap, read-only MOC3 inspector,
normalized Live2D IR, hierarchy reconstruction, external reference audit,
semantic keyform recovery). Later phases exist only as reserved, empty
crates.

AGENT.4 adds `crates/keyform-recovery` between the recovered project and any
future writer: it consumes `live2d-ir` + `hierarchy-recovery` read-only and
emits `live2d-recovery/recovered-keyforms/1` (EXPERIMENTAL). It never sees
binary offsets, never copies geometry and never interpolates. The AGENT.3.5
harness (`tools/reference-harness`) stays dev-only and is never a dependency
of any production crate.

## Pipeline

```text
MOC3 / model3.json / Texture
            |
       MOC3 Ingest          crates/moc3-ingest        (AGENT.1, implemented)
            |
   ParsedModel { RawMoc3 + ModelPools }               (parse_full for IR)
            |
   Mapper (normalizer)      crates/recovery-core::ir_mapper (AGENT.2)
            |
       Live2D IR            crates/live2d-ir          (AGENT.2, implemented)
            |
   IR Validator             live2d-ir::validate_ir    (AGENT.2)
            |
   Canonical JSON           export-ir / validate-ir   (AGENT.2)
            |
   Recovery Graph Engine    crates/hierarchy-recovery (AGENT.3, implemented)
   Hierarchy Resolver                               (deterministic; see
            |                                         docs/HIERARCHY_RECOVERY.md)
       RecoveredProjectIR   recovered-project/1 (experimental)
            |
   Keyform Recovery         crates/keyform-recovery  (AGENT.4, implemented)
   (bindings/grids/forms)                            (see
            |                                         docs/KEYFORM_RECOVERY.md)
   RecoveredKeyformModel    recovered-keyforms/1 (experimental)
            |
   CMO3 Writer              crates/cmo3-writer        (AGENT.5.2 complete:
            |                                         CAFF, identity, full
            |                                         semantic serializer,
            |                                         validators, inspector;
            |                                         gates 5A-5I structural -
            |                                         see docs/CMO3_VALIDATION.md)
            |
   Validation               crates/project-validator  (AGENT.7, reserved)
```

### Hierarchy layer (AGENT.3)

- `crates/hierarchy-recovery` consumes `Live2DModel` and produces
  `RecoveredProject` (graph + traces + statistics + diagnostics). **It must
  never depend on `moc3-ingest`/`recovery-core`**; the same workspace test
  pattern enforces the boundary.
- Stored relations stay `Exact`; at most one policy-gated heuristic rule
  (HR-010) exists and is disabled by default. Ambiguity is preserved.
- Geometry is never copied; nodes keep ids, evidence and metadata only.
- Traversals, cycle detection and rendering are iterative (deep chains are
  fixture-tested at 20k nodes, rendering at depth limits).

### Keyform layer (AGENT.4)

- `crates/keyform-recovery` consumes `Live2DModel` + optionally
  `RecoveredProject` and produces `RecoveredKeyformModel` (bands, axes,
  grids, typed target forms, traces, unresolved entries, statistics).
  **It must never depend on `moc3-ingest`/`recovery-core`**; the workspace
  smoke test enforces the boundary.
- Pipeline stages are separate functions (`collect_binding_evidence`,
  `normalize_axes`, `build_binding_bands`, `derive_grid_shape`, `map_forms`,
  `validate_recovered_keyforms`, `recover`); there is no single mega-rule.
- Stored axes stay `Exact`; cardinality/comparisons are `Derived`; ordering
  of multi-dimensional grids and unresolved semantics stay `Unknown`.
- Nothing is repaired: duplicates are not deduplicated, unsorted keys are
  not sorted, missing forms are never padded, mismatched grids stay
  partial/unknown.
- Geometry is never copied; target forms reference IR payloads by index and
  count (KF-009). Cardinality is checked arithmetic with hard caps and is
  never materialized (KF-002/KF-010).

### IR layer (AGENT.2)

- `crates/live2d-ir` owns the semantic model, typed ids, provenance,
  diagnostics, the validator and the JSON codec. **It must never depend on
  `moc3-ingest`**; a workspace test (`workspace-smoke`) checks both the
  manifest and the `cargo metadata` dependency graph.
- The mapper lives in `recovery-core` (which may depend on both sides). It
  consumes `ParsedModel` by value, so the flat pools are copied exactly once
  into IR structures and the parser buffers are dropped afterwards.
- Parser-only views (`inspect`) keep using `parse`, which does not read the
  bulk pools; `parse_full` is the IR entry point.
- Unknown/binary-shaped data lives in `unknowns` (counts and identifiers
  only); semantic entities never carry offsets or table slots.

Hard rules carried over from the master spec:

- The parser never writes. The writer never parses. Stages communicate only
  through explicit data structures (`RawMoc3`, later `ModelProject`).
- Confirmed data and inferred data are separate: the raw model contains only
  stored values; hierarchy edges carry a recovery confidence label.
- Nothing is deleted: unknown bytes/slots are preserved in `unknown_*` fields
  and reported.

## Crates and dependency direction

| Crate | Role | Depends on |
|---|---|---|
| `crates/moc3-ingest` | read-only MOC3 parser, limits, errors, report builder, bulk pools | `serde` |
| `crates/live2d-ir` | normalized IR, typed ids, validator, canonical JSON | `serde`, `serde_json` |
| `crates/hierarchy-recovery` | recovery graph, resolver, hierarchy validator, project JSON | `live2d-ir`, `serde`, `serde_json` |
| `crates/keyform-recovery` | binding bands, parameter axes, keyform grids, target forms, validator, canonical JSON | `live2d-ir`, `hierarchy-recovery`, `serde`, `serde_json` |
| `crates/recovery-core` | file IO, inspection orchestration, IR mapper, IR export/import | `moc3-ingest`, `live2d-ir`, `serde`, `serde_json` |
| `apps/recovery-cli` | `recovery` binary (clap) | `recovery-core`, `moc3-ingest`, `live2d-ir`, `hierarchy-recovery` |
| `crates/hierarchy-recovery` | recovery graph (reserved: AGENT.3) | - |
| `crates/cmo3-writer` | CAFF encoder/decoder, object-pool identity, GUID strategies, version profile, typed XML writer, semantic mapping (serializer pending) | `live2d-ir`, `hierarchy-recovery`, `keyform-recovery`, `serde` |
| `crates/project-validator` | structural comparison (reserved: AGENT.7) | - |
| `crates/texture-tools` | texture/PSD tooling (reserved: AGENT.8) | - |
| `tools/fixture-gen` | synthetic MOC3 fixture writer (tests only) | `moc3-ingest` |
| `tools/reference-harness` | dev-only differential validation vs external references (AGENT.3.5); binary `recovery-reference`; never shipped | `recovery-core`, `hierarchy-recovery`, `live2d-ir`, `serde` |
| `tests/workspace-smoke` | cross-crate integration tests | `recovery-core`, `fixture-gen` |

`moc3-ingest` is deliberately dependency-light (only `serde`): parsing must
stay auditable and buildable offline apart from crates.io basics.

## Parser design (`moc3-ingest`)

- `reader.rs` - `ByteView`: bounds-checked access at absolute offsets with
  byte-order awareness. No unchecked indexing; `clippy::indexing_slicing` is
  denied crate-wide.
- `version.rs` - version byte -> layout facts (offset slots, count ints,
  version-gated section blocks).
- `table.rs` - canonical section order (101/102/102/137/152/167 slots) plus
  named slot constants used by the parser and the fixture generator (single
  source of truth for both directions).
- `counts.rs` - count info table (32 ints for v1-4, 64 ints for v5+),
  non-negativity + `warp + rotation == deformers` + unknown-field retention.
- `parse.rs` - header, offset table walk (alignment 8, in-file bounds,
  monotonic data order), section content extraction and cross-reference
  validation, anomaly collection.
- `raw.rs` - `RawMoc3`: raw output with `FieldConfidence` taxonomy for format
  knowledge.
- `report.rs` / `text.rs` - deterministic inspection report (JSON and text).
- `error.rs` / `limits.rs` - structured errors (`code()`, offset, context,
  suggestion) and pre-allocation limits with `try_reserve` allocation.

### Failure model

- **Errors** (no report): header/version problems, offset table violations,
  section bounds/alignment/monotonicity, negative/oversized counts,
  cross-reference violations (index/range escape), allocation failure.
- **Anomalies** (report continues): non-zero unknown padding/slots,
  non-boolean flags where booleans are expected, unknown flag bits,
  identifier issues (UTF-8, terminator, duplicate), negative texture numbers.
- Corrupted input must always yield a structured outcome; panics are treated
  as bugs and are covered by corruption/fuzz-smoke tests.

### Memory safety strategy

- every count is checked against `Limits` before any allocation,
- every allocation uses `try_reserve_exact`,
- every read is bounds checked (`ByteView::slice`),
- no recursion in the parser (flat tables + iterative cycle detection),
- whole-file read with a configurable maximum size (default 512 MiB).

## Recovery confidence

Format knowledge uses the master spec taxonomy (`Unknown`, `Unverified`,
`Observed`, `Hypothesis`, `Confirmed`, see `raw.rs::FieldConfidence` and
`docs/FORMAT_NOTES.md`). Derived hierarchy edges use the recovery taxonomy
(`Exact`, `Derived`, `Heuristic`, `Unknown`, see
`report.rs::RecoveryConfidence`) and always name the stored field they came
from.

## Determinism

- no timestamps, no randomness, no hash-map iteration in output,
- textures are emitted from a `BTreeMap` (ascending page number),
- identifiers use the deterministic fallback scheme from master spec
  section 7 (`ArtMesh_0001`, `Parameter_0004`, ...) only when the stored name
  is empty,
- golden tests compare the exact JSON bytes produced by the CLI.

## Test strategy

See `docs/TEST_PLAN.md`. Summary: unit tests in `moc3-ingest`, `live2d-ir`
and `hierarchy-recovery`; integration tests for fixtures/corruption; golden
inspection reports, golden IR documents and golden recovered-project
documents; mapper/round-trip/validator tests; hierarchy resolution, trace,
ambiguity, orphan, cycle, deep and scale tests; CLI end-to-end tests
(determinism, exit codes, input immutability); workspace smoke tests including
the parser-independence boundary checks. All checked-in fixture data is
synthetic (see `fixtures/README.md`).

AGENT.3.5 adds two offline test surfaces: a layout-interop regression test
(`crates/moc3-ingest/tests/layout_interop.rs`, both body placements parse
identically) and the dev-only differential harness tests
(`tools/reference-harness/tests/harness.rs`, mock providers only). Real
external comparisons are manual and env-gated
(`docs/DIFFERENTIAL_FINDINGS.md`); they are never part of `cargo test`.

AGENT.4 adds keyform crate tests (axis normalization, 1D/2D/3D grids,
cardinality limits/overflow, duplicates/unsorted/non-finite, dangling
references, typed target payloads, determinism, round-trip, geometry
non-amplification, IR-level negative fixtures), recovery-core pipeline tests
(`crates/recovery-core/tests/keyform_recovery.rs`: per-fixture expectations,
17 golden keyform documents, import/validate, body-placement A/B equality),
CLI end-to-end tests (`apps/recovery-cli/tests/cli_keyforms.rs`) and the
extended differential keyform sections.
