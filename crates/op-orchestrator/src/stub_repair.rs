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
    collect_stub_ids(&v, &sizes, &mut ids);
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
    resolved_size: Option<(f64, f64)>,
) -> Option<String> {
    if is_empty_decorated_stub(v, resolved_size) {
        Some(format!(
            "{}: empty decorated frame — fill in its content or remove it",
            diag_label(v)
        ))
    } else {
        None
    }
}

fn collect_stub_ids(v: &Value, sizes: &HashMap<String, (f64, f64)>, ids: &mut Vec<String>) {
    let resolved_size = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| sizes.get(id))
        .copied();
    if is_empty_decorated_stub(v, resolved_size) {
        if let Some(id) = v.get("id").and_then(Value::as_str) {
            ids.push(id.to_string());
        }
        return;
    }
    for child in children(v) {
        collect_stub_ids(child, sizes, ids);
    }
}

fn is_empty_decorated_stub(v: &Value, resolved_size: Option<(f64, f64)>) -> bool {
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
        && has_visible_paint(v)
        && (padding_positive(v) || numeric(v, "cornerRadius").is_some_and(|r| r > 0.0))
        && width.is_some_and(|w| w > 0.0 && w < 80.0)
        && height.is_some_and(|h| h > 0.0 && h < 60.0)
}

fn resolved_sizes(state: &op_editor_core::EditorState) -> HashMap<String, (f64, f64)> {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let mut out = HashMap::new();
    for page in &scene.pages {
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
