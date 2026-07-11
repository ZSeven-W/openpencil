//! Protect authored horizontal-scroll geometry from vision-validation fixes.

use jian_ops_schema::node::{container::LayoutMode, PenNode};
use op_editor_core::{EditorState, NodeId, PenNodeExt};

pub(super) fn should_skip_scroller_layout_fix(
    state: &EditorState,
    node_id: &NodeId,
    property: &str,
) -> bool {
    if !is_scroller_layout_property(property) {
        return false;
    }

    is_protected_scroller_region(state, node_id)
}

pub fn is_protected_scroller_region(state: &EditorState, node_id: &NodeId) -> bool {
    state
        .active_children()
        .iter()
        .any(|root| target_has_scroller_context(root, node_id.as_str(), false).unwrap_or(false))
}

fn is_scroller_layout_property(property: &str) -> bool {
    matches!(
        property,
        "width" | "height" | "padding" | "gap" | "alignItems" | "justifyContent"
    )
}

fn target_has_scroller_context(
    node: &PenNode,
    target_id: &str,
    inside_scroller: bool,
) -> Option<bool> {
    let inside_scroller = inside_scroller || is_intentional_horizontal_scroller(node);
    if node.id_str() == target_id {
        return Some(inside_scroller || subtree_contains_scroller(node));
    }

    for child in node.children().into_iter().flatten() {
        if let Some(protected) = target_has_scroller_context(child, target_id, inside_scroller) {
            return Some(protected);
        }
    }
    None
}

fn subtree_contains_scroller(node: &PenNode) -> bool {
    is_intentional_horizontal_scroller(node)
        || node
            .children()
            .is_some_and(|children| children.iter().any(subtree_contains_scroller))
}

fn is_intentional_horizontal_scroller(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(node) => {
            node.container.layout == Some(LayoutMode::Horizontal)
                && node.container.clip_content == Some(true)
        }
        PenNode::Group(node) => {
            node.container.layout == Some(LayoutMode::Horizontal)
                && node.container.clip_content == Some(true)
        }
        PenNode::Rectangle(node) => {
            node.container.layout == Some(LayoutMode::Horizontal)
                && node.container.clip_content == Some(true)
        }
        _ => false,
    }
}
