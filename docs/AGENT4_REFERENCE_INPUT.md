# AGENT.4 Reference Input (from AGENT.3.5)

Hand-off note. Per `docs/FORMAT_NOTES.md`, AGENT.4 owns model evaluation /
interpolation semantics (keyform interpolation, deformer application, draw
state). This file lists what AGENT.3.5 learned that AGENT.4 can rely on, and
what it must not assume.

## Reusable assets

- Reference matrix with pinned revisions, licenses and modes:
  `docs/EXTERNAL_REFERENCE_MATRIX.md`.
- Real differential evidence and artifacts:
  `docs/DIFFERENTIAL_FINDINGS.md`,
  `docs/reports/agent-3.5/differential/`.
- Evaluation validation plan: `docs/RUNTIME_ORACLE_PLAN.md`.
- Layout interop guarantee: parser follows offset-table pointers and accepts
  both body placements (DF-001), regression-tested by
  `crates/moc3-ingest/tests/layout_interop.rs`.

## Rely on

- Parameter tables: parameter IDs, min/max/default, key values
  (`key_values`, plus `extension_key_values` where present), and key counts
  agreed with `py-moc3` on layout-conforming inputs (R2: `parameters 4/4`,
  `counts.keys = 2`, `counts.keyform_positions = 16`).
- Keyform position counts: art-mesh keyforms plus warp keyform control points
  sum to the on-disk `keyform_positions` count (harness derives this; 16/16
  match on R2).
- Reference "deformers" block existence: our IR intentionally exposes no
  `deformers.ids` string list; differential reports mark it
  `UNSUPPORTED_REFERENCE` (definitional, not a bug).

## Do not assume

- `py-moc3` is not an oracle for version-6 canvas, art-mesh counts/IDs or part
  IDs (DF-002, DF-004). Use it only for the groups that agreed in R2/R3.
- `counts.parameter_bindings` is not a standalone IR field; key-table identity
  is merged into parameter key values by design.
- Nothing here validates interpolation math; the runtime oracle plan must be
  executed before any evaluation claim.

## Pending spec decisions

1. Whether a future phase may build/run `PurismCore` or `ayagami` as the
   evaluation oracle, or must wait for a user-supplied Cubism Core runtime.
2. Whether owned real files will be provided
   (`docs/GROUND_TRUTH_BENCHMARK.md`); until then real-world compatibility
   stays UNVALIDATED.

No AGENT.4 code, crate or CLI surface was added in AGENT.3.5.
