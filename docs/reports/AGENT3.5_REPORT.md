# AGENT.3.5 COMPLETION REPORT

```text
=== LIVE2D RECOVERY AGENT.3.5 REPORT ===

Repository:
D:\test\liver2d

Starting HEAD:
159da8f  docs: finalize agent3 completion report

Ending HEAD:
<this report>  docs: add agent3.5 completion report
(chain: fb9a96c feat: add dev-only differential reference harness
 (AGENT.3.5), f1295af test: add relocated-body layout interop regression,
 fcd5967 docs: add AGENT.3.5 reference audit documents and evidence,
 111f745 fix: address AGENT.3.5 review round 1 findings (units,
 consensus, caps, timeout), 5450faa test: add harness output-limit and
 unsupported-key boundary tests (review round 2), 754c420 docs: refresh
 differential artifacts at clean harness revision, <report>)

Supervision:
spec_locked (master spec v0.1 authoritative; no SPEC CONFLICT encountered)

JEV implementation tier:
HARD (jev-hard route; parent implementer kept per supervisor rules)

Independent review:
two jev-review rounds - round 1 FAIL (9 findings: 2 MEDIUM, 7 LOW),
all fixed in 111f745; round 2 PASS (all 9 verified, 3 LOW observations
addressed by 5450faa and the artifact refresh; grandchild-pipe timeout
edge documented as accepted for a dev-only tool)

Scope completed:
- external reference matrix (8 references pinned by version/commit/license,
  mode, evidence class; production-dependency ban; LGPL behavioral-only)
- dev-only differential harness crate tools/reference-harness
  (binary recovery-reference; snapshot schema
  live2d-recovery/differential-snapshot/1; deterministic fingerprint;
  compare engine with MATCH/MISMATCH/ORDER_ONLY/MISSING_*/UNSUPPORTED/
  NOT_COMPARABLE + consensus; JSON + human report with UNVALIDATED marker
  and non-accuracy disclaimer)
- real differential runs against py-moc3 (MIT, 2fb112e11a): three archived
  run artifacts under docs/reports/agent-3.5/differential/
- findings DF-001..DF-004 (body-offset convention, v6 misreads, count
  units/coverage, ID/section-order disagreement) with impact and actions
- layout interop regression test (both body placements parse identically;
  slot count derived from MocVersion::offset_slots)
- planning documents: CMO3_MAPPING_PLAN, RUNTIME_ORACLE_PLAN,
  GROUND_TRUTH_BENCHMARK, AGENT4/5_REFERENCE_INPUT
- documentation updates: ARCHITECTURE, FORMAT_NOTES, RECOVERY_RULES,
  TEST_PLAN, LIMITATIONS, RESEARCH_LOG

Explicitly NOT in scope / not done:
- no AGENT.4/AGENT.5 code, no CMO3 writer, no keyform evaluation
- no production crate depends on any external reference; no code copied
  from any reference (Quadrism LGPL-3.0 audited behaviorally only)
- no accuracy claim of any kind; real-world compatibility UNVALIDATED
  (no owned real .moc3)

Differential results (cross-implementation comparison only):
- R1 fixture-002 (v3.3, native body offset): py-moc3 REFERENCE_ERROR
  (hardcoded body offset 1984) - DF-001
- R2 relocated-002-b (py-convention body): comparable 46, agreement 24,
  disagreement 3 (parts.ids, art_meshes.ids, hierarchy_edges) - DF-003,
  DF-004; canvas/parameters/counts agree (13/15 counts MATCH)
- R3 fixture-010 (v6): comparable 38, agreement 27, disagreement 8
  (5 canvas zeros + 3 art-mesh counts) - DF-002

Harness safety properties:
- external tools run only as subprocesses, env-gated
  (PY_MOC3_DIR/RUN_REFERENCE_TESTS), 30 s timeout, 8 MiB stdout cap
  (exact-limit accepted, overflow flagged only when bytes are dropped),
  stderr excerpt capped, timeout path kills and detaches readers,
  no network or secrets in default runs, deterministic outputs

Quality evidence (at 754c420):
cargo test --workspace: 182 passed, 0 failed
cargo clippy --workspace --all-targets --all-features: exit 0, no warnings
cargo fmt --all --check: clean
git status: clean

Docs delivered:
docs/EXTERNAL_REFERENCE_MATRIX.md, docs/DIFFERENTIAL_FINDINGS.md,
docs/CMO3_MAPPING_PLAN.md, docs/RUNTIME_ORACLE_PLAN.md,
docs/GROUND_TRUTH_BENCHMARK.md, docs/AGENT4_REFERENCE_INPUT.md,
docs/AGENT5_REFERENCE_INPUT.md, docs/reports/agent-3.5/differential/

Open questions / spec decisions surfaced:
1. whether a future phase may provision builds/runtimes for PurismCore or
   ayagami as an executable oracle (py-moc3 remains the only executable
   reference today)
2. owned real .moc3 files for ground-truth benchmarking (required before
   any production-readiness claim)
3. CMO3 writer questions listed in docs/CMO3_MAPPING_PLAN.md (schema
   target, GUID policy, deformer/parameter element names, CAFF details)

Residual risk:
- single executable external reader; its known deviations are documented
  and excluded from oracle use (DF-002/DF-004)
- line-by-line provenance of "no code copied" cannot be machine-proven;
  enforced by policy, license audit and repository state
- real-file behavior remains unvalidated
```
