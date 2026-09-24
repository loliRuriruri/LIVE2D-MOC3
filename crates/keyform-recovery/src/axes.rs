//! Stage 2: normalize raw binding axes into [`ParameterAxis`] records.
//!
//! Normalization validates and preserves; it never sorts, deduplicates or
//! clamps (work order sections 9, 25, 26, 27). Problems become diagnostics
//! and unresolved entries, and the axis is marked unusable when its key
//! semantics cannot be trusted.

use live2d_ir::{Diagnostic, EntityRef, Live2DModel, Provenance, Severity};

use crate::codes;
use crate::collect::{AxisDraft, BindingDraft};
use crate::model::ParameterAxis;
use crate::rule::RuleId;

/// Normalization result for one binding.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedBinding {
    /// Binding id.
    pub id: live2d_ir::BindingId,
    /// Normalized axes in binding order.
    pub axes: Vec<ParameterAxis>,
    /// Supported targets in stored used_by order.
    pub targets: Vec<crate::model::KeyformTarget>,
    /// Whether a glue entry references this binding.
    pub glue_used: bool,
}

/// Normalize all collected binding drafts.
pub fn normalize_axes(
    model: &Live2DModel,
    drafts: &[BindingDraft],
) -> (Vec<NormalizedBinding>, Vec<Diagnostic>) {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut normalized: Vec<NormalizedBinding> = Vec::with_capacity(drafts.len());

    for draft in drafts {
        if draft.axes.len() > crate::model::MAX_AXES_PER_BAND {
            diagnostics.push(Diagnostic::new(
                Severity::Warning,
                codes::AXIS_COUNT_LIMIT_EXCEEDED,
                format!(
                    "binding '{}' declares {} axes which exceeds the hard limit of {}; the cardinality stays unresolved",
                    draft.id.as_str(),
                    draft.axes.len(),
                    crate::model::MAX_AXES_PER_BAND
                ),
            ));
        }
        let mut axes: Vec<ParameterAxis> = Vec::with_capacity(draft.axes.len());
        for axis_draft in &draft.axes {
            if axis_draft.keys.len() > crate::model::MAX_AXIS_KEYS {
                diagnostics.push(Diagnostic::new(
                    Severity::Warning,
                    codes::AXIS_KEY_LIMIT_EXCEEDED,
                    format!(
                        "binding '{}' parameter '{}' stores {} keys which exceeds the hard limit of {}; the cardinality stays unresolved",
                        draft.id.as_str(),
                        axis_draft.parameter.as_str(),
                        axis_draft.keys.len(),
                        crate::model::MAX_AXIS_KEYS
                    ),
                ));
            }
            axes.push(normalize_axis(model, draft, axis_draft, &mut diagnostics));
        }
        normalized.push(NormalizedBinding {
            id: draft.id.clone(),
            axes,
            targets: draft.supported_targets.clone(),
            glue_used: draft.glue_used,
        });
    }

    (normalized, diagnostics)
}

fn normalize_axis(
    model: &Live2DModel,
    draft: &BindingDraft,
    axis: &AxisDraft,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParameterAxis {
    let mut rules: Vec<RuleId> = vec![RuleId::new("KB-002")];
    let parameter = axis.parameter.clone();
    let resolved_entity = model
        .parameters
        .iter()
        .find(|entity| entity.id == axis.parameter);

    let resolved = axis.resolved;
    if !resolved {
        rules.push(RuleId::new("KB-003"));
        diagnostics.push(Diagnostic::for_entity(
            Severity::Warning,
            codes::DANGLING_PARAMETER,
            format!(
                "binding '{}' references parameter '{}' which does not exist in the IR; the axis is preserved as unresolved",
                draft.id.as_str(),
                parameter.as_str()
            ),
            EntityRef::Parameter(parameter.clone()),
        ));
    }

    // Non-finite keys: preserved positionally, never embedded (canonical JSON
    // cannot carry them).
    let stored_key_count = axis.keys.len();
    let mut keys: Vec<f32> = Vec::with_capacity(stored_key_count);
    let mut non_finite: Vec<usize> = Vec::new();
    for (position, value) in axis.keys.iter().enumerate() {
        if value.is_finite() {
            keys.push(*value);
        } else {
            non_finite.push(position);
        }
    }
    if !non_finite.is_empty() {
        rules.push(RuleId::new("KB-004"));
        diagnostics.push(Diagnostic::for_entity(
            Severity::Warning,
            codes::NON_FINITE_PARAMETER_KEY,
            format!(
                "binding '{}' parameter '{}' stores {} non-finite key value(s) at position(s) {}",
                draft.id.as_str(),
                parameter.as_str(),
                non_finite.len(),
                render_positions(&non_finite)
            ),
            EntityRef::Parameter(parameter.clone()),
        ));
    }

    // Duplicates and ordering: reported, never repaired.
    let mut duplicate_positions: Vec<usize> = Vec::new();
    for position in 1..keys.len() {
        if keys.get(position) == keys.get(position - 1) {
            duplicate_positions.push(position);
        }
    }
    if !duplicate_positions.is_empty() {
        rules.push(RuleId::new("KB-004"));
        diagnostics.push(Diagnostic::for_entity(
            Severity::Warning,
            codes::DUPLICATE_PARAMETER_KEY,
            format!(
                "binding '{}' parameter '{}' stores duplicate consecutive key value(s) at position(s) {}; keys are preserved as stored",
                draft.id.as_str(),
                parameter.as_str(),
                render_positions(&duplicate_positions)
            ),
            EntityRef::Parameter(parameter.clone()),
        ));
    }
    let mut unsorted_positions: Vec<usize> = Vec::new();
    for position in 1..keys.len() {
        if let (Some(previous), Some(current)) = (keys.get(position - 1), keys.get(position)) {
            if current < previous {
                unsorted_positions.push(position);
            }
        }
    }
    if !unsorted_positions.is_empty() {
        rules.push(RuleId::new("KB-004"));
        diagnostics.push(Diagnostic::for_entity(
            Severity::Warning,
            codes::UNSORTED_PARAMETER_KEYS,
            format!(
                "binding '{}' parameter '{}' stores keys out of ascending order at position(s) {}; stored order is preserved",
                draft.id.as_str(),
                parameter.as_str(),
                render_positions(&unsorted_positions)
            ),
            EntityRef::Parameter(parameter.clone()),
        ));
    }

    // Range disagreement is reported but not treated as invalid: extension /
    // extrapolation semantics are possible (work order section 27).
    if let Some(entity) = resolved_entity {
        let out_of_range: Vec<usize> = keys
            .iter()
            .enumerate()
            .filter(|(_, value)| **value < entity.minimum || **value > entity.maximum)
            .map(|(position, _)| position)
            .collect();
        if !out_of_range.is_empty() {
            rules.push(RuleId::new("KB-004"));
            diagnostics.push(Diagnostic::for_entity(
                Severity::Warning,
                codes::OUT_OF_RANGE_PARAMETER_KEY,
                format!(
                    "binding '{}' parameter '{}' stores key value(s) outside the declared range [{}..{}] at position(s) {}; extension/extrapolation semantics are possible and are not assumed",
                    draft.id.as_str(),
                    parameter.as_str(),
                    entity.minimum,
                    entity.maximum,
                    render_positions(&out_of_range)
                ),
                EntityRef::Parameter(parameter.clone()),
            ));
        }
    }

    let experimental = axis.experimental;
    if experimental {
        rules.push(RuleId::new("KB-005"));
        diagnostics.push(Diagnostic::for_entity(
            Severity::Warning,
            codes::BLEND_SHAPE_BINDING_EXPERIMENTAL,
            format!(
                "binding '{}' parameter '{}' participates in blend-shape experimental semantics; the axis is preserved but no interpolation meaning is claimed",
                draft.id.as_str(),
                parameter.as_str()
            ),
            EntityRef::Parameter(parameter.clone()),
        ));
    }

    let usable = resolved && non_finite.is_empty() && duplicate_positions.is_empty();

    let provenance = if resolved {
        Provenance::exact("moc3:binding.parameter.key_values")
    } else {
        Provenance::unknown(
            "moc3:binding.parameter",
            "parameter reference did not resolve; stored keys are preserved",
        )
    };

    rules.sort();
    rules.dedup();
    ParameterAxis {
        parameter,
        resolved,
        stored_key_count,
        keys,
        non_finite_key_indices: non_finite,
        usable,
        provenance,
        source_binding: draft.id.clone(),
        rules,
    }
}

fn render_positions(positions: &[usize]) -> String {
    const MAX_RENDERED: usize = 16;
    let mut rendered: Vec<String> = positions
        .iter()
        .take(MAX_RENDERED)
        .map(|position| position.to_string())
        .collect();
    if positions.len() > MAX_RENDERED {
        rendered.push(format!("... ({} more)", positions.len() - MAX_RENDERED));
    }
    rendered.join(", ")
}

/// Recompute the checked Cartesian cardinality of normalized axes.
///
/// Enforces [`crate::model::MAX_AXES_PER_BAND`] and
/// [`crate::model::MAX_AXIS_KEYS`]; violations become `LimitExceeded`, never
/// a truncated product (work order sections 11, 59).
pub fn checked_cardinality(axes: &[ParameterAxis]) -> crate::model::Cardinality {
    if axes.len() > crate::model::MAX_AXES_PER_BAND {
        return crate::model::Cardinality::LimitExceeded;
    }
    let mut product: u64 = 1;
    for axis in axes {
        if axis.stored_key_count > crate::model::MAX_AXIS_KEYS {
            return crate::model::Cardinality::LimitExceeded;
        }
        let count = axis.stored_key_count as u64;
        match product.checked_mul(count) {
            Some(value) => product = value,
            None => return crate::model::Cardinality::Overflow,
        }
    }
    if product > crate::model::MAX_GRID_CARDINALITY {
        return crate::model::Cardinality::LimitExceeded;
    }
    crate::model::Cardinality::Exact { value: product }
}
