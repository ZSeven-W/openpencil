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
                // Pre-validate fill_hex BEFORE the mutable borrow,
                // so an invalid hex doesn't partially apply.
                let fill = match fill_hex {
                    None => None,
                    Some(hex) => match parse_hex_color(hex) {
                        Some(c) => Some(c),
                        None => return false,
                    },
                };
                let Some(node) = find_node_mut_in_doc(self, target) else {
                    return false;
                };
                if let Some(nx) = x {
                    node.bounds.origin.x = *nx as f32;
                }
                if let Some(ny) = y {
                    node.bounds.origin.y = *ny as f32;
                }
                if let Some(nw) = width {
                    if *nw < 0 {
                        return false;
                    }
                    node.bounds.size.x = *nw as f32;
                }
                if let Some(nh) = height {
                    if *nh < 0 {
                        return false;
                    }
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
