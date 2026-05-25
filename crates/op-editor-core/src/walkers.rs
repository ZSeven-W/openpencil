//! Free tree-walk helpers for the `EditorState` mutators, operating
//! on the canonical `PenDocument` node tree.
//!
//! shell-core's `document/walkers.rs` walked its own flat `Node`
//! tree; this is the equivalent layer for `jian_ops_schema::PenNode`,
//! reached through the [`PenNodeExt`] uniform-access trait so each
//! walk stays variant-agnostic.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use jian_ops_schema::node::PenNode;
use std::collections::HashSet;

/// Direction for `reorder_selected` — picks which sibling the
/// selected node swaps with in its parent's children vec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReorderDirection {
    /// Towards the front of the paint order (lower index,
    /// `children[0]` is top). Bound to `]`.
    Up,
    /// Towards the back of the paint order (higher index, drawn
    /// underneath). Bound to `[`.
    Down,
}

/// Find a node by id anywhere in a children forest. Returns a shared
/// reference, or `None` when the id resolves to no node.
pub fn find_node<'a>(children: &'a [PenNode], id: &NodeId) -> Option<&'a PenNode> {
    for child in children {
        if child.id_str() == id.as_str() {
            return Some(child);
        }
        if let Some(grand) = child.children() {
            if let Some(hit) = find_node(grand, id) {
                return Some(hit);
            }
        }
    }
    None
}

/// Mutable form of [`find_node`].
pub fn find_node_mut<'a>(children: &'a mut [PenNode], id: &NodeId) -> Option<&'a mut PenNode> {
    for child in children.iter_mut() {
        if child.id_str() == id.as_str() {
            return Some(child);
        }
        if let Some(grand) = child.children_mut() {
            if let Some(hit) = find_node_mut(grand, id) {
                return Some(hit);
            }
        }
    }
    None
}

/// True when `id` resolves to a node anywhere in the forest.
pub fn contains_node(children: &[PenNode], id: &NodeId) -> bool {
    find_node(children, id).is_some()
}

/// True when `target` equals `node` or appears anywhere in its
/// subtree. Used to reject reparenting cycles.
pub fn descendant_contains(node: &PenNode, target: &NodeId) -> bool {
    if node.id_str() == target.as_str() {
        return true;
    }
    if let Some(children) = node.children() {
        return children.iter().any(|c| descendant_contains(c, target));
    }
    false
}

/// Remove the first node matching `target` from `children` or any
/// descendant's children. True when something was removed.
pub fn remove_from_children(children: &mut Vec<PenNode>, target: &NodeId) -> bool {
    if let Some(idx) = children.iter().position(|n| n.id_str() == target.as_str()) {
        children.remove(idx);
        return true;
    }
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            if remove_from_children(grand, target) {
                return true;
            }
        }
    }
    false
}

/// Drain the node matching `target` out of the forest and return it.
/// Mirrors shell-core's `extract_node` — backs the reorder/reparent
/// extract phase.
pub fn extract_node(children: &mut Vec<PenNode>, target: &NodeId) -> Option<PenNode> {
    if let Some(idx) = children.iter().position(|n| n.id_str() == target.as_str()) {
        return Some(children.remove(idx));
    }
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            if let Some(extracted) = extract_node(grand, target) {
                return Some(extracted);
            }
        }
    }
    None
}

/// Insert `node` immediately before `anchor`. `Ok(())` on success;
/// `Err(node)` bounces the payload back on miss.
// `Err(PenNode)` is the bounced-back payload, not an error type.
#[allow(clippy::result_large_err)]
pub fn insert_before_in_children(
    children: &mut Vec<PenNode>,
    anchor: &NodeId,
    node: PenNode,
) -> Result<(), PenNode> {
    if let Some(idx) = children.iter().position(|n| n.id_str() == anchor.as_str()) {
        children.insert(idx, node);
        return Ok(());
    }
    let mut carry = node;
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            match insert_before_in_children(grand, anchor, carry) {
                Ok(()) => return Ok(()),
                Err(returned) => carry = returned,
            }
        }
    }
    Err(carry)
}

/// Insert `node` immediately after `anchor`. `Ok(())` / `Err(node)`.
// `Err(PenNode)` is the bounced-back payload, not an error type.
#[allow(clippy::result_large_err)]
pub fn insert_after_in_children(
    children: &mut Vec<PenNode>,
    anchor: &NodeId,
    node: PenNode,
) -> Result<(), PenNode> {
    if let Some(idx) = children.iter().position(|n| n.id_str() == anchor.as_str()) {
        children.insert(idx + 1, node);
        return Ok(());
    }
    let mut carry = node;
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            match insert_after_in_children(grand, anchor, carry) {
                Ok(()) => return Ok(()),
                Err(returned) => carry = returned,
            }
        }
    }
    Err(carry)
}

/// Append `node` as the LAST child of `parent`. `Ok(())` / `Err(node)`.
/// Fails (bounces the payload) when `parent` is not a container.
// `Err(PenNode)` is the bounced-back payload, not an error type — boxing
// it would be a needless allocation on the hot insert path.
#[allow(clippy::result_large_err)]
pub fn append_into(
    children: &mut [PenNode],
    parent: &NodeId,
    node: PenNode,
) -> Result<(), PenNode> {
    if let Some(idx) = children.iter().position(|n| n.id_str() == parent.as_str()) {
        match children[idx].children_mut() {
            Some(grand) => {
                grand.push(node);
                return Ok(());
            }
            None => return Err(node),
        }
    }
    let mut carry = node;
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            match append_into(grand, parent, carry) {
                Ok(()) => return Ok(()),
                Err(returned) => carry = returned,
            }
        }
    }
    Err(carry)
}

/// Swap the node matching `target` with its next / prev sibling.
pub fn reorder_in_children(
    children: &mut [PenNode],
    target: &NodeId,
    direction: ReorderDirection,
) -> bool {
    if let Some(idx) = children.iter().position(|n| n.id_str() == target.as_str()) {
        match direction {
            ReorderDirection::Up if idx > 0 => {
                children.swap(idx, idx - 1);
                return true;
            }
            ReorderDirection::Down if idx + 1 < children.len() => {
                children.swap(idx, idx + 1);
                return true;
            }
            _ => return false,
        }
    }
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            if reorder_in_children(grand, target, direction) {
                return true;
            }
        }
    }
    false
}

/// Insert every descendant id of the forest into `out`.
pub fn collect_ids(children: &[PenNode], out: &mut HashSet<NodeId>) {
    for child in children {
        if let Some(id) = NodeId::new_opt(child.id_str()) {
            out.insert(id);
        }
        if let Some(grand) = child.children() {
            collect_ids(grand, out);
        }
    }
}

/// Parse the numeric suffix of an editor-minted `n{N}` id. `None`
/// for any id not matching `^n\d+$` (the NONE sentinel or an
/// arbitrary canonical-schema id).
pub fn parse_n_id(id: &str) -> Option<u64> {
    let digits = id.strip_prefix('n')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u64>().ok()
}

/// Largest `n{N}` suffix in the subtree (0 when none exists).
pub fn max_id_walk(node: &PenNode) -> u64 {
    let mut max = parse_n_id(node.id_str()).unwrap_or(0);
    if let Some(children) = node.children() {
        for child in children {
            max = max.max(max_id_walk(child));
        }
    }
    max
}

/// Allocate the next free editor-minted id. Formats `n{*next_id}`,
/// advancing past any candidate already present in `taken`. Returns
/// `None` only on `u64` counter exhaustion.
pub fn alloc_n_id(next_id: &mut u64, taken: &mut HashSet<NodeId>) -> Option<NodeId> {
    loop {
        let candidate = NodeId::new(format!("n{}", *next_id));
        *next_id = next_id.checked_add(1)?;
        if taken.insert(candidate.clone()) {
            return Some(candidate);
        }
    }
}

/// Node count in `node`'s subtree, including `node` itself.
pub fn subtree_size(node: &PenNode) -> u64 {
    let mut n = 1u64;
    if let Some(children) = node.children() {
        for child in children {
            n = n.saturating_add(subtree_size(child));
        }
    }
    n
}

/// Deep-clone `node`, minting a fresh `n{N}` id for it and every
/// descendant from `next_id`. Geometry + style copy verbatim.
pub fn deep_clone_with_new_ids(
    node: &PenNode,
    next_id: &mut u64,
    taken: &mut HashSet<NodeId>,
) -> PenNode {
    let mut clone = node.clone();
    let new_id = alloc_n_id(next_id, taken)
        .unwrap_or_else(|| NodeId::new(format!("n{}-{}", u64::MAX, taken.len())));
    clone.base_mut().id = new_id.into();
    if let Some(children) = clone.children_mut() {
        let fresh: Vec<PenNode> = children
            .iter()
            .map(|c| deep_clone_with_new_ids(c, next_id, taken))
            .collect();
        *children = fresh;
    }
    clone
}

/// Translate `node` and its whole subtree by `(dx, dy)` document px.
/// Children carry document-absolute coords, so moving a container
/// must shift the descendants too or they detach.
pub fn translate_subtree(node: &mut PenNode, dx: f64, dy: f64) {
    {
        let base = node.base_mut();
        base.x = Some(base.x.unwrap_or(0.0) + dx);
        base.y = Some(base.y.unwrap_or(0.0) + dy);
    }
    if let Some(children) = node.children_mut() {
        for child in children {
            translate_subtree(child, dx, dy);
        }
    }
}

/// True when any ancestor of `target` within the forest is also in
/// `set` — used by `translate_selected` to dedupe a double-shift.
pub fn is_ancestor_in_set(children: &[PenNode], target: &NodeId, set: &[NodeId]) -> bool {
    for child in children {
        if child.id_str() != target.as_str()
            && descendant_contains(child, target)
            && set.iter().any(|s| s.as_str() == child.id_str())
        {
            return true;
        }
        if let Some(grand) = child.children() {
            if is_ancestor_in_set(grand, target, set) {
                return true;
            }
        }
    }
    false
}

/// First duplicate id found in the forest, or `None` when all ids
/// are unique.
pub fn find_duplicate(children: &[PenNode], seen: &mut HashSet<String>) -> Option<NodeId> {
    for child in children {
        if !seen.insert(child.id_str().to_string()) {
            return NodeId::new_opt(child.id_str());
        }
        if let Some(grand) = child.children() {
            if let Some(dup) = find_duplicate(grand, seen) {
                return Some(dup);
            }
        }
    }
    None
}
