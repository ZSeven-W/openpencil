//! Tree-walk helpers + small leaf utilities for `LayerPanel` —
//! extracted from `layer_panel.rs` so the spine stays under the
//! 800-line cap.

use crate::document::{Node, NodeId};
use crate::widgets::icons::Icon;

use super::layer_panel::LayerItem;

pub(super) fn walk(
    node: &Node,
    selected: NodeId,
    hovered: Option<NodeId>,
    depth: u8,
    out: &mut Vec<LayerItem>,
) {
    out.push(LayerItem {
        node_id: node.id,
        label: node.name.clone(),
        kind_label: node.kind.label().to_string(),
        icon: icon_for_kind(&node.kind),
        depth,
        selected: node.id == selected,
        has_children: !node.children.is_empty(),
        hidden: node.hidden,
        locked: node.locked,
        collapsed: node.collapsed,
        hovered: hovered == Some(node.id),
        is_container: matches!(
            node.kind,
            crate::document::NodeKind::Frame | crate::document::NodeKind::Group
        ),
        renaming: false,
    });
    // Collapsed nodes hide their subtree from the LayerPanel
    // (canvas paint / hit-test are unaffected — that's a
    // tree-view-only concern).
    if !node.collapsed {
        for child in &node.children {
            walk(child, selected, hovered, depth.saturating_add(1), out);
        }
    }
}

/// Variant of `walk` that skips `excluded`'s entire subtree. Used
/// by `from_document_with_drag_source` to mirror the post-commit
/// row layout while a drag-to-reorder is in flight.
pub(super) fn walk_excluding(
    node: &Node,
    selected: NodeId,
    hovered: Option<NodeId>,
    excluded: NodeId,
    depth: u8,
    out: &mut Vec<LayerItem>,
) {
    if node.id == excluded {
        return;
    }
    out.push(LayerItem {
        node_id: node.id,
        label: node.name.clone(),
        kind_label: node.kind.label().to_string(),
        icon: icon_for_kind(&node.kind),
        depth,
        selected: node.id == selected,
        has_children: !node.children.is_empty(),
        hidden: node.hidden,
        locked: node.locked,
        collapsed: node.collapsed,
        hovered: hovered == Some(node.id),
        is_container: matches!(
            node.kind,
            crate::document::NodeKind::Frame | crate::document::NodeKind::Group
        ),
        renaming: false,
    });
    if !node.collapsed {
        for child in &node.children {
            walk_excluding(
                child,
                selected,
                hovered,
                excluded,
                depth.saturating_add(1),
                out,
            );
        }
    }
}

pub(super) fn icon_for_kind(kind: &crate::document::NodeKind) -> Icon {
    use crate::document::NodeKind;
    match kind {
        NodeKind::Frame => Icon::Hash,
        NodeKind::Group => Icon::Square,
        NodeKind::Rect => Icon::Square,
        NodeKind::Ellipse => Icon::Circle,
        NodeKind::Polygon => Icon::Triangle,
        NodeKind::Line => Icon::Minus,
        NodeKind::Path => Icon::PenTool,
        NodeKind::Text => Icon::Type,
        NodeKind::Other(_) => Icon::Square,
    }
}
