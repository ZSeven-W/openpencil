use std::collections::HashMap;

use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::Value;

use crate::types::DocSink;

pub fn remove_empty_decorated_stubs(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let Some(root) = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    ) else {
        return false;
    };
    let Ok(v) = serde_json::to_value(root) else {
        return false;
    };
    let sizes = resolved_sizes(sink.state());
    let mut ids = Vec::new();
    collect_stub_ids(&v, None, &sizes, &mut ids);
    if ids.is_empty() {
        return false;
    }
    for id in ids {
        sink.apply(EditorCommand::DeleteNode {
            node_id: NodeId::new(id),
            page_id: None,
        });
    }
    true
}

pub(crate) fn empty_decorated_stub_diagnostic(
    v: &Value,
    parent: Option<&Value>,
    resolved_size: Option<(f64, f64)>,
) -> Option<String> {
    if is_empty_decorated_stub(v, parent, resolved_size) {
        Some(format!(
            "{}: empty decorated frame — fill in its content or remove it",
            diag_label(v)
        ))
    } else {
        None
    }
}

fn collect_stub_ids(
    v: &Value,
    parent: Option<&Value>,
    sizes: &HashMap<String, (f64, f64)>,
    ids: &mut Vec<String>,
) {
    let resolved_size = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| sizes.get(id))
        .copied();
    if is_empty_decorated_stub(v, parent, resolved_size) {
        if let Some(id) = v.get("id").and_then(Value::as_str) {
            ids.push(id.to_string());
        }
        return;
    }
    for child in children(v) {
        collect_stub_ids(child, Some(v), sizes, ids);
    }
}

fn is_empty_decorated_stub(
    v: &Value,
    parent: Option<&Value>,
    resolved_size: Option<(f64, f64)>,
) -> bool {
    let width = numeric(v, "width").or_else(|| resolved_size.map(|size| size.0));
    let height = numeric(v, "height").or_else(|| resolved_size.map(|size| size.1));
    v.get("type").and_then(Value::as_str) == Some("frame")
        && v.get("visible").and_then(Value::as_bool) != Some(false)
        && v.get("role").and_then(Value::as_str) != Some("icon-button")
        // An authored x/y is a deliberate OVERLAY (a notification dot pinned
        // on its bell — the dot-adoption pass creates exactly this shape),
        // not an abandoned container. Positioned nodes are never stubs.
        && v.get("x").is_none()
        && v.get("y").is_none()
        && children(v).is_empty()
        // A frame whose paint is an image is already meaningful content.
        // Compact avatar/photo frames are commonly authored this way, with
        // no child image node, so treating them as empty stubs destroys the
        // asset before the later avatar-repair pass can inspect it.
        && !is_meaningful_media_leaf(v)
        // A painted state/presence dot is itself the content. Weak models
        // often emit it as a tiny childless frame rather than an ellipse; once
        // the radius pass makes it circular it must not be mistaken for an
        // abandoned badge shell.
        && !has_state_indicator_semantics(v)
        && !is_circle_icon_backdrop(v, parent)
        && has_visible_paint(v)
        && (padding_positive(v) || numeric(v, "cornerRadius").is_some_and(|r| r > 0.0))
        && width.is_some_and(|w| w > 0.0 && w < 80.0)
        && height.is_some_and(|h| h > 0.0 && h < 60.0)
}

/// A sibling icon can supply the content of an absolute circular backdrop.
/// Keep this separate from empty button shells: require a smaller, visible
/// glyph inside the circle's bounds, without inventing its missing centering.
fn is_circle_icon_backdrop(v: &Value, parent: Option<&Value>) -> bool {
    let Some(parent) = parent else {
        return false;
    };
    if parent.get("layout").and_then(Value::as_str) != Some("none")
        || !crate::tree_heuristics::is_compact_solid_circle(v)
    {
        return false;
    }
    let size = numeric(v, "width").unwrap_or_default();
    let x = numeric(v, "x").unwrap_or_default();
    let y = numeric(v, "y").unwrap_or_default();
    children(parent).iter().any(|icon| {
        let (Some(w), Some(h)) = (numeric(icon, "width"), numeric(icon, "height")) else {
            return false;
        };
        let ix = numeric(icon, "x").unwrap_or_default();
        let iy = numeric(icon, "y").unwrap_or_default();
        icon.get("type").and_then(Value::as_str) == Some("icon_font")
            && icon.get("visible").and_then(Value::as_bool) != Some(false)
            && numeric(icon, "opacity").is_none_or(|opacity| opacity > 0.0)
            && icon
                .get("iconFontName")
                .and_then(Value::as_str)
                .is_some_and(|name| !name.trim().is_empty())
            && crate::tree_heuristics::has_visible_solid_fill(icon)
            && w > 0.0
            && h > 0.0
            && w < size
            && h < size
            && ix >= x
            && iy >= y
            && ix + w <= x + size
            && iy + h <= y + size
    })
}

fn is_meaningful_media_leaf(v: &Value) -> bool {
    has_image_fill(v) || has_avatar_semantics(v)
}

fn has_state_indicator_semantics(v: &Value) -> bool {
    let role = v
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        role.as_str(),
        "status-dot"
            | "status-indicator"
            | "presence-dot"
            | "presence-indicator"
            | "online-indicator"
            | "notification-dot"
    ) {
        return true;
    }

    let name = v
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    [
        "status dot",
        "status indicator",
        "active indicator",
        "presence dot",
        "presence indicator",
        "online dot",
        "online indicator",
        "notification dot",
        "状态点",
        "状态指示",
        "在线点",
        "在线指示",
    ]
    .iter()
    .any(|needle| name.contains(needle))
}

fn has_image_fill(v: &Value) -> bool {
    match v.get("fill") {
        Some(Value::Array(fills)) => fills.iter().any(is_image_paint),
        Some(fill) => is_image_paint(fill),
        None => false,
    }
}

fn has_avatar_semantics(v: &Value) -> bool {
    let role = v.get("role").and_then(Value::as_str).unwrap_or_default();
    if matches!(
        role.to_ascii_lowercase().as_str(),
        "avatar" | "user-avatar" | "profile-avatar" | "profile-photo"
    ) {
        return true;
    }

    let name = v
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.contains("avatar") || name.contains("profile photo") || name.contains("user photo")
}

fn is_image_paint(paint: &Value) -> bool {
    paint
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind.eq_ignore_ascii_case("image"))
}

fn resolved_sizes(state: &op_editor_core::EditorState) -> HashMap<String, (f64, f64)> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let mut out = HashMap::new();
    if let Some(page) = scene.active_page() {
        collect_sizes(&page.children, &mut out);
    }
    out
}

fn collect_sizes(nodes: &[SceneNode], out: &mut HashMap<String, (f64, f64)>) {
    for node in nodes {
        let bounds = node.aggregate_bounds();
        out.insert(
            node.id.clone(),
            (f64::from(bounds.size.x), f64::from(bounds.size.y)),
        );
        collect_sizes(&node.children, out);
    }
}

fn has_visible_paint(v: &Value) -> bool {
    match v.get("fill") {
        Some(Value::Array(a)) if !a.is_empty() => true,
        Some(Value::String(s)) if !s.trim().is_empty() => true,
        Some(other) if !other.is_null() && !matches!(other, Value::Array(_)) => true,
        _ => stroke_visible(v),
    }
}

fn stroke_visible(v: &Value) -> bool {
    v.get("stroke").is_some_and(|stroke| match stroke {
        Value::Null => false,
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        Value::String(s) => !s.trim().is_empty(),
        _ => true,
    })
}

fn padding_positive(v: &Value) -> bool {
    match v.get("padding") {
        Some(Value::Number(n)) => n.as_f64().is_some_and(|n| n > 0.0),
        Some(Value::Array(a)) => a.iter().any(|p| p.as_f64().is_some_and(|n| n > 0.0)),
        _ => false,
    }
}

fn numeric(v: &Value, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn diag_label(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("frame");
    let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
    format!("{name} ({id})")
}

#[cfg(test)]
#[path = "stub_repair_circle_tests.rs"]
mod circle_tests;
