# CMO3 Version Profile

One target profile only (work order section 20): **Cubism Editor 5.x
compatible**, pinned in `crates/cmo3-writer/src/profile.rs`
(`EDITOR_5_PROFILE`). Other editor versions are explicitly unsupported and
unvalidated.

## Profile values (E2 unless noted)

| Field | Value | Evidence |
|---|---|---|
| `root fileFormatVersion` | `402030000` | both pinned writers |
| `EditorEdition.edition` | `15` | both |
| `targetVersionNo` | `3000` | both |
| `latestVersionOfLastModelerNo` | `5000000` | both (Hiyori reference file shows `4020000`) |
| `CModelSource` version PI | `4` | both; avoids `rootParameterGroup`/`modelOptions`/`gameMotionSet` (E3 comment) |
| `isDefaultKeyformLocked` | `false` | moc2cmo; Stretchy writes `true` (both tolerated, E3) |

## `<?version Class:N?>` instructions

Emitted (union of both writers; the rotation-form entry is moc2cmo-only):

```text
CArtMeshSource:4
CRotationDeformerForm:1
KeyformGridSource:1
CParameterGroup:4
SerializeFormatVersion:2
CModelSource:4
CFloatColor:1
CLabelColor:0
CModelImage:3
```

Stretchy's Hiyori reference file uses a different set (`CModelSource:14`,
`CParameterGroup:3`, `ModelImageEntry:2`) — that profile is **not** our
target.

## Import instructions

`<?import fully.qualified.Class?>` (nested classes use `$`), emitted in a
fixed deterministic order from `IMPORT_CLASSES` (the moc2cmo list, 125
classes). Both writers emit the full list; the editor matches by name and
ignores unknown entries (E3).

## Version-sensitive objects

- `CModelSource:14` requires `rootParameterGroup`, `modelOptions` and
  `gameMotionSet` (not written at version 4).
- `CLabelColor:0` and `CFloatColor:1` cover the color structures used by
  forms.
- `CModelImage:3` is the image resource version referenced by the deferred
  image pipeline.
