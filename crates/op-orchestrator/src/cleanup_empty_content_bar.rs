//! Contract-tier removal of blank, content-width accent bars.

use super::{find_root, is_status_bar_from_json};
use crate::types::DocSink;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::Value;

const EXCLUDED_NAME_PARTS: &[&str] = &[
    "divider",
    "separator",
    "spacer",
    "line",
    "rule",
    "progress",
    "track",
    "slider",
    "handle",
    "indicator",
    "skeleton",
    "placeholder",
    "dot",
    "pill",
];

struct EmptyBarContext<'a> {
    root_id: &'a str,
    root_width: f64,
    variables: &'a op_design_lint::node_util::Variables,
    theme: &'a op_design_lint::node_util::Theme,
}

/// Remove childless, painted bars that occupy a content column without
/// carrying content. The predicate is intentionally narrow: structural
/// decoration names, map descendants, status chrome, and anything with
/// children stay untouched.
pub(super) fn remove_empty_content_bars(sink: &mut dyn DocSink, root_id: &str) {
    let Some(root) = find_root(sink.state(), root_id) else {
        return;
    };
    let Ok(root_value) = serde_json::to_value(root) else {
        return;
    };
    let Some(root_width) = root_value.get("width").and_then(Value::as_f64) else {
        return;
    };
    let document = {
        let mut document = sink.state().doc.clone();
        document.children = sink.state().active_children().to_vec();
        document.pages = None;
        document
    };
    let variables = document.variables.clone().unwrap_or_default();
    let theme = op_design_lint::node_util::default_theme(document.themes.as_ref());
    let context = EmptyBarContext {
        root_id,
        root_width,
        variables: &variables,
        theme: &theme,
    };
    let mut ids = Vec::new();
    collect_candidates(&root_value, None, &[], &context, &mut ids);
    for id in ids {
        sink.apply(EditorCommand::DeleteNode {
            node_id: NodeId::new(id),
            page_id: None,
        });
    }
}

fn collect_candidates<'a>(
    node: &'a Value,
    parent: Option<&'a Value>,
    ancestors: &[&'a Value],
    context: &EmptyBarContext<'_>,
    ids: &mut Vec<String>,
) {
    if let Some(parent) = parent {
        if is_empty_content_bar(node, parent, ancestors, context) {
            if let Some(id) = node.get("id").and_then(Value::as_str) {
                ids.push(id.to_string());
            }
        }
    }
    let mut next_ancestors = ancestors.to_vec();
    next_ancestors.push(node);
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            collect_candidates(child, Some(node), &next_ancestors, context, ids);
        }
    }
}

fn is_empty_content_bar(
    node: &Value,
    parent: &Value,
    ancestors: &[&Value],
    context: &EmptyBarContext<'_>,
) -> bool {
    if !matches!(
        node.get("type").and_then(Value::as_str),
        Some("frame" | "rectangle")
    ) || node
        .get("children")
        .and_then(Value::as_array)
        .is_some_and(|children| !children.is_empty())
    {
        return false;
    }
    if node.get("id").and_then(Value::as_str) == Some(context.root_id)
        || parent.get("id").and_then(Value::as_str) == Some(context.root_id)
        || parent.get("layout").and_then(Value::as_str) != Some("vertical")
        || parent
            .get("children")
            .and_then(Value::as_array)
            .is_none_or(|children| children.len() < 3)
    {
        return false;
    }
    if is_status_bar_from_json(node)
        || ancestors
            .iter()
            .any(|ancestor| is_status_bar_from_json(ancestor))
        || ancestors.iter().any(|ancestor| has_map_name(ancestor))
        || has_map_name(node)
    {
        return false;
    }
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if EXCLUDED_NAME_PARTS.iter().any(|part| name.contains(part)) {
        return false;
    }
    if ["src", "imageSearchQuery", "imagePrompt"]
        .iter()
        .any(|key| node.get(*key).is_some())
    {
        return false;
    }
    let Some(height) = node.get("height").and_then(Value::as_f64) else {
        return false;
    };
    if !(40.0..=120.0).contains(&height) {
        return false;
    }
    let width_ok = node.get("width").and_then(Value::as_str) == Some("fill_container")
        || node
            .get("width")
            .and_then(Value::as_f64)
            .is_some_and(|width| width >= context.root_width * 0.6);
    width_ok && has_visible_solid_fill(node, context.variables, context.theme)
}

fn has_map_name(node: &Value) -> bool {
    node.get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| name.to_ascii_lowercase().contains("map"))
}

fn has_visible_solid_fill(
    node: &Value,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> bool {
    let node_opacity = node.get("opacity").and_then(Value::as_f64).unwrap_or(1.0);
    node.get("fill")
        .and_then(Value::as_array)
        .is_some_and(|fills| {
            fills.iter().any(|fill| {
                if fill.get("type").and_then(Value::as_str) != Some("solid") {
                    return false;
                }
                let Some(raw) = fill.get("color").and_then(Value::as_str) else {
                    return false;
                };
                let Some(resolved) =
                    op_design_lint::node_util::resolve_color_ref(raw, variables, theme)
                else {
                    return false;
                };
                let Some(rgba) = crate::text_contrast_repair::parse_color_rgba(&resolved) else {
                    return false;
                };
                let fill_opacity = fill.get("opacity").and_then(Value::as_f64).unwrap_or(1.0);
                f64::from(rgba[3]) / 255.0 * fill_opacity * node_opacity >= 0.5
            })
        })
}

#[cfg(test)]
#[path = "cleanup_empty_content_bar_tests.rs"]
mod tests;
