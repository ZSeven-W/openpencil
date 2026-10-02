//! Strip a stroke-token plate that paints light on a dark container.
//!
//! `$--border` / `$--input` / `$--ring` are stroke colours: a hairline grey
//! on the light theme, invisible as a plate on a light card and a white
//! sticker on a dark one. Measured, arena-m01 (Opus 5.5, 2026-10-01): the
//! store page's cart pill (`#21140F`, radius 30) wrapped its cart icon in a
//! 52×48 `layout: "none"` frame filled `$--border` — `#FFFFFF` in that
//! document — with square corners. The icon sat in a white square pasted on
//! the dark pill. The corpus holds ~30 hard-cornered stroke-token tiles and
//! every other one sits on a light surface, where the token reads as a soft
//! plate the author meant; only the dark-container case is a mistake nobody
//! could intend, so only that case is repaired: the fill goes, and what the
//! plate wrapped (here a dark disc and a badge) shows through.

use super::{find_root, is_status_bar_from_json};
use crate::types::DocSink;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::Value;

/// A plate must be at least this big on both sides; thinner stroke-token
/// fills are rules and dividers, which are the token's intended use.
const MIN_PLATE_SIDE: f64 = 24.0;
/// The plate's fill must resolve this light, and the container this dark.
const LIGHT_LUMINANCE: f64 = 0.85;
const DARK_LUMINANCE: f64 = 0.25;
/// A fill below this alpha is a tint, not a plate.
const OPAQUE_ALPHA: f64 = 0.9;
const STROKE_TOKENS: &[&str] = &["$--border", "$--input", "$--ring"];

struct Context<'a> {
    variables: &'a op_design_lint::node_util::Variables,
    theme: &'a op_design_lint::node_util::Theme,
}

pub(super) fn strip_light_plates_on_dark(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let Some(root) = find_root(sink.state(), root_id) else {
        return 0;
    };
    let Ok(root_value) = serde_json::to_value(root) else {
        return 0;
    };
    let document = {
        let mut document = sink.state().doc.clone();
        document.children = sink.state().active_children().to_vec();
        document.pages = None;
        document
    };
    let variables = document.variables.clone().unwrap_or_default();
    let theme = op_design_lint::node_util::default_theme(document.themes.as_ref());
    let context = Context {
        variables: &variables,
        theme: &theme,
    };
    let mut ids = Vec::new();
    collect(&root_value, None, &context, &mut ids);
    let mut stripped = 0;
    for id in ids {
        if sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(id),
            patch_json: r#"{"fill":[]}"#.to_string(),
            page_id: None,
        }) {
            stripped += 1;
        }
    }
    stripped
}

/// `(raw colour, luminance, alpha)` of the node's first solid fill.
fn solid_fill(node: &Value, context: &Context<'_>) -> Option<(String, f64, f64)> {
    let fill = node
        .get("fill")?
        .as_array()?
        .iter()
        .find(|fill| fill.get("type").and_then(Value::as_str) == Some("solid"))?;
    let raw = fill.get("color")?.as_str()?.trim().to_string();
    let resolved =
        op_design_lint::node_util::resolve_color_ref(&raw, context.variables, context.theme)?;
    let rgba = crate::text_contrast_repair::parse_color_rgba(&resolved)?;
    let fill_opacity = fill.get("opacity").and_then(Value::as_f64).unwrap_or(1.0);
    let alpha = f64::from(rgba[3]) / 255.0 * fill_opacity;
    Some((raw, crate::hero_bleed::relative_luminance(rgba), alpha))
}

fn collect(
    node: &Value,
    container_luminance: Option<f64>,
    context: &Context<'_>,
    ids: &mut Vec<String>,
) {
    if is_status_bar_from_json(node) {
        return;
    }
    let fill = solid_fill(node, context);
    let stripped = container_luminance.is_some_and(|dark| dark <= DARK_LUMINANCE)
        && is_light_plate(node, fill.as_ref());
    if stripped {
        if let Some(id) = node.get("id").and_then(Value::as_str) {
            ids.push(id.to_string());
        }
    }
    // A stripped plate no longer paints: its children sit on the container.
    let next = match &fill {
        Some((_, luminance, alpha)) if *alpha >= OPAQUE_ALPHA && !stripped => Some(*luminance),
        _ => container_luminance,
    };
    for child in node
        .get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        collect(child, next, context, ids);
    }
}

fn is_light_plate(node: &Value, fill: Option<&(String, f64, f64)>) -> bool {
    if !matches!(
        node.get("type").and_then(Value::as_str),
        Some("frame" | "rectangle")
    ) {
        return false;
    }
    let Some((raw, luminance, alpha)) = fill else {
        return false;
    };
    let lower = raw.to_ascii_lowercase();
    if !STROKE_TOKENS.iter().any(|token| lower == *token) {
        return false;
    }
    if *luminance < LIGHT_LUMINANCE || *alpha < OPAQUE_ALPHA {
        return false;
    }
    if node
        .get("cornerRadius")
        .and_then(Value::as_f64)
        .is_some_and(|radius| radius > 0.0)
    {
        return false;
    }
    let side = |key: &str| node.get(key).and_then(Value::as_f64);
    matches!((side("width"), side("height")), (Some(w), Some(h)) if w >= MIN_PLATE_SIDE && h >= MIN_PLATE_SIDE)
}

#[cfg(test)]
#[path = "cleanup_light_plate_on_dark_tests.rs"]
mod tests;
