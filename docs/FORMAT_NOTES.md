# MOC3 Format Notes

Research log for the `.moc3` binary layout as understood at **AGENT.1**.
Every entry uses the master spec format:

```text
Observation / Evidence / Confidence / Fixture / Implementation / Open questions
```

Confidence levels: `Unknown`, `Unverified`, `Observed`, `Hypothesis`,
`Confirmed`.

> **Verification status (important).** These notes were derived from public
> format references and cross-checked between two or more independent
> implementations (see `LICENSE_NOTES.md`). The parser is exercised against
> checked-in *synthetic* fixtures (12 files, versions 3.0 through 5.3) built
> from the same documented layout. **No real Cubism export was parsed in
> this environment**; real-file validation is pending an owned model. Treat
> "Confirmed" below as "confirmed across independent public references and
> self-consistent fixtures", not as "validated against Cubism Core".

---

## 1. File header (64 bytes)

```text
0x00 u8[4]  magic "MOC3"
0x04 u8     version byte (1..=6)
0x05 u8     endian flag (0 = little endian, 1 = big endian)
0x06..0x40  reserved (58 bytes; Cubism Core uses part of it in place at runtime)
```

- **Evidence:** PurismCore `src/moc3.h` (`psm__moc3_header`, static-asserted to
  64 bytes; endian handling in `src/moc3.c`); OpenL2D `moc3.hexpat`
  (`Header`, magic/version/isBigEndian).
- **Confidence:** `Confirmed` (descriptions agree).
- **Fixture:** all 12 fixtures write this header; corruption tests patch
  magic/version/endian bytes.
- **Implementation:** `moc3-ingest/src/parse.rs` (header step),
  `version.rs`.
- **Open questions:** whether the reserved bytes are guaranteed zero by the
  exporter (we report non-zero bytes as anomalies instead of failing).

### Version bytes

| byte | Cubism Editor | offset slots | count ints |
|---|---|---|---|
| 1 | 3.0.00-3.2.07 | 160 | 32 |
| 2 | 3.3.00-3.3.03 | 160 | 32 |
| 3 | 4.0.00-4.1.05 | 160 | 32 |
| 4 | 4.2.00-4.2.04 | 160 | 32 |
| 5 | 5.0.00-5.2.03 | 160 | 64 |
| 6 | 5.3.00+ | 480 | 64 |

- **Evidence:** Live2D official Core API reference (`csmMocVersion_30/33/40`,
  version ranges); PurismCore `include/PurismCore.h` adds
  `csmMocVersion_42`, `_50`, `_53`; PurismCore `PSM__COUNT_INFO_INTS` and
  `psm__moc3_data_v52`/`v53` (160 vs 480 offsets).
- **Confidence:** `Confirmed`.
- **Implementation:** `version.rs`; unknown bytes (>6) and 0 are rejected as
  `UnsupportedVersion`.

---

## 2. Section offset table

```text
0x40  u32 offsets[160]   (versions 1-5)
0x40  u32 offsets[480]   (version 6+; only the first 167 are defined today)
```

Slot order is a cumulative sequence of version blocks:

| block | slots | content |
|---|---|---|
| v3.0 base | 0-100 | count/canvas, parts, deformers, warps, rotations, art meshes, parameters, keyform tables, key tables, keys, UVs, indices, masks, draw order, glue |
| v3.3 | 101 | `warp.quad_transform` |
| v4.2 | 102-136 | parameter extensions, per-object keyform color ranges, color pools, parameter types, blend shapes |
| v5.0 | 137-151 | per-keyform color offsets, blend shape targets (parts/rotations/glue) |
| v5.3 | 152-166 | part offscreen index, art mesh blend mode, offscreen surfaces, offscreen keyforms, blend shape offscreen targets |

- **Evidence:** PurismCore `PSM__SECTIONS_V30/V33/V42/V50/V53` macros define
  exactly this order; the block sizes are 101/1/35/15/15. Slot numbers for
  art meshes (34-48, 68-71, 78-80, 107-113) match the independent slot
  constants in the `mocari` crate sources and the read calls in `moc2cmo`
  `src/moc3/model/art_mesh.rs` **exactly, with identical field semantics**;
  `py-moc3` documents the same fixed 160-entry `u32` table.
- **Confidence:** `Confirmed` for v1-4.2 (three independent implementations);
  `Observed` for v5.0/v5.3 blocks (PurismCore, consistent with OpenL2D's
  v5.0 block naming; the offscreen/5.3 block exists only in PurismCore).
- **Fixture:** versions 1, 2, 3, 4, 5, 6 fixtures exercise all six layouts.
- **Implementation:** `table.rs` (`SLOT_DEFS`, `SLOT_COUNT_BY_VERSION`),
  `parse.rs` (walk + validation).
- **Open questions:** meaning of unused slots in a given version (we report
  non-zero values as `unknown_offset_slot` anomalies).

### Discrepancy with the OpenL2D ImHex pattern

`OpenL2D/moc3ingbird`'s `moc3.hexpat` models a *variable-length* per-item
pointer table starting at `0x2C0` and marks `0x40..0x2C0` as padding. Three
other implementations (PurismCore, mocari, moc2cmo) plus `py-moc3` model a
*fixed* slot table at `0x40`. We follow the fixed-slot model. The hexpat
itself carries the note "This pattern has not been fully verified for
correctness", and the fixed-slot model reproduces the exact same first data
offset (`0x2C0` for v1-5, `0x7C0` for v6) because the exporter writes count
info data right after the table, which likely explains the misreading.

- **Confidence:** `Observed` (we cannot test against real files here).
- **Open questions:** confirm on an owned model that `offsets[0]` really is
  the count-info pointer and not `0x2C0`-relative.

### Validation rules applied while walking the table

- every slot value must be `<= file size` (`SectionTableOutOfRange`),
- every used section must be 8-byte aligned (`MisalignedSection`),
- section data must fit the file (`SectionOutOfBounds`),
- dynamic sections must be monotonic in slot order (`SectionsNotMonotonic`),
- empty sections may point to EOF (observed as normal in public notes and
  permitted by validate-order; we allow `offset == len` when size is 0).

- **Evidence:** PurismCore `src/verify.c` (`psm__bounds_check_*`,
  monotonicity check, EOF allowance comment in the hexpat `fixOffset`).
- **Confidence:** `Confirmed` (design mirrored, not copied).

---

## 3. Count info table (slot 0)

- 32 `i32` values for v1-4 (128 bytes), 64 for v5+ (256 bytes).
- Counts are non-negative; `warp_deformers + rotation_deformers == deformers`.
- Unknown/padding entries beyond the documented 38 are preserved and
  reported when non-zero.

- **Evidence:** PurismCore `psm__count_info` + `psm__verify_count_info`.
- **Confidence:** `Confirmed`.
- **Implementation:** `counts.rs`.

---

## 4. Canvas info (slot 1)

```text
f32 pixels_per_unit, f32 origin_x, f32 origin_y, f32 width, f32 height, u8 flags
```

- flag bit 0 = Y axis reversed. Other bits are preserved raw and reported as
  `unknown_canvas_flag_bits` when set.
- We read the first 24 bytes; the exact size of the section in the file is
  ambiguous between references (PurismCore uses 24; the hexpat suggests more
  padding). Since only these 21 bytes are meaningful to consumers, we do not
  depend on the padding extent.
- **Evidence:** PurismCore `psm__canvas_info` + `PSM__CANVAS_FLAG_Y_REVERSED`;
  hexpat `CanvasInfo` (same five floats + flag).
- **Confidence:** `Confirmed` for the fields; `Unverified` for total size.

---

## 5. Objects, keyform bindings, key tables

- Every part/deformer/art mesh/glue stores a `binding_index` into the
  keyform binding table. A binding is a set of key table indices
  (`binding.key_table_begin/count` -> `key_table_index[]`).
- A key table maps to a contiguous run of key values
  (`key_table.begin/count` -> `key.values[]`).
- The number of keyforms a binding can address is the product of the key
  counts of its tables (counts of 1 do not extend the grid).
- Each object declares its keyform range (`*_keyform_begin/count`) into the
  matching per-category keyform arrays; the declared count must cover the
  binding product.
- **Evidence:** PurismCore `psm__binding_keyform_count`, `psm__check_key_combo`
  and `src/model.c` usage.
- **Confidence:** `Observed` (single primary source plus consistent field
  naming in two others).
- **Implementation:** `parse.rs` (`read_bindings`, `check_keyform_spans`).
- **Open questions:** exact interpolation semantics belong to AGENT.4.

## 6. Runtime scratch sections

Sections named `*.id_runtime`, `art_mesh.uv_runtime`,
`art_mesh.position_index_runtime`, `art_mesh.drawable_mask_runtime`,
`parameter.key_runtime`, `offscreen.drawable_mask_runtime` are **runtime
pointer scratch**: Cubic Core overwrites them in place at load time. On disk
they reserve 8 bytes per element in our model.

- **Evidence:** PurismCore treats them as native pointer arrays and requires
  room for `count * sizeof(pointer)` in 64-bit builds; the hexpat models the
  same regions as runtime space.
- **Confidence:** `Observed`.
- **Implementation:** the parser validates their placement but does not
  interpret their contents; they are listed under `runtime_sections`.
- **Open questions:** whether the on-disk reservation is exactly 8 bytes per
  element on all platforms (32-bit builds would need less; the file must
  satisfy the 64-bit bound, so we use 8).

## 7. Art mesh data

- `texture_number` selects the texture page; the moc3 file itself does **not**
  contain the texture count or file names - those live in `model3.json`.
- `drawable_flags` byte: bits 0-1 blend mode, bit 2 double sided, bit 3
  inverted mask, bits 4-7 unknown to us (preserved, reported).
- Geometry: `vertex_count`, UV range (`uv_begin` + `2 * vertex_count` floats
  into the global UV pool), triangle index range (`index_begin/count` into the
  global `u16` pool), drawable mask range (`mask_begin/count` into the global
  mask pool whose entries reference art meshes or -1).
- **Evidence:** PurismCore `psm__art_mesh_src` + verifier ranges; mocari
  `art_meshes.rs` uses the same slot semantics.
- **Confidence:** `Confirmed` for ranges/structure; `Observed` for flag bits.
- **Implementation:** `parse.rs::read_art_meshes`, `read_masks`.
- **Open questions:** full blend mode enum (AGENT.2/AGENT.6), triangle winding
  conventions, and the exact mask stack semantics (AGENT.3).

## 8. Parts and deformers

- Parts: id (64-byte field), binding, keyform range, visible/enabled flags,
  optional parent part (-1 = none). 5.3 adds an optional offscreen index.
- Deformers: id, binding, visible/enabled, optional parent part and parent
  deformer, type (0 = warp, 1 = rotation) and a type-local index into the
  warp/rotation specific arrays.
- Warp specifics: grid rows/columns, `vertex_count == (rows+1)*(cols+1)`,
  optional quad-transform flag (3.3+).
- Rotation specifics: base angle.
- **Evidence:** PurismCore `psm__verify_idx` (deformer type/local index
  checks, warp grid equation), `psm__deformer_src`.
- **Confidence:** `Confirmed` (structure and checks).
- **Implementation:** `parse.rs::read_parts`, `read_deformers`.

## 9. Draw order groups, glue, blend shapes

- Draw order groups reference items (type 0 = art mesh, 1 = part) with an
  optional nested group index; part items must reference a group.
- Glue objects connect two art meshes and carry paired vertex-index info
  (even count).
- Blend shape sections exist from 4.2; the parser validates their ranges but
  does not yet interpret them (AGENT.6).
- **Evidence:** PurismCore verifier; hexpat naming.
- **Confidence:** `Observed`.
- **Implementation:** `parse.rs::read_draw_order_*`, `read_glues`,
  `validate_blend_shape_sections`.
- **Open questions:** draw order *use* in rendering (AGENT.3/AGENT.6), glue
  weight meaning (AGENT.6).

## 10. Identifier fields

64-byte NUL-terminated UTF-8. We preserve invalid bytes (hex dump in JSON),
report missing terminators, trailing bytes after the terminator and duplicate
identifiers.

- **Evidence:** PurismCore `psm__id { char data[64] }`; hexpat
  `union ID { char value[64] }`.
- **Confidence:** `Confirmed`.

---

## 11. AGENT.2 additions (IR normalization)

- **Drawable flag bits** (slot `art_mesh.drawable_flags`, u8): bit 0 additive
  blending, bit 1 multiplicative blending (mutually exclusive), bit 2 double
  sided, bit 3 inverted mask; bits 4-7 unknown. Evidence: PurismCore
  `include/PurismCore.h` constant flags (`csmBlendAdditive`,
  `csmBlendMultiplicative`, `csmIsDoubleSided`, `csmIsInvertedMask`).
  Confidence: `Confirmed`. Implementation: mapper maps bits to
  `DrawableFlags` and a derived `blend_mode` (`AddCompatible` /
  `MultiplyCompatible` / `Normal`).
- **Blend mode field (5.3+)**: version-6 files carry an explicit i32 blend
  mode (slot 153) using the extended `csmColorBlendType` enum (0 Normal,
  1 AddCompatible, 2 MultiplyCompatible, 3 Add, 4 AddGlow, 5 Darken,
  6 Multiply, 7 ColorBurn, 8 LinearBurn, 9 Lighten, 10 Screen, 11 ColorDodge,
  12 Overlay, 13 SoftLight, 14 HardLight, 15 LinearLight, 16 Hue, 17 Color).
  Evidence: PurismCore `include/PurismCore.h` + `psm__remap_blend_mode`.
  Confidence: `Observed`. Unknown raw values are preserved and reported
  (`unknown_blend_mode`).
- **Parameter extension keys (4.2+)**: slots 103/104 give a per-parameter
  range into the global key value pool for blend-shape parameters.
  Confidence: `Observed` (single implementation; ranges validated).
  Implementation: `ModelPools::parameter_extension_key_begin/count`, mapped
  to `Parameter.extension_key_values`.
- **Keyform color pools (4.2+)**: counts and ranges exist and are validated,
  but the values are not mapped into the IR yet (AGENT.6); the IR reports an
  `unmapped_sections` diagnostic when the pools are non-empty.
- **Runtime scratch sections** are preserved in the IR as name + element
  count + byte size only (never offsets/payloads).
- **Regression note:** triangle index pools are `u16`; per-element reads must
  advance two bytes (the parser reader bug found by AGENT.2 tests).

## 12. Not yet verified (explicit list)

- any real-file behaviour (no owned `.moc3` available at AGENT.1 time),
- big-endian files (supported in the reader, never observed),
- blend shape windows and constraint evaluation (AGENT.6),
- offscreen rendering semantics for 5.3 (AGENT.6),
- keyform interpolation and parameter binding runtime rules (AGENT.4),
- hierarchy implications of draw order groups and mask stacks (AGENT.3).

## 13. AGENT.3.5 differential notes (body placement, py-moc3)

- **Body offset conventions.** The offset table ends at `0x2C0 = 704`.
  Community readers and our fixtures place the body (count info onward) at
  704 for versions 1-5 and at 1984 for version 6. `py-moc3` (pinned
  `2fb112e11a`) instead hardcodes the body at 1984 with
  `COUNT_INFO_SIZE = 128`, `COUNT_INFO_MAX = 23` for every version. Our parser
  follows the offset-table pointers, so it accepts both placements; this is
  locked by `crates/moc3-ingest/tests/layout_interop.rs` (relocate body by
  +1280, assert identical inspection semantics).
- **py-moc3 on version-6 input:** it parses but reports canvas `0/0/0/0/0` and
  art-mesh counts permuted (see DF-002 in `docs/DIFFERENTIAL_FINDINGS.md`).
  Not usable as a v6 canvas oracle.
- **Section-order disagreement:** on a body-relocated v3.3 file, `py-moc3`
  reads part IDs as empty and art-mesh IDs as a byte-shifted tail of the real
  ID (DF-004). Our slot order (four runtime-space slots before `ids` in each
  block) remains corroborated by PurismCore + `mocari` + `moc2cmo`.
- **Count units:** on-disk UV count is a float count (2 per UV pair); the
  harness snapshot was corrected accordingly. `keyform_positions` is the sum
  of position floats across art-mesh and warp keyforms; `keys` is the sum of
  parameter key values (including extension keys).
- **No semantic change:** none of these notes changed parser, IR or hierarchy
  behavior; they add interop evidence and guard tests.
