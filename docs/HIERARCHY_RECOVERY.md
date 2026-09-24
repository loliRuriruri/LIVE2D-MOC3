# Hierarchy Recovery (AGENT.3)

Recovery graph engine and deterministic hierarchy reconstruction over the
Live2D IR. The binary parser is unknown here: `hierarchy-recovery` depends on
`live2d-ir` only, and a workspace test enforces that boundary.

## Pipeline

```text
Live2D IR
  -> collect nodes            (metadata only: ids, kinds, associations, bindings)
  -> collect evidence edges   (stored relations = Exact, HR-010 = Heuristic)
  -> candidates               (classify, filter, rank; never guess)
  -> resolution               (confidence tiers, ambiguity preserved)
  -> cycle detection          (iterative over selected parents)
  -> RecoveredProject         (hierarchy + traces + statistics + diagnostics)
  -> hierarchy validation     (independent structural checks)
```

`recovery reconstruct-hierarchy <model.moc3|model.ir.json>` runs exactly this
pipeline. The IR itself is never mutated.

## Graph model

Nodes (work order section 5): `Part`, `WarpDeformer`, `RotationDeformer`,
`ArtMesh`, plus the synthetic `RecoveredRoot`. Parameters, textures, mask
groups, bindings and glue are **not** hierarchy nodes; they only contribute
evidence or metadata.

```text
RecoveryGraph { nodes, edges, candidates, diagnostics }
GraphNode { id, kind, source_index, source_name?, part_association?, binding? }
EvidenceEdge { child, parent, kind, confidence, rule, note? }
ParentCandidates { child, candidates[], rejected[] }
```

No geometry is copied: ids, kinds and evidence only.

### Evidence kinds

| Kind | Emitted in AGENT.3? | Notes |
|---|---|---|
| `ExplicitStoredRelation` | yes | stored parent fields (HR-001..HR-005) |
| `BindingRelationship` | yes (policy-gated) | HR-010 heuristic |
| `ReverseLink` | reserved | IR validation already enforces reverse links |
| `ObjectRange`, `DeformerRange` | reserved | need differential research before claiming ownership |
| `DrawOrderRelationship` | reserved | proximity is not parenting (work order section 22) |
| `StructuralConstraint` | reserved | currently expressed as filters, not edges |
| `DerivedConsistency` | reserved | HR-007 reports conflicts instead |
| `HeuristicFallback` | reserved | heuristic edges currently carry `BindingRelationship` |

Reserved kinds are documented, never claimed: an edge kind only appears in
output once a rule emits it.

### Typed parent rules (work order section 12)

| Parent \ Child | Part | Warp | Rotation | ArtMesh |
|---|---|---|---|---|
| Part | yes | yes | yes | yes |
| Warp / Rotation | yes | yes | yes | yes |
| ArtMesh | no | no | no | no |

The synthetic root never becomes a stored parent; the orphan fallback under
the root is intentionally not implemented (unresolved nodes go to a marked
synthetic container instead). Confidence of these constraints: `Exact`
(they follow the file's own reference tables).

## Rules

Rule ids are stable and recorded in every resolution trace.

| Rule | Description | Evidence | Confidence | Failure mode | Fixture |
|---|---|---|---|---|---|
| HR-001 | Stored part parent becomes the hierarchy parent of a part. | `part.parent` | Exact | none known | hierarchy-006 |
| HR-002 | Stored parent deformer becomes the hierarchy parent of a deformer. | `deformer.parent_deformer` | Exact | stored cycles/self parents are rejected or reported | hierarchy-004 |
| HR-003 | Stored part association becomes the parent of a deformer without a parent deformer. | `deformer.parent_part` | Exact | none known | hierarchy-002 |
| HR-004 | Stored part association becomes the parent of an art mesh without a parent deformer. | `art_mesh.parent_part` | Exact | none known | hierarchy-001 |
| HR-005 | Stored parent deformer becomes the hierarchy parent of an art mesh; the part stays an association. | `art_mesh.parent_deformer` | Exact | precedence may differ from authoring intent in non-standard files | hierarchy-002 |
| HR-006 | Deformer precedence: with an accepted deformer candidate, part candidates are demoted to the logical association. | structural rule over stored relations | Exact | association role is still preserved | hierarchy-002 |
| HR-007 | Part association must agree with the effective part of the selected deformer chain. | two stored values | Exact | mismatch = corrupt/hand-edited file; both values preserved and reported (`exact_relation_conflict`) | hierarchy-010 |
| HR-010 | Orphan art mesh whose binding is used by parts gets those parts as heuristic candidates. | `binding.used_by` | Heuristic | shared bindings are common; **disabled by default**, never treated as Exact | hierarchy-008 |

Heuristic budget (work order section 49): exactly one heuristic rule exists;
it is policy-gated, documented and fixture-covered. No "looks reasonable"
rules are present.

## Candidates and conflict resolution

1. classify edges: missing parent, self parent, wrong-type relation,
   duplicate relation, policy-blocked heuristic are rejected with reasons,
2. demote part candidates when a deformer candidate exists (HR-006),
3. deduplicate by parent keeping the strongest candidate,
4. rank by confidence `Exact > Derived > Heuristic > Unknown`,
5. resolve only when exactly one candidate has the highest confidence.

Equal-highest confidence (work order section 15) results in
`ambiguous_parent` with the tied candidates recorded; **no tie-break is
implemented** because none is justified by the format. Storage order is never
used to pick a winner.

## Cycles

Cycle detection is iterative (no recursion, no stack overflow) and runs over
selected parents. Stored cycles keep their edges and are reported
(`hierarchy_cycle`, one diagnostic per group); members get status
`cycle_detected`. Nothing is deleted: breaking an edge would fabricate
certainty the file does not contain.

## Orphans, unresolved nodes, synthetic structures

- Art meshes without any parent evidence are `unresolved` (orphans).
- Parts/deformers without stored parents are `top_level` (the file states no
  parent; this is not a guess).
- Nodes whose evidence was entirely rejected (illegal/dangling/duplicate) are
  `unresolved`.
- Unresolved and ambiguous nodes are preserved under the synthetic
  `$unresolved` container; cycles under `$cycles`. The synthetic root is
  `$root` with `synthetic: true`; model nodes never start with `$`
  (validator-enforced).

## Confidence handling

`Exact` = stored in the file. `Derived` = computed from stored values (unused
for parents in AGENT.3). `Heuristic` = produced by HR-010 only. `Unknown` is
not used for parent selection. Statistics count selections per confidence and
are called **Hierarchy Resolution Statistics** - never accuracy or
"recovered %".

## Determinism

Identical IR input yields byte-identical project JSON: nodes are ordered
(parts, deformers, art meshes; source order), candidates are sorted by
confidence then parent id, traces follow node order, and diagnostics are
appended in fixed stage order. No hash maps or randomness participate in
output.

Diagnostics are capped per code (256) with one summary entry per truncated
code, in every stage that can scale with input: guard collection, candidate
filters, resolution and hierarchy validation. Structured error output in the
CLI (for example fatal `HierarchyValidationFailed`) lists at most 50 fatal
findings plus the total count. Candidate/resolution hot paths use ordered
maps and index-aligned arrays; the effective-part walk is memoized, so a
20k-deep chain resolves in well under a second instead of O(n * depth).

Fatal hierarchy findings always block output: `reconstruct-hierarchy` exits 1
with a structured error and writes nothing when the project fails
`validate_hierarchy`.

## Recovery policy

```text
RecoveryPolicy {
    allow_heuristic_parenting: false,   // --allow-heuristic enables HR-010
    unresolved_strategy: preserve,      // only implemented mode
    conflict_strategy: leave_ambiguous, // only implemented mode
}
```

## CLI

```text
recovery reconstruct-hierarchy <model.moc3|model.ir.json>
         [--json] [--output FILE] [--strict] [--allow-heuristic]
         [--explain NODE] [--max-depth N] [--max-file-size BYTES]
```

- default: human tree + statistics + diagnostic summary,
- `--json` / `--output`: canonical recovered-project JSON,
- `--strict`: checked before writing; exits 1 without emitting output when
  `ambiguous_parent`, `hierarchy_cycle`, `dangling_candidate` or
  `illegal_relation` is present,
- `--explain`: selected parent, rule, evidence, rejected candidates with
  reasons (canonical id or stored source name; `--json` for machine output);
  diagnostic-only, so it is intentionally not strict-gated,
- `--max-file-size` also applies to IR JSON inputs,
- fatal hierarchy validation findings always exit 1 and write nothing,
- UTF-8 BOMs on IR JSON inputs are accepted.

## Project document

Schema `live2d-recovery/recovered-project/1` (status experimental):
`root`, `nodes[]` (status/parent/confidence/rule/part_association/binding/
children/trace), `containers[]`, `cycles[]`, `statistics`, `graph`
(nodes/edges/candidates), `diagnostics`, plus the copied source descriptor and
the policy. Geometry never appears.

## Negative tests (work order section 38)

| Case | Where it is covered |
|---|---|
| self parent | `hierarchy-014-self-parent` fixture + unit tests |
| two exact parents | schema stores at most one parent per relation kind; the conflicting case is HR-007 (`exact_relation_conflict`) + validator `multiple_parent` unit tests on hand-built projects |
| dangling parent | validator unit test (hand-built project) |
| cycle | `hierarchy-009-cycle` fixture + iterative detector unit tests (200k chain) |
| wrong entity type | validator unit test (hand-built project) |
| duplicate recovered id | validator unit test + `duplicate_relation` filter |

All cases produce structured diagnostics; none panics.

## Known limitations

- Real-world hierarchy accuracy is **UNVALIDATED** (no owned `.moc3`).
- HR-010 semantics are weak by nature and disabled by default.
- Range/draw-order/mask/glue evidence is deliberately not used for parenting
  (work order sections 21-24).
- Keyform selection (which keyform is active) is AGENT.4 work; this phase only
  groups and validates structure.
