//! Shrink measured single-line text, plus estimated unbreakable fixed-width
//! tokens, before a clipping/fixed-width ancestor falls back to cropping them.

use super::*;

const TEXT_FIT_EPS: f64 = 2.0;
const ESTIMATED_TEXT_FIT_RATIO: f64 = 1.04;

/// Collect font-size repairs for single-line text whose resolved width is
/// wider than the nearest clipping or width-constrained ancestor.
pub(super) fn collect_text_fit_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    let mut ancestors = Vec::new();
    collect_text_fit_fixes_in_tree(v, rects, cmds, &mut ancestors);
}

fn collect_text_fit_fixes_in_tree<'a>(
    v: &'a Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
    ancestors: &mut Vec<&'a Value>,
) {
    if v.get("type").and_then(Value::as_str) == Some("text") {
        collect_text_fit_command(v, ancestors, rects, cmds);
    }

    ancestors.push(v);
    for child in children(v) {
        collect_text_fit_fixes_in_tree(child, rects, cmds, ancestors);
    }
    ancestors.pop();
}

fn collect_text_fit_command(
    text: &Value,
    ancestors: &[&Value],
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    if ancestors
        .iter()
        .any(|node| is_status_bar_node(node) || is_horizontal_scroller(node))
        || is_status_bar_node(text)
    {
        return;
    }

    let Some(text_id) = text.get("id").and_then(Value::as_str) else {
        return;
    };
    let text_rect = rects.get(text_id);
    let Some(ancestor) = ancestors
        .iter()
        .rev()
        .copied()
        .find(|node| is_width_constraint(node))
    else {
        return;
    };
    let Some(ancestor_id) = ancestor.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(ancestor_rect) = rects.get(ancestor_id) else {
        return;
    };
    let available = (ancestor_rect.w - horizontal_padding(ancestor)).max(0.0);
    if !available.is_finite() {
        return;
    }

    // Jian measures an omitted font size as 14px; keep the same default when
    // the repair needs to materialize a smaller explicit size.
    let font_size = match text.get("fontSize") {
        None | Some(Value::Null) => 14.0,
        _ => num(text, "fontSize"),
    };
    if !font_size.is_finite() || font_size <= 0.0 {
        return;
    }

    if is_unbreakable_fixed_width_text(text) {
        let estimated_width = estimate_unbreakable_text_width(text, font_size);
        if !estimated_width.is_finite() || estimated_width <= available * ESTIMATED_TEXT_FIT_RATIO {
            return;
        }
        if let Some(new_font_size) = fitted_font_size(font_size, available, estimated_width) {
            cmds.push(EditorCommand::SetNodeFontSize {
                node_id: NodeId::new(text_id.to_string()),
                font_size: new_font_size as f32,
            });
        }
        return;
    }

    if !is_single_line_text(text)
        || !text_rect
            .map(|rect| rect.w.is_finite() && rect.w > available + TEXT_FIT_EPS)
            .unwrap_or(false)
    {
        return;
    }
    let measured_width = text_rect.map(|rect| rect.w).unwrap_or_default();
    let minimum = if font_size >= 32.0 { 24.0 } else { 12.0 };
    // Do not issue a repair when even the allowed minimum would still be too
    // wide; that case needs a different structural or author-level decision.
    if minimum * measured_width / font_size > available {
        return;
    }

    let new_font_size = (font_size * available / measured_width)
        .floor()
        .max(minimum)
        .min(font_size);
    if new_font_size >= font_size {
        return;
    }

    cmds.push(EditorCommand::SetNodeFontSize {
        node_id: NodeId::new(text_id.to_string()),
        font_size: new_font_size as f32,
    });
}

fn is_unbreakable_fixed_width_text(v: &Value) -> bool {
    if !matches!(
        v.get("textGrowth").and_then(Value::as_str),
        Some("fixed-width" | "fixed-width-height")
    ) {
        return false;
    }
    let Some(content) = v.get("content").and_then(Value::as_str) else {
        return false;
    };
    !content.is_empty() && content.is_ascii() && !content.chars().any(char::is_whitespace)
}

fn estimate_unbreakable_text_width(text: &Value, font_size: f64) -> f64 {
    text.get("content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .chars()
        .map(|character| font_size * character_em(character))
        .sum()
}

fn character_em(character: char) -> f64 {
    if character.is_ascii_digit() {
        0.62
    } else if matches!(character, ',' | '.') {
        0.30
    } else if character.is_ascii_uppercase() {
        0.68
    } else if character.is_ascii_lowercase() {
        0.55
    } else {
        0.35
    }
}

fn fitted_font_size(font_size: f64, available: f64, measured_width: f64) -> Option<f64> {
    let minimum = if font_size >= 32.0 { 24.0 } else { 12.0 };
    if minimum * measured_width / font_size > available {
        return None;
    }
    let new_font_size = (font_size * available / measured_width)
        .floor()
        .max(minimum)
        .min(font_size);
    (new_font_size < font_size).then_some(new_font_size)
}

fn is_single_line_text(v: &Value) -> bool {
    match v.get("textGrowth") {
        None | Some(Value::Null) => true,
        Some(Value::String(value)) => value == "auto",
        _ => false,
    }
}

fn is_status_bar_node(v: &Value) -> bool {
    crate::cleanup::is_status_bar_from_json(v)
}

fn is_horizontal_scroller(v: &Value) -> bool {
    layout_str(v) == Some("horizontal")
        && v.get("clipContent").and_then(Value::as_bool) == Some(true)
}

fn is_width_constraint(v: &Value) -> bool {
    v.get("clipContent").and_then(Value::as_bool) == Some(true)
        || fixed_width(v).is_some()
        || v.get("width").and_then(Value::as_str) == Some("fill_container")
}

/// Apply the text-fit repairs for one cleanup root using the current resolved
/// layout. The caller checkpoints immediately afterward under `text-fit`.
pub(crate) fn repair_text_fit(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let rects = resolved_rects(sink.state());
    let cmds = {
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.to_string()),
        ) else {
            return 0;
        };
        let Ok(v) = serde_json::to_value(root) else {
            return 0;
        };
        let mut cmds = Vec::new();
        collect_text_fit_fixes(&v, &rects, &mut cmds);
        cmds
    };
    let mut applied = 0;
    for cmd in cmds {
        if sink.apply(cmd) {
            applied += 1;
        }
    }
    applied
}

#[cfg(test)]
#[path = "text_fit_tests.rs"]
mod tests;
