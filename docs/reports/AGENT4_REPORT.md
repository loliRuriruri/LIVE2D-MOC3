# AGENT.4 COMPLETION REPORT

```text
=== LIVE2D RECOVERY AGENT.4 REPORT ===

Repository:
D:\test\liver2d

Starting HEAD:
12bc0c4  docs: add agent3.5 completion report

Ending HEAD:
<this report>  docs: add agent4 final report
(chain: 9bb8ea7 feat: add semantic keyform recovery crate (AGENT.4),
 d7c8093 feat: reconstruct parameter bindings and keyform grids with
 fixtures and goldens, a0ac24e feat: add recover-keyforms CLI command,
 966d76c test: extend differential keyform validation and archive AGENT.4
 evidence, efb95cd docs: document keyform recovery semantics and update
 phase docs, 04b9849 fix: address agent4 review findings (validator,
 confidence, NaN handling, caps), d9d3765 docs: refresh agent4
 differential evidence at fixes revision, d6c34b8 fix: address agent4
 review round 2 low findings and cap tests)

Working tree:
CLEAN

Supervision:
spec_locked (master spec v0.1 authoritative; no SPEC CONFLICT encountered)

JEV implementation tier:
HARD (jev-hard route, lock l2d-recovery-agent4; parent implementer kept)

Implemented:
- crates/keyform-recovery (dependency direction: live2d-ir +
  hierarchy-recovery only; enforced by a workspace smoke test)
- pipeline stages: collect_binding_evidence, normalize_axes,
  build_binding_bands, derive_grid_shape, map_forms,
  validate_recovered_keyforms, recover (no mega-rule)
- typing: ParameterAxis, BindingBand, KeyformGrid, typed target forms
  (Part/Warp/Rotation/ArtMesh), UnresolvedEntry, ReconstructionTrace,
  statistics; canonical JSON codec; human report; explain
- CLI: recovery recover-keyforms <model.moc3|model.ir.json>
  [--json] [--output PATH] [--strict] [--explain TARGET]
  [--max-file-size BYTES]
- fixtures keyform-001..014, 016, 019, 020 (17 files) + IR-level negative
  fixtures; 17 golden documents; differential harness keyform sections

Recovered keyform schema:
live2d-recovery/recovered-keyforms/1 (separate document; the IR and
recovered-project schemas are unchanged)

Schema status:
EXPERIMENTAL

Binding model:
stored references only (KB-001/KB-002); glue deferred (KB-006); targets
without a binding become zero-dimensional static bands (KB-007); bands
carry axis sets, checked expected cardinality, targets and provenance

Parameter axes:
stored key tables in stored order; duplicates/unsorted/out-of-range keys
reported and preserved (KB-004); dangling parameters unresolved (KB-003);
blend-shape axes marked experimental (KB-005); non-finite keys reported
positionally (never embedded; JSON cannot carry them)

Grid model:
expected cardinality = checked product of axis key counts (KF-001) with
overflow/limit -> unresolved (KF-002); stored form counts compared, never
repaired; form indices must be contiguous (KF-007); cardinality is never
materialized (KF-010)

Grid ordering:
0D/1D derived from the single stored sequence (KF-006); multi-dimensional
ordering is never guessed (KF-005, "observed" reserved and unused)

Dense/sparse handling:
dense = stored == expected (KF-003); stored > expected -> unknown layout +
diagnostic; stored < expected -> sparse (only reachable through IR-level
negative fixtures) + diagnostic; no padding, no duplication

Target types:
Part, WarpDeformer, RotationDeformer, ArtMesh (typed payloads; no generic
blob); glue is not a keyform target

Part keyforms:
draw order per form (opacity is not stored in .moc3 and stays absent)

Warp keyforms:
opacity per form + control-point geometry reference (position_count); the
warp grid and the parameter grid are never conflated

Rotation keyforms:
opacity, angle, origin, scale, reflect_x/reflect_y per form

ArtMesh keyforms:
opacity, draw_order per form + vertex geometry reference; static
uvs/indices and total geometry stay in the IR (KF-009)

Rule IDs:
KB-001..KB-007 (binding), KF-001..KF-010 (keyform/grid); documented in
docs/KEYFORM_RECOVERY.md with a code/document sync test and resolvable
fixture references

Confidence/provenance:
stored axes Exact; cardinality/classification Derived; unresolved or
experimental semantics Unknown; Heuristic unused; the "exact band =
usable axes + exact cardinality" contract is enforced by the validator

Reconstruction traces:
one per target (axes with status, cardinality, stored count, layout,
ordering, confidence, rules, notes); `--explain` renders a trace or the
same structure as JSON

Validation:
duplicate band/grid ids, dangling references, duplicate targets/axes,
wrong target types, form-index bounds (Fatal) and contiguity, non-finite
position bounds (Fatal), statistics accounting, confidence/provenance
contracts, layout/confidence re-derivation, band/grid/target identity
cross-checks, cross-layer project membership; per-code diagnostic caps and
an unresolved cap applied at the recover() boundary

CLI:
recover-keyforms reads .moc3 or .ir.json, reconstructs the project for
cross-checks, refuses fatal IR input, supports --json/--output/--strict/
--explain; never writes to inputs

Canonical output:
deterministic byte-identical JSON for identical inputs; JSON -> model ->
JSON round-trip is byte-identical (library and CLI); export refuses
non-finite payloads (NonFiniteValue) instead of silently writing null

Differential harness changes:
snapshot gained a keyforms section (targets, stored_forms, bindings, axes,
per-kind form counts, raw axis keys, ours-only grid-shape/per-target
evidence); py-moc3 adapter emits the raw aggregates it can read and marks
grid-shape/per-target evidence unsupported; runs archived under
docs/reports/agent-4/differential/

moc2cmo comparison:
source audit only (not executable in this environment): BindingBand,
ParameterBinding and target-keyform concepts are compared conceptually in
docs/CMO3_MAPPING_PLAN.md; no runtime comparison, no code copied

py-moc3 evidence:
R4 relocated-002-b 61 comparable / 30 agreement / 6 disagreement
(R5 fixture-010 v6: 53/36/8; R6 fixture-002: REFERENCE_ERROR preserved).
Raw axis keys agree; per-object form counts disagree on relocated bodies
(new DF-005, same family as DF-004)

DF-001..DF-004 status:
unchanged (none closed or reclassified); DF-005 added; archived AGENT.3.5
evidence retained and guarded by a regression test

Body placement regression:
PASS (fixture-002, keyform-003, keyform-019 recover identical keyform
documents from native and relocated bodies)

Fixtures:
17 keyform MOC3 fixtures (zero-dimensional, 1D/2D/3D, part/warp/rotation/
art-mesh forms, multiple bindings/targets, duplicate keys, unsorted keys,
cardinality mismatch, non-finite key, deep nested targets, 64-form
dataset); IR-level negatives: dangling parameter/binding/target, sparse
partial grid, cardinality limit/overflow, form-span gap, out-of-bounds
form index, non-finite payload, blend-shape experimental, glue deferred

Golden keyform outputs:
17 documents under fixtures/expected-keyforms/, cross-checked by explicit
per-fixture expectations and import/validate tests (not accepted merely
because the generator wrote them)

Tests:
previous: 182
current: 239
passed: 239
failed: 0
(workspace-wide, offline; cargo clippy --workspace --all-targets
--all-features exit 0; cargo fmt --all --check clean)

Determinism:
PASS (repeated recovery byte-identical across fixtures and CLI runs)

JSON round-trip:
PASS (byte-identical; validated documents only)

Large dataset:
PASS (500-form single-axis unit test, 64-form fixture, 300-form x 32
vertex geometry test)

Cardinality hardening:
PASS (checked_mul; MAX_GRID_CARDINALITY 1_000_000, MAX_AXIS_KEYS 65_536,
MAX_AXES_PER_BAND 64; overflow and limit stay unresolved)

Memory amplification:
geometry referenced, never copied: the 300-form x 32-vertex keyform
document is <1/5 of the IR JSON in the amplification test; no Cartesian
materialization

Parser independence:
PASS (manifest + cargo metadata graph check for keyform-recovery)

Production external dependency:
NO

Real-world compatibility:
UNVALIDATED

Real-world keyform accuracy:
UNVALIDATED

Runtime oracle readiness:
ORACLE READY BUT UNVALIDATED (interface prepared; no owned/compatible
model to run; no synthetic translation attempted)

CMO3 mapping readiness:
PARTIAL (semantic inputs explicit; element names, GUID policy and
editor round-trip still open; interpolation metadata must stay unset)

Security review:
round 1 (from-repository jev-review): FAIL, findings fixed -
HIGH confidence self-contradiction for duplicate axes; HIGH missing
validator span/bounds checks; HIGH panic on tampered imported JSON;
MEDIUM non-finite payload null serialization; MEDIUM sparse path untested;
LOW rule fixture drift, diagnostic amplification, validator
cross-consistency gaps. Round 2: PASS (all 8 verified fixed); its two new
LOW findings (trace NaN export guard, span-aware layout re-derivation) and
cap tests were fixed in d6c34b8.

JEV REVIEW:
PASS

Review source:
from-repository (authoritative); remote jev_verify supplementary

Review findings:
HIGH: 3
MEDIUM: 2
LOW: 5

Findings fixed:
- duplicate-key band confidence now derives from axis.usable; validator
  recomputes duplicates; golden 012 regenerated (unknown band/grid, no
  false inconsistency)
- validator checks form-index contiguity/bounds (Fatal out-of-bounds) and
  non-finite position bounds; negative tests added
- render_keys bounds-guarded (no panic on tampered documents); validator
  rejects out-of-bounds positions as Fatal
- non-finite payloads reported positionally and refused at export;
  non-contiguous spans no longer produce false layout findings; trace keys
  included in the export finiteness guard
- sparse/partial grid test added; rule fixture references resolved and
  test-guarded; per-code/unresolved caps implemented, documented and
  unit-tested; band/grid/binding identity cross-checks added

Remote jev_verify:
SUPPLEMENTARY / INCONSISTENT (cannot read repository files; returned
contradictory verdicts on identical evidence; deterministic repository
evidence and the from-repository reviews are governing per the work
order)

IR schema changes:
NONE

RecoveredProject schema changes:
NONE

Production code changed:
YES (additive: new crate, recovery-core pipeline wrappers, new CLI
command; no existing parser/IR/hierarchy semantics changed)

CMO3 writer implemented:
NO

PSD recovery implemented:
NO

GUI implemented:
NO

Open questions:
- confirm CMO3 keyform element/attribute names and GUID policy
- decide the writer's explicit grid-ordering mapping (the recovered
  document stays unknown on purpose)
- provide owned real .moc3 files and an executable runtime oracle
  (PurismCore/ayagami/Cubism Core) for behavioral validation
- decide whether blend-shape/glue keyform semantics become targets later

Ready for AGENT.5:
YES

Blocking issues:
none

Recommended next step:
start AGENT.5 (CMO3 writer) consuming Live2D IR + RecoveredProject +
RecoveredKeyformModel; keep serialization inference-free, resolve the
mapping open questions first, and do not fabricate interpolation metadata

=== END REPORT ===
```
