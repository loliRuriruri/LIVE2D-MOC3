# AGENT.2 COMPLETION REPORT

```text
=== LIVE2D RECOVERY AGENT.2 REPORT ===

Repository:
D:\test\liver2d

Starting HEAD:
a220f35  feat: complete read-only moc3 inspector agent1   (AGENT.1 PASS baseline)

Ending HEAD:
b2d4e14  docs: correct test total in agent2 test plan
(chain: 7bafe90 feat: add normalized live2d intermediate representation,
       1babbf4 fix: address agent2 ir review findings,
       96c20e3 fix: harden ir validation and nested draw order coverage,
       b2d4e14 docs: correct test total in agent2 test plan)

Supervision:
spec_locked (master spec v0.1 kept authoritative; no SPEC CONFLICT encountered)

JEV implementation tier:
HARD (route suggestion: jev-hard; parent implementer kept, per supervisor rules)

Implemented:
- crates/live2d-ir: normalized semantic model (no binary-format dependency)
- mapper layer in crates/recovery-core::ir_mapper (RawMoc3 + ModelPools -> IR)
- IR validator (live2d-ir::validate_ir) and canonical JSON codec
- CLI: recovery export-ir (--output, --compact) and recovery validate-ir (--json)
- moc3-ingest::parse_full + ModelPools (bulk pools for IR, inspect path unchanged)
- fixture-gen: glue support, non-default drawable flags, nested draw order groups
- 13 synthetic fixtures and golden IR documents; docs/IR_SPEC.md finalized

IR schema:
- id: live2d-ir/1, field "schema" at the top of every document
- source container described separately (source.format=moc3, version_byte,
  version_label, byte_order); schema version and MOC3 version never conflated
- top level: schema, schema_status, source, metadata, canvas, parameters,
  parts, deformers, art_meshes, drawables, draw_order_groups, mask_groups,
  textures, glue, bindings, unknowns, diagnostics

Schema status:
EXPERIMENTAL (no real-file validation yet; fields added during this phase
carry serde defaults so older experimental documents still import)

Entities implemented:
- Parameter (min/max/default/current=null/repeat/decimals/kind/key_values/
  extension_key_values), Part (parent/children/opacity=null/draw_order=null/
  keyform_draw_orders/binding/visible/enabled)
- Deformer (warp: rows/columns/vertex_count/quad_transform/keyforms with
  positions; rotation: base_angle/keyforms with angle/origin/scale/reflect)
- ArtMesh (parents/texture/vertex_count/uvs/indices/keyforms/flags/
  blend_mode(+raw 5.3+)/opacity=null/draw_order=null/binding/visible/enabled/
  mask_groups)
- DrawOrderGroup/DrawOrderItem (typed part/art mesh targets, nested groups)
- MaskGroup (target + sources), Texture (page_index; size/path null),
  Glue (mesh pair, binding, keyform intensities, raw info entries)
- Binding (keyform_grid, keyform_grid_size, resolved parameters + key
  values, derived used_by)
- Unknowns (header padding count, undefined offset slots, unknown count
  fields, runtime section inventory) and Diagnostics

ID strategy:
- stored identifier kept verbatim when non-empty and unique within its kind
- otherwise deterministic fallback: parameter:000004, part:000001,
  artmesh:000012, warp:000003, rotation:000002, texture:000000,
  mask:000001, glue:000002, binding:000005, drawgroup:000003
- first duplicate keeps the name, later ones get the fallback
  (duplicate_source_id, Recoverable); source_name still preserves the text
- no UUIDs, randomness, timestamps, or hash-order dependence (tested)

Reference strategy:
- typed ids everywhere (ParameterId/PartId/DeformerId/ArtMeshId/TextureId/
  MaskGroupId/GlueId/BindingId/DrawOrderGroupId); typed target enums
  (BindingTarget, DrawOrderTarget)
- no indices or file offsets on semantic entities
- validator enforces resolution, bidirectional reverse links (parts,
  deformers, masks, bindings), draw order references and nesting cycles

Provenance strategy:
- entity-level Provenance { confidence, source, note } with
  Exact/Derived/Heuristic/Unknown
- field_provenance overrides where confidence differs (current, base
  opacity/draw_order, canvas origin/y_axis_reversed, art mesh blend_mode,
  glue info pairing, pre-4.2 parameter kind)
- absent fields must carry Unknown provenance (missing_field_provenance
  warning otherwise) and any non-null value for a non-stored field is Fatal
  (field_not_stored)

Unknown preservation:
- semantic areas and binary-shaped leftovers kept separate: offsets/slots/
  pointers appear only inside "unknowns" (small, structured: counts and
  identifiers), never on entities
- byte-level details (for example non-zero header padding offsets) remain in
  the AGENT.1 inspection report; the whole binary is never embedded
- runtime scratch sections preserved by name + element count + byte size
- validator/mapper never drop unknown entries (tested)

CLI:
- recovery inspect <file> [--json]                      (AGENT.1, unchanged)
- recovery export-ir <file> [--output PATH] [--compact] (pretty default)
- recovery validate-ir <file.ir.json> [--json]
- structured error envelopes on stdout in machine modes; exit 1 on failure

IR validation:
- Fatal: schema_mismatch, metadata_count_mismatch, duplicate_id (all entity
  kinds incl. draw groups), duplicate_texture_page, dangling_reference,
  mask_reference_missing, inconsistent_reverse_link (4 kinds, bidirectional),
  drawables_mismatch, parameter_range_invalid, non_finite_value,
  uv_count_mismatch, vertex_count_mismatch, index_out_of_range,
  warp_grid_mismatch, binding_grid_mismatch, non_canonical_order,
  field_not_stored, invalid_reference (draw order)
- Warning/Recoverable/Info: cycles (parent chains, draw order nesting),
  non-triangular index count, empty mask group, missing provenance,
  unmapped sections/references, conflicting/unknown blend bits,
  fallback-id usage, parser anomalies
- near-linear in document size after review fixes (8k deformers: 0.11s)

Round-trip:
PASS  (MOC3 -> IR -> JSON -> IR -> JSON byte-identical and model-equal for
       all 13 fixtures; IR JSON -> MOC3 intentionally not implemented)

Golden IR fixtures:
13 / 13  (12 original fixtures + fixture-013-glue; all byte-compared against
          CLI output; goldens were reviewed by the independent reviewer)

Tests:
previous: 46 (AGENT.1 final, all retained)
current: 113
passed: 113
failed: 0
(cargo test --workspace; clippy --all-targets --all-features and
 cargo fmt --check both clean)

Determinism:
PASS  (repeated export-ir byte-identical; 13/13 goldens; no HashMap/
       SystemTime/rand in output paths)

Parser independence:
PASS  (workspace test checks live2d-ir Cargo.toml and the cargo metadata
       dependency graph; live2d-ir depends only on serde/serde_json)

Real-world compatibility:
UNVALIDATED  (no owned Cubism export available; all validation uses 13
              synthetic fixtures built from the documented layout)

Memory review:
- mapper consumes ParsedModel by value; each bulk pool element is copied
  exactly once into owned IR fields (no whole-pool clones)
- build_ir_from_file drops the input byte buffer before mapping
- key values are duplicated per parameter and per binding (bounded, small,
  documented); peak ~ input + pools + IR, all bounded by Limits

Security review:
- parser limits reused for parse_full; validator on untrusted IR: bounds,
  counts, references, non-finite values; quadratic validator path fixed
- duplicate-ID coverage completed for draw order groups
- malformed import -> structured JsonSyntax/SchemaMismatch; tampered docs
  fail with fatal dangling_reference / inconsistent_reverse_link /
  field_not_stored
- no recursion (iterative DFS), no unsafe, no unwrap/expect/panic in
  production code; serde escapes output; CLI writes only --output paths

JEV REVIEW verdict:
PASS  (independent jev-review subagent; round 1 PARTIAL, round 2 PARTIAL,
       round 3 PASS with one LOW doc-count nit since fixed)

Review findings:
HIGH: 1
MEDIUM: 6 (4 in round 1, 2 new in round 2)
LOW: 10 (6 round 1, 3 round 2, 1 round 3 doc nit)

Findings fixed:
- HIGH art-mesh visible/enabled silently dropped -> fields added + tested
- draw order groups modeled (types, ids, mapper, validator, counts, goldens)
- reverse-link completeness enforced for parts, deformers, masks (bindings
  already complete)
- Part.offscreen_surface_index raw index removed; unmapped_reference warning
- glue fixture-013 added; unsafe placeholder id replaced by documented
  artmesh:invalid:{side}:{index} + diagnostic
- quadratic deformer/mesh validation replaced by reverse indexes (14x faster
  at 8k deformers)
- duplicate DrawOrderGroupId detection added
- serde defaults for newly added fields (pre-fix documents still import)
- art-mesh field_not_stored/provenance enforcement (completing round-1 fix)
- header-padding count-only rationale, import/unknown-field behavior, and
  fallback-id list documented
- nested draw order groups now fixture-covered (fixture-007)
- self_group == -1 mapping bug found by tests and fixed
- u16 array stride bug found by IR tests and fixed (regression test added)

Open questions:
- real-file validation of the whole layout and of the IR mapping
- offscreen unmapped_reference path has no end-to-end fixture (parser
  rejects non-default references when no surfaces exist); validator-level
  only, documented
- glue weight/position pairing semantics remain Unknown (raw preserved)
- draw group object_total_count includes nested counts in the format; the
  generator writes flat counts and the IR does not model the field
- blend enum values beyond 0 are unobserved; unknown values are preserved
- schema may still change while EXPERIMENTAL

Files changed:
- Cargo.toml/Cargo.lock, crates/live2d-ir/** (new implementation),
  crates/recovery-core/src/ir_mapper.rs + lib.rs + tests,
  apps/recovery-cli/src/main.rs + tests/cli_ir.rs,
  crates/moc3-ingest: pools.rs, parse.rs (parse_full), reader.rs (u16 fix),
  tools/fixture-gen (glue/flags/nested groups + fixture-013),
  fixtures/synthetic (13), fixtures/expected (13), fixtures/expected-ir (13),
  docs: IR_SPEC, ARCHITECTURE, FORMAT_NOTES, TEST_PLAN, LIMITATIONS,
  RESEARCH_LOG, README, reports/AGENT2_REPORT.md

Production code changed:
YES (IR layer is production code for its phase; no CMO3/PSD/GUI output)

CMO3 writer implemented:
NO

PSD recovery implemented:
NO

GUI implemented:
NO

Ready for AGENT.3:
YES  (AGENT.3 can work from live2d-ir alone: typed relations, provenance,
      keyform lists and diagnostic taxonomy are in place; parser
      independence is enforced by test)

Recommended next step:
- AGENT.3 Hierarchy Reconstruction against the IR (parent candidates,
  orphan/cycle handling, deterministic order), then AGENT.4 keyform
  selection/interpolation using the binding grids
- schedule real-file validation (inspect / export-ir / validate-ir) as soon
  as an owned .moc3 is available; no AGENT.2 changes should be required for
  that pass

Note on scope vs work order:
The work order referenced 12 golden fixtures; one additional synthetic
fixture (fixture-013-glue) was added to close a review-identified coverage
gap. All 12 original fixtures remain and pass.

=== END REPORT ===
```
