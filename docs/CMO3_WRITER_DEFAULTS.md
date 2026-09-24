# CMO3 Writer Defaults and LINEAR Policy (AGENT.5.1)

Every writer-produced field must be classifiable (work order section 31):

```text
RECOVERED             value copied from stored data
DERIVED               computed from stored values without guessing
WRITER_REQUIRED_DEFAULT  the format needs a value the file does not store
SYNTHETIC_IDENTITY    ids/GUIDs/names created for serialization only
UNKNOWN               no basis; never emitted as if known
```

Writer-required defaults are recorded in `Cmo3Project.defaults` traces
(`field`, `reason`, `semantic`) and are never presented as recovered data.

## LINEAR interpolation decision (work order section 30)

MOC3 stores no interpolation type for bindings; the recovered keyform model
does not determine one. CMO3's `KeyformBindingSource` requires
`interpolationType`/`extendedInterpolationType`. Decision:

- these fields are emitted as **`WRITER_REQUIRED_DEFAULT` = `LINEAR`**, with
  an explicit defaults-trace entry per binding;
- they are **never** reported as `RecoveredInterpolation`; the recovered
  document's `GridOrdering::Unknown` stance is unchanged;
- this resolves the AGENT.5 open question recorded in
  `docs/CMO3_WRITER.md` (the AGENT.4 hand-off wording "forcing LINEAR is
  prohibited" applies to *claiming* recovered semantics, which we do not).

Open residual: if a future phase learns per-binding interpolation from
editor-owned files, the default must be replaced by recovered values with a
new rule id.

## Defaults used in the minimal (AGENT.5.1) writer

| Field | Value | Class | Reason |
|---|---|---|---|
| parameter name/id (single synthetic parameter) | `Param_Minimal`, keys `[0.0]`, min 0/max 1/default 0 | WRITER_REQUIRED_DEFAULT | CMO3 keyform bindings need a parameter; the minimal model carries no recovered parameters |
| keyform binding keys | `[0.0]` | WRITER_REQUIRED_DEFAULT | single static form |
| `interpolationType`/`extendedInterpolationType` | `LINEAR` | WRITER_REQUIRED_DEFAULT | section above |
| art-mesh form count | 1 static form (drawOrder 500, opacity 1.0) | WRITER_REQUIRED_DEFAULT | AGENT.4 keyform grids are out of scope for 5.1 |
| root part form | 1 static form | WRITER_REQUIRED_DEFAULT | matches pinned minimal writers |
| mesh/part/parameter display names | recovered when stored, else deterministic placeholders (`ArtMesh_000001`, ...) | RECOVERED or SYNTHETIC_IDENTITY | existing rule from AGENT.5 |
| model name | `RecoveredModel` (or `--name`) | SYNTHETIC_IDENTITY | no stored model name exists |
| layered image `psdFile`/`name` | `RecoveredLayeredImage.psd` | SYNTHETIC_IDENTITY | no original PSD exists; explicitly synthetic |
| layer names | mesh names (recovered or placeholder) | RECOVERED / SYNTHETIC_IDENTITY | one layer per visible mesh |
| layer ids | `00-00-{index:02}-01` | SYNTHETIC_IDENTITY | pinned writer formats |
| model-image/group names, `filter0_{i}` ids | deterministic synthetic strings | SYNTHETIC_IDENTITY | object identity only |
| GUIDs | deterministic mode (default) or random (explicit) | SYNTHETIC_IDENTITY | never enter the IR |
| `mipmapLevel`/`requiredMipmapLevel` | 1 | WRITER_REQUIRED_DEFAULT | pinned sources disagree (1 vs 64); we follow the pure MODEL_IMAGE writer |
| `isTextureInputModelImageMode` | `true` | DERIVED | MODEL_IMAGE mode consistency |
| identity `CAffine` values | `1.0/0.0` | DERIVED | identity transform |
| `snapEpsilon`, `decimalPlaces`, `previewReductionRatio`, `opacity255`, colors | pinned writer constants | WRITER_REQUIRED_DEFAULT | constants observed in both sources |
| texture resource width/height | canvas size | DERIVED (with WRITER_REQUIRED_DEFAULT trace when the asset dimensions are unknown) | canvas-sized layer policy (work order section 15) |
| object `xs.id`/`xs.idx` | sequential deterministic allocation | SYNTHETIC_IDENTITY | object pool |
| ALPHA `CColor`/`CFloatColor` values | 1.0 white | WRITER_REQUIRED_DEFAULT | editor defaults observed |

## Synthetic PSD policy (work order section 13)

The layered image is a **Synthetic Layered Image**: the writer creates one
canvas-sized layer per visible recovered mesh to satisfy the editor's texture
pipeline. It does not recover, guess or claim original layer names, group
names, PSD hierarchy or hidden layers. Layer names reuse the recovered mesh
names (or their deterministic placeholders).

## Not-yet-used defaults (reserved)

Deformer/keyform-grid serialization (AGENT.5 continuation) will need its own
entries (deformer root GUID is already fixed; grid ordering default is
fastest-first per `docs/CMO3_WRITER.md`). Nothing from AGENT.4 is claimed as
recovered interpolation.
