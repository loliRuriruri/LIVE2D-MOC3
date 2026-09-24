# CMO3 Validation and Gate Ledger (AGENT.5)

The AGENT.5 success definition is a gate ladder (work order section 1).
Gates must not be skipped: a failed structural gate is never "fixed" by
adding more fidelity.

| Gate | Meaning | Status | Evidence |
|---|---|---|---|
| 5A | CAFF archive structure valid | **PASS** | encoder + test decoder + validator; 15 CAFF tests (round-trip default/zero/negative keys, guard, corruption, unsafe/duplicate paths, caps, varints incl. negative-key multi-byte, encoder rejection of compressed modes, decoder recognition without guessing). Independently re-parsed and fuzzed (160k hostile inputs, zero panics) during review. |
| 5B | minimal `main.xml` structurally valid | **PASS (AGENT.5.1)** | typed project validator, XML scanner (unique ids, no dangling refs, GUID-type checks), minimal MODEL_IMAGE serializer, goldens `fixtures/expected-cmo3/5B-001..002.main.xml`; single-mesh and multi-mesh (1 document / N layers) gates tested |
| 5B.1 | typed project model valid | **PASS** | `validate_typed` + negatives (missing model image, dangling resource, layer count, duplicate `layerId`, geometry) |
| 5B.2 | main.xml well formed, no dangling/duplicate refs | **PASS** | `scan_xml` over generated documents (also golden-tested) |
| 5B.3 | main.xml + synthetic PNG -> CAFF -> `.cmo3` | **PASS** | `write_minimal_cmo3` packages via the Gate 5A encoder and re-decodes the archive before returning bytes |
| 5C | minimal CMO3 opens in Cubism Editor | **NOT TESTED** | no Cubism Editor environment; never claimed |
| 5D | Parts / ArtMeshes serialized | **PASS (AGENT.5.2)** | full part trees with child GUID lists and recovered draw orders; all art meshes with base geometry, per-form keyed geometry, masks, texture refs; e2e_001/006 |
| 5E | Warp + rotation deformers serialized | **PASS** | sources with typed parents, checked grid math, all recovered forms (opacity/positions; angle/origin/scale/reflect); stable parent-before-child order; e2e_002/003 |
| 5F | Parameters / bindings / grids / forms | **PASS** | all recovered parameters (min/max/default/keys/decimals/repeat); binding keys verbatim (never sorted/deduped); grids with KeyformOnGrid/KeyOnParameter; typed forms for Part/Warp/Rotation/ArtMesh; strict fails unresolved grids; e2e_004/005, best-effort regression |
| 5G | Textures / masks / draw state | **PASS (bounded)** | model3.json/texture-dir loading, multi-page resources with per-page dedup, full mask GUID lists, per-form draw order, blend policy (Normal/Add/Multiply; other enums fail strict, NORMAL+note in best-effort); draw-order groups/glue/offscreen preserved as UNSUPPORTED notes |
| 5H | Whole-project validation + differential | **PASS (bounded)** | typed + XML validators (ids/refs/GUIDs/cycles/form counts/geometry), CAFF re-decode, independent XML re-parse, semantic inspector; differential: internal semantic snapshot PASS, reference generator `REFERENCE_UNSUPPORTED` (moc2cmo not executable) |
| 5I | End-to-end CLI | **PASS** | `recovery recover` (moc3/model3.json, textures, strict/best-effort, report, atomic-ish write, --force) and `recovery inspect-cmo3`; release binary smoke PASS |

## Stop condition status

The AGENT.5 stop condition **"main.xml mandatory object set unresolved"** is
**CLEARED** for the minimal MODEL_IMAGE-mode document: the filter graph,
layered-image chain and texture-input linkage were resolved to field level
in `docs/CMO3_IMAGE_PIPELINE_EVIDENCE.md` (two independent MIT sources) and
are emitted with validated references. Remaining known gaps are declared
scope (AGENT.4 keyform grids, deformers, draw-order groups) rather than
evidence gaps. Gate 5C remains NOT_TESTED until a Cubism Editor environment
exists.

## Validators specified (for the serializer completion)

- `validate_object_pool()`: unique `xs.id`, all `xs.ref` resolve to an
  existing `xs.id`, expected target element type, no prohibited cycles,
  stable ordering, no duplicate `idstr` inside a namespace.
- `validate_cmo3_project()`: parameter/part/drawable/deformer/keyform
  grid/binding/texture/form references all resolve inside the writer model
  before serialization.
- XML scan validator over the produced text (small internal scanner, no
  external XML dependency, no DTD/entities): duplicate ids, dangling refs,
  unknown element detection.
- Negative fixtures (work order section 68): dangling `xs.ref`, duplicate
  `xs.id`, missing `main.xml`, missing texture, invalid CAFF offset, bad
  guard, malformed XML, missing form GUID/grid/binding. CAFF-side negatives
  exist today; XML-side negatives land with the serializer.

## Cubism manual matrix (work order section 69)

Not executable in this environment. When an editor is available, record per
fixture: editor version, open result, warnings, visual result, editable,
save/reopen. Until then every report must say `Cubism Open: NOT_TESTED`
and structural success is never called `Editor compatible`.
