//! GPU-cost budget for SkSL shader fills.
//!
//! The shader fill path is shipped end to end (`ShaderFillBody` in
//! `jian-ops-schema/src/style.rs` → `op-pen-loader` → `DrawOp::ShaderRect` →
//! both backends) and its FAILURE behaviour is already contracted: a shader
//! that fails to compile degrades to a visible solid fill and never panics.
//!
//! What has no contract at all is a shader that compiles and is simply too
//! expensive. Every shader fill is a full fragment pass over its node's rect,
//! so a generated document can stack full-screen shaders until the frame
//! budget is gone, and nothing upstream objects. These detectors are that
//! missing objection.
//!
//! **Two kinds of finding, deliberately separated.**
//!
//! - `ShaderInvalid` (Warning): the renderer cannot honour this fill — a
//!   uniform arity SkSL does not have, or source past the size bound. It
//!   degrades to a flat colour at paint time, so what ships is not the design
//!   that was authored. That is a generation defect, and callers that gate on
//!   severity should see it.
//! - `ShaderBudget` (Info): the fill renders, it is just expensive. Dropping a
//!   visual effect is a design decision, not a repair, so this stays advisory
//!   and carries no `suggested_value`.
//!
//! Neither offers an auto-fix: there is no safe machine edit for "this shader
//! costs too much" or "this uniform is the wrong shape".
//!
//! Thresholds are intentionally loose — they exist to catch the pathological
//! case (a pasted shader-toy, a screen tiled with full-bleed passes), not to
//! police craft. Anything a person would plausibly author on purpose passes.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenFill, ShaderFillBody, ShaderUniformValue};
use op_util::shader_preset::{
    resolve_num_octaves, OctaveResolution, DEFAULT_NUM_OCTAVES, MAX_NUM_OCTAVES, MIN_NUM_OCTAVES,
};
use serde_json::Value;

use crate::design_form::DesignForm;
use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{children, node_fills, node_id, numeric_height, numeric_width};

/// Largest SkSL source we consider authored rather than pasted. Comfortably
/// above a hand-written mesh-gradient or aurora shader; well below a
/// shader-toy scene dropped in whole. Source is stored RAW in the document,
/// so this also bounds what a shader costs the `.op` file.
const MAX_SKSL_CHARS: usize = 8_192;

/// Uniform count past which a fill is doing something other than being a
/// fill. Real parameterised gradients land in the low single digits.
const MAX_UNIFORMS: usize = 16;

/// SkSL has no vec1/vec5+; `RuntimeShaderBuilder` rejects a mismatched arity
/// at paint time and the fill degrades. Catching it here turns a silent
/// degrade into a diagnosis.
const VALID_VEC_ARITY: [usize; 3] = [2, 3, 4];

/// Share of the root's area past which a shader counts as "full-bleed" —
/// the expensive kind, because it is a fragment pass over most of the screen.
const FULL_BLEED_AREA_SHARE: f32 = 0.5;

/// How many full-bleed shader passes one screen may carry before the budget
/// is called. Mobile is stricter because it is where the GPU headroom is not:
/// the recipe scan found no mobile-specific "cool effect" form at all, so a
/// phone screen stacking full-bleed passes is buying nothing for the cost.
fn full_bleed_budget(form: DesignForm) -> usize {
    match form {
        DesignForm::MobileScreen => 2,
        _ => 4,
    }
}

/// Run the shader budget over one root.
pub fn detect_shader_budget(root: &PenNode, form: DesignForm) -> Vec<Issue> {
    let mut issues = Vec::new();
    let root_area = node_area(root);
    let mut full_bleed = Vec::new();
    walk(root, root_area, &mut issues, &mut full_bleed);

    let budget = full_bleed_budget(form);
    if full_bleed.len() > budget {
        // Reported on the LAST one over the line rather than on all of them:
        // the first `budget` passes are within contract, and blaming a node
        // that is individually fine reads as noise.
        let over = full_bleed.len();
        for node_id in full_bleed.into_iter().skip(budget) {
            issues.push(Issue {
                node_id,
                category: IssueCategory::ShaderBudget,
                severity: IssueSeverity::Info,
                property: FixProperty::Fill,
                current_value: Value::from(over),
                suggested_value: Value::Null,
                reason: format!(
                    "this screen carries {over} full-bleed shader fills; each is a fragment pass \
                     over most of the surface and the budget for this form is {budget}"
                ),
            });
        }
    }
    issues
}

fn walk(node: &PenNode, root_area: f32, issues: &mut Vec<Issue>, full_bleed: &mut Vec<String>) {
    if let Some(shader) = shader_fill(node) {
        let id = node_id(node).to_string();
        issues.extend(shader_issues(&id, shader));
        if root_area > 0.0 && node_area(node) / root_area >= FULL_BLEED_AREA_SHARE {
            full_bleed.push(id);
        }
    }
    for child in children(node) {
        walk(child, root_area, issues, full_bleed);
    }
}

/// Per-shader checks that do not depend on the rest of the document.
fn shader_issues(node_id: &str, shader: &ShaderFillBody) -> Vec<Issue> {
    let mut issues = Vec::new();
    // These are faults, not costs: the fill will not render as authored.
    let info = |reason: String, current: Value| Issue {
        node_id: node_id.to_string(),
        category: IssueCategory::ShaderInvalid,
        severity: IssueSeverity::Warning,
        property: FixProperty::Fill,
        current_value: current,
        suggested_value: Value::Null,
        reason,
    };

    match shader.preset.as_deref() {
        Some("turbulence") => {
            if shader
                .sksl
                .as_deref()
                .is_some_and(|source| !source.trim().is_empty())
            {
                issues.push(info(
                    "shader preset `turbulence` takes precedence, so the non-empty authored SkSL is ignored"
                        .to_string(),
                    Value::from("turbulence"),
                ));
            }
            issues.extend(turbulence_issues(node_id, shader));
        }
        Some(preset) => {
            let has_authored_fallback = shader
                .sksl
                .as_deref()
                .is_some_and(|source| !source.trim().is_empty());
            let fallback = if has_authored_fallback {
                "the loader falls back to the non-empty authored SkSL"
            } else {
                "without non-empty authored SkSL this fill does not render"
            };
            issues.push(info(
                format!("unknown shader preset `{preset}`; {fallback}"),
                Value::from(preset),
            ));
        }
        None => {}
    }

    // The loader replaces both absent and authored source for the recognized
    // preset, so linting that ignored source would report a non-rendered cost.
    // Unknown presets are treated as absent by the loader and retain raw-SkSL
    // compatibility, so their source still follows the ordinary size check.
    if shader.preset.as_deref() != Some("turbulence") {
        if let Some(sksl) = shader.sksl.as_deref() {
            let chars = sksl.chars().count();
            if chars > MAX_SKSL_CHARS {
                issues.push(info(
                    format!(
                        "SkSL source is {chars} characters (limit {MAX_SKSL_CHARS}); the source is stored \
                         raw in the document, and a shader this large is usually pasted rather than authored"
                    ),
                    Value::from(chars),
                ));
            }
        }
    }

    if let Some(uniforms) = shader.uniforms.as_ref() {
        if uniforms.len() > MAX_UNIFORMS {
            issues.push(info(
                format!(
                    "shader declares {} uniforms (limit {MAX_UNIFORMS})",
                    uniforms.len()
                ),
                Value::from(uniforms.len()),
            ));
        }
        for (name, value) in uniforms {
            if let ShaderUniformValue::Vec(components) = value {
                if !VALID_VEC_ARITY.contains(&components.len()) {
                    issues.push(info(
                        format!(
                            "uniform `{name}` has {} components; SkSL takes vec2/vec3/vec4, so this \
                             fill would degrade to a solid colour at paint time",
                            components.len()
                        ),
                        Value::from(components.len()),
                    ));
                }
            }
        }
    }
    issues
}

fn turbulence_issues(node_id: &str, shader: &ShaderFillBody) -> Vec<Issue> {
    let Some(uniforms) = shader.uniforms.as_ref() else {
        return Vec::new();
    };
    let issue = |reason: String, current: Value| Issue {
        node_id: node_id.to_string(),
        category: IssueCategory::ShaderInvalid,
        severity: IssueSeverity::Warning,
        property: FixProperty::Fill,
        current_value: current,
        suggested_value: Value::Null,
        reason,
    };
    let mut issues = Vec::new();

    if let Some(value) = uniforms.get("numOctaves") {
        match value {
            ShaderUniformValue::Float(value) => match resolve_num_octaves(*value) {
                OctaveResolution::Exact(_) => {}
                OctaveResolution::Clamped(effective) => issues.push(issue(
                    format!(
                        "numOctaves {value} is outside the supported range {MIN_NUM_OCTAVES}..={MAX_NUM_OCTAVES}; the turbulence loader clamps it to {effective}"
                    ),
                    Value::from(*value),
                )),
                OctaveResolution::Defaulted => issues.push(issue(
                    format!(
                        "numOctaves {value} must be a finite integer; the turbulence loader falls back to the default {DEFAULT_NUM_OCTAVES}"
                    ),
                    Value::from(*value),
                )),
            },
            ShaderUniformValue::Vec(_) | ShaderUniformValue::Color(_) => issues.push(issue(
                format!(
                    "numOctaves must be a number; the turbulence loader falls back to the default {DEFAULT_NUM_OCTAVES}"
                ),
                Value::from("numOctaves"),
            )),
        }
    }

    if let Some(value) = uniforms.get("baseFrequency") {
        let is_non_positive = match value {
            ShaderUniformValue::Float(value) => *value <= 0.0,
            ShaderUniformValue::Vec(values) => values.iter().any(|value| *value <= 0.0),
            ShaderUniformValue::Color(_) => false,
        };
        if is_non_positive {
            issues.push(issue(
                "baseFrequency must stay positive on every axis; zero or negative values make turbulence degenerate to a constant"
                    .to_string(),
                Value::from("baseFrequency"),
            ));
        }
    }

    issues
}

/// The shader body of a node's first shader fill, if it has one.
fn shader_fill(node: &PenNode) -> Option<&ShaderFillBody> {
    node_fills(node)?.iter().find_map(|fill| match fill {
        PenFill::Shader(body) => Some(body),
        _ => None,
    })
}

/// A node's painted area, or `0` when either dimension is not a plain number
/// (a `fill`/`hug` node contributes no measurable share here, and treating an
/// unknown as zero keeps the budget from firing on something it cannot size).
fn node_area(node: &PenNode) -> f32 {
    let w = numeric_width(node).unwrap_or(0.0);
    let h = numeric_height(node).unwrap_or(0.0);
    (w * h).max(0.0) as f32
}

#[cfg(test)]
#[path = "shader_budget_tests.rs"]
mod tests;
