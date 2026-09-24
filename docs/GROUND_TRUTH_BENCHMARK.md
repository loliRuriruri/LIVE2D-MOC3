# Ground Truth Benchmark Plan

Status: plan only. Real-world compatibility of the tool is currently
**UNVALIDATED** because the repository contains no owned real `.moc3` file.

## Why owned files

Synthetic fixtures prove self-consistency and cross-implementation layout
agreement; they cannot prove that real Cubism exports parse, nor that
recovered hierarchy matches what an artist authored. Only files exported by
the project owner from licensed Cubism Editor can serve as ground truth, and
they must be declared as owned before use.

## Acquisition requirements

1. Export a matrix of models from Cubism Editor, at least:
   - one minimal model, one model with warp + rotation deformers,
   - one with masks/clipping, one with multiple textures and parameters,
   - one multi-part model with non-trivial draw order.
2. Prefer several editor versions to cover several `.moc3` version bytes
   (3.0 through 5.3 at minimum; a version-6 export if available).
3. For each file record: editor version, export date, author, license basis
   ("exported by project owner"), and SHA-256.
4. Store under `fixtures/owned/` only after the owner confirms the storage
   policy; until then keep them outside the repository and mount by path.

## Benchmark procedure

1. Parse every file with `recovery inspect --json` and
   `recovery export-ir`; any structured error is a finding, not a crash
   investigation.
2. Run the differential harness against `py-moc3` (and any oracle that is
   executable at that time) and archive the reports.
3. Reconstruct hierarchy and validate against the author's declared
   structure (part -> deformer -> art mesh) for each file.
4. Record per-file: parse outcome, non-zero anomaly count, IR validation
   outcome, hierarchy conflicts, differential agreement numbers.

## Measurement definitions

- "Agreement" is always the differential statistic from
  `docs/DIFFERENTIAL_FINDINGS.md`; it is not an accuracy percentage.
- Until at least one runtime oracle (see `RUNTIME_ORACLE_PLAN.md`) and owned
  files are both in place, no accuracy claim of any kind is permitted.

## Promotion to CI

When two or more owned files exist with stable checksums, add an
env-gated integration test (`RUN_OWNED_FIXTURES=1`, path via env var) that
replays the procedure; keep default test runs offline and free of private
data.
