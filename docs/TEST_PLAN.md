# Test Plan

Scope: AGENT.0-AGENT.4 (bootstrap, read-only inspector, normalized IR,
hierarchy reconstruction, external reference audit, semantic keyform
recovery). Test types follow master spec section 14: unit, integration,
snapshot/golden, corruption, fuzz (smoke), regression. Current total:
**278 tests, all passing** (44 synthetic fixtures + generated CMO3
documents). AGENT.5.1 gates 5B.1-5B.3 pass for the minimal MODEL_IMAGE
document (`docs/CMO3_VALIDATION.md`); Cubism open NOT_TESTED. AGENT.5 is PARTIAL: only
the CAFF/identity/XML/profile/mapping layers exist (see
`docs/CMO3_VALIDATION.md`).

AGENT.3.5 additions: `crates/moc3-ingest/tests/layout_interop.rs` (body
placement tolerance) and `tools/reference-harness` (dev-only differential
harness, offline tests including `read_capped` output-limit boundaries and
`is_unsupported` key-segment boundaries). AGENT.4 additions: the
`keyform-recovery` crate suite, `recovery-core` keyform pipeline/golden
tests, CLI keyform tests and the extended differential keyform sections.
External requirements are never part of the default run:

## How to run

```text
cargo test                     # everything
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
cargo run -p fixture-gen -- fixtures/synthetic     # regenerate fixtures
$env:UPDATE_GOLDEN="1"; cargo test -p moc3-ingest --test parse_fixtures
$env:UPDATE_GOLDEN_IR="1"; cargo test -p recovery-core --test ir_golden golden_ir_documents_match
$env:UPDATE_GOLDEN_HIERARCHY="1"; cargo test -p recovery-core --test hierarchy_recovery golden_recovered_projects_match
$env:UPDATE_GOLDEN_KEYFORMS="1"; cargo test -p recovery-core --test keyform_recovery golden_keyform_documents_match
```

External differential runs (never required for development; env-gated):

```text
powershell tools/reference-harness/scripts/fetch_py_moc3.ps1   # clone pin 2fb112e11a
$env:PY_MOC3_DIR = "target/reference/py-moc3"
cargo run -p reference-harness -- compare fixtures/synthetic/fixture-010-v53.moc3 `
    --provider ours --provider py-moc3 --json --output report.json
```

## Matrix

| Layer | Location | What it proves |
|---|---|---|
| Unit | `moc3-ingest` `#[cfg(test)]` (table slot/name consistency), `fixture-gen` tests (name/order, determinism, round-trip) | internal invariants, generator honesty |
| Integration (fixtures) | `crates/moc3-ingest/tests/parse_fixtures.rs` | 12 fixtures across versions 1-6 parse with exact expected values (ids, counts, parents, masks, keyform spans, textures) |
| Golden | same file, `golden_reports_match` + `fixtures/expected/*.report.json` | exact JSON snapshot of the full report; regressions are visible in diffs |
| Corruption | `crates/moc3-ingest/tests/corruption.rs` | structured errors (no panics) for truncation, bad magic/version/endian, patched counts/offsets/references, random bytes |
| Fuzz smoke | corruption tests (`single_byte_flips_never_panic`, `random_inputs_never_panic`) | bounded, deterministic no-panic sweep; `catch_unwind` catches escapes |
| End-to-end CLI | `apps/recovery-cli/tests/cli_inspect.rs` | exit codes, JSON error envelope, golden equality, repeated-run equality, input file immutability |
| IR unit (no parser) | `crates/live2d-ir/tests/validate.rs`, in-crate unit tests | id assignment policy, validator matrix (fatal vs warning vs recoverable), JSON export refusal on fatal, import error codes, hand-built models only |
| IR mapping | `crates/recovery-core/tests/ir_mapping.rs` | per-fixture semantic assertions (ids, key values, grids, reverse links, masks, textures, blend modes), duplicate/empty-id determinism, unknown preservation, NaN/range failures |
| IR golden | `crates/recovery-core/tests/ir_golden.rs` + `fixtures/expected-ir/` | 12/12 canonical IR snapshots; goldens re-imported and validated |
| IR corruption | `crates/recovery-core/tests/ir_corruption.rs` | truncations/byte flips/random bytes through parse->map->validate->serialize with zero panics; malformed IR import (syntax/schema/dangling ref) |
| IR CLI | `apps/recovery-cli/tests/cli_ir.rs` | export determinism, `--output` equality, `--compact`, structured failures (parser and IR validation), validate-ir golden accept/reject |
| Hierarchy unit | `crates/hierarchy-recovery` unit tests (`cycles.rs`, `rule.rs`, hands-on graph tests) | iterative cycle detection (incl. 200k chain), rule id hygiene, self-parent/duplicate/wrong-type rejections, validator negative matrix, explain output, statistics semantics |
| Hierarchy fixtures | `crates/recovery-core/tests/hierarchy_recovery.rs` | per-fixture resolution assertions (Exact chains, deformer precedence, orphan preservation, ambiguity policy, cycles, HR-007 conflict, self-parent fallback), deep (1502 nodes) and scale (6001 nodes) runs, geometry-free JSON check |
| Hierarchy golden | same file, `fixtures/expected-hierarchy/` | 25 canonical recovered-project snapshots (large/deep fixtures excluded and covered by determinism runs) |
| Hierarchy CLI | `apps/recovery-cli/tests/cli_hierarchy.rs` | tree markers, JSON determinism, `--output` equality, `--strict` exit codes, `--allow-heuristic`, `--explain`, IR JSON input equivalence, corrupt input handling |
| Workspace smoke | `tests/workspace-smoke/tests/smoke.rs` | recovery-core pipeline determinism, fixture staleness detection, **parser-independence boundary checks for `live2d-ir` and `hierarchy-recovery`** (`cargo metadata` graph) |
| Layout interop | `crates/moc3-ingest/tests/layout_interop.rs` | relocated-body (py-moc3 offset convention) files parse to byte-identical inspection semantics (offsets/name/size excluded) |
| Differential harness | `tools/reference-harness/tests/harness.rs` (offline) | snapshot determinism, mock consensus/dispute/order-only tolerance, float policy, unsupported/missing classification, ours-unsupported handling, subprocess timeout/output caps, report determinism and wording, keyform evidence summary and comparison |
| Keyform unit + IR-level negatives | `crates/keyform-recovery/tests/keyform_recovery.rs` | 1D/2D/3D grids, 0D static bands, duplicate/unsorted/out-of-range/non-finite keys, dangling parameter/binding/target (IR-level negative fixtures), cardinality limit/overflow, axis/limit caps, sparse partial grid (fewer forms than the bound product, no padding), form-span gaps and out-of-bounds indices (validator Fatal), non-finite payload refusal, tampered-explain hardening, typed target payloads, blend-shape experimental, glue deferral, determinism, byte-identical JSON round-trip, duplicate-grid fatal validation, geometry non-amplification, large dataset, project cross-check, explain |
| Keyform pipeline + goldens | `crates/recovery-core/tests/keyform_recovery.rs` | per-fixture expectations for all 17 keyform fixtures, 17 golden `recovered-keyforms/1` documents (import + re-validate), determinism/round-trip over every fixture, body-placement A/B equality, strict violations, archived AGENT.3.5 evidence retention, large dense dataset |
| Keyform CLI | `apps/recovery-cli/tests/cli_keyforms.rs` | human report counts, deterministic JSON, `--output` equality, IR JSON input parity, `--strict` exit codes, non-finite CLI refusal, `--explain` trace, input immutability |
| CMO3 CAFF (AGENT.5) | `crates/cmo3-writer` unit tests | round-trip with default/zero/negative keys, guard bytes, truncation/bad magic, corrupted payload visibility, out-of-range offsets, unsafe/duplicate paths, entry caps, varint lengths, compressed-mode recognition (no guessing) |
| CMO3 identity/XML/profile | `crates/cmo3-writer` unit tests | pool allocation determinism, UUID shape/variant/determinism, escaping of all five XML entities, canonical float formatting, profile values match both pinned sources, unique import list |
| CMO3 mapping | `crates/cmo3-writer/tests/mapping.rs` | strict `MissingTextureAsset` failure, strict success with assets, writer-default trace separation, archive entry naming safety, determinism |
| CMO3 XML/validators | `crates/cmo3-writer/src/validate.rs` tests | XML scanner: duplicate `xs.id`, dangling `xs.ref`, GUID-type mismatch, balanced references |
| CMO3 gates 5B.1-5B.3 | `crates/cmo3-writer/tests/minimal_cmo3.rs` | single-mesh and multi-mesh (1 layered image / N layers, distinct layer selection) gates, byte determinism, CAFF integration (decode + validate + payload equality), typed/XML negatives without output, golden `main.xml` and normalized pipeline snapshots (`fixtures/expected-cmo3/`) |
| Boundary smoke | `tests/workspace-smoke/tests/smoke.rs` | keyform-recovery and cmo3-writer must not depend on `moc3-ingest`/`recovery-core`/`fixture-gen` (manifest + `cargo metadata` graph) |

## Fixture inventory (synthetic; `fixtures/synthetic/`)

| Fixture | Version | Content |
|---|---|---|
| 001 single artmesh | 3.3 | 1 art mesh, no params, Y-reversed canvas |
| 002 artmesh + param | 3.3 | 1 parameter (2 keys), 1 part, 2 keyforms |
| 003 warp | 3.3 | 2x2 warp deformer over an art mesh |
| 004 nested warp | 3.3 | warp -> warp -> art mesh |
| 005 rotation | 3.3 | rotation deformer (30°) |
| 006 clipping mask | 3.3 | art mesh masked by another mesh |
| 007 multi params | 4.2 | 5 parameters, 4 bindings, 2 parts, differing keyform grids, color sections |
| 008 multi textures | 4.2 | two texture pages |
| 009 v50 | 5.0 | 64-int count info, 5.0 sections |
| 010 v53 | 5.3 | 480-slot table, offscreen/blend-mode fields |
| 011 v30 | 3.0 | base layout, no quad transform |
| 012 v40 | 4.0 | warp with quad transform |
| 013 glue | 3.3 | two glued art meshes with info entries and one glue keyform |

Fixture 001 additionally exercises non-default drawable flags (additive +
double sided); fixture 007 exercises nested draw order groups (a part item
nesting a second group); fixture 013 exercises glue. The offscreen-surface
reference path has no fixture (the parser rejects non-default values when no
surfaces exist); that gap is documented in LIMITATIONS and covered by
validator-level tests only.

### Keyform fixture inventory (`fixtures/synthetic/keyform-*`, AGENT.4)

| Fixture | Shape |
|---|---|
| 001 | zero-dimensional: empty binding, one static form per target |
| 002 | 1D, three keys |
| 003 | 2D, 3x3 |
| 004 | 2D, 2x3 |
| 005 | 3D, 2x2x2 |
| 006 | part draw-order forms |
| 007 | warp deformer forms |
| 008 | rotation deformer forms |
| 009 | art mesh forms |
| 010 | multiple bindings (one parameter each) |
| 011 | multiple parameters on one binding, two targets |
| 012 | duplicate stored keys (preserved) |
| 013 | unsorted stored keys (preserved in order) |
| 014 | stored forms exceed expected cardinality (5 vs 4) |
| 016 | non-finite stored key (reported positionally; IR golden skipped) |
| 019 | deep nested hierarchy targets sharing one binding |
| 020 | larger dense dataset (64 forms) |

Negative cases that cannot exist in a valid `.moc3` file (dangling
parameter, dangling binding/target, sparse partial grid with fewer forms
than the bound grid, cardinality overflow/limit, non-finite handling at the
library layer, form-span gaps) are IR-level negative fixtures built in
`crates/keyform-recovery/tests/` (work order section 53).

### Hierarchy fixture inventory (`fixtures/synthetic/hierarchy-*`)

| Fixture | Shape |
|---|---|
| 001 | Part -> ArtMesh |
| 002 | Part -> Warp -> ArtMesh |
| 003 | Part -> Rotation -> ArtMesh |
| 004 | Part -> Warp -> Warp -> ArtMesh |
| 005 | Part -> Warp -> Rotation -> ArtMesh |
| 006 | Part -> Part -> Part chain + meshes |
| 007 | orphan ArtMesh preserved |
| 008 | shared-binding orphan (ambiguous under `--allow-heuristic`) |
| 009 | stored cycle (two warps mutually parented) |
| 010 | part association conflicting with the deformer chain (HR-007) |
| 011 | 1500-deep warp chain (golden-excluded; determinism + tree tests) |
| 012 | 1500 flat meshes under one part |
| 013 | 2000 warps + 4000 meshes scale run (golden-excluded) |
| 014 | self-parent deformer (illegal relation rejected, part fallback) |

Corruption sweeps skip the two largest fixtures in their full loops and use
deterministic sampled mutations instead (`large_fixtures_are_sampled*`).

Corruption fixtures are generated in-test (deterministic patches), so they
cannot rot and require no binary blobs. Regression rule (master spec
section 14) applies: any future bug fix must add either a checked-in fixture
or a named corruption case to `corruption.rs`.

## Acceptance criteria mapping (AGENT.1)

| Criterion (spec) | Evidence |
|---|---|
| valid fixture parses without crash | `-p moc3-ingest` parse_fixtures (12/12), CLI golden test |
| corrupted file does not panic | corruption tests: 0 truncation panics, full byte-flip sweep on 001, strided sweeps on all, LCG inputs |
| structured errors | `Moc3Error::code()/offset/context/suggestion`; CLI JSON error envelope test |
| input file unchanged | `input_file_is_never_modified` (CLI), `parse_is_deterministic_and_read_only` (parser) |
| deterministic output | repeated-run equality in CLI + smoke tests, golden bytes |
| unit tests exist | `table::tests`, `fixture-gen` tests |
| at least 3 fixture types | 12 fixtures across 6 format versions |
