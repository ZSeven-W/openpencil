//! Spend excess vertical whitespace before a requested fixed board clips content.
//! Typography, images, controls and business content are never scaled or removed.

use super::*;
use serde_json::Value;
#[path = "cleanup_fixed_control_rows.rs"]
mod control_rows;
#[path = "cleanup_fixed_mobile_scroll.rs"]
mod mobile_scroll;

pub(super) fn repair(sink: &mut dyn DocSink, root_id: &str) {
    // A phone is a viewport, not a poster. Preserve the authored rhythm and
    // scroll long content before spending its spacing to fit a single image.
    if mobile_scroll::repair(sink, root_id) {
        return;
    }
    if repair_bottom_room(sink, root_id) {
        return;
    }
    fit_spacing(sink, root_id, 0.0);
    mobile_scroll::repair(sink, root_id);
}

/// Reserve the closing inset on a clone before spending interior whitespace.
/// A failed attempt must not add padding that pushes business content offscreen.
fn repair_bottom_room(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let Some(gap) = crate::geometry_validation::fixed_mobile_bottom_gap(sink.state(), root_id)
    else {
        return false;
    };
    if gap >= 24.0 {
        return false;
    }
    let Some(root) = find_root(sink.state(), root_id) else {
        return false;
    };
    if root
        .children()
        .and_then(|children| children.last())
        .is_some_and(mobile_scroll::is_bottom_chrome)
    {
        return false;
    }
    let Ok(before) = serde_json::to_value(root) else {
        return false;
    };
    if before.get("padding").is_some_and(Value::is_string) {
        return false;
    }
    let mut inset = padding(&before);
    inset[2] = inset[2].max(28.0);
    let mut probe = sink.state().clone();
    let mut trial = crate::loop_finalize::StateDocSink { state: &mut probe };
    trial.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new(root_id),
        patch_json: serde_json::json!({"padding": inset}).to_string(),
        page_id: None,
    });
    if !fit_spacing(&mut trial, root_id, inset[2]) {
        return false;
    }
    if !crate::geometry_validation::fixed_mobile_bottom_gap(trial.state(), root_id)
        .is_some_and(|gap| gap >= 24.0)
    {
        return false;
    }
    let Some(root) = find_root(trial.state(), root_id) else {
        return false;
    };
    let Ok(after) = serde_json::to_value(root) else {
        return false;
    };
    apply_layout_changes(sink, &before, &after)
}

fn fit_spacing(sink: &mut dyn DocSink, root_id: &str, bottom_floor: f64) -> bool {
    for _ in 0..12 {
        let Some(root) = find_root(sink.state(), root_id) else {
            return false;
        };
        let Ok(mut value) = serde_json::to_value(root) else {
            return false;
        };
        let before = value.clone();
        let Some(height) = value.get("height").and_then(Value::as_f64) else {
            return false;
        };
        let width = value.get("width").and_then(Value::as_f64).unwrap_or(0.0);
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
        let Some(board) = scene.active_page().and_then(|p| p.find(root_id)) else {
            return false;
        };
        let bottom = flow_bottom(&value, board);
        let deficit = bottom - board.bounds.origin.y as f64 - height;
        if deficit <= 1.0 {
            return true;
        }
        if bottom_floor > 0.0 && control_rows::spend_blank_row_height(&mut value, board, deficit) {
            if !apply_layout_changes(sink, &before, &value) {
                return false;
            }
            continue;
        }
        let floor = if width <= 480.0 { 4.0 } else { 16.0 };
        let mut trial = value.clone();
        let slack = trim(&mut trial, floor, 1.0, false, bottom_floor);
        if slack < 1.0 {
            break;
        }
        let share = (deficit / slack).clamp(0.1, 0.5);
        trim(&mut value, floor, share, false, bottom_floor);
        value["height"] = serde_json::json!(height);
        if !apply_layout_changes(sink, &before, &value) {
            return false;
        }
    }
    false
}

pub(super) fn apply_layout_changes(sink: &mut dyn DocSink, before: &Value, after: &Value) -> bool {
    let mut applied = false;
    let mut patch = serde_json::Map::new();
    for key in ["height", "gap", "padding", "constraints", "x", "y"] {
        if before.get(key) != after.get(key) {
            if let Some(value) = after.get(key) {
                patch.insert(key.into(), value.clone());
            }
        }
    }
    if !patch.is_empty() {
        if let Some(id) = after.get("id").and_then(Value::as_str) {
            applied |= sink.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(id),
                patch_json: Value::Object(patch).to_string(),
                page_id: None,
            });
        }
    }
    if let (Some(old), Some(new)) = (
        before.get("children").and_then(Value::as_array),
        after.get("children").and_then(Value::as_array),
    ) {
        for (old, new) in old.iter().zip(new) {
            applied |= apply_layout_changes(sink, old, new);
        }
    }
    applied
}

fn flow_bottom(value: &Value, scene: &jian_scene::layout_scene::SceneNode) -> f64 {
    let mut bottom = scene.bounds.origin.y as f64;
    for child in value
        .get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if child.get("constraints").is_some() {
            continue;
        }
        if let Some(node) = child
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| scene.find(id))
        {
            bottom = bottom.max((node.bounds.origin.y + node.bounds.size.y) as f64);
            // Intentional scroll overflow must not trigger repeated spacing
            // compression when finalization is run again.
            if !is_scroll_viewport(child) {
                bottom = bottom.max(flow_bottom(child, node));
            }
        }
    }
    let padding = padding(value);
    bottom + padding[2]
}

fn is_scroll_viewport(node: &Value) -> bool {
    node.get("clipContent").and_then(Value::as_bool) == Some(true)
        && matches!(
            node.get("role").and_then(Value::as_str),
            Some("scroll" | "scroll-area" | "scroll-viewport" | "viewport")
        )
}

fn padding(node: &Value) -> [f64; 4] {
    match node.get("padding") {
        Some(Value::Number(n)) => [n.as_f64().unwrap_or(0.0); 4],
        Some(Value::Array(v)) if v.len() == 2 => [
            v[0].as_f64().unwrap_or(0.0),
            v[1].as_f64().unwrap_or(0.0),
            v[0].as_f64().unwrap_or(0.0),
            v[1].as_f64().unwrap_or(0.0),
        ],
        Some(Value::Array(v)) if v.len() == 4 => {
            std::array::from_fn(|i| v[i].as_f64().unwrap_or(0.0))
        }
        _ => [0.0; 4],
    }
}

fn trim(
    node: &mut Value,
    floor: f64,
    share: f64,
    in_vertical_flow: bool,
    bottom_floor: f64,
) -> f64 {
    if node.get("constraints").is_some() {
        return 0.0;
    }
    // App chrome and controls retain their touch targets and internal insets.
    let role = node.get("role").and_then(Value::as_str).unwrap_or("");
    if matches!(role, "status-bar" | "bottom-tab-bar" | "button" | "input") {
        return 0.0;
    }
    let mut slack = 0.0;
    let vertical = node.get("layout").and_then(Value::as_str) == Some("vertical");
    let layout = matches!(
        node.get("layout").and_then(Value::as_str),
        Some("vertical" | "horizontal")
    );
    if layout {
        // Token/expression insets are authored constraints, not numeric slack.
        // Preserve them even when an ancestor's gap can recover enough room.
        if !node.get("padding").is_some_and(Value::is_string) {
            let mut pad = padding(node);
            for i in [0, 2] {
                let minimum = if i == 2 {
                    floor.max(bottom_floor)
                } else {
                    floor
                };
                let excess = (pad[i] - minimum).max(0.0);
                slack += excess;
                pad[i] -= excess * share;
            }
            if node.get("padding").is_some() {
                node["padding"] = serde_json::json!(pad);
            }
        }
        if vertical {
            let gap = node.get("gap").and_then(Value::as_f64).unwrap_or(0.0);
            let excess = (gap - floor).max(0.0);
            let count = node
                .get("children")
                .and_then(Value::as_array)
                .map_or(0, |c| c.len().saturating_sub(1));
            slack += excess * count as f64;
            if excess > 0.0 {
                node["gap"] = serde_json::json!(gap - excess * share);
            }
        }
    }
    if in_vertical_flow && is_plain_spacer(node) {
        if let Some(height) = node.get("height").and_then(Value::as_f64) {
            let excess = (height - floor).max(0.0);
            slack += excess;
            node["height"] = serde_json::json!(height - excess * share);
        }
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            slack += trim(child, floor, share, vertical, 0.0);
        }
    }
    // Numeric heights in open flow wrappers are often stale layout estimates.
    // Release only the whitespace we just spent; the loader still grows the
    // open wrapper to its measured content floor. Clipped surfaces stay fixed.
    if vertical && node.get("clipContent").and_then(Value::as_bool) != Some(true) {
        if let Some(height) = node.get("height").and_then(Value::as_f64) {
            node["height"] = serde_json::json!((height - slack * share).max(0.0));
        }
    }
    slack
}

/// An empty, unpainted flow frame supplies only whitespace, regardless of
/// whether the generator named it in English, Chinese, or left it unnamed.
/// Painted, interactive, pinned and semantic surfaces retain their sizing.
fn is_plain_spacer(node: &Value) -> bool {
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let kind = node.get("type").and_then(Value::as_str);
    let spacer_kind = kind == Some("frame")
        || (kind == Some("rectangle") && (name.contains("spacer") || name.starts_with("gap-")));
    spacer_kind
        && node
            .get("children")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        && node
            .get("fill")
            .is_none_or(|fill| fill.as_array().is_some_and(Vec::is_empty))
        && node
            .get("effects")
            .is_none_or(|effects| effects.as_array().is_some_and(Vec::is_empty))
        && node.get("stroke").is_none()
        && node.get("rotation").and_then(Value::as_f64).unwrap_or(0.0) == 0.0
        && node.get("clipContent").and_then(Value::as_bool) != Some(true)
        && [
            "role",
            "events",
            "x",
            "y",
            "constraints",
            "position",
            "screen",
            "slot",
            "minHeight",
            "maxHeight",
        ]
        .iter()
        .all(|key| node.get(key).is_none_or(Value::is_null))
        && node.get("reusable").and_then(Value::as_bool) != Some(true)
}

#[cfg(test)]
#[path = "cleanup_fixed_board_spacing_tests.rs"]
mod tests;
