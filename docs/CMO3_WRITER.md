# CMO3 Writer (AGENT.5)

Status: **AGENT.5.1 complete for the minimal MODEL_IMAGE document** (Gate 5A
PASS; Gate 5B PASS structurally; Cubism open NOT_TESTED). The writer now
produces a `.cmo3` containing `main.xml`, one synthetic layered image with
one layer per visible mesh, per-mesh ModelImage filter graphs, texture
inputs, MODEL_IMAGE-mode texture manager, root/real parts and one static
quad per mesh. Evidence: `docs/CMO3_IMAGE_PIPELINE_EVIDENCE.md`; defaults
and LINEAR policy: `docs/CMO3_WRITER_DEFAULTS.md`; gate ledger:
`docs/CMO3_VALIDATION.md`. Full AGENT.5 fidelity (deformers, AGENT.4
keyform grids, draw-order groups, CLI) is explicitly NOT resumed.

## Scope

The writer performs **serialization only** (work order section 0): it
consumes the Live2D IR, the recovered project and the recovered keyform
model and produces `.cmo3` bytes. It never guesses hierarchy, reconstructs
keyforms, invents bindings or silently repairs ambiguity.

```text
semantic inputs -> Cmo3Project -> identity allocation -> main.xml
                -> CAFF entries -> CAFF encoder -> .cmo3
```

Dependency direction: `cmo3-writer` depends on `live2d-ir`,
`hierarchy-recovery` and `keyform-recovery` only; it must never depend on
`moc3-ingest` (enforced by a workspace smoke test).

## Implemented layers

| Layer | Status | Notes |
|---|---|---|
| CAFF encoder | DONE | big-endian, XOR int-key obfuscation, placeholder/patch offsets, guard bytes, entry caps; modes 16 supported for writing, 33/37 recognised |
| CAFF decoder/validator | DONE | test decoder resolves header/table/offsets/guard and extracts RAW payloads; strict mode rejects ZIP payloads instead of guessing |
| Identity | DONE | sequential `xs.id`/`xs.idx` pool with trace; GUID modes: deterministic (default, UUID-v4-shaped from kind+semantic id) and random (explicit) |
| XML writer | DONE | typed element tree, full escaping of `& < > " '`, no DTD/entities ever |
| Version profile | DONE | single Cubism Editor 5.x profile, `docs/CMO3_VERSION_PROFILE.md` |
| Texture asset model | DONE | assets supplied separately, page-derived internal archive names, FNV-1a dedup identity, no placeholder images |
| Semantic mapping | DONE (strict/best-effort) | `map::build_project`: strict fails on missing texture asset, unresolved texture reference, unresolved keyform grid/band/parameter, non-finite floats (parameters, canvas, rotations, forms, geometry) and invalid geometry; best-effort records what was omitted (and rolls back partially built bindings) and marks BEST_EFFORT. Also builds the typed image pipeline (resources, one layered image, one layer per textured mesh, model images) |
| `main.xml` serializer | DONE (minimal MODEL_IMAGE) | `serialize::serialize`: identity/pool allocation, filter graph, layered-image chain, texture inputs, canvas/parameter/part/mesh emission, one static form per target; declaration emitted exactly once |
| Validators | DONE | `validate_typed` (typed refs, counts, geometry, duplicates) and `scan_xml` (duplicate ids, dangling refs, malformed refs, GUID-type mismatches, single XML declaration) |
| Minimal `.cmo3` packaging | DONE | `write_minimal_cmo3`: validate -> serialize -> scan -> CAFF encode -> decode/verify; fatal findings produce no bytes |
| CLI `write-cmo3` / `inspect-cmo3` | NOT IMPLEMENTED | library API only in AGENT.5.1; CLI is AGENT.5 continuation work |

## Writer-required defaults vs recovered data

Anything the writer must choose because the file does not store it is
recorded as a `WriterDefault` trace entry (`field`, `reason`, `semantic`)
and is never presented as recovered data. Current defaults: placeholder
names (`Part_000001`, `WarpDeformer_000001`, `ArtMesh_000001`,
`Param_000001`), the synthetic model name, base art-mesh positions (first
stored form, or a zeroed placeholder when no form exists), and multi-axis
keyform ordering for dense grids (fastest-first, matching both pinned
writers' enumeration; the recovered document deliberately reports ordering
as unknown).

Interpolation metadata: the recovered keyform document does not determine
an interpolation type. The decision (recorded in
`docs/CMO3_WRITER_DEFAULTS.md`) is that `LINEAR` is emitted as a
**writer-required default with a per-grid trace entry**, never as recovered
semantics; the AGENT.4 hand-off wording about "forcing LINEAR" concerns
claiming recovery, which we do not.

## Evidence base

- CAFF: two independent MIT implementations agree
  (Stretchy Studio `caff_packer.py`, moc2cmo `src/caff/writer.rs`), plus a
  real Hiyori archive byte layout recorded in the Stretchy format document.
- XML skeleton/fields: the same two implementations (field names quoted
  verbatim), with documented open questions (root part id string, GUID
  randomness in Stretchy, image/filter graph contents).
- License discipline: source reading only; no code copied; the LGPL
  Quadrism project remains behavioral-only and was not consulted for code.

## Not implemented on purpose (AGENT.5.1 scope boundary)

- PSD recovery, GUI, preview images, compressed archive entries.
- Deformers, AGENT.4 keyform grids, draw-order groups, masks beyond the
  first GUID, and all sample/editor metadata: explicitly deferred to the
  AGENT.5 continuation (evidence exists; scope does not).
