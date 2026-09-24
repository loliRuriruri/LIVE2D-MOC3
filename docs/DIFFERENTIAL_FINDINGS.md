# Differential Findings (AGENT.3.5)

Status: findings recorded at the AGENT.3.5 baseline (HEAD `159da8f` plus the
uncommitted AGENT.3.5 work). Evidence artifacts are checked in under
`docs/reports/agent-3.5/differential/` (JSON + human rendering per run).

> These are **cross-implementation comparisons**, not recovery-accuracy
> measurements. Agreement with another tool does not prove correctness and
> disagreement does not prove a bug; each case below is classified and
> tracked as a concrete follow-up.

## Method

- Tooling: `tools/reference-harness` (`recovery-reference` binary), dev-only.
  Our side runs in process (`recovery-core` + `hierarchy-recovery`); external
  sides run as capped subprocesses.
- Snapshot schema: `live2d-recovery/differential-snapshot/1`; deterministic
  FNV-1a fingerprint over canonicalized fields; floats compared with
  `abs <= 1e-6` or `rel <= 1e-5`.
- Commands (see `tools/reference-harness/scripts/fetch_py_moc3.ps1`):

  ```powershell
  $env:PY_MOC3_DIR = "<checkout>"   # pinned at 2fb112e11a
  cargo run -p reference-harness -- compare <file.moc3> `
      --provider ours --provider py-moc3 --json --output <report.json>
  ```

## Runs

| Run | Input | External outcome | Comparable | Agreement | Disagreement | Notes |
|---|---|---|---|---|---|---|
| R1 | `fixtures/synthetic/fixture-002-artmesh-param.moc3` (v3.3, body at `0x2C0`) | `REFERENCE_ERROR` (py-moc3 traceback) | 0 | 0 | 0 | DF-001 |
| R2 | `target/reference/relocated-002-b.moc3` (same file, body shifted to py-moc3 offset 1984) | ok | 46 | 24 | 3 | DF-003, DF-004 |
| R3 | `fixtures/synthetic/fixture-010-v53.moc3` (version byte 6) | ok | 38 | 27 | 8 | DF-002 |

`relocated-002-b.moc3` is a generated interop artifact (insert 1280 zero bytes
at the end of the offset table, patch every non-zero offset by +1280). It is
not a fixture and is not checked in; the transformation is reproduced by the
test in `crates/moc3-ingest/tests/layout_interop.rs`.

## DF-001 - py-moc3 hardcodes the body at offset 1984

- **Observation:** `py-moc3` (`src/moc3/_core.py`, pinned `2fb112e11a`) reads
  the count-info table at `DEFAULT_OFFSET = 1984` with
  `COUNT_INFO_SIZE = 128` and `COUNT_INFO_MAX = 23`, regardless of the file's
  version byte or offset table. Our fixtures (and the v1-5 layout used by
  PurismCore / `mocari` / `moc2cmo`) place the body immediately after the
  640-byte offset table at `0x2C0 = 704`; version 6 places it at 1984.
- **Result:** py-moc3 aborts on every v1-5 input we produce:
  `struct.error: unpack_from requires a buffer of at least 2076 bytes ... at
  offset 1984` (see `fixture-002.py-moc3.txt`).
- **Classification:** external-reader deviation, `E3` (single implementation),
  corroborated negatively by our relocated-body experiment.
- **Impact on our tool:** none; framing/interop only.
- **Action:** our parser follows the offset table pointers, so it accepts both
  conventions. Regression test `layout_interop.rs` asserts that original and
  relocated bodies produce byte-identical inspection semantics (offsets,
  file name and file size excluded).

## DF-002 - py-moc3 misreads version-6 files it accepts

- **Observation (R3):** py-moc3 parses `fixture-010-v53.moc3` (version byte 6)
  but reports canvas `0/0/0/0/0` (all five fields) and swaps art-mesh counts:
  `vertex_count` 6 vs our 4, `uv_count` 12 vs our 8 (both float counts, see
  DF-003), `index_count` 4 vs our 6. Parts, parameters, hierarchy edges and
  the version byte agree.
- **Interpretation:** the swapped counts are consistent with py-moc3 applying
  its own art-mesh section order (see DF-004) while the canvas zeros are
  consistent with its fixed body offset (DF-001) not matching the v6 layout.
  Our canvas/version values come from the same bytes both tools read, and our
  canvas values are the ones our fixtures are generated with (see
  `FORMAT_NOTES.md` section on canvas).
- **Classification:** `E3` (single implementation supports our reading:
  `E2` across PurismCore + `mocari` + `moc2cmo` for the layout itself).
- **Impact on our tool:** none; py-moc3 cannot be used as an oracle for v6
  canvas or art-mesh counts.
- **Action:** documented; no production change.

## DF-003 - count units and coverage

- **Observation (R2):** after normalizing units, count fields agree
  (13/15 `MATCH`), including `uvs` = number of floats (8 for 4 UVs),
  `keys`, `keyform_positions`, `position_indices`, `keyform_bindings`. The
  per-mesh `uv_count` and the group `counts.uvs` were both corrected in the
  harness to float counts (they previously reported UV pairs).
- **Naming deviation (py-moc3):** py-moc3's own `CountIdx` names slot 12
  `KEYFORM_BINDING_BANDS` and slot 13 `KEYFORM_BINDINGS` and has no
  "parameter bindings" entry. The comparison uses index alignment with our
  parser's count table (which is validated independently), so the values
  compare validly; the label `counts.parameter_bindings` is ours, not
  py-moc3's, and this deviation is recorded here like DF-004 rather than
  treated as agreement on naming.
- **Remaining gaps:**
  - `counts.parameter_bindings`: our IR merges key-table identity into
    parameter key values by design, so the harness reports
    `UNSUPPORTED_REFERENCE` for our side instead of a missing value.
  - `counts.textures`: our snapshot exposes it; py-moc3 does not model
    textures at all, so it is `MISSING_REFERENCE` (expected).
- **Classification:** `E2` for the agreed counts; the two gaps are
  definitional, `E3`.
- **Impact:** none on parsing; harness reporting only.
- **Action:** harness normalization fixed (float counts, derived `keys` and
  `keyform_positions`, explicit unsupported entry for
  `counts.parameter_bindings`); covered by harness tests.

## DF-004 - ID and section-order disagreement on py-convention input

- **Observation (R2):** even when the body offset matches py-moc3's own
  convention, py-moc3 reads `Parts: [""]` and `ArtMesh IDs: ["c_00"]`, where
  our parser reads `Part_Synthetic_00` and `ArtMesh_Synthetic_00`.
  `"c_00"` is exactly the 64-byte window starting 16 bytes into
  `ArtMesh_Synthetic_00`; the parameter ID reads correctly.
- **Interpretation:** py-moc3's order/stride for the part and art-mesh ID
  arrays differs from ours. Our order is `runtime_space, id, ...` per block
  and is corroborated by three independent sources (`PurismCore` C reader,
  `mocari` docs.rs sources, `moc2cmo` sources) used since AGENT.1; the
  disagreement is confined to py-moc3.
- **Classification:** `E2` (ours = PurismCore + `mocari` + `moc2cmo`) vs
  `E3` (py-moc3 outlier).
- **Impact:** py-moc3 must not be used as an oracle for part/art-mesh ID
  arrays or per-mesh fields; the three disputed fields
  (`parts.ids`, `art_meshes.ids`, `hierarchy_edges`) are exactly this case.
  All other R2 groups (`canvas`, `parameters`, `counts`, `endian`, version,
  both ID-less part fields) agree.
- **Action:** documented; no production change. Any future differential
  report that disputes these fields has to carry this finding as context.

## What did not change

- No parser, IR, hierarchy or CLI semantics changed because of any finding.
- No rule (`HR-###`, naming, confidence taxonomy) was relaxed.
- Real-world compatibility remains **UNVALIDATED**; no owned real `.moc3`
  exists in the repository (see `docs/GROUND_TRUTH_BENCHMARK.md`).
