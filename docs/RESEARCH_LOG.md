# Research Log

Chronological record of AGENT.0/AGENT.1 research (2026-09-24). Format facts
extracted from these sources are consolidated in `FORMAT_NOTES.md`.

## 1. Local environment

- Workspace target: `D:\test\liver2d`.
- Toolchain: `cargo 1.98.1`, `rustc 1.98.1`, `git 2.50.1`, clippy/fmt present.
- Searched `D:\test` for `.moc3` / `.cmo3` / `.model3.json`: **none found**.
  No owned model available locally -> fixtures are synthetic only, and this
  limitation is recorded in `LIMITATIONS.md` and the AGENT.1 report.

## 2. Official Live2D material

- Cubism Editor manual, "File Types and Extensions": `.cmo3` = editor project,
  `.moc3` = runtime model exported for embedding, `.model3.json` = file
  reference manifest. Outcome: confirms the pipeline input set and that the
  texture count/names live in `model3.json`, not in `.moc3`.
- Cubism Core API Reference (PDF): `csmMocVersion` enumeration and version
  ranges (3.0.00-3.2.07, 3.3.00-3.3.03, 4.0.00-...), plus the existence of an
  official "MOC3 Consistency Checker" tool. Outcome: independent version
  table for `version.rs`.
- Cubism SDK manual: `.moc3` loading flow goes through the closed-source
  Cubism Core; there is no official binary format specification from
  Live2D. Outcome: format knowledge must come from public research, recorded
  with explicit confidence.

## 3. Format research sources (in evaluation order)

1. `https://github.com/OpenL2D/moc3ingbird` - `src/moc3.hexpat`
   ("MOC3 Format Specification 2.1b", FDPL-1.0-US). Fetched in full.
   Outcome: rich field names and version-gated block inventory; its
   variable-length table model later conflicted with other sources.
2. Search results pointed to several parsers: `moc3-rs` (Apache/MIT),
   `py-moc3` (MIT, ported from a Java decompilation), `mocari` and
   `moc2cmo` (no license declared), `QiE2035/moc3-reader-re` (explicitly a
   decompilation; **not used**).
   Outcome: `moc3-rs` and the decompilation-derived projects were excluded as
   implementation sources; documented in `LICENSE_NOTES.md`.
3. `https://github.com/SakuraMotion/PurismCore` - MIT clean-room
   reimplementation of Cubism Core. Downloaded `moc3.c`, `moc3.h`,
   `verify.c`, `private.h`, `model.c`, `param.c`, `artmesh.c`, `deformer.c`,
   `glue.c`, `part.c` into a temporary reference directory **outside** the
   repository. Outcome: definitive section-order macros
   (`PSM__SECTIONS_V30/V33/V42/V50/V53`), header/count-info sizes, alignment
   and monotonicity rules, and the runtime-scratch nature of `*_runtime`
   sections. All parser rules were then re-expressed in Rust (no code
   copied).
4. Cross-checks against `mocari` (docs.rs sources) and `moc2cmo`
   (`art_mesh.rs`, `layout.rs`): slot indices 34-48, 68-71, 78-80, 107-113
   matched PurismCore's macro order exactly, which upgraded the table layout
   to `Confirmed` and revealed that the hexpat model was wrong (its
   `0x40..0x2C0` "padding" is actually the offset table).
5. `py-moc3` README: corroborates the fixed 160 x `u32` offset table.
   Provenance caveat recorded; corroboration only.

## 4. Key decisions taken during AGENT.0

- Adopt the fixed-slot offset table model (three agreeing implementations)
  and document the hexpat discrepancy instead of silently picking one.
- Mirror PurismCore's structural validation strictness (alignment, bounds,
  monotonic order, cross-reference ranges) because Cubism Core itself is
  documented to be memory-unsafe on damaged files (the `moc3ingbird` project
  is a curated proof of that). Read-only tooling must be stricter than the
  runtime it analyses.
- Treat validation failures as structured errors; keep *non-structural*
  oddities (unknown bytes, identifier quirks, non-boolean flags) as
  anomalies so the inspector can still report on damaged-but-readable files.
- Keep runtime scratch sections out of interpretation; record them so
  unknown data is never dropped.

## 5. CMO3 / later-phase reconnaissance (not implemented)

- `.cmo3` remains an editor-project container whose internal structure is
  not publicly documented by Live2D; community tooling exists (`moc2cmo`,
  `Quadrism`, `D2Evil`) with mixed licensing. AGENT.5 must be preceded by
  its own differential-research phase against owned files; no CMO3 bytes are
  produced in this repository yet (per master spec phase gates).
- `Quadrism` (codeberg) is a single-author WIP CLI that already does
  moc/moc3 -> cmo3/can3 conversion. Recorded as a scope reference; license
  unclear, so it is off-limits as a code source.

## 6. Open questions carried into later phases

- Real-file confirmation of the offset table base (`offsets[0]` == count info
  pointer) and of the runtime scratch reservation size (8 bytes/element).
- 5.3 offscreen rendering semantics and blend shape windows (AGENT.6).
- CMO3 container format research plan (AGENT.5 pre-work).
- Whether the exporter guarantees zeroed reserved header bytes.

## 7. AGENT.2 research (IR normalization)

- **Blend mode semantics confirmed before mapping.** PurismCore's
  `include/PurismCore.h` documents the constant flags: bit 0 additive, bit 1
  multiplicative (mutually exclusive), bit 2 double sided, bit 3 inverted
  mask; and 5.3+ stores an explicit blend mode field with the extended
  `csmColorBlendType` enum (Normal=0 ... Color=17). The IR maps these into a
  named `blend_mode` plus the raw 5.3 value; unknown raw values stay
  `unknown` and are reported.
- **Parameter extension keys** (`parameter.extension_key_begin/count`,
  slots 103/104, 4.2+) were validated in AGENT.1 but not extracted; AGENT.2
  added them to `ModelPools` so blend-shape key values can be read without
  touching `RawMoc3` (which keeps the AGENT.1 inspect goldens byte-stable).
- **Regression discovered by the IR tests:** the generic array reader in
  `moc3-ingest/src/reader.rs` advanced 4 bytes per element for every type,
  which was invisible while `u16` pools were only counted, and broke as soon
  as `parse_full` read the triangle index pool. Fixed by passing the element
  stride explicitly; a reader unit test locks the behaviour (master spec
  section 14: every found bug gets a regression test).
- **Pools chosen for AGENT.2** (all bounded by the existing limits, read
  only by `parse_full`): key values, extension key ranges, UVs, triangle
  indices, keyform positions, per-part draw orders, warp/rotation/art mesh
  keyform opacity/geometry/color-begin arrays, glue info and intensities.
  Keyform color *pool contents* stay unmapped and are reported as
  `unmapped_sections` (AGENT.6).
- **Dependency direction enforced**: the IR crate has no path dependency on
  the parser (checked by a workspace test that walks the `cargo metadata`
  graph), and the mapper consumes the parser output by value so no large
  buffer is copied more than once.

## 8. AGENT.3 research (hierarchy reconstruction)

- **Semantic model of Live2D parenting.** In the file each object stores both
  a parent part and (for deformers/art meshes) an optional parent deformer.
  Cross-checking public references showed this is normal, not contradictory:
  the deformer is the structural parent, the part is the logical association
  that the deformer chain itself resolves through. This became rule HR-006
  (deformer precedence, association preserved) plus HR-007 (association vs
  chain-part consistency check), instead of inventing a conflict.
- **Top-level vs orphan.** Parts and deformers without stored parents are
  exact top-level nodes (the file states no parent). Art meshes without any
  parent evidence are orphans and stay unresolved; the work-order orphan
  fallback under the root was deliberately not implemented (unresolved nodes
  go to a marked synthetic container instead).
- **Ambiguity without fabrication.** MOC3 stores at most one parent per
  relation kind, so genuine many-parent ambiguity only appears through
  heuristic evidence. Exactly one heuristic rule (HR-010, shared keyform
  binding) is implemented, disabled by default, and used to build the
  ambiguity fixture. No storage-order tie-breaks exist anywhere.
- **Cycles are data.** The parser accepts deformer parent cycles (indices are
  in range); the resolver keeps those stored edges, marks the nodes and
  reports the group. The 200k-node detector unit test and the 1500-deep
  fixture prove the iterative traversals cannot overflow the stack.
- **Bugs found by the new tests:** (1) validator counted children per parent
  as "multiple parent", which flagged legitimate trees - replaced with a
  child-listener index; (2) the depth test itself built a self-referential
  chain, exposing how the fallback HR-003 masks self-parent errors (kept as a
  documented behavior: illegal deformer relations are rejected, the legal
  part association is still used); (3) large fixtures made corruption sweeps
  quadratic in wall time - sweeps now skip them and use bounded sampled
  mutations.
- **Boundary enforcement repeated:** `hierarchy-recovery` depends only on
  `live2d-ir`; the workspace smoke test checks the manifest and the
  `cargo metadata` graph.

## 9. AGENT.3.5 research (external reference audit)

- **Reference sweep.** Eight projects were re-audited and pinned (license,
  version, commit, mode, production-dependency policy) in
  `docs/EXTERNAL_REFERENCE_MATRIX.md`; metadata lives in
  `tools/reference-harness/src/providers.rs::pinned_references()` so every
  differential report is self-describing. `py-moc3` (MIT, `2fb112e11a`) was
  the only reference executed in this environment (pure Python); the others
  were source-audited only, and LGPL-3.0 `Quadrism` stays behavioral-only.
- **Real differential runs.** A new dev-only harness crate
  (`tools/reference-harness`, binary `recovery-reference`) produced three real
  runs against py-moc3; findings DF-001..DF-004 and artifacts are in
  `docs/DIFFERENTIAL_FINDINGS.md` and
  `docs/reports/agent-3.5/differential/`.
- **Body-placement interop.** py-moc3's fixed body offset (1984) versus the
  offset-table-driven placement (704 for v1-5) explained its abort on our
  fixtures; relocating the body by +1280 made py-moc3 parse and let us
  compare semantics for the first time. The relocation transformation is now
  a regression test (`layout_interop.rs`), not just an experiment.
- **Discovered harness normalization bugs** (all fixed, harness tests cover
  them): comparisons included "ours" as its own reference when a provider
  failed (self-agreement bug); UV counts were pairs while the format counts
  floats; `keys`/`keyform_positions` were not derived although the IR holds
  the data; an unsupported field on our side was reported as "missing ours".
- **No production impact.** Parser/IR/hierarchy semantics are unchanged; the
  audit adds guard tests, evidence artifacts and planning documents
  (`CMO3_MAPPING_PLAN.md`, `RUNTIME_ORACLE_PLAN.md`,
  `GROUND_TRUTH_BENCHMARK.md`, `AGENT4/5_REFERENCE_INPUT.md`).

## 10. AGENT.4 research (semantic keyform recovery)

- **Binding semantics.** MOC3 stores, per object, a keyform binding index
  and a keyform range; the binding carries key tables whose product is the
  *reachable* grid and the parser requires the declared forms to cover that
  product (`product > count` is an error, extra forms are allowed). This
  made "sparse" only reachable through hand-edited IR, and fixed the
  classification rules: stored == expected -> dense, stored > expected ->
  unknown layout, both reported, never repaired.
- **Ordering is evidence-free in the file.** No source states which axis
  varies fastest; AGENT.4 therefore records `unknown` ordering for every
  multi-axis grid (KF-005) and only derives ordering for 0D/1D sequences.
- **Count units confirmed again.** The per-object keyform counts are
  key-index counts (keyform positions agree at 16 floats), so the harness
  derives `stored_forms` from keyform entries rather than float pools, and
  the py-moc3 differential compares per-kind counts (DF-005 explains the
  per-object disagreement on relocated bodies).
- **Key values are raw.** Duplicate keys (`[-1,0,0,1]`), unsorted keys and
  out-of-range keys are all representable; all three are preserved verbatim
  with diagnostics; non-finite keys cannot enter canonical JSON, so they are
  reported positionally (`non_finite_key_indices`) and make the axis
  unusable.
- **Geometry ownership.** The IR already stores per-keyform geometry; the
  keyform document references it by form index/count (KF-009), keeping a
  300-form mesh's keyform document ~10x smaller than the IR in the
  amplification test.
- **Rule system extended:** binding rules `KB-001..KB-007`, keyform rules
  `KF-001..KF-010`, documented in `docs/KEYFORM_RECOVERY.md` and asserted in
  sync by a crate test. No AGENT.2/3 rule changed.

## 11. AGENT.5 research (CMO3 container and main.xml)

- **CAFF resolved to E2.** Two independent MIT writers agree on the exact
  binary layout (magic/versions/format id, int32 BE key, preview block,
  obfuscated varint strings, per-entry start/size/flags, guard bytes
  `[98, 99]`, integer-level XOR with the sign-extended int64 mask, modes
  16/33/37 with ZIP `contents` payloads). Stretchy's format document also
  records a real Hiyori archive (`imageFileBuf_3.png`, key `-816980164`).
- **main.xml skeleton resolved to E2** with the full object chain and field
  names (see `docs/CMO3_MINIMAL_REQUIREMENTS.md`); the image/filter/layer
  chain remains field-level UNKNOWN and is the active AGENT.5 stop
  condition - it is not implemented rather than guessed.
- **Version profile pinned:** Editor 5.x target (`fileFormatVersion`
  402030000, edition 15, target 3000, modeler 5000000, the nine version
  PIs, the 124-class import list).
- **License discipline:** all facts come from MIT sources (Stretchy Studio
  `5fd958def9ed`, moc2cmo `2527e24e93`) by source reading only; no code was
  copied, and the LGPL Quadrism project remained behavioral-only.

## 12. AGENT.5.1 research (image pipeline evidence recovery)

- **Blocker resolved at field level.** The ModelImage filter graph,
  LayeredImage chain and CTextureManager linkage are now documented
  identifier-by-identifier from two independent MIT sources
  (`docs/CMO3_IMAGE_PIPELINE_EVIDENCE.md`): fixed filter GUIDs, the eight
  `FilterValueId` strings, the nine `FilterValue` names (two with inline
  ids), selector/layer-filter instance wiring, `FilterSet` maps, the
  two-entry `ModelImageFilterEnv`, `CLayerSelectorMap`/`CLayerInputData`
  layer selection, the full `CModelImage`/`CCachedImageManager` field order,
  the layered-image/layer/group field order, `LayeredImageWrapper`, and the
  MODEL_IMAGE-mode `CTextureManager` flags.
- **Multi-mesh invariant confirmed:** one `CLayeredImage` with N `CLayer`s,
  one `CModelImage` per mesh; N layered images = no textures (documented
  known-bad structure).
- **Contradictions isolated to writer constants** (mipmapLevel 1 vs 64,
  mode flag true vs false); resolutions and all writer-required defaults are
  recorded in `docs/CMO3_WRITER_DEFAULTS.md`, including the explicit LINEAR
  decision.
- **No new version PIs or imports needed** for the pipeline; the pinned
  nine PIs and 124-class import list already cover it.
