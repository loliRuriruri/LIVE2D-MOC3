# Live2D IR Specification

**Schema:** `live2d-ir/1`
**Status:** `EXPERIMENTAL` (work order section 31). No real Cubism export has
been validated yet; the schema may still change. Do not treat it as a stable
public contract.
**Implemented:** AGENT.2 (`crates/live2d-ir`, mapper in `recovery-core`).

## 1. Purpose and boundaries

The IR is the normalized semantic model between the binary parser and every
downstream stage:

```text
MOC3 -> moc3-ingest (RawMoc3 + ModelPools)
     -> recovery-core::ir_mapper  (mapper layer)
     -> live2d-ir (Live2DModel)   (this specification)
     -> recovery-core::export_ir_json / import_ir_json
     -> canonical JSON
```

Hard rules:

- `live2d-ir` **must not depend on `moc3-ingest`** (or any binary-format
  crate). The dependency direction is enforced by a workspace test.
- Semantic entities never contain offsets, table slots, pointers or section
  indices. Binary-shaped leftovers live only inside `unknowns` (small,
  structured, preserved for future format research).
- Nothing is invented: fields the file does not store (base opacity, base
  draw order, parameter "current" values, texture dimensions/paths) are
  `null` and carry `Unknown` provenance.
- No hierarchy heuristics, interpolation, or keyform reconstruction; those
  are AGENT.3/AGENT.4.

## 2. Top-level document

```json
{
  "schema": "live2d-ir/1",
  "schema_status": "experimental",
  "source": { "format": "moc3", "version_byte": 2,
              "version_label": "2 (3.3.00-3.3.03)", "byte_order": "little" },
  "metadata": { "generator": "live2d-recovery", "generator_phase": "AGENT.2",
                "entity_counts": { "parameters": 1, "parts": 1, "...": 0 } },
  "canvas": { "...": "..." },
  "parameters": [], "parts": [], "deformers": [], "art_meshes": [],
  "drawables": [], "mask_groups": [], "textures": [], "glue": [],
  "bindings": [], "unknowns": { "...": "..." }, "diagnostics": []
}
```

Field order is the serialization order (deterministic). `source` describes the
source container; `schema` describes this document - the two versions are
never conflated. `metadata` contains no timestamps and no file paths.

## 3. Entities

| Entity | Id type | Key fields |
|---|---|---|
| `Parameter` | `ParameterId` | minimum/maximum/default, current (always null), repeat, decimal_places, kind (`normal`/`blend_shape`), key_values, extension_key_values (4.2+) |
| `Part` | `PartId` | parent, children (derived), opacity (null), draw_order (null), keyform_draw_orders, binding, visible, enabled |
| `Deformer` (`kind`: `warp`/`rotation`) | `DeformerId` | parent_part, parent_deformer, children_deformers/children_art_meshes (derived), binding, visible, enabled; warp: rows/columns/vertex_count/quad_transform + keyforms (opacity, positions); rotation: base_angle + keyforms (opacity, angle, origin, scale, reflect_x/y) |
| `ArtMesh` | `ArtMeshId` | parent_part, parent_deformer, texture, vertex_count, uvs, indices, keyforms (opacity, draw_order, positions), flags, blend_mode (+raw for 5.3+), opacity/draw_order (null), binding, visible, enabled, mask_groups |
| `DrawOrderGroup` / `DrawOrderItem` | `DrawOrderGroupId` | items (part or art mesh targets), optional nested group (`self_group`), maximum/minimum order |
| `MaskGroup` | `MaskGroupId` | target ArtMesh, sources (stored mask list of the target) |
| `Texture` | `TextureId` | page_index; width/height/source_path null until `model3.json` support |
| `Glue` | `GlueId` | art_mesh_a/b, binding, keyform_intensities, info entries |
| `Binding` | `BindingId` | keyform_grid (per-parameter key counts), keyform_grid_size, parameters (key values resolved through key table ownership), used_by (derived) |

`drawables` lists the art mesh ids in source order. Resolved draw order is a
later-phase concern; ordering here is canonical, not visual.

### Deliberately not modeled in AGENT.2

- blend shapes, constraints and keyform color pools (present in the file,
  validated, reported via an `unmapped_sections` diagnostic),
- offscreen surfaces (5.3): part references to them are reported with an
  `unmapped_reference` diagnostic instead of leaking a raw surface index,
- per-keyform interpolation/selection (AGENT.4),
- base opacity / base draw order (not stored in the file),
- texture dimensions and image paths (`model3.json` territory).
- glue *pairing semantics* (which position index belongs to which side) are
  preserved raw with `Unknown` provenance; see FORMAT_NOTES §11.

## 4. Identifier policy

1. `id` = the stored identifier text when it is non-empty and unique inside
   its entity kind.
2. Otherwise a deterministic fallback is generated from the source index:
   `parameter:000004`, `part:000001`, `artmesh:000012`, `warp:000003`,
   `rotation:000002`, `texture:000000`, `mask:000001`, `glue:000002`,
   `binding:000005`, `drawgroup:000003`.
   Dangling glue references (defensive, hand-edited IR only) use the
   documented error placeholder `artmesh:invalid:{side}:{index:06}`.
3. The first occurrence of a duplicated source name keeps the name; later
   occurrences receive the fallback (`duplicate_source_id`, Recoverable).
4. `source_name` always preserves the stored text when present, even when the
   canonical id had to fall back.
5. Forbidden: UUIDs, randomness, timestamps, hash-order dependence.

The warp/rotation fallbacks share one uniqueness namespace (a deformer can
never collide with another deformer).

## 5. Reference policy

- All references are typed ids (`PartId`, `DeformerId`, `ArtMeshId`,
  `TextureId`, `MaskGroupId`, `GlueId`, `BindingId`, `ParameterId`); wrong
  entity kinds are impossible by construction and are re-checked by the
  validator on import.
- Stored relations (`parent_part`, `parent_deformer`, mask lists, binding
  indices) are `Exact`.
- Structural facts computed by the mapper are `Derived`: reverse links
  (`children`, `children_deformers`, `children_art_meshes`, `used_by`),
  texture page entities, mask group entities, binding `keyform_grid`,
  `y_axis_reversed`, art mesh `blend_mode`.
- Hierarchy cycles are reported (`hierarchy_cycle`, Warning); resolution is
  AGENT.3 work.

## 6. Provenance

```json
{ "confidence": "exact" | "derived" | "heuristic" | "unknown",
  "source": "moc3:art_mesh.parent_part", "note": "optional" }
```

- Every entity carries one `provenance` object.
- Fields whose confidence differs from the entity get a `field_provenance`
  entry (`current`, base `opacity`/`draw_order`, `origin`,
  `y_axis_reversed`, `blend_mode`, glue `info` pairing semantics).
- Absent fields must be documented with an `Unknown` field provenance; the
  validator warns (`missing_field_provenance`) when that is missing.

## 7. Unknown preservation

`unknowns` keeps data the semantic layer does not interpret yet:

```json
{ "header_reserved_nonzero_bytes": 0,
  "undefined_offset_slots": [ { "slot": 120, "value": 1234 } ],
  "unknown_count_fields": [ { "index": 40, "value": 7 } ],
  "runtime_sections": [ { "name": "art_mesh.id_runtime",
                          "element_count": 2, "byte_size": 16 } ] }
```

Rules: empty/zero entries are omitted; contents are counts and identifiers
only (never payload dumps); the whole binary is never embedded. The
validator and mapper never drop these entries. Full byte-level details of
unknown regions (for example the offsets of non-zero header padding bytes)
stay in the AGENT.1 inspection report, which is the designated place for
byte-level research data.

## 8. Canonical ordering

| Collection | Order |
|---|---|
| parameters, parts, deformers, art_meshes, glue, bindings | source order |
| drawables | art mesh source order |
| draw_order_groups | source order (items in stored order) |
| mask_groups | ascending target art mesh index |
| textures | ascending page number |
| children / used_by | source order (parts, then deformers, then art meshes, then glue for bindings) |
| diagnostics | parser anomalies, mapper notes, validator findings (deterministic within each group) |

No output path depends on hash-map iteration.

## 9. Validation rules (`validate_ir`)

Fatal diagnostics block export (`to_json_str` refuses them, `export-ir` exits
1 with a structured error and the diagnostic list).

| Code | Severity | Meaning |
|---|---|---|
| `schema_mismatch` | Fatal | document is not `live2d-ir/1` |
| `metadata_count_mismatch` | Fatal | declared counts differ from arrays |
| `duplicate_id`, `duplicate_texture_page` | Fatal | id/page uniqueness |
| `dangling_reference`, `mask_reference_missing` | Fatal | typed references must resolve |
| `inconsistent_reverse_link` | Fatal | parts (children), deformers (children_deformers, children_art_meshes), masks (mask_groups <-> target) and bindings (used_by) must all be reciprocal |
| `field_not_stored` | Fatal | `current`, base `opacity`/`draw_order` must be null (not stored in .moc3) |
| `invalid_reference` | Fatal | draw order part item without a nested group |
| `drawables_mismatch` | Fatal | drawables must list art mesh ids in order |
| `parameter_range_invalid` | Fatal | `minimum <= default <= maximum` |
| `non_finite_value` | Fatal | NaN/Inf never enter canonical JSON |
| `uv_count_mismatch`, `vertex_count_mismatch` | Fatal | counts must agree with `vertex_count` |
| `index_out_of_range` | Fatal | triangle indices `< vertex_count` |
| `warp_grid_mismatch` | Fatal | `vertex_count == (rows+1)*(cols+1)` |
| `binding_grid_mismatch` | Fatal | grid size equals product of dimensions |
| `non_canonical_order` | Fatal | textures ascending; keyform indices sequential |
| `index_count_not_triangular` | Warning | index count not a multiple of 3 |
| `empty_mask_group` | Warning | mask group without sources |
| `missing_field_provenance` | Warning | absent field lacks `Unknown` provenance |
| `hierarchy_cycle` | Warning | stored parent chain or draw order nesting contains a cycle |
| `unmapped_reference` | Warning | entity references a section not modeled in AGENT.2 (offscreen surfaces) |
| `unmapped_sections`, `unknown_blend_mode`, `orphan_key_table` | Warning | mapper-level notes |
| `empty_source_id`, `duplicate_source_id` | Recoverable | deterministic fallback used |
| `moc3_*` parser anomalies | Info/Warning | forwarded from the parser |

## 10. Export / import / round-trip

- `recovery export-ir <file> [--output PATH] [--compact]` - pretty JSON is
  the default; output bytes are identical for identical input (SHA256-stable).
- `recovery validate-ir <file.ir.json> [--json]` - imports the document,
  re-validates it and reports diagnostics; exit 1 on any Fatal.
- Library: `export_ir_json` (validates first), `import_ir_json` (schema
  check), `validate_ir`.
- Round-trip contract: `MOC3 -> IR -> JSON -> IR -> JSON` must be
  byte-identical at the JSON stage and equal as models (tested for all 13
  fixtures). `IR JSON -> MOC3` is intentionally **not** implemented.
- `import_ir_json` does not validate and serde currently ignores unknown
  JSON fields; library embedders must call `validate_ir`, and the CLI
  `validate-ir` command re-validates every imported document. Fields added
  during the experimental phase carry `#[serde(default)]` where a safe
  default exists so older documents keep importing.

## 11. Example (trimmed from `fixture-002` golden)

```json
{
  "schema": "live2d-ir/1",
  "schema_status": "experimental",
  "source": { "format": "moc3", "version_byte": 2,
              "version_label": "2 (3.3.00-3.3.03)", "byte_order": "little" },
  "bindings": [
    {
      "id": "binding:000000",
      "keyform_grid_size": 2,
      "keyform_grid": [2],
      "parameters": [ { "parameter": "ParamSynthetic_AngleX",
                        "key_values": [-30.0, 30.0] } ],
      "used_by": [ { "kind": "part", "id": "Part_Synthetic_00" },
                   { "kind": "art_mesh", "id": "ArtMesh_Synthetic_00" } ],
      "provenance": { "confidence": "derived", "source": "moc3:binding",
                      "note": "parameters resolved through the parameter key table ranges; reverse links derived" }
    }
  ],
  "unknowns": { "runtime_sections": [ { "name": "part.id_runtime",
                                        "element_count": 1, "byte_size": 8 } ] },
  "diagnostics": []
}
```
