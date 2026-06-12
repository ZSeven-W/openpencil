//! Canvas-drag mutators: handle-resize (with descendant scaling) +
//! drag-end auto-layout reorder / reparent-to-page-root.
//!
//! Ports the TS behavior from `skia-interaction.ts` (`handleResizeMove`
//! / `handleDragEnd`), `drag-reparent-policy.ts`, and pen-core
//! `tree-utils.ts::scaleChildrenInPlace`. Split out of `mutators.rs`
//! to keep that file under the 800-line ceiling.

use crate::geometry::{own_bounds, DocRect};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers::{self, find_node_mut};
use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::text::TextGrowth;
use jian_ops_schema::node::PenNode;

/// Main axis of an auto-layout (flex) container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Vertical,
    Horizontal,
}

/// The container's explicit auto-layout direction, or `None` for
/// free-layout containers and leaf nodes.
pub fn auto_layout_direction(node: &PenNode) -> Option<FlexDirection> {
    let layout = match node {
        PenNode::Frame(n) => n.container.layout.as_ref(),
        PenNode::Group(n) => n.container.layout.as_ref(),
        PenNode::Rectangle(n) => n.container.layout.as_ref(),
        _ => None,
    }?;
    match layout {
        LayoutMode::Vertical => Some(FlexDirection::Vertical),
        LayoutMode::Horizontal => Some(FlexDirection::Horizontal),
        LayoutMode::None => None,
    }
}

/// TS `drag-reparent-policy.ts`: auto-reparenting a dragged child out
/// of its parent is surprising for frame/shape-style nodes — users
/// expect those to keep their parent while repositioning. Primitive
/// content nodes (text / image / icon / input) keep the legacy
/// "drag out to detach" behavior.
pub fn should_auto_reparent_outside_parent(node: &PenNode) -> bool {
    !matches!(
        node,
        PenNode::Frame(_)
            | PenNode::Group(_)
            | PenNode::Rectangle(_)
            | PenNode::Ellipse(_)
            | PenNode::Line(_)
            | PenNode::Polygon(_)
            | PenNode::Path(_)
            | PenNode::Ref(_)
    )
}

/// Immediate parent id of `target` anywhere in the forest, or `None`
/// when `target` is top-level (or missing).
pub fn parent_of(children: &[PenNode], target: &NodeId) -> Option<NodeId> {
    for child in children {
        if let Some(grand) = child.children() {
            if grand.iter().any(|c| c.id_str() == target.as_str()) {
                return NodeId::new_opt(child.id_str());
            }
            if let Some(found) = parent_of(grand, target) {
                return Some(found);
            }
        }
    }
    None
}

/// Recursively scale child relative positions and explicit pixel
/// sizes by `(sx, sy)` — a faithful port of pen-core
/// `tree-utils.ts::scaleChildrenInPlace`. Flex-flow children carry no
/// authored `x` / `y`, so only their sizes scale and the layout
/// engine reflows their positions, matching the TS semantics.
pub fn scale_children_in_place(children: &mut [PenNode], sx: f64, sy: f64) {
    for child in children.iter_mut() {
        {
            let base = child.base_mut();
            if let Some(x) = base.x {
                base.x = Some(x * sx);
            }
            if let Some(y) = base.y {
                base.y = Some(y * sy);
            }
        }
        if let Some(w) = child.width_px() {
            child.set_width_px(w * sx);
        }
        if let Some(h) = child.height_px() {
            child.set_height_px(h * sy);
        }
        if let Some(grand) = child.children_mut() {
            scale_children_in_place(grand, sx, sy);
        }
    }
}

impl EditorState {
    /// Overwrite the anchor node's axis-aligned bounds (doc-space
    /// `x`/`y` + `width`/`height`). No-op when the node is locked /
    /// hidden / missing.
    ///
    /// TS parity (`skia-interaction.ts:626-744` + pen-core
    /// `scaleChildrenInPlace`): resizing a container scales the whole
    /// subtree proportionally, and resizing an auto-grow text node
    /// pins it to `textGrowth: fixed-width` so the new width sticks.
    pub fn set_selected_bounds(&mut self, bounds: DocRect) {
        let sel = self.selection.anchor.clone();
        if !sel.is_real() || !self.is_editable(&sel) {
            return;
        }
        let is_flow_child = walkers::is_flow_child_of_flex(self.active_children(), &sel);
        if let Some(node) = find_node_mut(self.active_children_mut(), &sel) {
            // Only write a real rect — container nodes that derive
            // their size from children keep deriving it.
            let own = own_bounds(node);
            if own.w > 0.0 || own.h > 0.0 {
                if !is_flow_child {
                    node.base_mut().x = Some(bounds.x);
                    node.base_mut().y = Some(bounds.y);
                }
                node.set_width_px(bounds.w);
                node.set_height_px(bounds.h);
                if let PenNode::Text(text) = &mut *node {
                    if text.text_growth.is_none() {
                        text.text_growth = Some(TextGrowth::FixedWidth);
                    }
                }
                // Container resize carries the subtree with it. Each
                // incremental drag step scales current → next, so the
                // composition over the whole drag equals the total
                // scale TS commits once at drag end.
                let sx = if own.w > 0.0 { bounds.w / own.w } else { 1.0 };
                let sy = if own.h > 0.0 { bounds.h / own.h } else { 1.0 };
                if sx != 1.0 || sy != 1.0 {
                    if let Some(children) = node.children_mut() {
                        scale_children_in_place(children, sx, sy);
                    }
                }
            } else if !is_flow_child {
                node.base_mut().x = Some(bounds.x);
                node.base_mut().y = Some(bounds.y);
            }
        }
    }

    /// Move `child` to position `index` within `parent`'s children
    /// (index counted in the list WITHOUT the child — TS
    /// `moveNode(id, parentId, newIndex)` after the midpoint walk).
    /// True when the order actually changed.
    pub fn reorder_child_to_index(
        &mut self,
        parent: &NodeId,
        child: &NodeId,
        index: usize,
    ) -> bool {
        if !parent.is_real() || !child.is_real() || parent == child {
            return false;
        }
        if !self.is_editable(child) {
            return false;
        }
        let children = self.active_children_mut();
        let Some(parent_node) = find_node_mut(children, parent) else {
            return false;
        };
        let Some(siblings) = parent_node.children_mut() else {
            return false;
        };
        let Some(cur) = siblings.iter().position(|n| n.id_str() == child.as_str()) else {
            return false;
        };
        let node = siblings.remove(cur);
        let idx = index.min(siblings.len());
        siblings.insert(idx, node);
        cur != idx
    }

    /// Detach `id` from its parent and re-insert it as the FIRST
    /// top-level child of the active page, preserving its visual
    /// position via the absolute `(abs_x, abs_y)` the caller read off
    /// the layout scene. TS parity: `handleDragEnd`'s
    /// `updateNode({x, y}) + moveNode(id, null, 0)`.
    pub fn reparent_to_page_root(&mut self, id: &NodeId, abs_x: f64, abs_y: f64) -> bool {
        if !id.is_real() || !self.is_subtree_editable(id) {
            return false;
        }
        let children = self.active_children_mut();
        // Already top-level — nothing to reparent.
        if children.iter().any(|n| n.id_str() == id.as_str()) {
            return false;
        }
        let Some(mut node) = walkers::extract_node(children, id) else {
            return false;
        };
        {
            let base = node.base_mut();
            base.x = Some(abs_x);
            base.y = Some(abs_y);
        }
        children.insert(0, node);
        true
    }
}
