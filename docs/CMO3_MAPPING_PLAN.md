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
