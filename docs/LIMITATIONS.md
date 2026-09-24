# Limitations (AGENT.1)

## Verification status

- **No real Cubism export has been parsed yet.** All checked-in fixtures are
  synthetic files produced by `tools/fixture-gen` from the documented layout.
  The layout itself is cross-checked between independent public references
  (see `FORMAT_NOTES.md`) and, since AGENT.3.5, against a real external reader
  (`py-moc3`) on layout-conforming inputs. Real-file validation is still
  pending. Do not claim production readiness before an owned `.moc3` passes
  inspection and the results are compared against an independent loader.
- Big-endian files are implemented in the reader but never observed.

## What the inspector does not extract (yet)

- keyform vertex positions, UV values, triangle indices and key values
  (counts are reported; values arrive with the Live2D IR in AGENT.2),
- per-keyform opacity/draw order/color values,
- blend shape and offscreen data (ranges validated, contents untouched),
- glue weights (only structural ranges/indices are validated),
- motions, physics, expressions, pose - those live in separate files
  (`.motion3.json`, `.physics3.json`, ...) and are out of scope for AGENT.1,
- texture images; only texture page numbers referenced by art meshes are
  reported (the page count comes from `model3.json`, which is not read yet).

Each skipped area is listed in the report under `not_extracted` with counts,
so nothing is silently dropped.

## Validation depth

- Structural validation mirrors the safety checks of a clean-room Core
  implementation: alignment, bounds, monotonic data order, count sanity,
  index/range cross-references, warp grid equation, keyform-span coverage.
- Not yet implemented: full blend shape keyform-window verification
  (4.2+/5.0 targets), offscreen keyform windows, and interpolation
  semantics. Those belong to AGENT.3/AGENT.4/AGENT.6.
- Non-structural oddities are reported as anomalies and do not fail the
  inspection; a `--strict` mode is intentionally not offered yet.

## Performance and memory

- The whole file is read into memory (default cap 512 MiB, raise with
  `--max-file-size` only for trusted inputs). Streaming/lazy parsing was
  considered; the cap plus pre-allocation limits makes it unnecessary for
  AGENT.1.
- Counts are limited before allocation and allocations use `try_reserve`.
  Worst-case memory is bounded by `Limits::default()`
  (`max_total_elements = 120,000,000`).
- Parse time is O(sections + extracted arrays); large keyform/position pools
  are counted but not materialised.

## Interpretive limits

- The file does not contain original layer names, PSD structure, hidden or
  deleted layers, or artist workflow data. Nothing in this repository may be
  used to claim "original project recovery"; later phases must speak of
  *reconstructed* structure and *structural comparison* only.
- Runtime scratch sections and several flag bits are preserved but not
  interpreted; their semantics are marked `Unverified`/`Unknown` in
  `FORMAT_NOTES.md`.

## AGENT.2 (IR) limitations

- **Schema status is EXPERIMENTAL** (`live2d-ir/1`). It is not a stable
  public contract until real-file validation has happened.
- Not modeled yet (present in the file, validated, reported through an
  `unmapped_sections` diagnostic): blend shapes and constraints, offscreen
  surfaces (5.3), keyform color pools (multiply/screen). Part references to
  offscreen surfaces additionally produce an `unmapped_reference` diagnostic;
  that path is not exercised by the fixtures (the generator writes `-1`).
- Reverse-link checks now cover parts, deformers, masks and bindings;
  hierarchy cycles and draw order nesting cycles are diagnostics only.
- Glue structure is mapped and fixture-tested (`fixture-013`), but the
  weight/position pairing semantics remain `Unknown` (raw entries preserved).
- Keyform *selection/interpolation* (which keyform is active for a given
  parameter state) is AGENT.4 work; the IR stores the raw per-object
  keyform lists plus the binding grid dimensions.
- Base opacity and base draw order do not exist in `.moc3`; they are `null`
  with `Unknown` provenance. Original names, layer names and PSD structure
  are absent by definition.
- Texture `width`/`height`/`source_path` stay `null` until `model3.json`
  support lands in a later phase.
- Import validates structure, not provenance: a hand-edited document can
  claim `Exact` confidence for invented values. Trust imported IR only from
  trusted sources.
- Memory: the mapper consumes the parser output by value and copies each
  bulk pool element once (parser `ModelPools` to IR fields); no whole-pool
  clone happens. The file API drops the input byte buffer before mapping.
  Peak memory is roughly input + pools + IR; key values are deliberately
  duplicated per parameter and per binding (small, bounded). All bounded by
  `Limits`.

## AGENT.3 (hierarchy) limitations

- **Real-world hierarchy accuracy: UNVALIDATED** (no owned `.moc3`). Only
  structure statistics are reported; they are never an accuracy measure.
- Exactly one heuristic rule exists (HR-010, disabled by default). Its
  evidence (shared keyform binding) is weak by nature; enable it only as an
  experiment (`--allow-heuristic`).
- No automatic tie-breaking for ambiguous parents: equal-confidence
  candidates stay ambiguous. This is deliberate (work order section 15).
- Stored cycles are reported, not repaired; cycle-free hierarchies are not
  guaranteed for corrupt files.
- Range/draw-order/mask/glue evidence is deliberately unused for parenting;
  those relations live in the IR and remain available for later phases.
- Keyform selection and interpolation are AGENT.4 work; this phase groups
  and validates structure only.
- Offscreen surfaces (5.3) are not hierarchy nodes; part references to them
  are reported via IR-level `unmapped_reference` diagnostics.
- Performance: resolution is near-linear (memoized ancestor walks,
  index-aligned candidates, per-code diagnostic caps). The `.moc3` path is
  additionally bounded by parser limits; IR JSON inputs are bounded only by
  the file-size cap, so treat arbitrary hand-made IR documents as untrusted
  input.

## AGENT.3.5 (external reference audit) limitations

- **Cross-implementation agreement is not accuracy.** All differential
  numbers are per-field agreement with an external tool
  (`docs/DIFFERENTIAL_FINDINGS.md`); none of them validate the recovered
  hierarchy against author intent.
- **Only one external reader was executable here** (`py-moc3`, MIT). Its
  layout deviations (DF-001, DF-002, DF-004) mean it cannot be used as an
  oracle for version-6 canvas, part/art-mesh ID arrays or per-mesh counts.
  Other references were source-audited only; executing them needs build or
  runtime provisioning.
- Real-world compatibility stays **UNVALIDATED**; owned files and a runtime
  oracle are planned in `docs/GROUND_TRUTH_BENCHMARK.md` and
  `docs/RUNTIME_ORACLE_PLAN.md`. No accuracy claim of any kind is permitted
  until both exist.
- `docs/AGENT4_REFERENCE_INPUT.md` / `AGENT5_REFERENCE_INPUT.md` record the
  open spec decisions this audit surfaced (oracle provisioning, owned files,
  writer questions).

## AGENT.4 (keyform recovery) limitations

- **Real-world keyform accuracy: UNVALIDATED.** No owned `.moc3` exists; all
  keyform evidence comes from synthetic fixtures plus the py-moc3 raw
  differential (which has its own documented deviations, DF-001..DF-005).
- Multi-dimensional grid ordering is deliberately `Unknown`: AGENT.4 does
  not know which axis varies fastest, so consumers must not assume
  row-major/column-major from this document.
- Glue keyform semantics are deferred (KB-006); glue forms are preserved
  only as IR data and are not keyform targets.
- Blend-shape axes are preserved but experimental (KB-005): no
  interpolation semantics are claimed for them.
- Sparse grids are only reachable through hand-edited IR (the MOC3 parser
  rejects files whose declared forms do not cover the bound grid); the
  keyform layer reports them without repair.
- Non-finite stored keys are reported positionally by the keyform layer, but
  the CLI refuses such inputs earlier: NaN/Inf are Fatal IR diagnostics
  (`live2d-ir/1` invariant), so `recover-keyforms` exits 1 on those files.
- Interpolation/evaluation (parameter vector -> final vertices) is out of
  scope; see `docs/RUNTIME_ORACLE_PLAN.md` for the planned oracle.
- Cardinality is capped (`MAX_GRID_CARDINALITY = 1_000_000`); larger grids
  stay unresolved rather than expanded. Diagnostics are capped per code
  (`MAX_DIAGNOSTICS_PER_CODE = 256`) and unresolved entries at
  `MAX_UNRESOLVED_ENTRIES = 4096`, with explicit summary entries when the
  caps fire.
- Non-finite payload floats (opacity/draw order/angle/origin/scale) are
  reported positionally; canonical export refuses documents that contain
  them (`NonFiniteValue`), because JSON cannot represent NaN/Inf.

## AGENT.5 (CMO3 writer) limitations

- **The writer is minimal only.** CAFF (Gate 5A) and the structural
  `main.xml`/`.cmo3` gate (Gate 5B) pass for the minimal MODEL_IMAGE-mode
  document (`write_minimal_cmo3`). Deformers, AGENT.4 keyform grids,
  draw-order groups, masks beyond the first GUID and all sample/editor
  metadata are explicitly out of scope; the CLI is not implemented.
- The layered image is synthetic (one canvas-sized layer per visible mesh),
  never a recovered PSD; layer/group names are recovered mesh names or
  placeholders (`docs/CMO3_WRITER_DEFAULTS.md`).
- Cubism Editor open remains NOT_TESTED; structural validation is never
  reported as editor compatibility.
- Cubism Editor open is `NOT_TESTED`; structural success is never called
  editor compatibility.
- CAFF writing supports RAW entries only; ZIP modes 33/37 are recognised
  but their payloads are not encoded or decoded.
- Semantic mapping (strict/best-effort) exists and is tested; it records
  writer-required defaults separately from recovered data.
- PSD recovery and GUI remain out of scope.

## Tooling

- `cargo-fuzz` integration is planned after parser stabilisation (master
  spec section 15). Until then, deterministic corruption sweeps provide the
  no-panic evidence (now including the full IR pipeline).
- Golden reports and golden IR documents are machine-generated; review diffs
  instead of trusting them blindly when a schema changes.
- The IR golden importer test expects the checked-in goldens; regenerate
  them in two steps (`golden_ir_documents_match` with `UPDATE_GOLDEN_IR=1`,
  then the full suite) because the two golden tests run in parallel.
