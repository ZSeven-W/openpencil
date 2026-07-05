use crate::plan::OrchestratorPlan;
use op_ai_skills::resolve_style::{resolve_style, Fonts, ResolveOutcome, Shadow, StyleParams};
use std::collections::BTreeMap;

fn format_design_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}

fn default_resolved_style_params() -> StyleParams {
    StyleParams {
        color_palette: "Alloy Blue".to_string(),
        roundness: "medium".to_string(),
        elevation: "low".to_string(),
        fonts: Fonts {
            headings: String::new(),
            body: String::new(),
            captions: String::new(),
            data: String::new(),
        },
        decorative_imagery: None,
    }
}

pub(crate) fn build_resolved_style_instruction_for_plan(plan: &OrchestratorPlan) -> Option<String> {
    build_resolved_style_instruction(
        plan.style_guide_name.as_deref()?,
        &default_resolved_style_params(),
    )
}

/// Build the concrete-token style reference block for the new OpenPencil
/// style catalog. This is prompt text only: v1 bakes values into authored
/// nodes and never creates document variables.
pub fn build_resolved_style_instruction(name: &str, params: &StyleParams) -> Option<String> {
    let guide = match resolve_style(name, params) {
        ResolveOutcome::Hit(guide) => guide,
        ResolveOutcome::Miss { .. } => return None,
    };
    let tokens = &guide.tokens;
    let name = name.trim();
    let palette = params.color_palette.trim();

    Some(
        [
            format!("RESOLVED STYLE REFERENCE ({name} / {palette}):"),
            "Authoring rule: Bake these reference values directly into node fills, text colors, border stroke colors, cornerRadius, effect shadows, and font fields. Do NOT create document variables. Do NOT call set_variables. Author concrete values only.".to_string(),
            "StyleGuide prose:".to_string(),
            guide.prose.trim().to_string(),
            "Reference tokens:".to_string(),
            format_string_tokens("surface", &tokens.surface),
            format_string_tokens("foreground", &tokens.foreground),
            format_string_tokens("accent", &tokens.accent),
            format_string_tokens("border", &tokens.border),
            format_number_tokens("rounded", &tokens.rounded, "px"),
            format_shadow_tokens(&tokens.shadow),
            format!(
                "typography: headings={}, body={}, captions={}, data={}",
                tokens.typography.headings,
                tokens.typography.body,
                tokens.typography.captions,
                tokens.typography.data
            ),
            format_string_tokens("on", &tokens.on),
        ]
        .join("\n"),
    )
}

fn format_string_tokens(prefix: &str, values: &BTreeMap<String, String>) -> String {
    let tokens = values
        .iter()
        .map(|(role, value)| format!("{prefix}.{role}={value}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{prefix}: {tokens}")
}

fn format_number_tokens(prefix: &str, values: &BTreeMap<String, f64>, unit: &str) -> String {
    let tokens = values
        .iter()
        .map(|(role, value)| format!("{prefix}.{role}={}{}", format_design_number(*value), unit))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{prefix}: {tokens}")
}

fn format_shadow_tokens(values: &BTreeMap<String, Shadow>) -> String {
    let tokens = values
        .iter()
        .map(|(role, shadow)| {
            format!(
                "shadow.{role}={} {} offset({}px,{}px) blur={}px",
                shadow.shadow_type,
                shadow.color,
                format_design_number(shadow.offset_x),
                format_design_number(shadow.offset_y),
                format_design_number(shadow.blur)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("shadow: {tokens}")
}
