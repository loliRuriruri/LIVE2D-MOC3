# Limitations (AGENT.1)

## Verification status

- **No real Cubism export has been parsed yet.** All checked-in fixtures are
  synthetic files produced by `tools/fixture-gen` from the documented layout.
  The layout itself is cross-checked between independent public references
  (see `FORMAT_NOTES.md`), but real-file validation is still pending. Do not
  claim production readiness before an owned `.moc3` passes inspection and
  the results are compared against an independent loader.
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

## Tooling

- `cargo-fuzz` integration is planned after parser stabilisation (master
  spec section 15). Until then, deterministic corruption sweeps provide the
  no-panic evidence.
- Golden reports are machine-generated; review diffs instead of trusting
  them blindly when the report schema changes.
