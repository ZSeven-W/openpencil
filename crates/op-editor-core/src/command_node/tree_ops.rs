//! Tree-plumbing helpers shared by the raw-node commands: slot-preserving
//! replacement, parent-or-root insertion, copy-override merging, and the
//! subtree id remapper. Carved off `command_node.rs` to keep every file
//! under the 800-line cap.

use crate::id_allocator::{IdAllocError, IdAllocator, SequentialIdAllocator};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use jian_ops_schema::node::PenNode;
use std::collections::HashSet;

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
pub(super) fn insert_into_parent_or_root(
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

pub(super) fn apply_copy_overrides(node: &mut PenNode, overrides_json: Option<&str>) -> bool {
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
/// allocator pair [`crate::walkers::alloc_n_id`] uses; `taken` must be seeded
/// with the document's live ids. Returns `false` on id-space
/// exhaustion.
pub fn remap_subtree_ids(
    nodes: &mut [PenNode],
    next_id: &mut u64,
    taken: &mut HashSet<NodeId>,
) -> bool {
    remap_subtree_ids_mapping(nodes, next_id, taken).is_some()
}

/// Allocator-aware form of [`remap_subtree_ids`].
pub(crate) fn remap_subtree_ids_with_allocator(
    nodes: &mut [PenNode],
    allocator: &mut dyn IdAllocator,
    taken: &mut HashSet<NodeId>,
) -> Result<(), IdAllocError> {
    remap_subtree_ids_mapping_with_allocator(nodes, allocator, taken).map(drop)
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
    let mut allocator = SequentialIdAllocator::new(*next_id);
    let result = remap_subtree_ids_mapping_with_allocator(nodes, &mut allocator, taken).ok();
    *next_id = allocator.next_counter();
    result
}

/// Allocator-aware id remap with a typed exhaustion failure.
pub fn remap_subtree_ids_mapping_with_allocator(
    nodes: &mut [PenNode],
    allocator: &mut dyn IdAllocator,
    taken: &mut HashSet<NodeId>,
) -> Result<Vec<(String, String)>, IdAllocError> {
    let mut staged = nodes.to_vec();
    let mut map = Vec::new();
    remap_collect(&mut staged, allocator, taken, &mut map)?;
    for (destination, remapped) in nodes.iter_mut().zip(staged) {
        *destination = remapped;
    }
    Ok(map)
}

fn remap_collect(
    nodes: &mut [PenNode],
    allocator: &mut dyn IdAllocator,
    taken: &mut HashSet<NodeId>,
    map: &mut Vec<(String, String)>,
) -> Result<(), IdAllocError> {
    for node in nodes.iter_mut() {
        let fresh = allocator.allocate(taken)?;
        let old = node.base().id.clone();
        let new = fresh.as_str().to_string();
        node.base_mut().id = new.clone();
        map.push((old, new));
        if let Some(children) = node.children_mut() {
            remap_collect(children, allocator, taken, map)?;
        }
    }
    Ok(())
}
