# Keyform Recovery (AGENT.4)

**Schema:** `live2d-recovery/recovered-keyforms/1`
**Status:** `EXPERIMENTAL`
**Implemented:** AGENT.4 (`crates/keyform-recovery`; CLI `recover-keyforms`).

This stage answers one question: **which parameter axis combination selects
or composes which stored form of which target object?** It consumes the
Live2D IR (and optionally the recovered project for cross-layer checks) and
emits a separate keyform document. It does **not** interpolate, evaluate or
render anything.

## 0. Layer separation (work order section 4)

```text
stored evidence (IR) -> normalized axes -> bands -> grids -> target forms
                                                        |
                                          runtime interpolation/evaluation
                                          is NOT part of this document
```

The IR schema and the recovered-project schema are unchanged. Geometry
payloads stay in the IR; this document references them by form index and
count (rule KF-009).

## 1. Pipeline (work order sections 23, 31)

```text
Live2D IR
  -> collect_binding_evidence   stored references only      (KB-001)
  -> normalize_axes             preserve + validate         (KB-002..005)
  -> build_binding_bands        axis sets + target usage    (KB-006, KB-007)
  -> derive_grid_shape          cardinality + classification(KF-001..007)
  -> map_forms                  typed target payloads       (KF-008)
  -> RecoveredKeyformModel      + validation
```

Function boundaries are deliberate: `collect_binding_evidence`,
`normalize_axes`, `build_binding_bands`, `derive_grid_shape`, `map_forms`,
`validate_recovered_keyforms`, `recover`. There is no
`recover_all_keyforms()` mega-function.

## 2. Parameter axis

A `ParameterAxis` is the stored key table of one binding parameter:

```json
{ "parameter": "ParamAngleX", "keys": [-30.0, 0.0, 30.0],
  "resolved": true, "usable": true, "stored_key_count": 3 }
```

- `keys` keep the stored order verbatim; finite values only. Non-finite
  stored keys are reported positionally (`non_finite_key_indices`) because
  canonical JSON cannot carry NaN/Inf.
- Validation (KB-004): parameter existence, finiteness, duplicates,
  ordering, and range disagreement. **None of these are repaired**:
  duplicates are not deduplicated, unsorted keys are not sorted, and
  out-of-range keys are preserved with an `out_of_range_parameter_key`
  diagnostic (extension/extrapolation semantics are possible but not
  assumed).
- A parameter that does not resolve produces a dangling axis (KB-003): the
  axis is preserved with its stored keys and marked unresolved.

## 3. Binding band

A `BindingBand` is the normalized axis set of one binding plus the targets
that use it (`band:000005`):

```json
{ "id": "band:000005", "binding": "binding:000005",
  "axes": [ ... ], "expected_cardinality": {"state": "exact", "value": 9},
  "targets": [ {"kind": "art_mesh", "id": "..."} ] }
```

- Glue references are collected but never promoted to targets (KB-006):
  glue pairing semantics remain Unknown, so glue entries produce a
  deferred entry instead.
- A target without a stored binding is represented by a zero-dimensional
  static band (KB-007): no axes, cardinality 1. Several stored forms
  without a binding raise `static_target_multiple_forms` and stay Unknown.
- Blend-shape participation marks the whole band `experimental = true`
  (KB-005): axes are preserved, but no interpolation semantics are claimed
  and the band/grid confidence stays Unknown.

## 4. Grid model

A `KeyformGrid` associates one target's stored forms with its band:

```json
{ "id": "grid:binding:000005:art_mesh:...", "band": "band:000005",
  "target": {"kind": "art_mesh", "id": "..."},
  "axes": [{"parameter": "ParamAngleX", "key_count": 3, "resolved": true}],
  "expected_cardinality": {"state": "exact", "value": 9},
  "stored_form_count": 9, "layout": "dense", "ordering": "unknown",
  "confidence": "derived" }
```

- `expected_cardinality` is the checked product of axis key counts
  (KF-001). Overflow (`keyform_cardinality_overflow`) and products above
  `MAX_GRID_CARDINALITY` (`keyform_cardinality_limit_exceeded`) stay
  unresolved (KF-002); the Cartesian space is never materialized.
- `dense` means: stored form count equals the expected cardinality
  (KF-003). It is a count comparison, not proof of a Cartesian layout.
- `sparse` means: fewer stored forms than expected (KF-004). This is only
  reachable through hand-edited IR; the MOC3 parser itself rejects files
  whose declared forms do not cover the bound grid. No padding, no
  duplication.
- more stored forms than expected: `unknown_layout` + diagnostic (KF-004).
- Stored form indices must be exactly `0..n`; gaps raise
  `keyform_form_span_mismatch` and are preserved without reindexing
  (KF-007).

## 5. Grid ordering (work order section 13)

- 0D/1D: `derived` - the stored sequence is the only ordering evidence
  (KF-006).
- 2D+: `unknown` - AGENT.4 never guesses row-major/column-major (KF-005).
  `observed` is reserved for independently validated evidence and is never
  claimed yet.
- Stored axis key order is preserved; if it is unsorted, that is reported
  on the axis, not silently normalized.

## 6. Target keyforms (work order sections 16-21)

Each target kind has its own payload; there is no generic untyped blob:

- **Part** (`part`): `draw_order` per form. Part opacity is not stored in
  `.moc3`, so it is absent - not defaulted (KF-008).
- **WarpDeformer**: `opacity` per form plus a geometry reference
  (`position_count`). The warp **grid** (`rows`/`columns` control points)
  and the parameter **grid** are different grids and are never conflated.
- **RotationDeformer**: `opacity`, `angle`, `origin`, `scale`,
  `reflect_x`, `reflect_y` per form. Base angle stays in the IR.
- **ArtMesh**: `opacity`, `draw_order` per form plus a vertex geometry
  reference. Static uvs/indices stay in the IR.

## 7. Memory and complexity (work order sections 21, 59-63)

- geometry is never copied (KF-009): a 300-form x 32-vertex mesh yields a
  keyform document far smaller than the IR (tested);
- no combinatorial materialization (KF-010): only axis summaries and a
  cardinality are stored;
- hard limits: `MAX_GRID_CARDINALITY = 1_000_000`,
  `MAX_AXIS_KEYS = 65_536`, `MAX_AXES_PER_BAND = 64`; violations are
  unresolved, never truncated silently;
- complexity is O(bindings + axes + stored forms + payload scalars); there
  is no per-form scan over all bindings.

## 8. Confidence and provenance

The AGENT.2/AGENT.3 taxonomy is kept: `exact` (stored axis values),
`derived` (cardinality, static bands, dense/sparse classification),
`heuristic` (none in AGENT.4), `unknown` (dangling/experimental/overflow).
A band is `exact` only when all axes are usable and the cardinality is
exact; the validator re-checks this contract.

## 9. Rules

Binding rules (`KB-`):

| Rule | Description | Confidence |
|---|---|---|
| KB-001 | stored binding references are collected as evidence | exact |
| KB-002 | axis = stored key table, stored order preserved | exact |
| KB-003 | dangling parameter axis preserved as unresolved | unknown |
| KB-004 | duplicates/unsorted/out-of-range keys reported, never fixed | exact (values) |
| KB-005 | blend-shape axes marked experimental | unknown |
| KB-006 | glue bindings deferred, never promoted | unknown |
| KB-007 | target without binding = zero-dimensional static band | derived |

Keyform/grid rules (`KF-`):

| Rule | Description | Confidence |
|---|---|---|
| KF-001 | cardinality = checked product of axis key counts | derived |
| KF-002 | overflow / limit exceeded stays unresolved | derived |
| KF-003 | stored == expected -> dense | derived |
| KF-004 | stored != expected -> sparse/unknown + diagnostic | derived |
| KF-005 | multi-dimensional ordering never guessed | unknown |
| KF-006 | 0D/1D ordering follows the stored sequence | derived |
| KF-007 | form indices must be contiguous; gaps preserved | derived |
| KF-008 | target-specific stored payloads; absent fields absent | exact |
| KF-009 | geometry referenced, never copied | exact |
| KF-010 | no Cartesian materialization | derived |

## 10. Validation

`validate_recovered_keyforms(document, ir, project?)` checks duplicate band
and grid ids, dangling references, wrong target types, duplicate axes,
form-index bounds, statistics accounting, confidence/provenance contracts
and (when a project is given) that every target exists in the recovered
project. Structural corruption is Fatal and blocks export; expected
unresolved states (dangling parameters, cardinality mismatches,
experimental blend shapes) stay non-fatal.

## 11. CLI

```text
recovery recover-keyforms <model.moc3|model.ir.json>
    [--json] [--output PATH] [--strict] [--explain TARGET]
    [--max-file-size BYTES]
```

- human output prints the statistics report (section 12),
- `--json`/`--output` emit the canonical document,
- `--explain` prints one target/band/grid trace,
- `--strict` exits non-zero on dangling parameters/targets, non-finite
  keys, cardinality overflow/limit/mismatch and form-span mismatches.

## 12. Report

```text
Keyform Recovery

Targets             94
Bindings            121
Parameter axes      143
Stored keyforms     687

Exact bindings      110
Derived bindings     11
...
```

These are exact structural counts from stored data. They are never
accuracy percentages (work order section 72).

## 12b. Fixtures, goldens and negative cases

- Positive fixtures: `fixtures/synthetic/keyform-001..020` (see
  `docs/TEST_PLAN.md` for the shape of each).
- Golden documents: `fixtures/expected-keyforms/<fixture>.keyforms.json`
  (17 files), regenerated with `UPDATE_GOLDEN_KEYFORMS=1` and cross-checked
  by explicit per-fixture expectations in
  `crates/recovery-core/tests/keyform_recovery.rs` (the goldens are not
  accepted merely because the generator wrote them).
- IR-level negative fixtures (not expressible in a valid `.moc3`):
  dangling parameter/binding/target, sparse grid (fewer forms than the
  bound product), cardinality overflow/limit, non-finite handling at the
  library layer, form-span gaps. They live in
  `crates/keyform-recovery/tests/keyform_recovery.rs`.
- `keyform-016` has no IR golden (non-finite keys are Fatal at the IR
  layer) but does have a keyform golden: the keyform layer reports the
  non-finite positions instead of embedding them.

## 13. Differential validation and known unknowns

- The AGENT.3.5 harness is extended with binding/keyform sections
  (`bindings`, `binding_axes`, `axis_keys`, `keyform_counts`,
  `keyform_grid_shape`, `target_type`, `target_id`); `py-moc3` remains a
  raw-layout evidence source, and DF-001..DF-004 keep their documented
  status (`docs/DIFFERENTIAL_FINDINGS.md`).
- `moc2cmo` remains source-audited (not executable here); its
  BindingBand/ParameterBinding/Keyform concepts are compared
  conceptually in the mapping documents, not treated as ground truth.
- Known unknowns: multi-dimensional form ordering, glue keyform semantics,
  blend-shape interpolation, out-of-range key semantics, and real-world
  keyform behavior (no owned `.moc3`; **real-world keyform accuracy:
  UNVALIDATED**).
- Runtime evaluation (parameter vectors -> final vertex positions) is
  intentionally out of scope; see `docs/RUNTIME_ORACLE_PLAN.md`.
