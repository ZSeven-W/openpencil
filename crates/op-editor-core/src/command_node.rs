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
    ContainerProps, EllipseNode, FrameNode, GroupNode, LineNode, PathNode, PenNode, PenNodeBase,
    PolygonNode, RectangleNode, TextContent, TextNode,
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
            image_search_query: None,
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

#[allow(clippy::result_large_err)]
fn insert_into_parent_or_root(
    children: &mut Vec<PenNode>,
    parent: &NodeId,
    node: PenNode,
    index: Option<usize>,
) -> Result<(), PenNode> {
    if !parent.is_real() {
        let idx = index.unwrap_or(children.len()).min(children.len());
        children.insert(idx, node);
        return Ok(());
    }
    insert_into_parent(children, parent, node, index)
}

#[allow(clippy::result_large_err)]
fn insert_into_parent(
    children: &mut [PenNode],
    parent: &NodeId,
    node: PenNode,
    index: Option<usize>,
) -> Result<(), PenNode> {
    if let Some(idx) = children.iter().position(|n| n.id_str() == parent.as_str()) {
        match children[idx].children_mut() {
            Some(grand) => {
                let insert_idx = index.unwrap_or(grand.len()).min(grand.len());
                grand.insert(insert_idx, node);
                return Ok(());
            }
            None => return Err(node),
        }
    }
    let mut carry = node;
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            match insert_into_parent(grand, parent, carry, index) {
                Ok(()) => return Ok(()),
                Err(returned) => carry = returned,
            }
        }
    }
    Err(carry)
}

impl EditorState {
    /// Compute the numeric seed for the next editor-minted `n{N}` id —
    /// `max_node_id() + 1`. `None` on `u64` exhaustion. Pub so the
    /// `batch_design` MCP tool can predict (off a doc snapshot) the ids the
    /// host will allocate at apply, mirroring `cmd_insert_subtree`'s seed.
    pub fn next_node_id_seed(&self) -> Option<u64> {
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
    // Args mirror the `InsertNode` command fields one-for-one; bundling
    // them into a struct would just shadow the DTO with no real gain.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cmd_insert_node(
        &mut self,
        kind: &str,
        name: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_hex: &Option<String>,
        target_parent: &NodeId,
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
        if target_parent.is_real() {
            match walkers::find_node(self.active_children(), target_parent) {
                Some(parent) if parent.is_container() => {}
                _ => return false,
            }
        }
        let Some(new_id) = self.next_node_id() else {
            return false;
        };
        let Some(mut node) = build_leaf_node(kind, new_id.as_str(), name, x, y, width, height)
        else {
            return false;
        };
        if let Some(hex) = fill_hex {
            set_primary_fill_hex(&mut node, hex);
        }
        if target_parent.is_real() {
            let root = self.active_children_mut();
            let Some(parent) = walkers::find_node_mut(root, target_parent) else {
                return false;
            };
            let Some(children) = parent.children_mut() else {
                return false;
            };
            children.push(node);
        } else {
            self.active_children_mut().push(node);
        }
        true
    }

    /// `UpdateNode` — patch optional fields on an existing node.
    // Args mirror the `UpdateNode` command fields one-for-one.
    #[allow(clippy::too_many_arguments)]
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
        let Some(node) = walkers::find_node_mut(self.active_children_mut(), node_id) else {
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

    /// `PatchNodeData` — TS-style shallow merge on a canonical PenNode.
    pub(crate) fn cmd_patch_node_data(&mut self, node_id: &NodeId, patch_json: &str) -> bool {
        if !node_id.is_real() {
            return false;
        }
        let Ok(serde_json::Value::Object(patch)) =
            serde_json::from_str::<serde_json::Value>(patch_json)
        else {
            return false;
        };
        let Some(current) = walkers::find_node(self.active_children(), node_id) else {
            return false;
        };
        let Ok(mut value) = serde_json::to_value(current) else {
            return false;
        };
        let Some(obj) = value.as_object_mut() else {
            return false;
        };
        for (key, value) in patch {
            obj.insert(key, value);
        }
        let Ok(replacement) = serde_json::from_value::<PenNode>(value) else {
            return false;
        };
        if replacement.id_str().is_empty() {
            return false;
        }
        let mut slot = Some(replacement);
        replace_node_in_children(self.active_children_mut(), node_id, &mut slot)
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
        index: Option<usize>,
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
                let Some(target) = walkers::find_node(children, target_parent) else {
                    return false;
                };
                if target.children().is_none() {
                    return false;
                }
            }
        }
        let children = self.active_children_mut();
        let Some(detached) = walkers::extract_node(children, node_id) else {
            return false;
        };
        insert_into_parent_or_root(children, target_parent, detached, index).is_ok()
    }

    /// `CopyNode` — deep-clone a node + subtree under a new parent
    /// (`NONE` = active page root). Fresh ids minted past the id space.
    pub(crate) fn cmd_copy_node(
        &mut self,
        node_id: &NodeId,
        target_parent: &NodeId,
        overrides_json: Option<&str>,
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
            if target_parent.is_real() {
                let Some(target) = walkers::find_node(children, target_parent) else {
                    return false;
                };
                if target.children().is_none() {
                    return false;
                }
            }
        }
        let Some(mut next_id) = self.next_node_id_seed() else {
            return false;
        };
        let mut taken = self.collect_node_ids();
        // Clone the owned subtree before re-borrowing the tree mutably.
        let mut clone = {
            let children = self.active_children();
            let src = walkers::find_node(children, node_id).expect("validated");
            walkers::deep_clone_with_new_ids(src, &mut next_id, &mut taken)
        };
        if !apply_copy_overrides(&mut clone, overrides_json) {
            return false;
        }
        let children = self.active_children_mut();
        insert_into_parent_or_root(children, target_parent, clone, None).is_ok()
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
            let has_children = target.children().map(|c| !c.is_empty()).unwrap_or(false);
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

    /// `ReplaceSubtree` — swap an existing node for a fully-authored
    /// canonical subtree. The destructive-swap guard matches
    /// `ReplaceNode`: replacing a node WITH children requires explicit
    /// opt-in.
    pub(crate) fn cmd_replace_subtree(
        &mut self,
        node_id: &NodeId,
        node: PenNode,
        drop_children: bool,
    ) -> bool {
        if !node_id.is_real() {
            return false;
        }
        {
            let children = self.active_children();
            let Some(target) = walkers::find_node(children, node_id) else {
                return false;
            };
            let has_children = target.children().map(|c| !c.is_empty()).unwrap_or(false);
            if has_children && !drop_children {
                return false;
            }
        }
        let Some(mut next_id) = self.next_node_id_seed() else {
            return false;
        };
        let mut taken = self.collect_node_ids();
        let mut nodes = vec![node];
        if !remap_subtree_ids(&mut nodes, &mut next_id, &mut taken) {
            return false;
        }
        let mut slot = nodes.pop();
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

    /// Insert one or more nested `PenNode` subtrees. `parent_id` of
    /// `NONE` appends to the active page root; otherwise the parent
    /// must exist and be a container variant. Every incoming node id
    /// (recursively) is remapped to a fresh editor id so an
    /// externally-authored subtree can't collide with live doc ids.
    pub(crate) fn cmd_insert_subtree(&mut self, nodes: Vec<PenNode>, parent_id: &NodeId) -> bool {
        if nodes.is_empty() {
            return false;
        }
        // Validate the parent up front (when not the page root).
        if parent_id.is_real() {
            match walkers::find_node(self.active_children(), parent_id) {
                Some(p) if p.is_container() => {}
                _ => return false, // missing or non-container
            }
        }
        // Allocate fresh ids for the whole incoming forest.
        let Some(mut next_id) = self.next_node_id_seed() else {
            return false;
        };
        let mut taken: HashSet<NodeId> = self.collect_node_ids();
        let mut nodes = nodes;
        if !remap_subtree_ids(&mut nodes, &mut next_id, &mut taken) {
            return false;
        }
        // All validation + allocation passed — now mutate.
        if parent_id.is_real() {
            let root = self.active_children_mut();
            let Some(parent) = walkers::find_node_mut(root, parent_id) else {
                return false;
            };
            let Some(slot) = parent.children_mut() else {
                return false;
            };
            slot.extend(nodes);
        } else {
            self.active_children_mut().extend(nodes);
        }
        true
    }
}

fn apply_copy_overrides(node: &mut PenNode, overrides_json: Option<&str>) -> bool {
    let Some(raw) = overrides_json else {
        return true;
    };
    let Ok(serde_json::Value::Object(mut overrides)) =
        serde_json::from_str::<serde_json::Value>(raw)
    else {
        return false;
    };
    overrides.remove("id");

    let Ok(mut node_value) = serde_json::to_value(&*node) else {
        return false;
    };
    let Some(node_object) = node_value.as_object_mut() else {
        return false;
    };
    for (key, value) in overrides {
        node_object.insert(key, value);
    }
    let Ok(overridden) = serde_json::from_value::<PenNode>(node_value) else {
        return false;
    };
    *node = overridden;
    true
}

/// Reassign every node id in `nodes` (recursively, including
/// `children`) to a fresh unique id. `next_id` + `taken` are the same
/// allocator pair [`walkers::alloc_n_id`] uses; `taken` must be seeded
/// with the document's live ids. Returns `false` on id-space
/// exhaustion.
pub(crate) fn remap_subtree_ids(
    nodes: &mut [PenNode],
    next_id: &mut u64,
    taken: &mut HashSet<NodeId>,
) -> bool {
    remap_subtree_ids_mapping(nodes, next_id, taken).is_some()
}

/// Like [`remap_subtree_ids`] but returns the `(old_id, new_id)` pairs in
/// depth-first allocation order (`None` on id-space exhaustion). The mutation
/// is IDENTICAL to `remap_subtree_ids` — that wrapper just discards the map.
/// The `batch_design` MCP tool runs this on a CLONE of the to-be-inserted
/// forest to PREDICT the ids the host will assign (single-user localhost MCP:
/// the tool's snapshot == the live doc at apply, and the apply runs the exact
/// same allocation), so it can report TS's `results:[{binding,nodeId}]`.
pub fn remap_subtree_ids_mapping(
    nodes: &mut [PenNode],
    next_id: &mut u64,
    taken: &mut HashSet<NodeId>,
) -> Option<Vec<(String, String)>> {
    let mut map = Vec::new();
    remap_collect(nodes, next_id, taken, &mut map).then_some(map)
}

fn remap_collect(
    nodes: &mut [PenNode],
    next_id: &mut u64,
    taken: &mut HashSet<NodeId>,
    map: &mut Vec<(String, String)>,
) -> bool {
    for node in nodes.iter_mut() {
        let Some(fresh) = walkers::alloc_n_id(next_id, taken) else {
            return false;
        };
        let old = node.base().id.clone();
        let new = fresh.as_str().to_string();
        node.base_mut().id = new.clone();
        map.push((old, new));
        if let Some(children) = node.children_mut() {
            if !remap_collect(children, next_id, taken, map) {
                return false;
            }
        }
    }
    true
}
