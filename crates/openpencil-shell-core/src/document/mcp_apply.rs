//! MCP-write command application against the document. Pulled out
//! of `mutators.rs` so that file stays under the 800-line cap as
//! more write commands land.

use super::variables::parse_hex_color;
use super::{Document, Node, NodeId, NodeKind};

/// Find a mutable reference to the node with `target` id anywhere
/// in the document, walking every page + every descendant. None
/// when the id doesn't resolve. The walk uses raw recursion rather
/// than the existing `Document::find` because that returns `&Node`,
/// not `&mut Node`.
fn find_node_mut_in_doc(doc: &mut Document, target: NodeId) -> Option<&mut Node> {
    for page in doc.pages.iter_mut() {
        if let Some(node) = find_in_subtree(&mut page.children, target) {
            return Some(node);
        }
    }
    None
}

fn find_in_subtree(children: &mut [Node], target: NodeId) -> Option<&mut Node> {
    // Locate by id first using a fresh iter (immutable position
    // scan); separate the recursive walk so the borrow checker
    // doesn't see two overlapping `iter_mut` ranges on the same
    // slice.
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        return Some(&mut children[idx]);
    }
    for node in children.iter_mut() {
        if let Some(found) = find_in_subtree(&mut node.children, target) {
            return Some(found);
        }
    }
    None
}

/// Remove the node with `target` id from `children` or any nested
/// descendant. Returns true when removed.
fn remove_in_subtree(children: &mut Vec<Node>, target: NodeId) -> bool {
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        children.remove(idx);
        return true;
    }
    for node in children.iter_mut() {
        if remove_in_subtree(&mut node.children, target) {
            return true;
        }
    }
    false
}

/// Immutable lookup for a node anywhere in the document. Used by
/// the move cycle-check (which needs to walk a subtree without
/// holding a mutable borrow on it).
fn find_node_in_doc(doc: &Document, target: NodeId) -> Option<&Node> {
    for page in doc.pages.iter() {
        if let Some(node) = find_in_subtree_ref(&page.children, target) {
            return Some(node);
        }
    }
    None
}

fn find_in_subtree_ref(children: &[Node], target: NodeId) -> Option<&Node> {
    for node in children.iter() {
        if node.id == target {
            return Some(node);
        }
    }
    for node in children.iter() {
        if let Some(found) = find_in_subtree_ref(&node.children, target) {
            return Some(found);
        }
    }
    None
}

/// True when `node` or any descendant has id == `target`. Used by
/// move's cycle guard: reparenting source under a descendant of
/// itself would orphan + cycle the subtree.
fn subtree_contains(node: &Node, target: NodeId) -> bool {
    if node.id == target {
        return true;
    }
    node.children.iter().any(|c| subtree_contains(c, target))
}

/// Find + detach the node with `target` id from its parent's
/// children vec, returning the owned Node. None when not found.
/// Walks every page recursively.
fn detach_node(doc: &mut Document, target: NodeId) -> Option<Node> {
    for page in doc.pages.iter_mut() {
        if let Some(node) = detach_from_subtree(&mut page.children, target) {
            return Some(node);
        }
    }
    None
}

fn detach_from_subtree(children: &mut Vec<Node>, target: NodeId) -> Option<Node> {
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        return Some(children.remove(idx));
    }
    for node in children.iter_mut() {
        if let Some(detached) = detach_from_subtree(&mut node.children, target) {
            return Some(detached);
        }
    }
    None
}

/// Deep-clone a node + its descendants, allocating fresh ids
/// from the supplied counter so the resulting subtree has no id
/// collision with the live document. `next_id` is bumped past
/// every emitted id. Returns None when the id space is exhausted
/// mid-walk (callers surface that as an apply-time failure).
fn clone_subtree(node: &Node, next_id: &mut u64) -> Option<Node> {
    // Reuse Node's #[derive(Clone)] for the bulk copy (preserves
    // every field including new ones added later), then rewrite
    // the id + recurse into children with fresh ids.
    let mut clone = node.clone();
    clone.id = NodeId::new_opt(*next_id)?;
    *next_id = next_id.checked_add(1)?;
    let mut fresh_children = Vec::with_capacity(node.children.len());
    for child in &node.children {
        fresh_children.push(clone_subtree(child, next_id)?);
    }
    clone.children = fresh_children;
    Some(clone)
}

/// Replace the node with `target` id with `replacement` at its
/// current slot. Walks every page; on the first match,
/// swaps in place (`children[idx] = replacement`) so the
/// sibling order is preserved. Returns true on success.
fn replace_node_in_doc(doc: &mut Document, target: NodeId, replacement: Node) -> bool {
    // Wrap the replacement in an Option so we can take() it inside
    // the recursive walk without giving up the Some-on-failure
    // contract (replacement returned untouched is irrelevant here
    // — we either consumed it or replace_node already returned).
    let mut slot = Some(replacement);
    for page in doc.pages.iter_mut() {
        if replace_in_subtree(&mut page.children, target, &mut slot) {
            return true;
        }
    }
    false
}

fn replace_in_subtree(
    children: &mut [Node],
    target: NodeId,
    slot: &mut Option<Node>,
) -> bool {
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        if let Some(replacement) = slot.take() {
            children[idx] = replacement;
            return true;
        }
    }
    for node in children.iter_mut() {
        if replace_in_subtree(&mut node.children, target, slot) {
            return true;
        }
    }
    false
}

/// Resolve an MCP `kind` arg into a NodeKind. Accepts the same
/// lowercase strings the read-side tools emit (`frame`, `group`,
/// `rect`, `ellipse`, `polygon`, `line`, `text`, `path`).
pub(super) fn parse_node_kind(s: &str) -> Option<NodeKind> {
    match s {
        "frame" => Some(NodeKind::Frame),
        "group" => Some(NodeKind::Group),
        "rect" => Some(NodeKind::Rect),
        "ellipse" => Some(NodeKind::Ellipse),
        "polygon" => Some(NodeKind::Polygon),
        "line" => Some(NodeKind::Line),
        "text" => Some(NodeKind::Text),
        "path" => Some(NodeKind::Path),
        _ => None,
    }
}

impl Document {
    /// Apply an MCP write command against the document. Returns
    /// true when the command actually changed something so callers
    /// can decide whether to push an undo snapshot. False on apply-
    /// time validation failure (unknown variable, value not in axis,
    /// unknown node kind, etc.). Routes variable + theme commands
    /// to `VariableTable::apply_mcp_command`; `InsertNode` lives
    /// here because it needs Pages + the id allocator.
    pub fn apply_mcp_command(&mut self, cmd: &crate::mcp::McpCommand) -> bool {
        match cmd {
            crate::mcp::McpCommand::InsertNode {
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
            } => {
                let Some(node_kind) = parse_node_kind(kind) else {
                    return false;
                };
                // Compute the fresh id BEFORE taking a mutable
                // borrow on `pages` — `max_node_id()` reads pages
                // immutably and would conflict otherwise.
                let Some(next_id) = self.next_node_id_seed() else {
                    // Id space exhausted (existing node at
                    // u64::MAX). Codex stop-gate: the previous
                    // saturating_add wrapped to u64::MAX,
                    // colliding with the live node. Refuse the
                    // insert + force the LLM client to handle the
                    // error rather than silently overwriting.
                    return false;
                };
                let active_idx = self.active_page_index;
                let Some(page) = self.pages.get_mut(active_idx) else {
                    return false;
                };
                let mut node = Node::leaf(next_id, node_kind, name.clone());
                node.bounds = crate::Rect::xywh(
                    *x as f32,
                    *y as f32,
                    *width as f32,
                    *height as f32,
                );
                if let Some(hex) = fill_hex {
                    if let Some(color) = parse_hex_color(hex) {
                        node.fill = Some(color);
                    } else {
                        return false;
                    }
                }
                page.children.push(node);
                true
            }
            crate::mcp::McpCommand::UpdateNode {
                node_id,
                x,
                y,
                width,
                height,
                name,
                fill_hex,
            } => {
                let Some(target) = NodeId::new_opt(*node_id) else {
                    return false;
                };
                // Pre-validate EVERY field BEFORE the mutable
                // borrow + writes (codex stop-gate: partial mutation
                // before rejecting invalid geometry — width<0 was
                // checked AFTER x/y had already been applied).
                let fill = match fill_hex {
                    None => None,
                    Some(hex) => match parse_hex_color(hex) {
                        Some(c) => Some(c),
                        None => return false,
                    },
                };
                if let Some(nw) = width {
                    if *nw < 0 {
                        return false;
                    }
                }
                if let Some(nh) = height {
                    if *nh < 0 {
                        return false;
                    }
                }
                let Some(node) = find_node_mut_in_doc(self, target) else {
                    return false;
                };
                // All validation passed — every field now applies
                // atomically (no early-return after this point).
                if let Some(nx) = x {
                    node.bounds.origin.x = *nx as f32;
                }
                if let Some(ny) = y {
                    node.bounds.origin.y = *ny as f32;
                }
                if let Some(nw) = width {
                    node.bounds.size.x = *nw as f32;
                }
                if let Some(nh) = height {
                    node.bounds.size.y = *nh as f32;
                }
                if let Some(new_name) = name {
                    node.name = new_name.clone();
                }
                if let Some(c) = fill {
                    node.fill = Some(c);
                }
                true
            }
            crate::mcp::McpCommand::DeleteNode { node_id } => {
                let Some(target) = NodeId::new_opt(*node_id) else {
                    return false;
                };
                let mut removed = false;
                for page in self.pages.iter_mut() {
                    if remove_in_subtree(&mut page.children, target) {
                        removed = true;
                        break;
                    }
                }
                removed
            }
            crate::mcp::McpCommand::CopyNode {
                node_id,
                target_parent_id,
            } => {
                let Some(source) = NodeId::new_opt(*node_id) else {
                    return false;
                };
                let target_parent = NodeId::new_opt(*target_parent_id);
                // Validate source + target up front; clone the
                // owned subtree before mutating pages so the clone
                // has stable ids regardless of where the live source
                // lives.
                let Some(src_node) = find_node_in_doc(self, source) else {
                    return false;
                };
                // Allow target == source: copying a node into
                // itself is a no-op semantically (clone lands as a
                // descendant of source). LLM use case: duplicate a
                // group's contents under the group itself.
                if let Some(target_id) = target_parent {
                    if find_node_in_doc(self, target_id).is_none() {
                        return false;
                    }
                } else if self.pages.get(self.active_page_index).is_none() {
                    return false;
                }
                // Allocate a fresh id base past max_node_id() so the
                // clone subtree's every node has a unique id.
                let Some(mut next_id) = self.next_node_id_seed() else {
                    return false;
                };
                let clone = match clone_subtree(src_node, &mut next_id) {
                    Some(c) => c,
                    None => return false,
                };
                // All validation + allocation done — attach the clone.
                match target_parent {
                    None => {
                        let active_idx = self.active_page_index;
                        self.pages[active_idx].children.push(clone);
                    }
                    Some(pid) => {
                        find_node_mut_in_doc(self, pid)
                            .expect("target validated")
                            .children
                            .push(clone);
                    }
                }
                true
            }
            crate::mcp::McpCommand::ReplaceNode {
                node_id,
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
                drop_children,
            } => {
                let Some(target) = NodeId::new_opt(*node_id) else {
                    return false;
                };
                let Some(node_kind) = parse_node_kind(kind) else {
                    return false;
                };
                // Pre-validate EVERYTHING before mutating (same
                // discipline as update_node + move_node — no
                // half-mutated state on a bad arg).
                if *width < 0 || *height < 0 {
                    return false;
                }
                let fill = match fill_hex {
                    None => None,
                    Some(hex) => match parse_hex_color(hex) {
                        Some(c) => Some(c),
                        None => return false,
                    },
                };
                // Resolve target + check the destructive-swap
                // guard BEFORE allocating an id. If the target
                // has children the caller MUST have set
                // `drop_children=true`; otherwise refuse the
                // swap so a Frame / Group can't silently lose
                // its subtree.
                let Some(target_node) = find_node_in_doc(self, target) else {
                    return false;
                };
                if !target_node.children.is_empty() && !*drop_children {
                    return false;
                }
                let Some(next_id) = self.next_node_id_seed() else {
                    return false;
                };
                let mut replacement = Node::leaf(next_id, node_kind, name.clone());
                replacement.bounds = crate::Rect::xywh(
                    *x as f32,
                    *y as f32,
                    *width as f32,
                    *height as f32,
                );
                replacement.fill = fill;
                replace_node_in_doc(self, target, replacement)
            }
            crate::mcp::McpCommand::MoveNode {
                node_id,
                target_parent_id,
            } => {
                let Some(source) = NodeId::new_opt(*node_id) else {
                    return false;
                };
                if source.raw() == *target_parent_id {
                    return false;
                }
                let target_parent = NodeId::new_opt(*target_parent_id);

                // Pre-validate EVERYTHING before detaching the node
                // (codex stop-gate: a bad target_parent_id would
                // cause detach → reattach-fail → silent drop of
                // the source node).
                //
                // 1. Source must exist.
                let Some(src_node) = find_node_in_doc(self, source) else {
                    return false;
                };
                // 2. If target is Some, it must resolve AND must
                //    not be a descendant of source (cycle).
                if let Some(target_id) = target_parent {
                    if subtree_contains(src_node, target_id) {
                        return false;
                    }
                    if find_node_in_doc(self, target_id).is_none() {
                        return false;
                    }
                } else {
                    // target_parent_id == 0 → page root. Active
                    // page must exist.
                    if self.pages.get(self.active_page_index).is_none() {
                        return false;
                    }
                }
                // All validation passed — detach + reattach is
                // now infallible.
                let Some(detached) = detach_node(self, source) else {
                    return false;
                };
                match target_parent {
                    None => {
                        let active_idx = self.active_page_index;
                        self.pages[active_idx].children.push(detached);
                    }
                    Some(pid) => {
                        // unwrap-OK: validated above; find_node_in_doc
                        // returned Some + nothing else mutates pages
                        // between the validation and here.
                        find_node_mut_in_doc(self, pid)
                            .expect("target validated")
                            .children
                            .push(detached);
                    }
                }
                true
            }
            crate::mcp::McpCommand::InstantiateComponent { component_id } => {
                let Some(target) = NodeId::new_opt(*component_id) else {
                    return false;
                };
                // Reuse the existing mutator. It handles fresh-id
                // allocation, history snapshot, + selection. We
                // need a mutable next_id seed; thread one based on
                // `max_node_id()`. The mutator advances it.
                let Some(mut next) = self.next_node_id_seed() else {
                    return false;
                };
                self.instantiate_component(target, &mut next).is_some()
            }
            crate::mcp::McpCommand::CreateComponent { node_id, name } => {
                let Some(target) = NodeId::new_opt(*node_id) else {
                    return false;
                };
                self.create_component_from_node(target, name.clone())
                    .is_some()
            }
            crate::mcp::McpCommand::DeleteComponent { component_id } => {
                let Some(target) = NodeId::new_opt(*component_id) else {
                    return false;
                };
                self.components.remove(target)
            }
            crate::mcp::McpCommand::RenameComponent { component_id, name } => {
                let Some(target) = NodeId::new_opt(*component_id) else {
                    return false;
                };
                self.components.rename(target, name.clone())
            }
            crate::mcp::McpCommand::BatchInsert { items } => {
                // Validate EVERY descriptor before any mutation.
                // A single bad entry rejects the entire batch so
                // callers can't end up with a partial design tree.
                if items.is_empty() {
                    return false;
                }
                let active_idx = self.active_page_index;
                if self.pages.get(active_idx).is_none() {
                    return false;
                }
                // Pre-validate kinds + geometry + fill_hex up front.
                struct Resolved {
                    kind: NodeKind,
                    name: String,
                    x: i32,
                    y: i32,
                    width: i32,
                    height: i32,
                    fill: Option<crate::Color>,
                }
                let mut resolved: Vec<Resolved> = Vec::with_capacity(items.len());
                for item in items {
                    let Some(kind) = parse_node_kind(&item.kind) else {
                        return false;
                    };
                    if item.width < 0 || item.height < 0 {
                        return false;
                    }
                    let fill = match &item.fill_hex {
                        None => None,
                        Some(hex) => match parse_hex_color(hex) {
                            Some(c) => Some(c),
                            None => return false,
                        },
                    };
                    resolved.push(Resolved {
                        kind,
                        name: item.name.clone(),
                        x: item.x,
                        y: item.y,
                        width: item.width,
                        height: item.height,
                        fill,
                    });
                }
                // Allocate fresh ids up front; bail if id space
                // would be exhausted partway through.
                let mut next_id = match self.next_node_id_seed() {
                    Some(n) => n,
                    None => return false,
                };
                let mut allocated_ids: Vec<u64> = Vec::with_capacity(resolved.len());
                for _ in 0..resolved.len() {
                    allocated_ids.push(next_id);
                    next_id = match next_id.checked_add(1) {
                        Some(n) => n,
                        None => return false,
                    };
                }
                // All validation + allocation passed — now mutate.
                let page = &mut self.pages[active_idx];
                for (r, id) in resolved.into_iter().zip(allocated_ids) {
                    let mut node = Node::leaf(id, r.kind, r.name);
                    node.bounds = crate::Rect::xywh(
                        r.x as f32,
                        r.y as f32,
                        r.width as f32,
                        r.height as f32,
                    );
                    if let Some(c) = r.fill {
                        node.fill = Some(c);
                    }
                    page.children.push(node);
                }
                true
            }
            _ => self.var_table.apply_mcp_command(cmd),
        }
    }

    /// Compute a fresh node id that won't collide with any existing
    /// node across pages. Returns `None` when `max_node_id()` is
    /// `u64::MAX` — a saturating add would wrap back to the live
    /// id and silently overwrite it (codex stop-gate). Callers
    /// surface the None as an apply-time failure.
    fn next_node_id_seed(&self) -> Option<u64> {
        let max = self.max_node_id();
        max.checked_add(1).map(|n| n.max(1))
    }
}
