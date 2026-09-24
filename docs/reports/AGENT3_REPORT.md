# AGENT.3 COMPLETION REPORT

```text
=== LIVE2D RECOVERY AGENT.3 REPORT ===

Repository:
D:\test\liver2d

Starting HEAD:
a5b8d19  docs: add agent2 completion report

Ending HEAD:
53e38bf  docs: clarify diagnostic caps and analyze_graph usage
(chain: 032f0b0 feat: add hierarchy recovery graph and deterministic
 resolver, ce3dfc9 test: add hierarchy reconstruction fixtures,
 c8d4553 docs: document hierarchy recovery rules,
 5592b58 fix: gate hierarchy output on validation and bound resolver
 complexity, 53e38bf doc clarification; this report is committed on top)

Supervision:
spec_locked (master spec v0.1 authoritative; no SPEC CONFLICT encountered)

JEV implementation tier:
HARD (jev-hard route; parent implementer kept per supervisor rules)

Implemented:
- crates/hierarchy-recovery (RecoveryGraph, evidence, candidates, resolver,
  hierarchy validator, recovered-project JSON, human tree/statistics/explain)
- CLI: recovery reconstruct-hierarchy <model.moc3|model.ir.json>
  [--json] [--output] [--strict] [--allow-heuristic] [--explain NODE]
  [--max-depth N] [--max-file-size N]
- 14 hierarchy fixtures (001..014) + 25 recovered-project goldens
- workspace boundary test extended to hierarchy-recovery

RecoveryGraph:
- nodes (metadata only: id, kind, source_index, source_name, part
  association, binding), evidence edges, candidate sets with rejection
  reasons, guard diagnostics; geometry is never copied (tested)
- node order canonical: parts, then deformers, then art meshes (source order)

Node types:
Root (synthetic, "$root"), Part, WarpDeformer, RotationDeformer, ArtMesh.
Parameters/textures/mask groups/bindings/glue are not hierarchy nodes; they
only provide evidence or metadata.

Evidence types:
ExplicitStoredRelation (HR-001..HR-005, Exact), BindingRelationship
(HR-010, Heuristic, policy-gated). ReverseLink/ObjectRange/DeformerRange/
DrawOrderRelationship/StructuralConstraint/DerivedConsistency/
HeuristicFallback are documented as reserved and never claimed. HR-007
(association vs deformer-chain part) reports exact_relation_conflict;
HR-006 (deformer precedence) demotes part candidates to associations.

Recovery rules:
HR-001 part->parent part, HR-002 deformer->parent deformer, HR-003
deformer->part, HR-004 art mesh->part (no deformer), HR-005 art
mesh->deformer (part kept as association), HR-006 deformer precedence,
HR-007 association consistency check, HR-010 binding-share heuristic
(disabled by default). All documented with evidence/confidence/failure
mode/fixture in docs/HIERARCHY_RECOVERY.md; a test keeps the doc in sync.

Candidate strategy:
classify edges -> reject missing/self/wrong-type/duplicate/policy-blocked
with reasons -> apply deformer precedence -> deduplicate by parent ->
rank Exact > Derived > Heuristic > Unknown. No storage-order tie-breaks.

Resolution strategy:
resolve only when exactly one candidate holds the highest confidence tier;
otherwise status ambiguous with tied candidates recorded. Parts/deformers
without stored parents are exact top-level; art meshes without evidence are
unresolved orphans. Manually rejected-only evidence (self/illegal/dangling/
duplicate) yields unresolved, never fabricated top-level status.

Confidence handling:
Exact = stored; Heuristic only via policy-gated HR-010; Derived reserved for
parents; Unknown never selects a parent. Statistics count selections per
confidence; no accuracy-like claims anywhere.

Ambiguity handling:
left ambiguous (status + tied candidates + ambiguous_parent warning); the
1 first-wins behavior is explicitly forbidden and untested against.

Orphan handling:
preserved (never deleted) under the synthetic "$unresolved" container
(unresolved + ambiguous), clearly marked synthetic=true.

Cycle handling:
stored cycles keep their edges; members get status cycle_detected and one
hierarchy_cycle diagnostic per group; detection is iterative (200k-chain
unit test, 0.44s) and rendering/explain are cycle-safe.

RecoveredProjectIR:
live2d-recovery/recovered-project/1 (EXPERIMENTAL). Root, nodes (status,
parent, confidence, rule, part_association, binding, children, full trace),
containers, cycles, Statistics, graph (nodes/edges/candidates), diagnostics.
No geometry; no IR mutation; AGENT.4/5 can work from this document without
the parser.

CLI:
recovery reconstruct-hierarchy (moc3 or canonical IR JSON input; mocs and IR
runs are byte-identical for the same model). Human tree + Hierarchy
Resolution Statistics by default; --json/--output for canonical JSON
(--output also prints the human report); --strict checked before writing;
--explain diagnostic-only (documented exception); fatal hierarchy findings
always exit 1 with structured HierarchyValidationFailed and write nothing;
UTF-8 BOM accepted; --max-file-size applies to JSON inputs too.

Canonical output:
byte-identical across runs (verified on cycle and 6001-node scale fixtures);
25/25 golden documents byte-matched; goldens are env-gated (never silently
regenerated); large/deep fixtures are GOLDEN_SKIP with determinism runs.

Hierarchy fixtures:
001 Part->ArtMesh, 002 Part->Warp->ArtMesh, 003 Part->Rotation->ArtMesh,
004 nested warp, 005 warp->rotation->ArtMesh, 006 multiple/nested parts,
007 orphan, 008 ambiguous (heuristic), 009 stored cycle, 010 part mismatch
(HR-007), 011 deep (1500), 012 large flat (1500), 013 scale (2000 warps /
4000 meshes), 014 self-parent. Negative matrix also covers two-exact-parent
conflict (HR-007), dangling parent, wrong entity type, duplicate relation and
duplicate recovered id via validator/filter unit tests - all structured, no
panics.

Hierarchy statistics:  (measured, not accuracy)
- reconstructed hierarchy fixtures report exact/derived/heuristic counts,
  unresolved, ambiguous, cycles and orphans; e.g. hierarchy-006 = 1 top
  level + 5 exact; hierarchy-013-scale = 6001 nodes (6000 resolved, 1 top
  level); hierarchy-011-deep = 1502 nodes (1501 resolved).
- statistics are explicitly titled "Hierarchy Resolution Statistics" and
  never called recovery accuracy.

Tests:
previous: 113 (all retained)
current: 161
passed: 161
failed: 0
(cargo test --workspace; clippy -D warnings and fmt clean)

Determinism:
PASS

Large graph test:
PASS (hierarchy-013-scale 6001 nodes, ~1s; validator wide-flat 6000, ~1s)

Deep hierarchy test:
PASS (hierarchy-011-deep 1502 nodes; 20k chain unit test; 200k chain cycle
unit test; iterative traversal and rendering; no stack overflow)

Parser independence:
PASS (hierarchy-recovery depends only on live2d-ir; manifest + cargo
metadata workspace test; CLI is the only combiner)

Geometry duplication:
NO

Real-world compatibility:
UNVALIDATED

Real-world hierarchy accuracy:
UNVALIDATED

Security review:
- cycles cannot hang (iterative detection/render/effective-part walk, cycle
  guards; 200k-chain test)
- deep chains cannot overflow the stack (no recursion; 20k fixture test)
- diagnostics amplification bounded (256/code per stage + summaries; CLI
  fatal list capped at 50)
- pathological complexity fixed: 20k-deep chain 160.3s -> 2.02s after
  memoization/indexing (reviewer-measured); wide-flat validator O(deg^2)
  removed
- malicious node counts: mocs path bounded by parser Limits; IR JSON path
  bounded by the file-size cap (documented residual risk)
- JSON depth: deeply nested JSON fails structurally (JsonSyntax, no crash)
- fatal validator findings can no longer be exported (review HIGH fixed)

JEV REVIEW verdict:
PASS  (independent jev-review subagent; baseline PARTIAL with 2 HIGH, 2
       MEDIUM, 7 LOW; all fixed; follow-up verification PASS)

Review findings:
HIGH: 2
MEDIUM: 2
LOW: 7

Findings fixed:
- fatal hierarchy findings now gate output (structured error, nothing
  written) in every mode,
- effective-part walk memoized + index-aligned candidates + prebuilt kind
  map (quadratic CPU path removed),
- diagnostic budget keys by exact code (no "other" bucket) and is used by
  guard and validator stages; CLI error list capped,
- validator reciprocal check uses a prebuilt children index (no O(deg^2)),
- strict gating moved before output writing; --max-file-size applies to IR
  JSON; UTF-8 BOM accepted; explain documented as diagnostic-only; rule doc
  sync test added; vacuous filter test replaced with real analyze_graph
  assertions; unused parameter removed.

IR schema changes:
NONE (AGENT.3 consumes the IR read-only; its own document is
live2d-recovery/recovered-project/1, status EXPERIMENTAL)

Open questions:
- real-world hierarchy validation on an owned .moc3 (still unavailable),
- CLI-level validation of recovered-project JSON documents is intentionally
  not implemented (projects are produced, not imported, in this phase),
- HR-010 heuristic quality is unmeasurable until real files exist,
- some diagnostic codes can legitimately appear up to 256 times per stage
  (documented nuance).

Files changed:
- crates/hierarchy-recovery/** (new implementation + tests)
- apps/recovery-cli/** (command, tests)
- tools/fixture-gen/** (hierarchy fixtures, GOLDEN_SKIP)
- crates/recovery-core/tests/** (hierarchy suite, golden support, skip
  handling), crates/moc3-ingest/tests (skip handling, sampled sweeps)
- tests/workspace-smoke (boundary test)
- fixtures/synthetic (+14), fixtures/expected (+14), fixtures/expected-ir
  (+14), fixtures/expected-hierarchy (25)
- docs: HIERARCHY_RECOVERY.md (new), ARCHITECTURE, IR_SPEC, RECOVERY_RULES,
  TEST_PLAN, LIMITATIONS, RESEARCH_LOG, README, fixtures/README

Production code changed:
YES (hierarchy layer is production code for its phase; no CMO3/PSD/GUI)

CMO3 writer implemented:
NO

Keyform reconstruction implemented:
NO

PSD recovery implemented:
NO

GUI implemented:
NO

Ready for AGENT.4:
YES (resolved ownership, deformer hierarchy, art mesh parents, part
associations, binding identities, confidence/provenance, unresolved nodes
and full traces are all present; keyform work is untouched)

Recommended next step:
- AGENT.4 keyform recovery on top of the recovered project (binding grids,
  keyform selection/interpolation), then AGENT.5 CMO3 writer
- schedule real-file validation (inspect / export-ir / validate-ir /
  reconstruct-hierarchy) as soon as an owned .moc3 is available; no AGENT.3
  changes should be required for that pass

Supervision note:
JEV verify supported the boundary, fatal-gate and cap claims strongly; a few
compound claims were flagged at low confidence because the terse verifier
could not correlate the evidence blob. Deterministic test output and the
independent review transcript (baseline PARTIAL -> fixes -> PASS) are the
authoritative evidence per the supervisor rules.

=== END REPORT ===
```
