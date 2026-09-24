# CMO3 Mapping Plan (pre-AGENT.5)

Status: planning document produced by AGENT.3.5. **No writer exists yet**;
`crates/cmo3-writer` stays reserved and empty. This plan lists the mapping
facts that AGENT.5 must consume and the questions it must resolve before the
first byte is written.

Sources: source audit of `moc2cmo` 1.1.18 (`2527e24e93`) - MIT - plus the
same CMO3-adjacent material used in AGENT.0. Facts below are observations at
`E2`/`E3` as marked; everything else is an open question.

## Confirmed-at-audit facts (E2/E3)

| Fact | Value | Class |
|---|---|---|
| Root model element | `CModelSource` | E3 (moc2cmo) |
| Part element | `CPartSource`; root model references the root part | E3 |
| Editor version attribute | `EditorEdition="15"` | E3 |
| Target model version attribute | `targetVersionNo="3000"` | E3 |
| Modeler version attribute | `latestVersionOfModelerNo="5000000"` | E3 |
| Draw-order compatibility flag | `useLegacyDrawOrder__testImpl="false"` | E3 |
| CAFF obfuscation key | `0x42` (as seen in reference writers) | E3 |
| Import sidecar naming | `.cmo3` with CAFF container + XML payload | E3 (moc2cmo, caff-archive) |

Attribute/element names not listed above are **not** confirmed; do not
invent them.

## Mapping from recovered structures

| Recovery source | Target concept (CMO3) | Status |
|---|---|---|
| `RecoveredProject` root | `CModelSource` + root `CPartSource` | plan |
| `Part` nodes | `CPartSource` tree | plan |
| `WarpDeformer` nodes | warp deformer source + keyform arrays | open question (element names, keyform layout) |
| `RotationDeformer` nodes | rotation deformer source + keyforms | open question |
| `ArtMesh` nodes | art mesh source (vertex/index/UV, texture ref, masks, draw order) | open question |
| Parameters | parameter source with key values and bindings | open question (bindings are `UNSUPPORTED_REFERENCE` in our IR) |
| Texture refs | texture path/number entries | open question |
| Hierarchy edges | parent ordering in the part/deformer trees | plan (use resolved parent only; ambiguity must fail closed) |
| Draw order | draw-order parameter + per-art-mesh reference | plan (respect `useLegacyDrawOrder` behavior of the target) |

## AGENT.5 evidence upgrade (pinned sources)

The CAFF layer and the XML skeleton are now **E2** (two independent MIT
implementations agree) with field-level names quoted in
`docs/CMO3_MINIMAL_REQUIREMENTS.md`; the version profile is pinned in
`docs/CMO3_VERSION_PROFILE.md`. Remaining field-level UNKNOWNs are the
ModelImage filter graph, the layered-image chain and texture-input
extensions - the AGENT.5 stop condition recorded in
`docs/CMO3_VALIDATION.md`.

## AGENT.4 hand-off: RecoveredKeyformModel -> CMO3 concepts

AGENT.4 produced `live2d-recovery/recovered-keyforms/1`; the writer must map
it without re-inference (work order sections 42, 75):

| Recovered keyform concept | CMO3 concept (from Stretchy docs / moc2cmo audit) | Status |
|---|---|---|
| `BindingBand` + `ParameterAxis` | `KeyformBindingSource` + parameter references (`parameterGuid`/parameter id, `keys`) | names unconfirmed (E3) |
| `KeyformGrid` (axes, cardinality, dense/sparse) | `KeyformGridSource` + `KeyformOnGrid` entries | names unconfirmed (E3) |
| Target forms (`PartTargetKeyforms`, `WarpTargetKeyforms`, `RotationTargetKeyforms`, `ArtMeshTargetKeyforms`) | keyform entries per object source | plan |
| `GridOrdering::Unknown` | writer must choose an explicit ordering and document the mapping; it must not claim the recovered document determined it | open question |
| Interpolation metadata | `interpolationType` / `extendedInterpolationType` | **must stay unset/Unknown**: MOC3 stores no interpolation type; forcing LINEAR for all bindings is prohibited (work order section 44) |
| Geometry (`FormGeometryRef`) | writer resolves positions from the Live2D IR (single owner, KF-009); the keyform document carries indices/counts only | plan |

Writer readiness: the semantic inputs are now explicit, but the CMO3
element/attribute names above are not confirmed, GUID policy is undefined,
and no owned editor round-trip exists - so **CMO3 mapping readiness:
PARTIAL**.

## Sequencing constraints (from the master spec fidelity order)

Hierarchy -> Parameters -> Keyforms -> Draw order -> Opacity -> Masks ->
Warp -> Rotation -> Texture atlas -> metadata. AGENT.5 must not write a
concept whose fidelity stage has not passed its regression tests in AGENT.6.

## Failure checklist borrowed from attribute-only audit (Quadrism, E3)

- strict vs lax section ordering must be a deliberate mode, never accidental,
- keyforms may need duplication in some writers; do not copy without
  verifying against owned editor round-trips,
- color/label metadata is optional and must not be fabricated,
- missing sidecars/textures must be surfaced, not silently stubbed,
- output size and obfuscation flags must be deterministic.

## Open questions AGENT.5 must settle (spec input needed)

1. Which XML schema version corresponds to which `targetVersionNo`?
2. Are GUIDs required, and what is their generation/collision policy?
3. Exact element/attribute names for deformers, art meshes and parameters.
4. CAFF wrapper details beyond the obfuscation key (header, chunking).
5. Round-trip oracle: which owned editor version validates the output?

## Prohibited

- No bytes of `moc2cmo`, Quadrism or any other reference may be copied into
  the writer; only behavior and public format facts may be reimplemented.
- No reverse-engineered Live2D Core code, no SDK redistribution.
