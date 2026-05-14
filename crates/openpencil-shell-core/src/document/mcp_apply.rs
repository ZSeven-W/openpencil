//! MCP-write command application against the document. Pulled out
//! of `mutators.rs` so that file stays under the 800-line cap as
//! more write commands land.

use super::variables::parse_hex_color;
use super::{Document, Node, NodeKind};

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
