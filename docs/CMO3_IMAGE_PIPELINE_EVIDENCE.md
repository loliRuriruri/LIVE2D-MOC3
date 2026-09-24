# CMO3 Image Pipeline Evidence (AGENT.5.1)

Field-level evidence for the ModelImage / LayeredImage pipeline, extracted by
source reading from two independent MIT-licensed projects that agree on the
document structure. No code or data was copied; every fact below is a quoted
identifier or structure observation. Retrieved **2026-09-24**.

| Ref | Repository | Commit | Files | License |
|---|---|---|---|---|
| A | `Eatgrapes/moc2cmo` | `2527e24e9331966fc78516b38141e4e041a13952` | `src/caff/*`, `src/decompiler/xml/*` (incl. `texture/{mod,filters,model_image,layers}.rs`) | MIT (`LICENSE`, "Copyright (c) 2026 Eatgrapes") |
| B | `pelmentor/stretchystudio` (`MangoLion` mirror path used) | `5fd958def9ed2894878d23b7f1aa69e7ce3efa8e` | `docs/live2d/scripts/cmo3_generate.py`, `cmo3_multi_test.py`, `docs/live2d/CMO3_FORMAT.md` | MIT (`LICENSE`, "Copyright (c) 2026 Nguyen Phan") |

Agreement column: `both` = identical identifiers/structure in A and B;
`A`/`B` = single source; `differs` = both write it but with different values
(then the writer-defaults document decides).

Confidence: `E2` = two independent implementations agree; `E3` = one
implementation; `UNKNOWN` = not resolvable from the pinned sources.

## 1. Filter value ids and values (shared)

| Object | Field | Type | Required? | A | B | Agreement | Confidence | Our mapping | Open question |
|---|---|---|---|---|---|---|---|---|---|
| `FilterValueId` ×8 | `idstr` | string | REQUIRED | same list/order | same list/order | both | E2 | emitted in pinned order | none |
| `StaticFilterDefGuid` | `uuid` | GUID | REQUIRED | `5e9fe1ea-...abe301` (`CLayerSelector`) | same value | both | E2 | fixed GUIDs from `profile::fixed_guids` | none |
| `StaticFilterDefGuid` | `uuid` | GUID | REQUIRED | `4083cd1f-...d55ed8` (`CLayerFilter`) | same value | both | E2 | fixed | none |
| `FilterValue` ×9 | `name` | string | REQUIRED | 9 names (incl. 2 inline ids) | same set | both | E2 | emitted via `FILTER_VALUE_NAMES` | none |
| `FilterValue` | `FilterValueId id` | ref or inline | REQUIRED | refs: values 0-5 -> ids 0-5, value 7 -> `mi_output_transform`; inline `ilf_outputImageRes` (value 6) / `ilf_outputTransform` (value 8) | same | both | E2 | explicit index table `FILTER_VALUE_ID_INDEX` | none |
| `FilterValue` | `defaultValueInitializer` | null | REQUIRED | `<null>` | `<null>` | both | E2 | emitted | none |

Fixed id strings (verbatim order): `ilf_outputLayerData`,
`mi_input_layerInputData`, `ilf_inputLayerData`, `mi_currentImageGuid`,
`ilf_currentImageGuid`, `mi_output_image`, `mi_output_transform`,
`ilf_inputLayer`; inline: `ilf_outputImageRes`, `ilf_outputTransform`.

## 2. Per-mesh filter graph (shared)

| Object | Field | Type | Required? | Agreement | Confidence | Our mapping |
|---|---|---|---|---|---|---|
| `ModelImageFilterSet` | `FilterSet super` | containment | REQUIRED | both | E2 | per mesh (not shared) |
| `FilterSet` | `filterMap` | linked_map (FilterInstanceId→FilterInstance, count 2) | REQUIRED | both | E2 | selector+layer filter |
| `FilterSet` | `_externalInputs` | linked_map (FilterValueId→EnvConnection, count 2) | REQUIRED | both | E2 | mi_input_layerInputData, mi_currentImageGuid |
| `FilterSet` | `_externalOutputs` | linked_map (count 2) | REQUIRED | both | E2 | mi_output_image, mi_output_transform |
| `EnvConnection` | `_envValueDef`,`filter`,`filterValueDef` | FilterValue/FilterInstance/FilterValue refs | REQUIRED | both | E2 | emitted |
| `FilterInstance` (selector) | `filterName` | `"CLayerSelector"` | REQUIRED | both | E2 | emitted |
| `FilterInstance` (selector) | `inputConnectors` | hash_map count 2 of EnvValueConnector (`ilf_inputLayerData`→`mi_input_layerInputData`, `ilf_currentImageGuid`→`mi_currentImageGuid`) | REQUIRED | both | E2 | emitted |
| `FilterInstance` (selector) | `outputConnectors` | hash_map count 1 → FilterOutputValueConnector | REQUIRED | both | E2 | emitted |
| `FilterInstance` (layer) | `filterName` | `"CLayerFilter"` | REQUIRED | both | E2 | emitted |
| `FilterInstance` (layer) | `inputConnectors` | count 1 (`ilf_inputLayer` → same connector) | REQUIRED | both | E2 | emitted |
| `FilterInstance` (layer) | `outputConnectors` | count 0, `keyType="string"` | REQUIRED | both | E2 | emitted |
| `FilterInstance` ×2 | `filterDefGuid`,`filterDef`,`filterId`,`ownerFilterSet` | refs/null | REQUIRED | both | E2 | emitted |
| `FilterInstanceId` ×2 | `idstr` | `filter0_{i}`/`filter1_{i}` | REQUIRED | differs in exact format ([A] `filter0_{index}`, [B] `filter0`/`filter0_{mi}`) | E3 | [A] format chosen |
| `FilterOutputValueConnector` | `super`,`instance`,`id`,`valueDef` | refs | REQUIRED | both | E2 | emitted |

## 3. Model environment

| Object | Field | Type | Required? | Agreement | Confidence | Our mapping |
|---|---|---|---|---|---|---|
| `ModelImageFilterEnv` | `FilterEnv super` | containment | REQUIRED | both | E2 | emitted per model image |
| `FilterEnv` | `parentEnv` | null | REQUIRED | both | E2 | emitted |
| `FilterEnv` | `envValues` | hash_map count 2 | REQUIRED | both | E2 | exactly 2 entries |
| `EnvValueSet` (current guid) | `id` + `value` + `updateTimeMs` | FilterValueId ref + `CLayeredImageGuid` + `l 0` | REQUIRED | both | E2 | key `mi_currentImageGuid` |
| `EnvValueSet` (layer input) | `id` + `value` + `updateTimeMs` | FilterValueId ref + `CLayerSelectorMap` + `l 0` | REQUIRED | both | E2 | key `mi_input_layerInputData` |
| `CLayerSelectorMap` | `_imageToLayerInput` | linked_map count 1 (`CLayeredImageGuid` key → `array_list` of `CLayerInputData`) | REQUIRED | both | E2 | one entry per model image |
| `CLayerInputData` | `layer`,`affine`,`clippingOnTexturePx` | `CLayer` ref + identity `CAffine` + null | REQUIRED | both | E2 | mesh selects its own layer |

Selection semantics: the referenced `CLayer` **is** the selection. Multi-mesh:
each `CModelImage` has its own selector map with the **same**
`CLayeredImageGuid` key and a different `CLayer` (confirmed in [B]
`cmo3_multi_test.py` per-mesh loop and [A] `model_image.rs`).

## 4. CModelImage and group

| Object | Field | Type | Required? | Agreement | Confidence | Our mapping |
|---|---|---|---|---|---|---|
| `CModelImageGroup` | `memo`,`groupName`,`_linkedRawImageGuids`,`_modelImages` | shared object; one `CLayeredImageGuid`; inline `CModelImage`s | REQUIRED | both | E2 | one group, N inline images |
| `CModelImage` | `modelImageVersion` attr | `"0"` | REQUIRED | both | E2 | emitted |
| `CModelImage` | `guid`,`name`,`inputFilter`,`inputFilterEnv`,`_filteredImage`,`icon16`,`_materialLocalToCanvasTransform`,`_group`,`linkedRawImageGuids`,`cachedImageManager`,`memo` | in this order | REQUIRED | both | E2 | emitted |
| `CCachedImageManager` | `defaultCacheType` | `SCALE_1` | REQUIRED | both | E2 | emitted |
| `CCachedImageManager` | `rawImage`,`cachedImages`,`requiredMipmapLevel` | refs, array 1, int | REQUIRED | both | E2 | emitted |
| `CCachedImage` | `_cachedImageResource`,`isSharedImage`,`rawImageSize`,`reductionRatio`,`mipmapLevel`,`hasMargin`,`isCleaned`,`transformRawImageToCachedImage` | fields | REQUIRED | values differ for `mipmapLevel`/`requiredMipmapLevel` ([A] 64, [B] 1) | differs | writer default: 1 ([B], pure MODEL_IMAGE mode) |

## 5. Layered image chain

| Object | Field | Type | Required? | Agreement | Confidence | Our mapping |
|---|---|---|---|---|---|---|
| `CLayeredImage` | `name`,`memo`,`width`,`height`,`psdFile`,`description`,`guid`,`psdBytes`,`psdFileLastModified`,`_rootLayer`,`layerSet`,`icon16`,`icon64` | in this order | REQUIRED | both | E2 | one synthetic image |
| `CLayeredImage` | `psdFile` | `<file xs.n="psdFile">NAME</file>` text-only; `psdBytes` always null | REQUIRED | both | E2 | synthetic name (traced) |
| `CLayerGroup` (root) | `ACLayerGroup super`→`ACLayerEntry super` (`name`, `memo`, `isVisible`, `isClipping`, `blend`, `guid`, `group`, `opacity255`, `_optionOfIOption`, `_layeredImage`) then `_children`, `layerIdentifier` | shared | REQUIRED | both | E2 | one root group |
| `CLayer` | same `ACLayerEntry` prefix + `imageResource`, `boundsOnImageDoc`, `layerIdentifier`, `icon16/64`, `layerInfo`, `_optionOfIOption` | shared | REQUIRED | both | E2 | one layer per mesh |
| `CLayer` | `layerId` | `00-00-{index:02}-01` ([A]) / `00-00-00-{i+1:02}` ([B]) | differs in width | E3 | [A] format chosen |
| `CLayerGuid` | inline element with `uuid`+`note` (not a ref) | REQUIRED | both | E2 | inline, synthetic uuid |
| `LayerSet` | `_layeredImage` + `_layerEntryList` count = root group + N layers | REQUIRED | both | E2 | emitted |
| `LayerSet` entries | one `CLayerGroup` ref + N `CLayer` refs | REQUIRED | both | E2 | emitted |

Multi-mesh invariant confirmed by both sources: **one `CLayeredImage`, N
`CLayer`s**, one `CModelImage` per mesh selecting its layer. N separate
`CLayeredImage`s = geometry visible but no textures ([B] ADR-008 / Session 4).
Documented as `known-bad reference structure` for the multi-mesh regression
note (no automated Cubism verification available).

## 6. Texture manager and resources

| Object | Field | Type | Required? | Agreement | Confidence | Our mapping |
|---|---|---|---|---|---|---|
| `CTextureManager` | `textureList`,`_rawImages`,`_modelImageGroups`,`_textureAtlases`,`isTextureInputModelImageMode`,`previewReductionRatio`,`artPathBrushUsingLayeredImageIds` | main object | REQUIRED | both | E2 | emitted |
| `_rawImages` | `LayeredImageWrapper` (NOT raw `CLayeredImage`) | REQUIRED | both (fix note in [B]) | E2 | emitted |
| `LayeredImageWrapper` | `image`,`importedTimeMSec`,`lastModifiedTimeMSec`,`isReplaced` | REQUIRED | both | E2 | emitted |
| `_modelImageGroups` | one `CModelImageGroup` ref | REQUIRED | both | E2 | emitted |
| `isTextureInputModelImageMode` | bool | `true` for MODEL_IMAGE ([B]); [A] writes `false` (TextureAtlas mode) | differs | [B] is the pure MODEL_IMAGE reference | `true` |
| `CImageResource` | `width`,`height`,`type=INT_ARGB`,`imageFileBuf_size`,`previewFileBuf_size=0` + `file imageFileBuf path` | shared | REQUIRED | both | E2 | canvas-sized synthetic PNG |
| `GTexture2D` | `GTexture super`(`name`,`wrapMode=CLAMP_TO_BORDER`,`FilterMode`→owner/`LINEAR_MIPMAP_LINEAR`/`LINEAR`,`guid`,`anisotropy=ON`),`srcImageResource`,`transformImageResource01toLogical01`,`mipmapLevel`,`isPremultiplied` | shared | REQUIRED | `mipmapLevel` 64 [A] vs 1 [B] | differs | writer default: 1 |

## 7. Texture input and ArtMesh linkage

| Object | Field | Type | Required? | Agreement | Confidence | Our mapping |
|---|---|---|---|---|---|---|
| `CArtMeshSource._extensions` | count 3: `CEditableMeshExtension`, `CTextureInputExtension` ref, `CMeshGeneratorExtension` | REQUIRED | both | E2 | emitted in this order |
| `CTextureInputExtension` | `ACExtension super`(`guid`,`_owner`→`CArtMeshSource`), `_textureInputs` (count 1 in [B] MODEL_IMAGE mode), `currentTextureInputData` | REQUIRED | both | E2 | emitted |
| `CTextureInput_ModelImage` | `ACTextureInput super`(`optionalTransformOnCanvas` identity, `_owner`→extension), `_modelImageGuid` | REQUIRED | both | E2 | emitted |
| `ArtMesh.textureState` | `MODEL_IMAGE` ([B]) | REQUIRED for mode consistency | [B] | E3 | MODEL_IMAGE only |

## 8. Version/import requirements

- The nine `<?version?>` PIs already pinned in `docs/CMO3_VERSION_PROFILE.md`
  cover the image pipeline (both writers emit the same set; [A] adds the
  rotation form PI we already include). No additional PI is required.
- All image-pipeline `com.live2d.*` import classes are already present in our
  124-class `IMPORT_CLASSES` list (filter, layer, texture, cached-image and
  type imports verified individually).

## 9. Contradictions and resolutions (writer defaults)

| Contradiction | Resolution | Basis |
|---|---|---|
| `mipmapLevel`/`requiredMipmapLevel` 1 ([B]) vs 64 ([A]) | choose 1; traced as writer default | [B] is the pure MODEL_IMAGE writer we follow |
| `isTextureInputModelImageMode` true ([B]) vs false ([A]) | choose true (MODEL_IMAGE mode) | mode consistency with `TextureState=MODEL_IMAGE` |
| Shared vs per-mesh `ModelImageFilterSet` | per mesh (both writers in their multi-mesh paths) | [B] docs mention sharing is possible but untested; not used |
| `filterInstanceId` exact format | `filter0_{i}`/`filter1_{i}` ([A] form) | deterministic and unique per mesh |
| `layerId` format | `00-00-{index:02}-01` ([A] form) | matches both for the first layer |

## 10. Explicit unknowns

- Whether the Editor validates `count=` attributes against list lengths
  (UNKNOWN; both writers always derive counts from actual lengths).
- Whether a truly shared `ModelImageFilterSet` loads (UNKNOWN; not used).
- `_linkedRawImageGuids` with more than one layered image (UNKNOWN; both
  write exactly one).
- Editor acceptance of raw-mode `main.xml` CAFF entries (UNKNOWN; Cubism open
  not testable here).
- Any `filterValue*`-style string ids (absent from both sources).
