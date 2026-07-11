//! Protect chart marks from validator-added card borders.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, NodeId, PenNodeExt};

pub(super) fn should_skip_chart_stroke_fix(
    state: &EditorState,
    node_id: &NodeId,
    property: &str,
) -> bool {
    if !matches!(property, "strokeColor" | "strokeWidth") {
        return false;
    }

    state
        .active_children()
        .iter()
        .any(|root| target_is_chart_mark(root, node_id.as_str(), false).unwrap_or(false))
}

fn target_is_chart_mark(node: &PenNode, target_id: &str, in_chart_context: bool) -> Option<bool> {
    let in_chart_context = in_chart_context || is_chart_context_node(node);
    if node.id_str() == target_id {
        return Some(in_chart_context && is_chart_mark(node));
    }

    for child in node.children().into_iter().flatten() {
        if let Some(is_mark) = target_is_chart_mark(child, target_id, in_chart_context) {
            return Some(is_mark);
        }
    }
    None
}

fn is_chart_context_node(node: &PenNode) -> bool {
    let label = node_identity_label(node);
    [
        "chart",
        "graph",
        "histogram",
        "sparkline",
        "weekly bars",
        "activity bars",
        "图表",
        "柱状",
    ]
    .iter()
    .any(|needle| label.contains(needle))
}

fn is_chart_mark(node: &PenNode) -> bool {
    let label = node_identity_label(node);
    if ["card", "panel", "section", "sidebar", "container"]
        .iter()
        .any(|needle| label.contains(needle))
    {
        return false;
    }
    if is_chart_context_node(node) {
        return true;
    }
    let mark_name = label.contains("track")
        || label.contains("column")
        || label.contains('柱')
        || label
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|token| matches!(token, "bar" | "bars"));
    if mark_name {
        return true;
    }

    node.children().map_or(0, Vec::len) <= 3 && is_slender_mark(node)
}

fn is_slender_mark(node: &PenNode) -> bool {
    let (Some(width), Some(height)) = (node.width_px(), node.height_px()) else {
        return false;
    };
    width > 0.0
        && height > 0.0
        && ((width <= 110.0 && height >= width * 1.25)
            || (height <= 110.0 && width >= height * 1.25))
}

fn node_identity_label(node: &PenNode) -> String {
    let mut label = node.id_str().to_ascii_lowercase();
    for text in [node.base().role.as_deref(), node.base().name.as_deref()]
        .into_iter()
        .flatten()
    {
        label.push(' ');
        label.push_str(&text.to_ascii_lowercase());
    }
    label
}
