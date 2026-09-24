# External Reference Matrix (AGENT.3.5)

Status: audit complete at HEAD `159da8f` (+ uncommitted AGENT.3.5 work).
Scope: this document records which external projects were inspected, at which
pinned revision, under which license, and for what purpose. It does not grant
any of them a role in the shipped tool.

## Hard rules

- No production crate may depend on, link, invoke or bundle any external
  reference. The differential harness in `tools/reference-harness` is a
  **dev-only** tool; the workspace dependency-graph test still governs the
  production crates.
- No code is copied from any reference. Licenses below differ; even
  permissive code is not copied because provenance must stay auditable.
- `Quadrism` is LGPL-3.0 (copyleft). No code, no linking, no bundled binary;
  only its documented CLI behavior (option surface) was used as a checklist.
- Differential runs execute external tools only as isolated subprocesses with
  timeout, output caps and explicit env gating (`PY_MOC3_DIR`). Default test
  runs are fully offline.

## Evidence classes used in AGENT.3.5

| Class | Meaning |
|---|---|
| `E1` | Primary/official artifact (Live2D official documentation or bytes produced by Live2D tooling). None available in this environment. |
| `E2` | Two or more independent third-party sources/implementations agree. |
| `E3` | Exactly one third-party source/implementation supports the observation. |
| `E4` | No external support; our own synthetic fixtures / self-consistency only. |
| `E5` | Inference or hypothesis; not directly supported by any artifact. |

## References

| Reference | Pinned revision | License | Mode in AGENT.3.5 | Used for | Production dependency |
|---|---|---|---|---|---|
| `moc2cmo` | 1.1.18 / `2527e24e93` | MIT | source audit (not executed) | CMO3 XML element/attribute names for the mapping plan; CAFF context | no |
| `py-moc3` | `2fb112e11a` | MIT | **executed** (pure-Python subprocess) | real differential snapshots of `.moc3` parsing | no |
| `PurismCore` | v1.1.0 / `1069334965` | MIT | source audit (not executed) | C parser layout cross-checks (section order, counts, header) | no |
| `ayagami` | `0d1d7aa3ef` | Apache-2.0 | source audit (not executed) | format cross-reading, runtime-oracle candidate | no |
| `Quadrism` | `cb3f8557bd` | LGPL-3.0 | behavioral audit only (not executed) | failure-mode checklist for CMO3 writers (ordering, duplication, obfuscation) | no |
| `caff-archive` | 0.1.0 / `76f208cf01` | MIT (per repository README; no detected license file) | reference-only | CMO3/CAFF archival notes | no |
| `StretchyStudio` (pelmentor) | head, pushed 2026-06-19 | MIT | reference-only | CMO3 export reference in `docs/live2d/README.md` | no |
| `StretchyStudio` (image pipeline, AGENT.5.1) | `5fd958def9ed` | MIT ("Copyright (c) 2026 Nguyen Phan"), retrieved 2026-09-24 | source audit | `cmo3_generate.py`, `cmo3_multi_test.py`, `CMO3_FORMAT.md` - ModelImage/LayeredImage field evidence | no |
| `moc2cmo` (image pipeline, AGENT.5.1) | `2527e24e93` | MIT ("Copyright (c) 2026 Eatgrapes"), retrieved 2026-09-24 | source audit | `src/decompiler/xml/texture/*` - filter graph, layers, model image | no |
| `fixture-gen` (first-party) | in-repo | project | executed | synthetic fixtures and differential baseline (`ours`) | n/a |

Pinned metadata is stored once in
`tools/reference-harness/src/providers.rs::pinned_references()` and is emitted
into every differential report, so each JSON artifact is self-describing.

## Execution status and why

- `py-moc3` is the only reference that can be executed here without a build
  toolchain: it is pure Python and the snapshot adapter
  (`tools/reference-harness/scripts/py_moc3_snapshot.py`) only imports the
  pinned checkout. It produced the real differential evidence in
  `docs/DIFFERENTIAL_FINDINGS.md`.
- `PurismCore` (C) and `ayagami` (application runtime) were audited by source
  reading only; executing them would require provisioning build/runtime
  environments that the work order does not require. Their layout statements
  remain the cross-check backbone used since AGENT.1.
- `moc2cmo`, `Quadrism`, `caff-archive` and `StretchyStudio` are not
  `.moc3` readers in a form we can execute against our fixtures; they inform
  the mapping plan and failure checklists only.

## What the audit changed

- Parser semantics: **nothing**. All 163 pre-existing tests are unchanged.
- Confidence accounting: the `py-moc3` layout deviations (body offset,
  version-6 handling, art-mesh section order) are now documented findings
  (DF-001..DF-004) instead of silent assumptions.
- One parser-level regression test was added
  (`crates/moc3-ingest/tests/layout_interop.rs`) proving that both body
  placements (offset table end, and the py-moc3 offset 1984) parse to
  identical semantics.
