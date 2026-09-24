# CMO3 Minimal Requirements (AGENT.5)

Status: evidence classification for the minimal `.cmo3` object set, derived
from two pinned MIT sources that agree (E2 unless marked):
Stretchy Studio `docs/live2d/scripts/cmo3_generate.py` (commit
`5fd958def9ed`) and moc2cmo `src/decompiler/xml/*`, `src/caff/*` (commit
`2527e24e93`). Field-level extraction was performed by source reading only;
no code or data was copied.

Classification: **R** = REQUIRED for both pinned generators, **C** =
CONDITIONALLY_REQUIRED (needed for editor texture rendering or specific
modes), **O** = OPTIONAL/tolerated, **U** = UNKNOWN (not resolvable from the
pinned sources).

## 1. CAFF container (E2)

| Item | Class | Evidence |
|---|---|---|
| magic `CAFF`, 3-byte archive version, format id `----`, 3-byte format version | R | both writers |
| int32 BE obfuscation key (default `0x2A`; Hiyori uses `-816980164`) | R | both |
| preview metadata block (format byte `127` = none, color `127`, i16 width/height, i64 start, i32 size, 8 reserved) | R | both |
| i32 entry count (obfuscated) | R | both |
| per entry: varint path, varint tag, i64 start, i32 size, obfuscated flag, compression byte, 8 reserved | R | both |
| guard bytes `[98, 99]`, never obfuscated | R | both |
| compression modes: `16` raw, `33` fast (ZIP with single `contents` entry), `37` small | R (decode), C (encode) | both; this writer emits raw only |
| entry `main.xml` with tag `main_xml`, obfuscated, mode 33 in references | R | both |
| image entries `imageFileBuf[_N].png`, empty tag, raw | R (when textures exist) | Stretchy (`imageFileBuf.png`), moc2cmo (`imageFileBuf_{i}.png`); real files `imageFileBuf_3.png` |
| preview entry | O | no pinned writer emits one |

## 2. Document skeleton (E2)

| Item | Class | Evidence |
|---|---|---|
| `<?xml version="1.0" encoding="UTF-8"?>` | R | both |
| `<?version Class:N?>` processing instructions (profile list below) | R | both |
| `<?import fully.qualified.Class?>` list (`$` for nested classes) | R | both |
| `<root fileFormatVersion="402030000">` with exactly `<shared>` then `<main>` | R | both |
| `<main>` containing one `<CModelSource isDefaultKeyformLocked>` | R | both |
| `CModelSource` children order: `CModelGuid` ref, `s name`, `EditorEdition`, `CImageCanvas`, `CParameterSourceSet`, `CTextureManager`, `b useLegacyDrawOrder__testImpl`, `CDrawableSourceSet`, `CDeformerSourceSet`, `CAffecterSourceSet`, `CPartSourceSet`, `CPartSource rootPart` ref, `CParameterGroupSet`, `CModelInfo`, `i targetVersionNo`, `i latestVersionOfLastModelerNo` | R | both |
| `EditorEdition` -> `i edition` = 15 | R | both |
| root part id string | U | Stretchy `PartRoot` vs moc2cmo/doc `__RootPart__` |
| object pool ids `xs.id="#N"` sequential from 0, `xs.idx` separate counter | R | both |
| element order significant | O (reader matches by `xs.n`) | doc |
| `xs.id` gaplessness | U (writer behavior known, reader requirement unknown) | doc |

## 3. Identity (E2)

- GUID atoms carry `uuid="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" note="..."`;
  lowercase, hyphenated, no braces.
- Fixed well-known UUIDs (R where used): deformer root
  `71fae776-e218-4aee-873e-78e8ac0cb48a`; parameter group root
  `e9fe6eff-953b-4ce2-be7c-4a7c3913686b`; filter definitions
  `5e9fe1ea-...abe301`, `4083cd1f-...d55ed8`.
- Integer/string ids live in `idstr` (`CParameterId`, `CPartId`,
  `CDrawableId`, `CDeformerId`, `CAffecterId`, `FilterInstanceId`,
  `FilterValueId`).
- Root `CParameterGroupGuid` UUID: Stretchy generator uses a random UUID
  while the doc requires the fixed constant (inconsistency U).
- `CModelSource` version PI 4 avoids requiring `rootParameterGroup`,
  `modelOptions`, `gameMotionSet` (Stretchy comment, E3).

## 4. Minimal object chain (E2/E3)

| Object | Class | Notes |
|---|---|---|
| `CImageCanvas` with `i pixelWidth/pixelHeight` and empty `CColor background` | R | both |
| `CParameterSourceSet` -> `carray_list _sources` -> `CParameterSource` (order: `decimalPlaces`, `CParameterGuid guid`, `snapEpsilon`, `minValue`, `maxValue`, `defaultValue`, `isRepeat`, `CParameterId id`, `Type paramType` (`NORMAL`), `s name`, `s description`, `combined`, `CParameterGroupGuid parentGroupGuid`) | R (when parameters exist) | both, field names verbatim |
| `CParameterGroupSet` -> `_groups count=0` (no `CParameterGroup` object emitted) | R | both |
| `CModelInfo` -> `pixelsPerUnit`, `CPoint originInPixels` (`i x`, `i y`) | R | both |
| `CTextureManager` -> `TextureImageGroup textureList`, `_rawImages`, `_modelImageGroups`, `_textureAtlases`, `isTextureInputModelImageMode`, `previewReductionRatio`, `artPathBrushUsingLayeredImageIds` | R | both; child contents differ per mode |
| `CImageResource` (`width`, `height`, `type=INT_ARGB`, `imageFileBuf_size`, `previewFileBuf_size`) -> `file imageFileBuf path` | R (when textures exist) | both |
| `GTexture2D` -> `GTexture` (`s name`, `WrapMode CLAMP_TO_BORDER`, `FilterMode`/`MinFilter`/`MagFilter`, `GTextureGuid`, `Anisotropy ON`), `srcImageResource`, `CAffine transformImageResource01toLogical01`, `mipmapLevel`, `isPremultiplied` | R (when textures exist) | both; `mipmapLevel` 1 (S) vs 64 (M) |
| ModelImage chain: `CModelImageGroup` + inline `CModelImage` + filter graph (`ModelImageFilterSet`, `FilterInstance`, `FilterValue`, `EnvValueConnector`, `CLayerSelectorMap`, `CLayerInputData`, ...) | C | mandatory in both minimal generators, but the pinned sources do not expose the field-level contents needed to reimplement it (U) |
| Layered image chain: `CLayeredImage`, `CLayerGroup`, `CLayer`, `LayerSet`, `CLayerIdentifier`, `CImageResource` per layer | C | required for editor texture rendering per Stretchy doc (one CLayeredImage with canvas-sized layers); fields listed in evidence, not yet emitted |
| `CPartSource` (super `ACParameterControllableSource`: `localName`, `isVisible`, `isLocked`, `parentGuid`, `keyformGridSource`, morph target set, `_extensions`, `internalColor_direct_argb`; then `guid`, `CPartId id`, `enableDrawOrderGroup`, `defaultOrder_forEditor` 500, `isSketch`, `partsEditColor`, `_childGuids`, `targetDeformerGuid`, `keyforms` `CPartForm`) | R (root part always) | both |
| `CArtMeshSource` (super chain incl. `_extensions` count 3: `CEditableMeshExtension`, `CTextureInputExtension`, `CMeshGeneratorExtension`; `CDrawableId`, `guid`, `targetDeformerGuid`, `clipGuidList`, `invertClippingMask`, `indices`, `keyforms`, `positions`, `uvs`, `texture`, `ColorComposition`, `culling`, `TextureState`, `userData`) | R | both |
| `GEditableMesh2` (`point`, `pointPriority`, `edge`, `edgePriority`, `pointUid`, `meshGuid`, `coordType`, attrs `nextPointUid`, `useDelaunayTriangulation`) | R | both write all seven fields |
| `CMeshGeneratorExtension` -> `MeshGenerateSetting` (densities/margins/threshold) | R | Stretchy comment: without it Editor marks the mesh "recovered" |
| `CTextureInputExtension` + `CTextureInput_ModelImage`/`_TextureAtlasRegion` | C | mode-dependent |
| `CWarpDeformerSource` / `CRotationDeformerSource` + forms | C (when deformers exist) | moc2cmo only |
| `KeyformGridSource`, `KeyformBindingSource`, `KeyformOnGrid`, `KeyformGridAccessKey`, `KeyOnParameter`, `CFormGuid` | R (when keyforms exist) | both; first binding varies fastest |
| `CBlend_Normal` -> `ACBlend` -> `s displayName` | R | both |
| `CoordType` -> `s coordName` (`DeformerLocal`; moc2cmo adds `Canvas`) | R | both |
| `CDrawableSourceSet`, `CDeformerSourceSet`, `CAffecterSourceSet` (may be empty) | R | both |

## 5. Editor behaviour notes (E3, Stretchy doc)

- Random root `CParameterGroupGuid` silently disables the Random Pose
  dialog; random deformer-root UUID breaks deformer lookups (compare by
  UUID equality).
- A mesh without `CMeshGeneratorExtension` is marked "recovered"; a
  part-hierarchy walk through `rootPart._childGuids` must reach every
  drawable or drawables become "(recovered)".
- `CPartSource` must be a shared object with a self `_source` reference.
- `CModelSource` version 14 requires `rootParameterGroup`/`modelOptions`/
  `gameMotionSet`; version 4 avoids them.
- Unknown elements are ignored and missing elements default (Java
  deserializer), except the documented `checkNotNull` fields.

## 6. What exists today vs what the serializer will emit

**Exists today (structurally validated):** the CAFF encoder/decoder and
validator, the object-pool identity allocator with deterministic GUIDs, the
typed XML writer, the pinned version profile, the texture-asset input
model, and the strict/best-effort semantic mapping model with typed
parent/child references. **No `main.xml` is produced yet** - the serializer
is not implemented (`docs/CMO3_VALIDATION.md`, Gate 5B NOT REACHED).

**Planned serializer output (not yet written):** the full skeleton,
identities, parameters, parts with child GUID lists and forms, art meshes
with geometry/keyforms and the editable-mesh/mesh-generator extensions,
deformers with forms, keyform grids/bindings/forms, texture resources
(`CImageResource` + `GTexture2D`) and CAFF entries.

**Deferred with explicit status (never faked):** the ModelImage filter
graph, the layered-image chain (texture rendering in the editor),
texture-input extensions and preview entries. These are **C/U** above and
must be resolved (owned `.cmo3` reference or deeper pinned-source
extraction) before the serializer can claim a structurally complete
editor-grade document.
