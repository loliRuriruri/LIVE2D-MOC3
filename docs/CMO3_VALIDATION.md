# CMO3 Validation and Gate Ledger (AGENT.5)

The AGENT.5 success definition is a gate ladder (work order section 1).
Gates must not be skipped: a failed structural gate is never "fixed" by
adding more fidelity.

| Gate | Meaning | Status | Evidence |
|---|---|---|---|
| 5A | CAFF archive structure valid | **PASS** | encoder + test decoder + validator; 15 CAFF tests (round-trip default/zero/negative keys, guard, corruption, unsafe/duplicate paths, caps, varints incl. negative-key multi-byte, encoder rejection of compressed modes, decoder recognition without guessing). Independently re-parsed and fuzzed (160k hostile inputs, zero panics) during review. |
| 5B | minimal `main.xml` structurally valid | **IN PROGRESS (AGENT.5.1)** | image-pipeline field-level evidence resolved (`docs/CMO3_IMAGE_PIPELINE_EVIDENCE.md`); typed image-pipeline model landed; serializer emission and validators in progress |
| 5C | minimal CMO3 opens in Cubism Editor | **NOT TESTED** | no Cubism Editor environment; never claimed |
| 5D | Parts / ArtMeshes serialized | NOT REACHED | blocked by 5B |
| 5E | Deformers serialized | NOT REACHED | blocked by 5B |
| 5F | Bindings / keyform grids serialized | NOT REACHED | blocked by 5B |
| 5G | Textures / masks / draw state serialized | NOT REACHED | blocked by 5B |
| 5H | Differential + structural validation | NOT REACHED | blocked by 5B |

## Active stop condition (work order section 94)

**"main.xml mandatory object set unresolved"** applies to a subset of the
required chain: the ModelImage filter graph, layered-image chain and
texture-input extensions are mandatory in both pinned minimal generators
but their field-level contents are not resolvable from the pinned sources
(the sources document the object set, not the exact values). Writing those
objects from guesswork would fabricate editor metadata and would violate
the no-fake-recovery rule, so the serializer is deliberately not shipped in
a dangling-reference state.

Resolution requires one of:

1. an owned `.cmo3` reference file (preferred; `docs/GROUND_TRUTH_BENCHMARK.md`),
2. a deeper pinned-source extraction for the filter/layer graph with the
   same evidence discipline (facts only, recorded in
   `docs/CMO3_MINIMAL_REQUIREMENTS.md`).

`Ready for AGENT.6: NO` until gate 5B is reached.

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
