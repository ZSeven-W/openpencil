//! Raw-node command application — `InsertNode` / `UpdateNode` /
//! `DeleteNode` / `MoveNode` / `CopyNode` / `ReplaceNode` /
//! `BatchInsert`.
//!
//! shell-core's `mcp_apply.rs` built its flat `Node` struct directly;
//! `op-editor-core` operates on the canonical `jian_ops_schema::PenNode`
//! tree, so this module ports the build / find / detach / replace
//! helpers onto `PenNode`. Carved off `command_apply.rs` to keep both
//! files under the 800-line cap.
//!
//! Every helper preserves the pre-validate-then-mutate discipline: a
//! caller validates kind / geometry / hex / id space BEFORE any tree
//! write, so a bad arg never leaves the document half-mutated.

use crate::command::BatchInsertItem;
use crate::fills::set_primary_fill_hex;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;
use jian_ops_schema::node::{
    ContainerProps, EllipseNode, FrameNode, GroupNode, LineNode, PathNode, PenNode,
    PenNodeBase, PolygonNode, RectangleNode, TextContent, TextNode,
};
use jian_ops_schema::sizing::SizingBehavior;
use std::collections::HashSet;

/// Resolve an editor `kind` arg into a canonical-schema leaf node.
/// Accepts the same lowercase strings the read-side tools emit
/// (`frame` / `group` / `rect` / `ellipse` / `polygon` / `line` /
/// `text` / `path`). `None` for an unknown kind.
///
/// `width` / `height` write the variant's literal `SizingBehavior`;
/// `(x, y)` write `base`. Container kinds (`frame` / `group`) start
/// with an empty `children` so a follow-up `MoveNode` can reparent
/// into them.
pub fn build_leaf_node(
    kind: &str,
    id: &str,
    name: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Option<PenNode> {
    let base = PenNodeBase {
        id: id.to_string(),
        name: Some(name.to_string()),
        x: Some(x as f64),
        y: Some(y as f64),
        ..Default::default()
    };
    let w = SizingBehavior::Number(width.max(0) as f64);
    let h = SizingBehavior::Number(height.max(0) as f64);
    let node = match kind {
        "frame" => PenNode::Frame(FrameNode {
            base,
            container: ContainerProps {
                width: Some(w),
                height: Some(h),
                ..Default::default()
            },
            children: Some(Vec::new()),
            reusable: None,
            slot: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        "group" => PenNode::Group(GroupNode {
            base,
            container: ContainerProps {
                width: Some(w),
                height: Some(h),
                ..Default::default()
            },
            children: Some(Vec::new()),
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        "rect" => PenNode::Rectangle(RectangleNode {
            base,
            container: ContainerProps {
                width: Some(w),
                height: Some(h),
                ..Default::default()
            },
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        "ellipse" => PenNode::Ellipse(EllipseNode {
            base,
            width: Some(w),
            height: Some(h),
            corner_radius: None,
            inner_radius: None,
            start_angle: None,
            sweep_angle: None,
            fill: None,
            stroke: None,
            effects: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        "polygon" => PenNode::Polygon(PolygonNode {
            base,
            polygon_count: 3,
            width: Some(w),
            height: Some(h),
            corner_radius: None,
            fill: None,
            stroke: None,
            effects: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        "line" => PenNode::Line(LineNode {
            base,
            x2: Some((x + width) as f64),
            y2: Some((y + height) as f64),
            stroke: None,
            effects: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        "text" => PenNode::Text(TextNode {
            base,
            width: Some(w),
            height: Some(h),
            content: TextContent::Plain(name.to_string()),
            font_family: None,
            font_size: None,
            font_weight: None,
            font_style: None,
            letter_spacing: None,
            line_height: None,
            text_align: None,
            text_align_vertical: None,
            text_growth: None,
            underline: None,
            strikethrough: None,
            fill: None,
            effects: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        "path" => PenNode::Path(PathNode {
            base,
            icon_id: None,
            d: None,
            anchors: Some(Vec::new()),
            closed: None,
            width: Some(w),
            height: Some(h),
            fill: None,
            stroke: None,
            effects: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        _ => return None,
    };
    Some(node)
}

/// True when `kind` resolves to a buildable leaf node. Used by the
/// `BatchInsert` pre-validation pass.
pub fn kind_is_valid(kind: &str) -> bool {
    matches!(
        kind,
        "frame" | "group" | "rect" | "ellipse" | "polygon" | "line" | "text" | "path"
    )
}

/// Replace the node with `target` id with `replacement` at its current
/// slot, preserving sibling order. True on the first match.
pub fn replace_node_in_children(
    children: &mut [PenNode],
    target: &NodeId,
    replacement: &mut Option<PenNode>,
) -> bool {
    if let Some(idx) = children.iter().position(|n| n.id_str() == target.as_str()) {
        if let Some(r) = replacement.take() {
            children[idx] = r;
            return true;
        }
    }
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            if replace_node_in_children(grand, target, replacement) {
                return true;
            }
        }
    }
    false
}

impl EditorState {
    /// Compute the numeric seed for the next editor-minted `n{N}` id —
    /// `max_node_id() + 1`. `None` on `u64` exhaustion.
    pub(crate) fn next_node_id_seed(&self) -> Option<u64> {
        self.max_node_id().checked_add(1).map(|n| n.max(1))
    }

    /// Allocate one fresh `n{N}` id, skipping any candidate that
    /// collides with a live id. `None` on counter exhaustion.
    pub(crate) fn next_node_id(&self) -> Option<NodeId> {
        let mut seed = self.next_node_id_seed()?;
        let mut live = self.collect_node_ids();
        walkers::alloc_n_id(&mut seed, &mut live)
    }

    /// `InsertNode` — build + append a fresh leaf on the active page.
    pub(crate) fn cmd_insert_node(
        &mut self,
        kind: &str,
        name: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_hex: &Option<String>,
    ) -> bool {
        if !kind_is_valid(kind) || width < 0 || height < 0 {
            return false;
        }
        // Pre-validate hex BEFORE minting an id / mutating the tree.
        if let Some(hex) = fill_hex {
            if crate::color_picker::parse_hex_rgb(hex).is_none() {
                return false;
            }
        }
        let Some(new_id) = self.next_node_id() else {
            return false;
        };
        let Some(mut node) =
            build_leaf_node(kind, new_id.as_str(), name, x, y, width, height)
        else {
            return false;
        };
        if let Some(hex) = fill_hex {
            set_primary_fill_hex(&mut node, hex);
        }
        self.active_children_mut().push(node);
        true
    }

    /// `UpdateNode` — patch optional fields on an existing node.
    pub(crate) fn cmd_update_node(
        &mut self,
        node_id: &NodeId,
        x: Option<i32>,
        y: Option<i32>,
        width: Option<i32>,
        height: Option<i32>,
        name: &Option<String>,
        fill_hex: &Option<String>,
    ) -> bool {
        if !node_id.is_real() {
            return false;
        }
        // Pre-validate EVERY field before the mutable borrow + writes.
        if let Some(w) = width {
            if w < 0 {
                return false;
            }
        }
        if let Some(h) = height {
            if h < 0 {
                return false;
            }
        }
        if let Some(hex) = fill_hex {
            if crate::color_picker::parse_hex_rgb(hex).is_none() {
                return false;
            }
        }
        let Some(node) = walkers::find_node_mut(self.active_children_mut(), node_id)
        else {
            return false;
        };
        // All validation passed — every field applies atomically.
        if let Some(nx) = x {
            node.base_mut().x = Some(nx as f64);
        }
        if let Some(ny) = y {
            node.base_mut().y = Some(ny as f64);
        }
        if let Some(nw) = width {
            node.set_width_px(nw as f64);
        }
        if let Some(nh) = height {
            node.set_height_px(nh as f64);
        }
        if let Some(new_name) = name {
            node.base_mut().name = Some(new_name.clone());
        }
        if let Some(hex) = fill_hex {
            set_primary_fill_hex(node, hex);
        }
        true
    }

    /// `DeleteNode` — remove a node + descendants from the active page.
    pub(crate) fn cmd_delete_node(&mut self, node_id: &NodeId) -> bool {
        if !node_id.is_real() {
            return false;
        }
        walkers::remove_from_children(self.active_children_mut(), node_id)
    }

    /// `MoveNode` — reparent a node. A `NONE` target reparents to the
    /// active page root; a real target must resolve + must not create
    /// a cycle (target is a descendant of the moved node).
    pub(crate) fn cmd_move_node(
        &mut self,
        node_id: &NodeId,
        target_parent: &NodeId,
    ) -> bool {
        if !node_id.is_real() || target_parent == node_id {
            return false;
        }
        // Pre-validate EVERYTHING before detaching: a bad target would
        // detach → reattach-fail → silently drop the source.
        {
            let children = self.active_children();
            let Some(src) = walkers::find_node(children, node_id) else {
                return false;
            };
            if target_parent.is_real() {
                if walkers::descendant_contains(src, target_parent) {
                    return false;
                }
                if walkers::find_node(children, target_parent).is_none() {
                    return false;
                }
            }
        }
        let children = self.active_children_mut();
        let Some(detached) = walkers::extract_node(children, node_id) else {
            return false;
        };
        if target_parent.is_real() {
            walkers::append_into(children, target_parent, detached).is_ok()
        } else {
            children.push(detached);
            true
        }
    }

    /// `CopyNode` — deep-clone a node + subtree under a new parent
    /// (`NONE` = active page root). Fresh ids minted past the id space.
    pub(crate) fn cmd_copy_node(
        &mut self,
        node_id: &NodeId,
        target_parent: &NodeId,
    ) -> bool {
        if !node_id.is_real() {
            return false;
        }
        // Validate source + target up front.
        {
            let children = self.active_children();
            if walkers::find_node(children, node_id).is_none() {
                return false;
            }
            if target_parent.is_real()
                && walkers::find_node(children, target_parent).is_none()
            {
                return false;
            }
        }
        let Some(mut next_id) = self.next_node_id_seed() else {
            return false;
        };
        let mut taken = self.collect_node_ids();
        // Clone the owned subtree before re-borrowing the tree mutably.
        let clone = {
            let children = self.active_children();
            let src = walkers::find_node(children, node_id).expect("validated");
            walkers::deep_clone_with_new_ids(src, &mut next_id, &mut taken)
        };
        let children = self.active_children_mut();
        if target_parent.is_real() {
            walkers::append_into(children, target_parent, clone).is_ok()
        } else {
            children.push(clone);
            true
        }
    }

    /// `ReplaceNode` — swap an existing node for a freshly-built leaf at
    /// the same slot. The destructive-swap guard: replacing a node WITH
    /// children requires `drop_children == true`, else the swap is
    /// refused so a container can't silently lose its subtree.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cmd_replace_node(
        &mut self,
        node_id: &NodeId,
        kind: &str,
        name: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_hex: &Option<String>,
        drop_children: bool,
    ) -> bool {
        if !node_id.is_real() || !kind_is_valid(kind) || width < 0 || height < 0 {
            return false;
        }
        if let Some(hex) = fill_hex {
            if crate::color_picker::parse_hex_rgb(hex).is_none() {
                return false;
            }
        }
        // Resolve target + check the destructive-swap guard BEFORE
        // minting an id. A target WITH children needs explicit consent.
        {
            let children = self.active_children();
            let Some(target) = walkers::find_node(children, node_id) else {
                return false;
            };
            let has_children =
                target.children().map(|c| !c.is_empty()).unwrap_or(false);
            if has_children && !drop_children {
                return false;
            }
        }
        let Some(new_id) = self.next_node_id() else {
            return false;
        };
        let Some(mut replacement) =
            build_leaf_node(kind, new_id.as_str(), name, x, y, width, height)
        else {
            return false;
        };
        if let Some(hex) = fill_hex {
            set_primary_fill_hex(&mut replacement, hex);
        }
        let mut slot = Some(replacement);
        replace_node_in_children(self.active_children_mut(), node_id, &mut slot)
    }

    /// `BatchInsert` — insert N leaf nodes on the active page in one
    /// atomic shot. EVERY descriptor is validated before any mutation;
    /// a single bad entry rejects the whole batch.
    pub(crate) fn cmd_batch_insert(&mut self, items: &[BatchInsertItem]) -> bool {
        if items.is_empty() {
            return false;
        }
        // Pre-validate kinds + geometry + fill hex up front.
        for item in items {
            if !kind_is_valid(&item.kind) || item.width < 0 || item.height < 0 {
                return false;
            }
            if let Some(hex) = &item.fill_hex {
                if crate::color_picker::parse_hex_rgb(hex).is_none() {
                    return false;
                }
            }
        }
        // Allocate every fresh id up front; bail on id-space exhaustion.
        let Some(mut next_id) = self.next_node_id_seed() else {
            return false;
        };
        let mut live: HashSet<NodeId> = self.collect_node_ids();
        let mut ids: Vec<NodeId> = Vec::with_capacity(items.len());
        for _ in 0..items.len() {
            match walkers::alloc_n_id(&mut next_id, &mut live) {
                Some(id) => ids.push(id),
                None => return false,
            }
        }
        // All validation + allocation passed — now mutate.
        let children = self.active_children_mut();
        for (item, id) in items.iter().zip(ids) {
            // `kind` already validated, so `build_leaf_node` is Some.
            let mut node = build_leaf_node(
                &item.kind,
                id.as_str(),
                &item.name,
                item.x,
                item.y,
                item.width,
                item.height,
            )
            .expect("kind validated");
            if let Some(hex) = &item.fill_hex {
                set_primary_fill_hex(&mut node, hex);
            }
            children.push(node);
        }
        true
    }
}
