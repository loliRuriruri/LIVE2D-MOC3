# Recovery Rules (draft)

**Status: draft for AGENT.3+. AGENT.1 only reports stored relationships and
derived hierarchy candidates; it does not "recover" anything.**

## Confidence taxonomy

Format knowledge (used in `FORMAT_NOTES.md` and `FieldConfidence`):

`Unknown` -> `Unverified` -> `Observed` -> `Hypothesis` -> `Confirmed`

Recovered structure (used by hierarchy edges today, recovery output later):

- `Exact` - value is stored in the file and copied verbatim,
- `Derived` - computed from stored values without guessing,
- `Heuristic` - produced by an inference rule that may be wrong,
- `Unknown` - no basis for the value.

## Naming rules (master spec section 7)

- Stored identifiers are used verbatim.
- When a stored identifier is empty, deterministic fallbacks are generated:
  `Parameter_0004`, `Part_0001`, `ArtMesh_0001`, `WarpDeformer_0012`,
  `RotationDeformer_0007`, `Deformer_0003`, `Glue_0002`.
- Same input -> same names; no randomness, no timestamps.

## Hierarchy rules (AGENT.1 candidate edges, AGENT.3 graph)

Currently reported as edges (all `Exact`, each naming its stored field):

- `part.parent_part` -> part -> parent part
- `deformer.parent_deformer` -> deformer -> parent deformer
- `deformer.parent_part` -> part -> deformer
- `art_mesh.parent_deformer` -> deformer -> art mesh
- `art_mesh.parent_part` -> part -> art mesh
- drawable mask entries -> `masked_by` (masking art mesh -> masked art mesh)

AGENT.1 additionally detects cycles in part/deformer parent chains
(`hierarchy_cycle` anomalies). AGENT.3 will add orphan detection,
multi-parent conflict resolution, parent-candidate scoring and deterministic
ordering for the recovery graph.

## Planned order of fidelity work (AGENT.6, from the master spec)

1. Hierarchy, 2. Parameters, 3. Keyforms, 4. Draw order, 5. Opacity,
6. Masks, 7. Warp deformers, 8. Rotation deformers, 9. Texture atlas,
10. metadata. Each item gets its own regression test before the next starts.

## Reporting rules

- Never call structural comparison numbers an "original recovery rate";
  use "Structural Recovery Report" wording and the exact derived confidence
  ratio, as defined by AGENT.7.
- "Reconstructed PSD" is the only acceptable name for AGENT.8 output.
