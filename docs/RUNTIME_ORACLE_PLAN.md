# Runtime Oracle Plan

Status: plan only; nothing in this document is executed by AGENT.3.5. It
defines how a future phase can validate recovered semantics against an
independent evaluator instead of only comparing file layouts.

## Why a runtime oracle

Layout agreement (AGENT.3.5) proves that two parsers read the same numbers.
It cannot prove that reconstructed keyforms/deformers behave like the original
model. A runtime oracle evaluates the model (parameters -> vertices, opacities,
draw order) and produces a behavior snapshot that can be compared against our
IR-driven evaluation.

## Candidate oracles

| Oracle | License | Status | Notes |
|---|---|---|---|
| Cubism Core native runtime | proprietary, redistribution restricted | not available here | would be the `E1` reference; must be installed by the user, never bundled |
| `PurismCore` | MIT | candidate | C reimplementation; source-audited in AGENT.3.5, not executed; needs a build toolchain |
| `ayagami` | Apache-2.0 | candidate | runtime behavior; needs its full app environment |
| `py-moc3` | MIT | rejected for evaluation | it does not evaluate deformers/keyforms, only reads layout |

## Procedure (future phase)

1. Pin an oracle revision and record it exactly as in
   `docs/EXTERNAL_REFERENCE_MATRIX.md`.
2. Build a minimal harness app that loads a `.moc3`, sets a parameter vector,
   and dumps: drawable vertex positions, opacities, draw order, and mask
   states, with floats canonicalized.
3. Run the same parameter vectors (grid + randomized, fixed seed) through our
   IR-based evaluator and diff the snapshots with the AGENT.3.5 tolerance
   policy (abs 1e-6 / rel 1e-5).
4. Every divergence becomes a numbered finding (RF-###) with an evidence
   class; do not "fix" semantics until the divergence is classified.

## Constraints

- The oracle must never become a production dependency; it stays in
  `tools/` with env-gated execution.
- Proprietary runtimes must be supplied by the user's own installation;
  no binaries are committed.
- The oracle validates behavior, not identity: matching vertices do not prove
  the recovered hierarchy is what the artist authored.

## Exit criteria for calling runtime validation "done"

- at least one oracle revision pinned and reproducible,
- a fixed parameter-vector corpus with deterministic outputs,
- all divergences either explained (with `E1-E5` class) or fixed with
  regression tests,
- owned real files under `docs/GROUND_TRUTH_BENCHMARK.md` used at least once.
