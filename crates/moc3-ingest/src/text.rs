//! Human-readable rendering of an [`InspectionReport`].
//!
//! The layout follows the AGENT.1 report sketch in the master spec: version,
//! size, counts, parameter ids, and hierarchy candidates. Listings are capped
//! deterministically with an explicit `(+N more)` marker so huge models stay
//! readable.

use std::fmt::Write as _;

use crate::report::InspectionReport;

const MAX_LISTED: usize = 200;

/// Render the full text report.
pub fn render_human(report: &InspectionReport) -> String {
    let mut out = String::new();
    let model = &report.model;

    let _ = writeln!(out, "Live2D Recovery - MOC3 Inspection Report");
    let _ = writeln!(out, "========================================");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "File:         {}",
        report.source.file_name.as_deref().unwrap_or("(unknown)")
    );
    let _ = writeln!(out, "File size:    {} bytes", report.source.file_size);
    let _ = writeln!(out, "MOC3 version: {}", report.summary.moc3_version_label);
    let _ = writeln!(out, "Byte order:   {}", report.summary.byte_order);
    let _ = writeln!(
        out,
        "Canvas:       {} x {} @ {} px/unit (Y axis: {})",
        model.canvas.width,
        model.canvas.height,
        model.canvas.pixels_per_unit,
        if model.canvas.flag_y_reversed {
            "reversed"
        } else {
            "normal"
        }
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "Counts");
    let _ = writeln!(out, "------");
    let _ = writeln!(out, "Parameters            {}", model.counts.parameters);
    let _ = writeln!(out, "Parts                 {}", model.counts.parts);
    let _ = writeln!(out, "Drawables             {}", model.counts.art_meshes);
    let _ = writeln!(out, "  ArtMeshes           {}", model.counts.art_meshes);
    let _ = writeln!(out, "Deformers             {}", model.counts.deformers);
    let _ = writeln!(out, "  Warp                {}", model.counts.warp_deformers);
    let _ = writeln!(
        out,
        "  Rotation            {}",
        model.counts.rotation_deformers
    );
    let _ = writeln!(
        out,
        "Keyform bindings      {}",
        model.counts.keyform_bindings
    );
    let _ = writeln!(
        out,
        "Keyforms (total)      {}",
        report.summary.total_keyforms
    );
    let _ = writeln!(out, "Glue objects          {}", model.counts.glue);
    let _ = writeln!(
        out,
        "Texture references    {}",
        report.summary.texture_reference_count
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "Parameters ({})", model.parameters.len());
    let _ = writeln!(out, "------------");
    for (count, parameter) in model.parameters.iter().enumerate() {
        if count == MAX_LISTED {
            let _ = writeln!(
                out,
                "  ... (+{} more)",
                model.parameters.len().saturating_sub(MAX_LISTED)
            );
            break;
        }
        let _ = writeln!(
            out,
            "  [{:04}] {}  min={} max={} default={} repeat={} decimals={} key_tables=[{}..{})",
            parameter.index,
            printable(&parameter.id.text),
            parameter.minimum,
            parameter.maximum,
            parameter.default,
            parameter.repeat,
            parameter.decimal_places,
            parameter.key_table_begin,
            parameter.key_table_begin + parameter.key_table_count
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "Parts ({})", model.parts.len());
    let _ = writeln!(out, "-----");
    for (count, part) in model.parts.iter().enumerate() {
        if count == MAX_LISTED {
            let _ = writeln!(
                out,
                "  ... (+{} more)",
                model.parts.len().saturating_sub(MAX_LISTED)
            );
            break;
        }
        let _ = writeln!(
            out,
            "  [{:04}] {}  parent={} keyforms={} binding={} visible={} enabled={}",
            part.index,
            printable(&part.id.text),
            format_parent(part.parent_part),
            part.keyform_count,
            part.binding_index,
            part.visible,
            part.enabled
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "Drawables ({})", model.art_meshes.len());
    let _ = writeln!(out, "---------");
    for (count, mesh) in model.art_meshes.iter().enumerate() {
        if count == MAX_LISTED {
            let _ = writeln!(
                out,
                "  ... (+{} more)",
                model.art_meshes.len().saturating_sub(MAX_LISTED)
            );
            break;
        }
        let _ = writeln!(
            out,
            "  [{:04}] {}  part={} deformer={} texture={} verts={} tris={} masks={} keyforms={}",
            mesh.index,
            printable(&mesh.id.text),
            format_parent(mesh.parent_part),
            format_parent(mesh.parent_deformer),
            mesh.texture_number,
            mesh.vertex_count,
            mesh.index_count / 3,
            mesh.mask_count,
            mesh.keyform_count
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "Deformers ({})", model.deformers.len());
    let _ = writeln!(out, "---------");
    for (count, deformer) in model.deformers.iter().enumerate() {
        if count == MAX_LISTED {
            let _ = writeln!(
                out,
                "  ... (+{} more)",
                model.deformers.len().saturating_sub(MAX_LISTED)
            );
            break;
        }
        let detail = match &deformer.specific {
            crate::raw::DeformerSpecific::Warp {
                vertex_count,
                rows,
                columns,
                ..
            } => format!("warp grid={}x{} verts={}", columns, rows, vertex_count),
            crate::raw::DeformerSpecific::Rotation { base_angle } => {
                format!("rotation base_angle={base_angle}")
            }
        };
        let _ = writeln!(
            out,
            "  [{:04}] {}  {}  parent_part={} parent_deformer={} binding={}",
            deformer.index,
            printable(&deformer.id.text),
            detail,
            format_parent(deformer.parent_part),
            format_parent(deformer.parent_deformer),
            deformer.binding_index
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "Textures");
    let _ = writeln!(out, "--------");
    if report.textures.is_empty() {
        let _ = writeln!(out, "  (none)");
    } else {
        for texture in &report.textures {
            let _ = writeln!(
                out,
                "  page {} <- {} art mesh(es) {:?}",
                texture.texture_number,
                texture.art_mesh_indices.len(),
                texture.art_mesh_indices
            );
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(
        out,
        "Hierarchy candidates ({} edges)",
        report.hierarchy.edges.len()
    );
    let _ = writeln!(out, "-------------------------------");
    for (count, edge) in report.hierarchy.edges.iter().enumerate() {
        if count == MAX_LISTED {
            let _ = writeln!(
                out,
                "  ... (+{} more)",
                report.hierarchy.edges.len().saturating_sub(MAX_LISTED)
            );
            break;
        }
        let _ = writeln!(
            out,
            "  {}[{}] {} -> {}[{}] {}  ({}, {:?})",
            edge.parent.kind,
            edge.parent.index,
            printable(&edge.parent.name),
            edge.child.kind,
            edge.child.index,
            printable(&edge.child.name),
            edge.relation,
            edge.confidence
        );
    }
    let _ = writeln!(out);

    if !model.runtime_sections.is_empty() {
        let _ = writeln!(
            out,
            "Runtime space sections ({})",
            model.runtime_sections.len()
        );
        let _ = writeln!(out, "--------------------------");
        for section in &model.runtime_sections {
            let _ = writeln!(
                out,
                "  {} offset={} elements={} bytes={}",
                section.name, section.offset, section.elements, section.byte_size
            );
        }
        let _ = writeln!(out);
    }

    let anomaly_count = report.summary.anomaly_count;
    let _ = writeln!(out, "Anomalies ({anomaly_count})");
    let _ = writeln!(out, "-------------");
    if anomaly_count == 0 {
        let _ = writeln!(out, "  (none)");
    } else {
        for anomaly in report
            .model
            .anomalies
            .iter()
            .chain(report.hierarchy.anomalies.iter())
        {
            let _ = writeln!(out, "  [{}] {}", anomaly.code, anomaly.message);
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "Not extracted in this phase");
    let _ = writeln!(out, "---------------------------");
    for entry in &model.not_extracted {
        let _ = writeln!(
            out,
            "  {}: {} element(s) ({})",
            entry.field, entry.count, entry.reason
        );
    }

    out
}

fn format_parent(value: Option<u32>) -> String {
    match value {
        Some(index) => index.to_string(),
        None => "-".to_string(),
    }
}

/// Replace control characters so crafted identifiers cannot inject terminal
/// escape sequences into the human report.
fn printable(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                '\u{FFFD}'
            } else {
                character
            }
        })
        .collect()
}
