# AGENT.5 COMPLETION REPORT

```text
=== LIVE2D RECOVERY AGENT.5 REPORT ===

Repository:
D:\test\liver2d

Starting HEAD:
21ee245  docs: add agent4 final report

Ending HEAD:
<this report>  docs: add agent5 final report
(chain: 7dc6c4d feat: add caff archive layer, identity and xml
 infrastructure (AGENT.5 gate 5A), e17affc docs: record cmo3 writer
 evidence, gates and limitations, 150decf fix: address agent5 review
 round 1 findings (typed parents, strict unresolved, caff modes),
 96f8000 fix: address agent5 review round 2 low findings)

Working tree:
CLEAN

Supervision:
spec_locked (master spec v0.1 authoritative; no SPEC CONFLICT encountered)

JEV implementation tier:
HARD (jev-hard route, lock l2d-recovery-agent5; parent implementer kept)

Implemented:
- crates/cmo3-writer with the CAFF layer (encoder, test decoder,
  validator), object-pool identity allocation with deterministic/random
  GUID modes, typed XML writer with full escaping, pinned Editor 5.x
  version profile, texture-asset input model, and the strict/best-effort
  semantic mapping model with typed parent/child references
- workspace boundary smoke test (no moc3-ingest/recovery-core/fixture-gen)
- CAFF: big-endian layout, integer-level XOR obfuscation (sign-extended
  int64 mask), placeholder/patch offsets, guard bytes, entry/size caps,
  path-safety validation, structured errors only (no panics)

NOT implemented (declared, not faked):
- main.xml serializer, object-pool/semantic XML validators, write-cmo3 /
  inspect-cmo3 CLI, CMO3 fixtures/goldens, CMO3 differential harness

CMO3 target profile:
Cubism Editor 5.x compatible: fileFormatVersion 402030000,
EditorEdition 15, targetVersionNo 3000, latestVersionOfLastModelerNo
5000000, nine version PIs (union of both pinned writers), 124-class
import list (docs/CMO3_VERSION_PROFILE.md)

Writer architecture:
semantic inputs -> Cmo3Project -> identity allocation -> main.xml ->
CAFF entries -> CAFF encoder -> .cmo3. Layers exist up to and including
identity/XML infrastructure and the mapping model; the XML document
builder is the missing piece (docs/CMO3_WRITER.md)

CAFF:
PASS (Gate 5A). Two independent MIT sources agree on every field
(Stretchy Studio caff_packer.py 5fd958def9ed, moc2cmo src/caff/writer.rs
2527e24e93). Encoder writes RAW entries (mode 16) and rejects modes
33/37 rather than writing unencoded payloads; decoder recognises
compressed modes without guessing; review re-parsed output with an
independent pinned-fact parser (0 failures) and fuzzed the decoder
(160,004 hostile inputs, 0 panics)

main.xml:
NOT REACHED. Field-level skeleton and object chain are fully documented
(docs/CMO3_MINIMAL_REQUIREMENTS.md) but the ModelImage filter graph,
layered-image chain and texture-input extensions lack field-level
evidence, so no serializer was shipped (a draft with dangling xs.ref
values was discarded rather than committed)

Object pool:
Allocator implemented (sequential xs.id/xs.idx, trace of
(id, kind, semantic)); XML-side pool validation specified but not
implemented (serializer absent)

ID allocation:
Deterministic: allocation order fixed by the mapping model; same
semantic input yields the same ids (tested)

GUID strategy:
Deterministic mode (default): UUID-v4-shaped GUIDs derived from
(namespace, kind, semantic id, occurrence counter); Random mode
explicit opt-in; semantic IR never modified (docs/CMO3_WRITER.md)

Part serialization:
Mapping done (typed parents/children, draw orders, grids); XML not
emitted yet

ArtMesh serialization:
Mapping done (geometry validated: positions/uvs/indices, finite floats,
mask refs, texture page resolution, blend bits); XML not emitted yet

Warp serialization:
Mapping done (rows/columns/quad, forms, typed parents); XML not emitted

Rotation serialization:
Mapping done (base angle, per-form angle/origin/scale/reflect, typed
parents); XML not emitted

Hierarchy serialization:
Typed ParentRef/ChildRef model with no mixed warp/rotation index spaces
(review round 1 finding fixed and regression-tested); XML not emitted

Binding serialization:
Mapping done per target grid with axis keys verbatim; XML not emitted

Keyform serialization:
Mapping done (dense grids only; form coordinates use the traced
fastest-first writer default for multi-axis grids; non-dense grids fail
strict and are omitted with notes in best-effort); XML not emitted

Texture handling:
TextureAssets input model (page, bytes, path metadata, dimensions,
FNV-1a dedup hash); archive names are page-derived internal names, never
source paths; missing assets fail strict mode (MissingTextureAsset);
no placeholder images are ever generated

Mask/draw-order:
Mask refs mapped from the IR mask groups; draw orders mapped per form;
draw-order groups are not yet emitted (serializer absent)

Glue:
Not serialized (glue keyform semantics remain Unknown; AGENT.4 KB-006
deferral is respected)

Blend/offscreen:
Blend bits mapped to ColorComposition values; blend-mode enum and
offscreen surfaces remain unmapped (documented in CMO3_MAPPING_PLAN.md)

Writer-required synthetic defaults:
Traced separately from recovered data: placeholder names, synthetic
model name, base art-mesh positions (first stored form, or zeroed when
no form exists), multi-axis grid ordering. BEST_EFFORT output carries an
explicit marker; no default is ever presented as recovered

Strict mode:
Default. Fails on: missing texture asset, unresolved texture reference,
unresolved keyform grid/band/parameter, non-finite values (parameters,
canvas, draw orders, rotation and art-mesh form fields, geometry),
invalid geometry, numeric overflow (tested)

Best-effort mode:
Explicit only (--best-effort surface planned with the CLI; library flag
exists). Rolls back partially built bindings so no orphan references
remain; records every omission as an UnsupportedNote and labels the
project BEST_EFFORT. Still fails on numeric overflow and invalid
geometry (work order section 82)

Structural validator:
CAFF validator implemented (header/table/offsets/guard/paths/caps);
object-pool and semantic XML validators specified in
docs/CMO3_VALIDATION.md but not implemented (serializer absent)

CMO3 inspector:
NOT IMPLEMENTED (planned inspect-cmo3 CLI)

Differential harness:
NOT IMPLEMENTED for CMO3. moc2cmo is not executable in this environment
and byte equality is explicitly not a goal; the semantic CMO3 diff design
(counts/hierarchy/keys/geometry/masks with GUID and order normalization)
is recorded in docs/DIFFERENTIAL_FINDINGS.md

moc2cmo comparison:
Source audit only (MIT, pinned 2527e24e93): CAFF and XML facts extracted
and cross-checked against Stretchy Studio; no execution, no code copied

Quadrism regressions:
Checklist retained (hierarchy serialization ordering, duplicate
textures, keyform duplication, large-model allocation, missing
version/import declarations); texture dedup identity and version PIs are
implemented, the remaining items are pending the serializer

Fixtures:
None for CMO3 yet (work order cmo3-001..013 pending the serializer);
mapping tests use hand-built IR-level inputs

Golden outputs:
None for CMO3 (no serializer output exists)

Tests:
previous: 239
current: 268
passed: 268
failed: 0
(cargo clippy --workspace --all-targets --all-features exit 0; cargo fmt
--all --check clean)

Determinism:
PASS (CAFF byte-identical for identical input; pool ids deterministic;
GUID deterministic mode collision-free after review fix 150decf; mapping
equality tested)

CAFF round-trip:
PASS (encode -> decode_strict_raw -> payload/guard/header equality for
default, zero and negative keys; corrupted/truncated/hostile inputs
produce structured errors)

Reference integrity:
CAFF-level PASS (offsets, guard, paths, duplicates, main.xml presence).
XML-level xs.ref resolution: NOT APPLICABLE (no XML emitted yet)

Large-model test:
NOT APPLICABLE yet (no serializer); mapping is O(entities) with BTreeMap
lookups only

Memory amplification:
Mapping holds one writer model plus a trace; no geometry duplication
beyond the single mapping copy; textures are referenced by hash/length,
bytes are never cloned into the model

Input immutability:
PASS for the implemented library layers (no file writes exist yet; the
CLI that would need SHA before/after testing is not implemented)

Cubism Editor open:
NOT_TESTED

Cubism Editor version:
-

Save/reopen:
NOT_TESTED

Real-world compatibility:
UNVALIDATED

Real-world hierarchy accuracy:
UNVALIDATED

Real-world keyform accuracy:
UNVALIDATED

Production external dependency:
NO

JEV REVIEW:
PASS (two from-repository rounds; see findings below)

Review source:
from-repository (authoritative)

Review findings:
HIGH: 3
MEDIUM: 7
LOW: 8
(round 1) plus 8 LOW residuals in round 2 (N1-N8), of which N1-N5/N8
were fixed in 96f8000 and N6/N7 remain documented residuals

Findings fixed:
- docs overclaim removed (CMO3_MINIMAL_REQUIREMENTS section 6 now states
  that no main.xml is produced)
- deformer parent/child resolution moved to typed ParentRef/ChildRef
  (mixed warp/rotation index spaces eliminated; regression test added)
- writer-required defaults traced for zero-form base geometry and the
  synthetic model name
- strict mode now fails on unresolved texture references and missing
  keyform grids/bands; best-effort rolls back orphan bindings
- non-finite validation extended to parameters, canvas, draw orders and
  all rotation/art-mesh form fields
- CAFF encoder rejects unsupported compression modes; decoder verification
  covers foreign FAST entries and negative-key multi-byte varints
- GUID allocator counter prevents deterministic duplicates; strict decoder
  checks guard bytes; XML escaping covers CR/LF/TAB/control characters;
  path allowlist tightened; unused dependencies removed; import count and
  obfuscation-key docs corrected; round_i32 boundary fixed

Remote jev_verify:
SUPPLEMENTARY / INCONSISTENT (cannot read repository files; deterministic
repository evidence and the from-repository reviews govern)

IR schema changes:
NONE

RecoveredProject schema changes:
NONE

RecoveredKeyform schema changes:
NONE

Production code changed:
YES (additive: new cmo3-writer layers only)

PSD recovery:
NO

GUI:
NO

Open questions:
- image/filter/layer chain field-level evidence (owned .cmo3 reference or
  deeper pinned-source extraction) - the active stop condition
- LINEAR interpolation policy for the future serializer (CMO3_WRITER.md
  vs CMO3_MAPPING_PLAN.md conflict recorded as an open spec question)
- root part idstr (PartRoot vs __RootPart__) and default archive key
  (0x2A assumed; pinned sources silent)
- editor acceptance of RAW-mode main.xml entries and of the 124-class
  import list
- whether xs.id must be gapless and whether the editor requires ZIP
  entries for main.xml

Ready for AGENT.6:
NO

Blocking issues:
- main.xml mandatory object set unresolved for the image pipeline
  (work order stop condition; docs/CMO3_VALIDATION.md)

Recommended next step:
resolve the image/filter/layer evidence (owned .cmo3 export preferred,
docs/GROUND_TRUTH_BENCHMARK.md) and decide the LINEAR policy, then
implement the typed serializer + object-pool validators + CLI and walk
gates 5B-5H in order

=== END REPORT ===
```
