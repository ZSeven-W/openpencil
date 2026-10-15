//! Reflow only owned map-template geometry after the final viewport is known.
//!
//! Generic overflow repairs move individual rectangles. A resized map instead
//! needs its street grid, paths and endpoint markers to use one coordinate
//! system. Ownership markers keep authored overlays and edited styles intact.

use crate::image_fallback_policy::ResolvedRect;
use crate::map_templates::{children, MapTemplate};
use crate::role_defaults::Theme;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{walkers, EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};
use std::collections::HashMap;

pub(super) fn settle(sink: &mut dyn DocSink, root_ids: &[String]) {
    let rects = crate::image_fallback_policy::resolved_rects(sink.state());
    let mut patches = Vec::new();
    for id in root_ids {
        if let Some(root) = walkers::find_node(sink.state().active_children(), &NodeId::new(id)) {
            collect(root, &rects, &mut patches);
        }
    }
    for (id, patch) in patches {
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(id),
            patch_json: patch.to_string(),
            page_id: None,
        });
    }
}

fn collect(
    node: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    patches: &mut Vec<(String, Value)>,
) {
    if let Some((kind, old_width, old_height)) = template_viewport(node) {
        if let Some(rect) = rects.get(node.id_str()) {
            let (w, h) = (rect.width, rect.height);
            if w.is_finite()
                && h.is_finite()
                && w > 0.0
                && h > 0.0
                && ((w - old_width).abs() > 0.01 || (h - old_height).abs() > 0.01)
            {
                // Colors are not copied: generated geometry is theme-independent.
                let expected = children(node.id_str(), w, h, Theme::Light, kind);
                for part in node.children().into_iter().flatten() {
                    let Some(name) = part.base().name.as_deref() else {
                        continue;
                    };
                    if part
                        .base()
                        .explain
                        .as_deref()
                        .and_then(|value| value.strip_prefix("map-template-part:"))
                        != Some(name)
                    {
                        continue;
                    }
                    if let Some(target) = expected.iter().find(|value| value["name"] == name) {
                        collect_part_geometry(part, target, patches);
                    }
                }
                let explain = node.base().explain.as_deref().unwrap_or_default();
                let prefix = explain.split("; viewport:").next().unwrap_or(explain);
                patches.push((
                    node.id_str().to_string(),
                    json!({"explain": format!("{prefix}; viewport:{w}x{h}")}),
                ));
            }
        }
        return;
    }
    for child in node.children().into_iter().flatten() {
        collect(child, rects, patches);
    }
}

fn template_viewport(node: &PenNode) -> Option<(MapTemplate, f64, f64)> {
    if node.base().role.as_deref() != Some("map") {
        return None;
    }
    let explain = node.base().explain.as_deref()?;
    let (prefix, viewport) = explain.split_once("; viewport:")?;
    let kind = match prefix.split(';').next()? {
        "map-template:running" => MapTemplate::Running,
        "map-template:delivery" => MapTemplate::Delivery,
        "map-template:location" => MapTemplate::Location,
        _ => return None,
    };
    let (w, h) = viewport.split_once('x')?;
    Some((kind, w.parse().ok()?, h.parse().ok()?))
}

fn collect_part_geometry(node: &PenNode, target: &Value, patches: &mut Vec<(String, Value)>) {
    let Ok(current) = serde_json::to_value(node) else {
        return;
    };
    if current["type"] != target["type"] {
        return;
    }
    let mut patch = serde_json::Map::new();
    for key in ["x", "y", "width", "height", "d"] {
        if let Some(value) = target.get(key) {
            if current.get(key) != Some(value) {
                patch.insert(key.into(), value.clone());
            }
        }
    }
    if !patch.is_empty() {
        patches.push((node.id_str().to_string(), Value::Object(patch)));
    }
    for child in node.children().into_iter().flatten() {
        if let Some(target) = target["children"].as_array().and_then(|targets| {
            targets
                .iter()
                .find(|target| child.base().name.as_deref() == target["name"].as_str())
        }) {
            collect_part_geometry(child, target, patches);
        }
    }
}

#[cfg(test)]
#[path = "map_template_geometry_tests.rs"]
mod tests;
