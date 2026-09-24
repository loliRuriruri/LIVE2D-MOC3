# AGENT.5.1 COMPLETION REPORT

```text
=== LIVE2D RECOVERY AGENT.5.1 REPORT ===

Repository:
D:\test\liver2d

Starting HEAD:
3466a91  docs: add agent5 final report (partial, gate 5A pass)

Ending HEAD:
<this report>  docs: add agent5.1 completion report
(chain: 005d667 feat: add image-pipeline evidence, typed model and
 writer-defaults policy, c4ef39f feat: emit minimal main.xml and cmo3 via
 model image pipeline, 1c30512 docs: record agent5.1 gates 5B pass,
 4cc6cb7 fix: address agent5.1 review findings (well-formed XML, filter
 pairing, layer group wiring), 500266a fix: address agent5.1 round 2
 residuals (layerIdentifier placement, serialize contract, traces))

Working tree:
CLEAN

Supervision:
spec_locked (master spec v0.1 authoritative; no SPEC CONFLICT encountered)

Evidence sources:
- Stretchy Studio (pelmentor/MangoLion) docs/live2d/scripts/cmo3_generate.py,
  cmo3_multi_test.py, CMO3_FORMAT.md - image pipeline field extraction
- moc2cmo src/decompiler/xml/texture/{mod,filters,model_image,layers}.rs,
  constants.rs - writer-side field extraction
- AGENT.3.5 pinned references retained (EXTERNAL_REFERENCE_MATRIX.md)

Pinned commits/versions:
- Stretchy Studio: 5fd958def9ed2894878d23b7f1aa69e7ce3efa8e
- moc2cmo: 2527e24e9331966fc78516b38141e4e041a13952
- retrieved 2026-09-24; recorded in docs/EXTERNAL_REFERENCE_MATRIX.md

License audit:
- Stretchy Studio: MIT, LICENSE "Copyright (c) 2026 Nguyen Phan"
- moc2cmo: MIT, LICENSE "Copyright (c) 2026 Eatgrapes"
- source reading only; no code copied; Quadrism (LGPL) untouched, still
  behavioral-only

Image pipeline evidence:
docs/CMO3_IMAGE_PIPELINE_EVIDENCE.md - per-object field tables with
Object/Field/Type/Required/RefA/RefB/Agreement/Confidence/Our mapping/Open
question; every field marked E2/E3; contradictions isolated to writer
constants with explicit resolutions

CLayeredImage:
field order pinned (name, memo, width, height, psdFile, description, guid,
psdBytes, psdFileLastModified, _rootLayer, layerSet, icon16, icon64);
psdFile text-only element; synthetic PSD name traced as SYNTHETIC_IDENTITY

CLayerGroup / CLayer:
CLayerGroup contains ACLayerGroup super (ACLayerEntry + _children) with
layerIdentifier directly under CLayerGroup; CLayer contains ACImageLayer
super (ACLayerEntry with group ref to the root group) + imageResource +
boundsOnImageDoc + layerIdentifier + option maps; layerId format
00-00-{index:02}-01; CLayerGuid emitted as UUID-shaped inline atoms

CModelImage:
inline in CModelImageGroup._modelImages with modelImageVersion="0" and the
full pinned child order (guid, name, inputFilter, inputFilterEnv,
_filteredImage, icon16, _materialLocalToCanvasTransform, _group,
linkedRawImageGuids, cachedImageManager, memo)

ModelImageFilterSet:
per mesh: FilterSet super with filterMap (2 FilterInstanceId->FilterInstance),
_externalInputs (2 EnvConnection), _externalOutputs (2 EnvConnection);
selector/layer FilterInstances with exact connector maps; one
FilterOutputValueConnector shared by both inputs

ModelImageFilterEnv:
exactly two env entries: mi_currentImageGuid -> EnvValueSet(CLayeredImageGuid),
mi_input_layerInputData -> EnvValueSet(CLayerSelectorMap); updateTimeMs 0

CLayerSelectorMap:
linked_map _imageToLayerInput keyed by the shared CLayeredImageGuid with one
CLayerInputData (layer ref, identity affine, null clipping) per model image;
multi-mesh selects distinct layers (verified in both sources)

CImageResource:
shared, one per archive PNG (canvas-sized), attrs width/height/type=
INT_ARGB/imageFileBuf_size/previewFileBuf_size=0 + file imageFileBuf path;
reused by layer.imageResource, model _filteredImage, cached manager and
GTexture2D srcImageResource

CModelImageGroup:
shared; memo/groupName/linkedRawImageGuids(1)/_modelImages(count derived
from emitted entries)

LayeredImageWrapper:
_rawImages count 1 wrapping CLayeredImage + importedTimeMSec/
lastModifiedTimeMSec 0 + isReplaced false (raw CLayeredImage would
ClassCastException)

CTextureManager:
MODEL_IMAGE mode only: textureList empty, _rawImages 1, _modelImageGroups 1,
_textureAtlases 0, isTextureInputModelImageMode=true,
previewReductionRatio 1, artPathBrushUsingLayeredImageIds 0

CTextureInputExtension:
CExtensionGuid ref + CArtMeshSource _owner, _textureInputs count 1,
currentTextureInputData -> CTextureInput_ModelImage;
CTextureInput_ModelImage carries ACTextureInput super (identity
optionalTransformOnCanvas + _owner) and _modelImageGuid

ArtMesh linkage:
_extensions count 3 in pinned order (CEditableMeshExtension,
CTextureInputExtension ref, CMeshGeneratorExtension); TextureState
MODEL_IMAGE; one static form per quad; child GUID lists make the rootPart
walk reach every drawable

Writer defaults:
docs/CMO3_WRITER_DEFAULTS.md - field-origin taxonomy
(RECOVERED/DERIVED/WRITER_REQUIRED_DEFAULT/SYNTHETIC_IDENTITY/UNKNOWN);
traced entries for names, synthetic model name, base positions, dimensions,
psd/group names, mipmap level, grid ordering, keyform interpolation/keys/
forms and parameter.substitution (unconditional)

LINEAR policy:
DECIDED - interpolationType/extendedInterpolationType are emitted as
WriterRequiredDefault LINEAR with a per-grid trace entry, never as
RecoveredInterpolation; the AGENT.4 "no forcing" wording concerns claiming
recovery, which is not done

Single-mesh fixture:
PASS (fixtures/expected-cmo3/5B-001.main.xml + pipeline snapshot; CAFF
round-trip; pipeline trace all steps PASS)

Multi-mesh fixture:
PASS (5B-002: one CLayeredImage, 3 CLayers, 3 CModelImages selecting
distinct layers; snapshot golden)

Reference integrity:
PASS (scanner over both goldens: 0 duplicate ids, 0 dangling refs, 0
malformed refs, 0 GUID-type mismatches, single XML declaration; .NET
XmlDocument.Load succeeds on both)

GUID integrity:
PASS (fixed well-known UUIDs verified against moc2cmo constants; all GUID
atoms UUID-shaped; deterministic mode default; 5,000-allocation probe with
zero collisions)

Canonical XML:
PASS (byte-identical determinism test; golden main.xml equality)

CAFF Gate 5A:
PASS (unchanged; 15 CAFF tests + round-trip and validation in the write
path)

main.xml Gate 5B:
PASS (structural) - 5B.1 typed project valid, 5B.2 well formed with clean
references, 5B.3 packaged and re-decoded

Minimal .cmo3 generated:
YES (in-memory; no binary checked into git - goldens are main.xml +
normalized pipeline snapshots, per the small-artifact policy)

Cubism Editor open:
NOT_TESTED

Texture visible:
NOT_TESTED

Recovered warning:
NOT_TESTED

Save/reopen:
NOT_TESTED

Tests:
previous: 268
current: 278
passed: 278
failed: 0
(cargo clippy --workspace --all-targets --all-features exit 0;
cargo fmt --all --check clean)

Determinism:
PASS (two writes byte-identical; GUID deterministic mode collision-free;
pool ids captured at allocation time)

Memory:
Cmo3Project holds no texture bytes (page hash/length only); write path
clones each asset once into CAFF entries plus the encoder copy (~2x
transient), documented in LIMITATIONS as acceptable for minimal fixtures

Security:
archive paths restricted to internal page-derived names; no DTD/entities
emitted and the scanner never interprets them; non-finite/oversized
geometry rejected at mapping; entry/archive size caps enforced; XML scanner
tolerant of hostile strings (0 panics); CAFF decoder fuzzed in the AGENT.5
review (160k hostile inputs)

JEV REVIEW:
PASS (three rounds: round 1 FAIL - 1 CRITICAL, 4 HIGH, 5 MEDIUM, 8 LOW -
all fixed; round 2 PASS for the gating set with residuals; round 3 PASS on
all bounded residuals)

Review findings:
HIGH: 4
MEDIUM: 5
LOW: 8
(plus 1 CRITICAL in round 1: duplicate XML declaration)

Findings fixed:
- duplicate XML declaration removed (render_body) + scanner counts
  declarations and rejects malformed refs
- FilterValue #7 now refs mi_output_transform via an explicit pairing table
- ACLayerGroup wrapper added; layerIdentifier placement pinned
- write_minimal_cmo3 no longer panics on model-image count mismatch
  (fatal validate_typed finding; serializer uses get())
- CLayer.group now refs the root CLayerGroup (root keeps null)
- CLayerGuid values are UUID-shaped via GuidAllocator
- serialize() precondition documented (validate first; write_minimal_cmo3
  is the safe entry)
- writer-default traces for keyform interpolation/keys/forms and
  parameter.substitution (unconditional)
- docs: textured-mesh wording, CMO3_WRITER body reconciled, evidence doc
  pins layerIdentifier placement, memory note, AGENT.5 report superseded

Production external dependency:
NO

Production code changed:
YES (cmo3-writer only: image pipeline model, serializer, validators, write
API; no other production crate touched)

IR schema changes:
NONE

RecoveredProject changes:
NONE

RecoveredKeyform changes:
NONE

Ready to resume AGENT.5:
YES (remaining fidelity: deformers, AGENT.4 keyform grids, draw-order
groups, CLI - all evidence-backed, deliberately not resumed here)

Ready for AGENT.6:
NO (per work order; AGENT.5 continuation must precede it)

Blocking issues:
- Cubism Editor open (Gate 5C) untested: no editor environment
- Full AGENT.5 fidelity is not resumed in 5.1 by instruction

Recommended next step:
resume AGENT.5 continuation (deformers -> keyform grids -> draw order ->
masks -> CLI) on explicit instruction, then Gate 5C with an owned editor
environment

=== END REPORT ===
```
