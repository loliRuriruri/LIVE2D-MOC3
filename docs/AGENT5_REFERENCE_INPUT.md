# AGENT.5 Reference Input (from AGENT.3.5)

Hand-off note for the CMO3 writer phase (`crates/cmo3-writer`,
`docs/ARCHITECTURE.md`). AGENT.3.5 produced **no writer code**; it produced
reference facts, a mapping plan and a failure checklist.

## Primary artifact

- `docs/CMO3_MAPPING_PLAN.md` - recovered-structure -> CMO3 concept mapping,
  confirmed-at-audit facts (`CModelSource`, `CPartSource`, `EditorEdition=15`,
  `targetVersionNo=3000`, `latestVersionOfModelerNo=5000000`,
  `useLegacyDrawOrder__testImpl=false`, CAFF obfuscation key `0x42`), fidelity
  sequencing constraints, open questions and prohibited practices.

## Constraints inherited from AGENT.3.5

- No production dependency on any external reference; no code copied,
  especially from LGPL-3.0 `Quadrism` (behavioral audit only).
- Everything not listed as confirmed in the mapping plan is an open question
  for the spec authority; do not invent XML names or CAFF details.
- The writer must be deterministic (no timestamps/GUID randomness without a
  documented policy) and must fail closed on unresolved hierarchy ambiguity.
- `project-validator` (AGENT.7) will compare writer output against owned
  editor round-trips; until owned files exist, writer output cannot be
  declared correct (`docs/GROUND_TRUTH_BENCHMARK.md`).

## Failure checklist (behavioral audit, `E3`)

- strict vs lax section ordering must be an explicit mode,
- keyform duplication is a per-writer behavior and must be verified against
  editor round-trips before being emulated,
- optional label/color metadata must not be fabricated,
- missing sidecars/textures must be reported, not stubbed silently,
- obfuscation/size flags must be fixed and reproducible.

## Evidence artifacts

`docs/reports/agent-3.5/differential/` (JSON + text) and
`docs/DIFFERENTIAL_FINDINGS.md` remain the audit trail; no AGENT.5 decision
may contradict them without a new finding.
