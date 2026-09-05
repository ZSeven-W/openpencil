//! Shrink measured single-line text before a clipping/fixed-width ancestor
//! falls back to cropping it.

use super::*;

const TEXT_FIT_EPS: f64 = 2.0;

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
    if !is_single_line_text(text)
        || ancestors
            .iter()
            .any(|node| is_status_bar_node(node) || is_horizontal_scroller(node))
        || is_status_bar_node(text)
    {
        return;
    }

    let Some(text_id) = text.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(text_rect) = rects.get(text_id) else {
        return;
    };
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
    if !text_rect.w.is_finite() || !available.is_finite() || text_rect.w <= available + TEXT_FIT_EPS
    {
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
    let minimum = if font_size >= 32.0 { 24.0 } else { 12.0 };
    // Do not issue a repair when even the allowed minimum would still be too
    // wide; that case needs a different structural or author-level decision.
    if minimum * text_rect.w / font_size > available {
        return;
    }

    let new_font_size = (font_size * available / text_rect.w)
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
