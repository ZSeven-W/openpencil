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

/// Insert `node` as the FIRST child of `parent`. `Ok(())` / `Err(node)`.
/// Mirrors `append_into` but lands the payload at index 0 so a
/// layer-panel "drop into container" matches TS (`layer-dnd-utils.ts`
/// inserts at index 0 — the top of the child list). Fails (bounces the
/// payload) when `parent` is not a container.
#[allow(clippy::result_large_err)]
pub fn prepend_into(
    children: &mut [PenNode],
    parent: &NodeId,
    node: PenNode,
) -> Result<(), PenNode> {
    if let Some(idx) = children.iter().position(|n| n.id_str() == parent.as_str()) {
        match children[idx].children_mut() {
            Some(grand) => {
                grand.insert(0, node);
                return Ok(());
            }
            None => return Err(node),
        }
    }
    let mut carry = node;
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            match prepend_into(grand, parent, carry) {
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

/// Translate the subtree rooted at `node` by `(dx, dy)` document px.
///
/// Only the root's own `x` / `y` move. Child coords in the canonical
/// schema are PARENT-RELATIVE (jian-core's layout engine resolves a
/// child AABB against its parent origin), so shifting the root alone
/// carries the whole subtree visually. Cascading the delta into
/// descendants — as a doc-absolute model would need — double-shifts
/// every nested free-layout child (proven by layout probe), and TS
/// agrees: the drag commit writes only the dragged node
/// (`skia-interaction-select.ts` `docStore.updateNode(orig.id,
/// {x: orig.x + dx, y: orig.y + dy})`; same in pen-engine's
/// `select-handler.ts`). It would also materialize explicit `x` / `y`
/// onto flex flow children, detaching them from auto-layout flow.
pub fn translate_subtree(node: &mut PenNode, dx: f64, dy: f64) {
    let base = node.base_mut();
    base.x = Some(base.x.unwrap_or(0.0) + dx);
    base.y = Some(base.y.unwrap_or(0.0) + dy);
}

/// Locate `target`'s parent container anywhere in the forest.
/// `Some((None, idx))` when `target` sits at the top level,
/// `Some((Some(parent_id), idx))` for a nested target, `None` when
/// absent from the forest entirely.
pub fn find_parent_and_index(
    children: &[PenNode],
    target: &NodeId,
) -> Option<(Option<NodeId>, usize)> {
    if let Some(idx) = children.iter().position(|n| n.id_str() == target.as_str()) {
        return Some((None, idx));
    }
    for child in children {
        if let Some(grand) = child.children() {
            if let Some((parent, idx)) = find_parent_and_index(grand, target) {
                return Some((parent.or_else(|| NodeId::new_opt(child.id_str())), idx));
            }
        }
    }
    None
}

/// Insert `node` into the container `parent` (`None` = top level) at
/// `index` (clamped to the child count; `None` = append). False when
/// the parent container no longer exists or cannot hold children.
pub fn insert_into_parent(
    children: &mut Vec<PenNode>,
    parent: Option<&NodeId>,
    index: Option<usize>,
    node: PenNode,
) -> bool {
    let Some(parent_id) = parent else {
        let at = index.unwrap_or(children.len()).min(children.len());
        children.insert(at, node);
        return true;
    };
    let Some(container) = find_node_mut(children, parent_id) else {
        return false;
    };
    let Some(slot) = container.children_mut() else {
        return false;
    };
    let at = index.unwrap_or(slot.len()).min(slot.len());
    slot.insert(at, node);
    true
}

/// True when `target`'s immediate parent (anywhere in the forest) is an
/// auto-layout (flex) container — i.e. `target` is positioned by the
/// layout engine, not by stored `x` / `y`.
///
/// Such a node must not be free-dragged: writing `x` / `y` onto it (even
/// the tiny delta of a jittered click) flips it to `Position::Absolute`
/// in jian-core and detaches it from its parent's flex flow, collapsing
/// the siblings. A drag of an auto-layout child is therefore a no-op
/// (reorder-on-drag is a separate, future affordance). Top-level nodes
/// and children of free-layout parents are NOT affected.
pub fn is_flow_child_of_flex(children: &[PenNode], target: &NodeId) -> bool {
    for child in children {
        if let Some(grand) = child.children() {
            if grand.iter().any(|c| c.id_str() == target.as_str()) {
                return child.is_auto_layout_container();
            }
            if is_flow_child_of_flex(grand, target) {
                return true;
            }
        }
    }
    false
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

/// Single-pass replacement for the old `is_flow_child_of_flex` +
/// `is_ancestor_in_set` + `find_node_mut` triple (one full-tree walk
/// PER selected id). This descends the forest exactly once, carrying
/// two contexts down the recursion instead of recomputing them from
/// scratch for every id:
///
/// - `parent_is_flex` — whether the CURRENT node's immediate parent is
///   an auto-layout (flex) container. Mirrors `is_flow_child_of_flex`,
///   which only cares about the immediate parent, not any ancestor —
///   so this is recomputed fresh (from the node just visited) at each
///   recursion level, never accumulated.
/// - `ancestor_in_set` — whether ANY proper ancestor's id is in
///   `editable`. Mirrors `is_ancestor_in_set`'s top-down ancestor-chain
///   search. Accumulates via OR as the recursion descends, and is
///   evaluated BEFORE folding in the current node's own membership (a
///   node is never its own ancestor).
///
/// A node translates iff its id is in `editable` AND neither guard
/// applies — matching `translate_selected`'s per-id skip order (flex
/// check first, then the ancestor dedup) exactly. `editable` already
/// reflects the `is_editable` pre-filter (hidden / locked ids excluded
/// by the caller), so this walk never re-checks visibility / lock
/// state itself.
///
/// Recursion is gated on the IMMUTABLE [`PenNodeExt::children`] check
/// first, only reaching for [`PenNodeExt::children_mut`] when that
/// already reports `Some`. `children_mut` eagerly upgrades a
/// container-capable node's `children: None` to `Some(vec![])`
/// (`Option::get_or_insert_with`) — harmless for the few nodes
/// `find_node_mut`'s per-id search happened to pass over in the old
/// three-walk body, but this walk visits every node in the forest
/// exactly once, so calling `children_mut` unconditionally here would
/// silently materialize an empty `children` list on every leaf
/// Rectangle/Frame in the document on every drag frame — bloating the
/// in-memory doc and any subsequent `.op` serialization. A node with
/// no children has nothing left to translate inside it either way, so
/// skipping it is both cheaper and correct.
pub fn translate_editable_subtree(
    children: &mut [PenNode],
    editable: &HashSet<&str>,
    dx: f64,
    dy: f64,
    parent_is_flex: bool,
    ancestor_in_set: bool,
) -> bool {
    let mut moved = false;
    for child in children.iter_mut() {
        let in_set = editable.contains(child.id_str());
        if in_set && !parent_is_flex && !ancestor_in_set {
            translate_subtree(child, dx, dy);
            moved = true;
        }
        let child_is_flex = child.is_auto_layout_container();
        let child_ancestor_in_set = ancestor_in_set || in_set;
        if child.children().is_some() {
            if let Some(grand) = child.children_mut() {
                if translate_editable_subtree(
                    grand,
                    editable,
                    dx,
                    dy,
                    child_is_flex,
                    child_ancestor_in_set,
                ) {
                    moved = true;
                }
            }
        }
    }
    moved
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
