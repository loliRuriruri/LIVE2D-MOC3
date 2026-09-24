# Live2D IR Specification (draft)

**Status: draft for AGENT.2. Not implemented in AGENT.1.**

The IR is the normalized representation between the binary parser and all
downstream recovery stages. It must not leak parser internals (raw offsets,
parallel arrays, count tables).

## Target shape (master spec section 6)

```text
ModelProject
+- Metadata          (source version, canvas, provenance notes)
+- Canvas
+- Parameters[]      id, name, minimum, maximum, default, current, keyforms
+- Parts[]           id, name, parent, children, opacity, draw_order
+- ArtMeshes[]       id, name, parent, vertices, uv, indices,
                     texture_reference, opacity, draw_order,
                     clipping_masks, parameter_bindings
+- Deformers[]       Deformer = WarpDeformer | RotationDeformer
                     id, name, parent, children, transform, bindings, keyforms
+- Drawables[]       unified draw list ordered by draw order resolution
+- Textures[]        texture_page, width, height, source, region
+- Masks[]           mask relationships between drawables
+- Bindings[]        parameter key tables and interpolation data
```

## Rules

1. **Normalization only.** The IR contains resolved references (indices ->
   ids), no raw byte offsets. Parallel arrays become objects.
2. **IDs.** `source_id` (as stored), `recovered_id` (deterministic fallback,
   `ArtMesh_0001` style), `display_name` (may be empty -> fallback used).
3. **Confidence.** Every derived field carries a recovery confidence
   (`Exact`, `Derived`, `Heuristic`, `Unknown`) like the hierarchy edges in
   AGENT.1.
4. **Determinism.** Same `RawMoc3` -> byte-identical IR JSON. No unordered
   containers in output paths.
5. **No lossy drops.** Data AGENT.2 does not interpret yet stays in an
   `uninterpreted` section rather than disappearing.

## Inputs available from AGENT.1

`moc3-ingest` currently extracts: header, canvas, counts, parameters, parts,
deformers (typed), art meshes (ids/parents/texture/flags/vertex counts/
ranges/resolved masks), masks, draw order groups/items, glue structure,
bindings and key tables, runtime scratch inventory, unknown slots, anomalies.

AGENT.2 must additionally read the pools that AGENT.1 only counts:
`keyform_position.xy`, `uv.xy`, `position_indices`, `key.values`,
per-keyform values (opacity/draw order/rotations/colors) and glue info.

## CLI contract (planned)

```text
recovery export-ir model.moc3        -> model.ir.json
```

The IR JSON needs its own schema identifier (for example
`live2d-recovery/ir/1`) and golden tests, exactly like the AGENT.1 report.
