"""Our adapter that turns a py-moc3 checkout into a differential snapshot.

This script is OUR code (MIT). It imports the pinned py-moc3 package from
PY_MOC3_DIR (a local checkout at commit 2fb112e11a) and prints one
DifferentialSnapshot JSON document on stdout. py-moc3 itself is dev-only and
never becomes a production dependency.

Usage:
    python py_moc3_snapshot.py path/to/model.moc3

Environment:
    PY_MOC3_DIR  path to a py-moc3 checkout (repo root containing src/moc3)
"""

import json
import os
import sys

SCHEMA = "live2d-recovery/differential-snapshot/1"
PROVIDER = "py-moc3"

COUNT_NAMES = {
    0: "parts",
    1: "deformers",
    2: "warp_deformers",
    3: "rotation_deformers",
    4: "art_meshes",
    5: "parameters",
    10: "keyform_positions",
    12: "keyform_bindings",
    13: "parameter_bindings",
    14: "keys",
    15: "uvs",
    16: "position_indices",
    17: "drawable_masks",
    20: "glue",
}


def fnv1a64(data):
    value = 0xCBF29CE484222325
    for byte in data:
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{value:016x}"


def section(moc, name):
    try:
        return moc[name]
    except Exception:
        return None


def ids_list(moc, name):
    values = section(moc, name)
    return list(values) if values else []


def index_name(ids, index):
    if index is None or index < 0 or index >= len(ids):
        return None
    return ids[index]


def main():
    if len(sys.argv) != 2:
        print("usage: py_moc3_snapshot.py model.moc3", file=sys.stderr)
        return 2
    path = sys.argv[1]
    with open(path, "rb") as handle:
        raw = handle.read()

    checkout = os.environ.get("PY_MOC3_DIR")
    if not checkout:
        print("PY_MOC3_DIR is not set", file=sys.stderr)
        return 2
    sys.path.insert(0, os.path.join(checkout, "src"))
    from moc3 import Moc3  # noqa: E402  (pinned external, dev-only)

    moc = Moc3.from_file(path)

    unsupported = {}
    snapshot = {
        "schema": SCHEMA,
        "provider": PROVIDER,
        "input": {
            "file_name": os.path.basename(path),
            "size": len(raw),
            "fnv1a64": fnv1a64(raw),
        },
        "moc3_version": raw[4] if len(raw) > 4 else None,
        "endian": ("big" if raw[5] == 1 else "little") if len(raw) > 5 else None,
        "canvas": None,
        "counts": {},
        "parameters": [],
        "parts": [],
        "deformers": [],
        "art_meshes": [],
        "hierarchy_edges": [],
        "diagnostics": [],
        "unsupported": {},
    }

    canvas = getattr(moc, "canvas", None)
    if canvas is not None:
        snapshot["canvas"] = {
            "width": float(canvas.canvas_width),
            "height": float(canvas.canvas_height),
            "pixels_per_unit": float(canvas.pixels_per_unit),
            "origin_x": float(canvas.origin_x),
            "origin_y": float(canvas.origin_y),
        }
    else:
        unsupported["canvas"] = "py-moc3 exposed no canvas"

    counts = getattr(moc, "counts", None) or []
    for index, name in COUNT_NAMES.items():
        if index < len(counts):
            snapshot["counts"][name] = int(counts[index])

    parameter_ids = ids_list(moc, "parameter.ids")
    minimums = section(moc, "parameter.min_values") or []
    maximums = section(moc, "parameter.max_values") or []
    defaults = section(moc, "parameter.default_values") or []
    if parameter_ids:
        for index, identifier in enumerate(parameter_ids):
            snapshot["parameters"].append(
                {
                    "id": identifier,
                    "minimum": float(minimums[index]) if index < len(minimums) else 0.0,
                    "maximum": float(maximums[index]) if index < len(maximums) else 0.0,
                    "default": float(defaults[index]) if index < len(defaults) else 0.0,
                }
            )
    else:
        unsupported["parameters"] = "parameter.ids unavailable"

    part_ids = ids_list(moc, "part.ids")
    part_parents = section(moc, "part.parent_part_indices") or []
    if part_ids:
        for index, identifier in enumerate(part_ids):
            parent_index = int(part_parents[index]) if index < len(part_parents) else -1
            snapshot["parts"].append(
                {"id": identifier, "parent": index_name(part_ids, parent_index)}
            )
    else:
        unsupported["parts"] = "part.ids unavailable"

    deformer_ids = ids_list(moc, "deformer.ids")
    deformer_types = section(moc, "deformer.types") or []
    deformer_parent_deformers = section(moc, "deformer.parent_deformer_indices") or []
    deformer_parent_parts = section(moc, "deformer.parent_part_indices") or []
    if deformer_ids:
        for index, identifier in enumerate(deformer_ids):
            kind = "warp" if int(deformer_types[index]) == 0 else "rotation"
            parent_deformer = (
                int(deformer_parent_deformers[index])
                if index < len(deformer_parent_deformers)
                else -1
            )
            parent_part = (
                int(deformer_parent_parts[index]) if index < len(deformer_parent_parts) else -1
            )
            snapshot["deformers"].append(
                {
                    "id": identifier,
                    "kind": kind,
                    "parent_deformer": index_name(deformer_ids, parent_deformer),
                    "parent_part": index_name(part_ids, parent_part),
                }
            )
    else:
        unsupported["deformers"] = "deformer.ids unavailable"

    art_ids = ids_list(moc, "art_mesh.ids")
    art_parent_parts = section(moc, "art_mesh.parent_part_indices") or []
    art_parent_deformers = section(moc, "art_mesh.parent_deformer_indices") or []
    art_textures = section(moc, "art_mesh.texture_indices") or []
    art_vertices = section(moc, "art_mesh.vertex_counts") or []
    art_index_counts = section(moc, "art_mesh.position_index_counts") or []
    art_mask_begin = section(moc, "art_mesh.mask_begin_indices") or []
    art_mask_counts = section(moc, "art_mesh.mask_counts") or []
    mask_values = section(moc, "drawable_mask.art_mesh_indices") or []
    if art_ids:
        for index, identifier in enumerate(art_ids):
            vertex_count = int(art_vertices[index]) if index < len(art_vertices) else 0
            index_count = int(art_index_counts[index]) if index < len(art_index_counts) else 0
            mask_begin = int(art_mask_begin[index]) if index < len(art_mask_begin) else 0
            mask_count = int(art_mask_counts[index]) if index < len(art_mask_counts) else 0
            mask_refs = []
            if mask_count > 0 and mask_begin >= 0:
                mask_refs = [
                    int(value)
                    for value in mask_values[mask_begin : mask_begin + mask_count]
                ]
            parent_part = (
                int(art_parent_parts[index]) if index < len(art_parent_parts) else -1
            )
            parent_deformer = (
                int(art_parent_deformers[index])
                if index < len(art_parent_deformers)
                else -1
            )
            snapshot["art_meshes"].append(
                {
                    "id": identifier,
                    "parent_part": index_name(part_ids, parent_part),
                    "parent_deformer": index_name(deformer_ids, parent_deformer),
                    "texture": int(art_textures[index]) if index < len(art_textures) else None,
                    "vertex_count": vertex_count,
                    "uv_count": vertex_count * 2,
                    "index_count": index_count,
                    "mask_refs": mask_refs,
                }
            )
    else:
        unsupported["art_meshes"] = "art_mesh.ids unavailable"

    # Hierarchy edges follow the same structural rule as our resolver:
    # deformer parent wins over part association.
    def edge(child, parent, relation):
        snapshot["hierarchy_edges"].append(
            {"child": child, "parent": parent, "relation": relation}
        )

    for entry in snapshot["parts"]:
        if entry["parent"]:
            edge(entry["id"], entry["parent"], "part")
    for index, entry in enumerate(snapshot["deformers"]):
        if entry["parent_deformer"]:
            edge(entry["id"], entry["parent_deformer"], "deformer")
        elif entry["parent_part"]:
            edge(entry["id"], entry["parent_part"], "part")
        _ = index
    for entry in snapshot["art_meshes"]:
        if entry["parent_deformer"]:
            edge(entry["id"], entry["parent_deformer"], "deformer")
        elif entry["parent_part"]:
            edge(entry["id"], entry["parent_part"], "part")

    snapshot["hierarchy_edges"].sort(key=lambda item: (item["child"], item["parent"]))
    snapshot["counts"] = {key: snapshot["counts"][key] for key in sorted(snapshot["counts"])}
    snapshot["unsupported"] = {key: unsupported[key] for key in sorted(unsupported)}

    print(json.dumps(snapshot, indent=2, sort_keys=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
