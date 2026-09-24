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

## Keyform rules (AGENT.4 implemented)

The keyform stage uses stable rule ids `KB-001..KB-007` (binding evidence)
and `KF-001..KF-010` (grids/forms); the full table lives in
`docs/KEYFORM_RECOVERY.md` (a test keeps the code table and the document in
sync). Summary:

- `KB-001..KB-002`: stored binding references and key tables become bands
  and axes (`Exact`); stored key order is preserved.
- `KB-003..KB-005`: dangling parameters, duplicate/unsorted/out-of-range
  keys and blend-shape axes are reported and preserved, never repaired.
- `KB-006`: glue bindings are deferred (glue pairing semantics Unknown).
- `KB-007`: targets without a stored binding become zero-dimensional static
  bands with a single form.
- `KF-001..KF-002`: expected cardinality is checked arithmetic; overflow or
  limit violations stay unresolved and are never materialized.
- `KF-003..KF-004`: dense means stored count == expected; any mismatch is
  reported and the grid stays sparse/unknown (no padding, no duplication).
- `KF-005..KF-006`: multi-dimensional ordering is never guessed; 0D/1D
  ordering follows the single stored sequence.
- `KF-007..KF-010`: form spans must be contiguous; payloads are typed and
  fields the file does not store stay absent; geometry is referenced, not
  copied; no Cartesian materialization.

Confidence follows the same taxonomy as AGENT.2/3: stored axes `Exact`,
cardinality/classification `Derived`, ordering and unresolved semantics
`Unknown`; `Heuristic` is unused in AGENT.4.

## Planned order of fidelity work (AGENT.6, from the master spec)

1. Hierarchy, 2. Parameters, 3. Keyforms, 4. Draw order, 5. Opacity,
6. Masks, 7. Warp deformers, 8. Rotation deformers, 9. Texture atlas,
10. metadata. Each item gets its own regression test before the next starts.

## Reporting rules

- Never call structural comparison numbers an "original recovery rate";
  use "Structural Recovery Report" wording and the exact derived confidence
  ratio, as defined by AGENT.7.
- "Reconstructed PSD" is the only acceptable name for AGENT.8 output.
- Cross-implementation numbers are "Cross-Implementation Agreement", never
  accuracy (`docs/DIFFERENTIAL_FINDINGS.md`).

## AGENT.3.5 note (no rule changes)

The external reference audit changed **no** rule, confidence level or naming
policy. It added interop evidence (DF-001..DF-004) and one regression test
for body-placement tolerance. All differential disagreements are resolved
against the multi-source layout (PurismCore + `mocari` + `moc2cmo`) and are
recorded in `docs/DIFFERENTIAL_FINDINGS.md`; none of them justify relaxing a
rule or promoting a heuristic.
