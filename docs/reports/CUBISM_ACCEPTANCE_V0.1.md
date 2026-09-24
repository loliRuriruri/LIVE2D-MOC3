# Cubism Acceptance Record — Live2DRecovery 0.1.0-alpha

Editor version: **NOT RECORDED** (no Cubism Editor environment available)
OS: Windows (build environment)
Test date: not executed

## Status

```text
Cubism acceptance:
NOT EXECUTED
```

Structural precheck (`inspect-cmo3`) passed for all three acceptance files
before any editor test; this is **not** an editor-compatibility claim.
`Ready for AGENT.6` therefore remains **NO** until a real editor run exists.

## How to execute (manual workflow)

1. Unzip `dist/Live2DRecovery-0.1.0-alpha-win-x64.zip`.
2. Open `acceptance/01-minimal.cmo3`, `02-deformer.cmo3`, `03-keyform.cmo3`
   in the installed Cubism Editor.
3. Record the exact editor version and OS above.
4. Fill the checklists below with OPEN_PASS / OPEN_WARNING / OPEN_FAIL and
   capture any warning text verbatim (a file repaired by the editor's
   recovery dialog is OPEN_WARNING, never PASS).
5. Optionally save/reopen each file (Save As -> Close -> Reopen).

## Structural precheck evidence

`dist/Live2DRecovery-0.1.0-alpha-win-x64/acceptance/expected/*.inspection.json`
(CAFF valid, XML valid, 0 dangling refs, 0 duplicate ids, texture
resources present).

| File | CAFF | XML | Dangling | Duplicates | Textures |
|---|---|---|---|---|---|
| 01-minimal.cmo3 | PASS | PASS | 0 | 0 | 1 |
| 02-deformer.cmo3 | PASS | PASS | 0 | 0 | 1 |
| 03-keyform.cmo3 | PASS | PASS | 0 | 0 | 1 |

## 01 Minimal (1 Part, 1 ArtMesh, 1 texture)

```text
Open:            NOT_TESTED
Warnings:        -
Texture:         NOT_TESTED (visible / orientation / not black)
ArtMesh:         NOT_TESTED
Save/reopen:     NOT_TESTED
```

## 02 Deformer (Part -> Warp -> Rotation -> ArtMesh)

```text
Open:            NOT_TESTED
Hierarchy:       NOT_TESTED (Part, Warp, Rotation, ArtMesh nesting)
Geometry:        NOT_TESTED
Save/reopen:     NOT_TESTED
```

## 03 Keyform (1 parameter, 3 keys, keyed forms)

```text
Open:            NOT_TESTED
Parameter:       NOT_TESTED (min -30 / default 0 / max 30 expectations)
Key movement:    NOT_TESTED
Save/reopen:     NOT_TESTED
```

Expected parameter for 03: `Param_AngleX` with keys `[-30, 0, 30]`
(min -30 / default 0 / max 30; fixture `keyform-002-1d-three-keys`); the
recovered document stores the key values verbatim, and the three keyed
forms correspond to those three parameter positions.

## AGENT.5.4 trigger

If any file opens with warnings, fails to open, or the editor repairs it,
AGENT.5.4 (Cubism Compatibility Repair) starts from the captured editor
messages; do not proceed to AGENT.6.
