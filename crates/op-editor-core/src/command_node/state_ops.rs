//! `impl EditorState` for the raw-node command family — `InsertNode` /
//! `UpdateNode` / `PatchNodeData` / `DeleteNode` / `MoveNode` /
//! `CopyNode` / `ReplaceNode` / `ReplaceSubtree` / `BatchInsert` /
//! `InsertSubtree`, plus the editor id allocator.
//!
//! Every method preserves the pre-validate-then-mutate discipline: kind /
//! geometry / hex / id space are validated BEFORE any tree write, so a
//! bad arg never leaves the document half-mutated. Carved off
//! `command_node.rs` to keep every file under the 800-line cap.

use super::builders::{build_leaf_node, kind_is_valid};
use super::tree_ops::{
    apply_copy_overrides, insert_into_parent_or_root, remap_subtree_ids_mapping_with_allocator,
    remap_subtree_ids_with_allocator, replace_node_in_children,
};
use crate::command::BatchInsertItem;
use crate::fills::set_primary_fill_hex;
use crate::id_allocator::{IdAllocError, IdAllocator, SequentialIdAllocator};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;
use jian_ops_schema::node::PenNode;
use std::collections::HashSet;

impl EditorState {
    /// Compute the numeric seed for the next editor-minted `n{N}` id —
    /// `max_node_id() + 1`. `None` on `u64` exhaustion. Pub so the
    /// `batch_design` MCP tool can predict (off a doc snapshot) the ids the
    /// host will allocate at apply, mirroring `cmd_insert_subtree`'s seed.
    pub fn next_node_id_seed(&self) -> Option<u64> {
        self.max_node_id().checked_add(1).map(|n| n.max(1))
    }

    /// `InsertNode` — build + append a fresh leaf on the active page.
    // Args mirror the `InsertNode` command fields one-for-one; bundling
    // them into a struct would just shadow the DTO with no real gain.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cmd_insert_node_with_allocator(
        &mut self,
        kind: &str,
        name: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_hex: &Option<String>,
        target_parent: &NodeId,
        allocator: &mut dyn IdAllocator,
    ) -> Result<bool, IdAllocError> {
        if !kind_is_valid(kind) || width < 0 || height < 0 {
            return Ok(false);
        }
        // Pre-validate hex BEFORE minting an id / mutating the tree.
        if let Some(hex) = fill_hex {
            if crate::color_picker::parse_hex_rgb(hex).is_none() {
                return Ok(false);
            }
        }
        if target_parent.is_real() {
            match walkers::find_node(self.active_children(), target_parent) {
                Some(parent) if parent.is_container() => {}
                _ => return Ok(false),
            }
        }
        let mut taken = self.collect_node_ids();
        let new_id = allocator.allocate(&mut taken)?;
        let Some(mut node) = build_leaf_node(kind, new_id.as_str(), name, x, y, width, height)
        else {
            return Ok(false);
        };
        if let Some(hex) = fill_hex {
            set_primary_fill_hex(&mut node, hex);
        }
        if target_parent.is_real() {
            let root = self.active_children_mut();
            let Some(parent) = walkers::find_node_mut(root, target_parent) else {
                return Ok(false);
            };
            let Some(children) = parent.children_mut() else {
                return Ok(false);
            };
            children.push(node);
        } else {
            self.active_children_mut().push(node);
        }
        Ok(true)
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
                // A container that has never held a child carries NO `children`
                // array — it is still a container, and `children_mut` mints the
                // array on demand. Refusing it here made an empty media slot
                // unreachable: a photo could not be moved into the very slot
                // the design authored for it (measured test0711-1-glm).
                if !target.is_container() {
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
    pub(crate) fn cmd_copy_node_with_allocator(
        &mut self,
        node_id: &NodeId,
        target_parent: &NodeId,
        overrides_json: Option<&str>,
        allocator: &mut dyn IdAllocator,
    ) -> Result<bool, IdAllocError> {
        if !node_id.is_real() {
            return Ok(false);
        }
        // Validate source + target up front.
        {
            let children = self.active_children();
            if walkers::find_node(children, node_id).is_none() {
                return Ok(false);
            }
            if target_parent.is_real() {
                let Some(target) = walkers::find_node(children, target_parent) else {
                    return Ok(false);
                };
                // Same rule as MoveNode: childless container, still a container.
                if !target.is_container() {
                    return Ok(false);
                }
            }
        }
        let mut taken = self.collect_node_ids();
        // Clone the owned subtree before re-borrowing the tree mutably.
        let mut clone = {
            let children = self.active_children();
            let src = walkers::find_node(children, node_id).expect("validated");
            walkers::deep_clone_with_allocator(src, allocator, &mut taken)?
        };
        if !apply_copy_overrides(&mut clone, overrides_json) {
            return Ok(false);
        }
        let children = self.active_children_mut();
        Ok(insert_into_parent_or_root(children, target_parent, clone, None).is_ok())
    }

    /// `ReplaceNode` — swap an existing node for a freshly-built leaf at
    /// the same slot. The destructive-swap guard: replacing a node WITH
    /// children requires `drop_children == true`, else the swap is
    /// refused so a container can't silently lose its subtree.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cmd_replace_node_with_allocator(
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
        allocator: &mut dyn IdAllocator,
    ) -> Result<bool, IdAllocError> {
        if !node_id.is_real() || !kind_is_valid(kind) || width < 0 || height < 0 {
            return Ok(false);
        }
        if let Some(hex) = fill_hex {
            if crate::color_picker::parse_hex_rgb(hex).is_none() {
                return Ok(false);
            }
        }
        // Resolve target + check the destructive-swap guard BEFORE
        // minting an id. A target WITH children needs explicit consent.
        {
            let children = self.active_children();
            let Some(target) = walkers::find_node(children, node_id) else {
                return Ok(false);
            };
            let has_children = target.children().map(|c| !c.is_empty()).unwrap_or(false);
            if has_children && !drop_children {
                return Ok(false);
            }
        }
        let mut taken = self.collect_node_ids();
        let new_id = allocator.allocate(&mut taken)?;
        let Some(mut replacement) =
            build_leaf_node(kind, new_id.as_str(), name, x, y, width, height)
        else {
            return Ok(false);
        };
        if let Some(hex) = fill_hex {
            set_primary_fill_hex(&mut replacement, hex);
        }
        let mut slot = Some(replacement);
        Ok(replace_node_in_children(
            self.active_children_mut(),
            node_id,
            &mut slot,
        ))
    }

    /// `ReplaceSubtree` — swap an existing node for a fully-authored
    /// canonical subtree. The destructive-swap guard matches
    /// `ReplaceNode`: replacing a node WITH children requires explicit
    /// opt-in.
    pub(crate) fn cmd_replace_subtree_with_allocator(
        &mut self,
        node_id: &NodeId,
        node: PenNode,
        drop_children: bool,
        allocator: &mut dyn IdAllocator,
    ) -> Result<bool, IdAllocError> {
        if !node_id.is_real() {
            return Ok(false);
        }
        {
            let children = self.active_children();
            let Some(target) = walkers::find_node(children, node_id) else {
                return Ok(false);
            };
            let has_children = target.children().map(|c| !c.is_empty()).unwrap_or(false);
            if has_children && !drop_children {
                return Ok(false);
            }
        }
        let mut taken = self.collect_node_ids();
        let mut nodes = vec![node];
        remap_subtree_ids_with_allocator(&mut nodes, allocator, &mut taken)?;
        let mut slot = nodes.pop();
        Ok(replace_node_in_children(
            self.active_children_mut(),
            node_id,
            &mut slot,
        ))
    }

    /// `BatchInsert` — insert N leaf nodes on the active page in one
    /// atomic shot. EVERY descriptor is validated before any mutation;
    /// a single bad entry rejects the whole batch.
    pub(crate) fn cmd_batch_insert_with_allocator(
        &mut self,
        items: &[BatchInsertItem],
        allocator: &mut dyn IdAllocator,
    ) -> Result<bool, IdAllocError> {
        if items.is_empty() {
            return Ok(false);
        }
        // Pre-validate kinds + geometry + fill hex up front.
        for item in items {
            if !kind_is_valid(&item.kind) || item.width < 0 || item.height < 0 {
                return Ok(false);
            }
            if let Some(hex) = &item.fill_hex {
                if crate::color_picker::parse_hex_rgb(hex).is_none() {
                    return Ok(false);
                }
            }
        }
        // Allocate every fresh id up front; bail on id-space exhaustion.
        let mut live: HashSet<NodeId> = self.collect_node_ids();
        let mut ids: Vec<NodeId> = Vec::with_capacity(items.len());
        for _ in 0..items.len() {
            ids.push(allocator.allocate(&mut live)?);
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
            // A full canonical fill stack overrides the solid `fill_hex`
            // shortcut so gradient / mesh / image fills survive the batch
            // insert; otherwise fall back to the single-colour path.
            if let Some(fills) = &item.fill {
                if let Some(slot) = crate::fills::node_fills_mut(&mut node) {
                    *slot = fills.clone();
                }
            } else if let Some(hex) = &item.fill_hex {
                set_primary_fill_hex(&mut node, hex);
            }
            children.push(node);
        }
        Ok(true)
    }

    /// Insert one or more nested `PenNode` subtrees. `parent_id` of
    /// `NONE` appends to the active page root; otherwise the parent
    /// must exist and be a container variant. Every incoming node id
    /// (recursively) is remapped to a fresh editor id so an
    /// externally-authored subtree can't collide with live doc ids.
    pub(crate) fn cmd_insert_subtree_with_allocator(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
        allocator: &mut dyn IdAllocator,
    ) -> Result<bool, IdAllocError> {
        if nodes.is_empty() {
            return Ok(false);
        }
        // Validate the parent up front (when not the page root).
        if parent_id.is_real() {
            match walkers::find_node(self.active_children(), parent_id) {
                Some(p) if p.is_container() => {}
                _ => return Ok(false), // missing or non-container
            }
        }
        // Allocate fresh ids for the whole incoming forest.
        let mut taken: HashSet<NodeId> = self.collect_node_ids();
        let mut nodes = nodes;
        let replacement = crate::command_root_replace::prepare_root_frame_replacement(
            self.active_children(),
            &mut nodes,
            parent_id,
        );
        remap_subtree_ids_with_allocator(&mut nodes, allocator, &mut taken)?;
        // All validation + allocation passed — now mutate.
        if parent_id.is_real() {
            let root = self.active_children_mut();
            let Some(parent) = walkers::find_node_mut(root, parent_id) else {
                return Ok(false);
            };
            let Some(slot) = parent.children_mut() else {
                return Ok(false);
            };
            slot.extend(nodes);
        } else {
            let roots = self.active_children_mut();
            if let Some(replacement) = replacement.as_ref() {
                if !crate::command_root_replace::remove_root_frame_replacement(roots, replacement) {
                    return Ok(false);
                }
            }
            roots.extend(nodes);
        }
        Ok(true)
    }

    /// Same mutation as [`cmd_insert_subtree`] but returns the
    /// **post-remap** ids of the forest roots (the incoming top-level
    /// nodes, in order). `None` = rejected (no mutation), mirroring
    /// `cmd_insert_subtree`'s `false`. Used by the orchestrator so
    /// append-mode cleanup can scope to exactly the newly-inserted roots
    /// (their ids are remapped on apply — the caller's ids are placeholders).
    pub fn insert_subtree_returning_root_ids(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
    ) -> Option<Vec<String>> {
        let mut allocator = SequentialIdAllocator::for_document(&self.doc, 1).ok()?;
        self.insert_subtree_returning_root_ids_with_allocator(nodes, parent_id, &mut allocator)
            .ok()
            .flatten()
    }

    /// Allocator-aware form of [`Self::insert_subtree_returning_root_ids`].
    pub fn insert_subtree_returning_root_ids_with_allocator(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
        allocator: &mut dyn IdAllocator,
    ) -> Result<Option<Vec<String>>, IdAllocError> {
        if nodes.is_empty() {
            return Ok(None);
        }
        // Validate the parent up front (when not the page root).
        if parent_id.is_real() {
            match walkers::find_node(self.active_children(), parent_id) {
                Some(p) if p.is_container() => {}
                _ => return Ok(None),
            }
        }
        let mut taken: HashSet<NodeId> = self.collect_node_ids();
        let mut nodes = nodes;
        let replacement = crate::command_root_replace::prepare_root_frame_replacement(
            self.active_children(),
            &mut nodes,
            parent_id,
        );
        // remap_subtree_ids_mapping mutates every node id IN PLACE (DFS order).
        // Reading root ids from the mapping by index is incorrect: for a
        // forest where root0 has children, mapping[0..root_count] would yield
        // [root0, child0a, ...] instead of [root0, root1, ...]. Instead, read
        // the root ids directly from the top-level nodes after remap — they
        // are already updated in place and ordering is exact.
        remap_subtree_ids_mapping_with_allocator(&mut nodes, allocator, &mut taken)?;
        let root_ids: Vec<String> = nodes.iter().map(|n| n.id_str().to_string()).collect();
        // All validation + allocation passed — now mutate.
        if parent_id.is_real() {
            let root = self.active_children_mut();
            let Some(parent) = walkers::find_node_mut(root, parent_id) else {
                return Ok(None);
            };
            let Some(slot) = parent.children_mut() else {
                return Ok(None);
            };
            slot.extend(nodes);
        } else {
            let roots = self.active_children_mut();
            if let Some(replacement) = replacement.as_ref() {
                if !crate::command_root_replace::remove_root_frame_replacement(roots, replacement) {
                    return Ok(None);
                }
            }
            roots.extend(nodes);
        }
        Ok(Some(root_ids))
    }
}
