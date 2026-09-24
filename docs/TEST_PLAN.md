# Test Plan

Scope: AGENT.0/AGENT.1 (bootstrap + read-only inspector) and AGENT.2
(normalized IR). Test types follow master spec section 14: unit, integration,
snapshot/golden, corruption, fuzz (smoke), regression. Current total: **113
tests, all passing** (13 synthetic fixtures).

## How to run

```text
cargo test                     # everything
cargo clippy --workspace --all-targets --all-features
cargo fmt --all --check
cargo run -p fixture-gen -- fixtures/synthetic     # regenerate fixtures
$env:UPDATE_GOLDEN="1"; cargo test -p moc3-ingest --test parse_fixtures
$env:UPDATE_GOLDEN_IR="1"; cargo test -p recovery-core --test ir_golden golden_ir_documents_match
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
| Workspace smoke | `tests/workspace-smoke/tests/smoke.rs` | recovery-core pipeline determinism, fixture staleness detection, **parser-independence boundary check** (`cargo metadata` graph) |

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
