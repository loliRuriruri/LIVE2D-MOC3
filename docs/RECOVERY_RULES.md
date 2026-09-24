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

## Hierarchy rules (AGENT.3 implemented)

The hierarchy stage uses stable rule ids `HR-001..HR-010`; the full table
(evidence, confidence, failure modes, fixtures) lives in
`docs/HIERARCHY_RECOVERY.md`. Summary:

- `HR-001..HR-005`: stored parent fields become hierarchy parents (`Exact`),
  the stored part of a deformer-parented node stays a logical association,
- `HR-006`: deformer precedence demotes part candidates to associations,
- `HR-007`: part association vs deformer-chain part mismatches are reported
  (`exact_relation_conflict`), both values preserved,
- `HR-010`: the single heuristic rule (binding sharing), disabled by default.

AGENT.1's inspection-only hierarchy candidates were superseded by this rule
system; the `inspect` report remains unchanged for its phase.

## Planned order of fidelity work (AGENT.6, from the master spec)

1. Hierarchy, 2. Parameters, 3. Keyforms, 4. Draw order, 5. Opacity,
6. Masks, 7. Warp deformers, 8. Rotation deformers, 9. Texture atlas,
10. metadata. Each item gets its own regression test before the next starts.

## Reporting rules

- Never call structural comparison numbers an "original recovery rate";
  use "Structural Recovery Report" wording and the exact derived confidence
  ratio, as defined by AGENT.7.
- "Reconstructed PSD" is the only acceptable name for AGENT.8 output.
