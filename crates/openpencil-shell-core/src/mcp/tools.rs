//! First-party MCP tools. Each tool is a snapshot-at-registration
//! struct: the host snapshots the relevant slice of the document on
//! every state change + re-registers the tool. `McpTool::call` then
//! formats the cached state without re-walking the tree.
//!
//! Pulled out of `mcp.rs` to honor the 800-line cap as the tool
//! surface grows. New first-party tools land in this file.

use std::collections::BTreeMap;

use super::{McpTool, ToolErrorCode, ToolOutcome};

pub struct GetDocumentInfo {
    pub page_count: usize,
    pub active_page_index: usize,
    pub total_nodes: usize,
}

impl McpTool for GetDocumentInfo {
    fn name(&self) -> &str {
        "get_document_info"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("page_count".into(), self.page_count.to_string());
        out.insert(
            "active_page_index".into(),
            self.active_page_index.to_string(),
        );
        out.insert("total_nodes".into(), self.total_nodes.to_string());
        ToolOutcome::Ok(out)
    }
}

/// Snapshot the document into a `GetDocumentInfo` tool. Counts all
/// nodes recursively across every page. The MCP server registers
/// one of these per document; replays on every `get_document_info`
/// call without re-walking the tree.
pub fn document_info_snapshot(doc: &crate::document::Document) -> GetDocumentInfo {
    let total_nodes: usize = doc
        .pages
        .iter()
        .map(|p| p.children.iter().map(count_subtree).sum::<usize>())
        .sum();
    GetDocumentInfo {
        page_count: doc.pages.len(),
        active_page_index: doc.active_page_index,
        total_nodes,
    }
}

fn count_subtree(n: &crate::document::Node) -> usize {
    1 + n.children.iter().map(count_subtree).sum::<usize>()
}

/// First-party `get_selection` tool — reports the currently selected
/// node's id, kind, and bounds. Empty fields when nothing is
/// selected (id="0", kind="none"). Snapshot pattern matches
/// `GetDocumentInfo` so the host can re-register on selection
/// change.
pub struct GetSelection {
    pub selected_id: u64,
    pub kind: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl McpTool for GetSelection {
    fn name(&self) -> &str {
        "get_selection"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("selected_id".into(), self.selected_id.to_string());
        out.insert("kind".into(), self.kind.clone());
        out.insert("x".into(), self.x.to_string());
        out.insert("y".into(), self.y.to_string());
        out.insert("width".into(), self.width.to_string());
        out.insert("height".into(), self.height.to_string());
        ToolOutcome::Ok(out)
    }
}

/// Snapshot the document selection into a `GetSelection` tool. When
/// nothing's selected returns an `id=0, kind="none"` placeholder so
/// LLM clients can distinguish "no selection" from a parse error.
pub fn selection_snapshot(doc: &crate::document::Document) -> GetSelection {
    let selected_id = doc.selected.raw();
    if selected_id == 0 {
        return GetSelection {
            selected_id: 0,
            kind: "none".into(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
    }
    if let Some(node) = doc.selected_node() {
        let bounds = node.aggregate_bounds();
        let kind_label = match &node.kind {
            crate::document::NodeKind::Frame => "frame",
            crate::document::NodeKind::Group => "group",
            crate::document::NodeKind::Rect => "rect",
            crate::document::NodeKind::Ellipse => "ellipse",
            crate::document::NodeKind::Polygon => "polygon",
            crate::document::NodeKind::Line => "line",
            crate::document::NodeKind::Text => "text",
            crate::document::NodeKind::Path => "path",
            crate::document::NodeKind::Other(_) => "other",
        };
        GetSelection {
            selected_id,
            kind: kind_label.into(),
            x: bounds.origin.x as i32,
            y: bounds.origin.y as i32,
            width: bounds.size.x as i32,
            height: bounds.size.y as i32,
        }
    } else {
        GetSelection {
            selected_id,
            kind: "missing".into(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

/// First-party `list_pages` tool — reports the page count, active
/// index, and a comma-separated list of page names. LLM clients use
/// this to pick a target page for `insert_node` / `batch_design`.
pub struct ListPages {
    pub page_count: usize,
    pub active_page_index: usize,
    pub names: String,
}

impl McpTool for ListPages {
    fn name(&self) -> &str {
        "list_pages"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("page_count".into(), self.page_count.to_string());
        out.insert(
            "active_page_index".into(),
            self.active_page_index.to_string(),
        );
        out.insert("names".into(), self.names.clone());
        ToolOutcome::Ok(out)
    }
}

pub fn list_pages_snapshot(doc: &crate::document::Document) -> ListPages {
    let names = doc
        .pages
        .iter()
        .map(|p| p.name.clone())
        .collect::<Vec<_>>()
        .join(",");
    ListPages {
        page_count: doc.pages.len(),
        active_page_index: doc.active_page_index,
        names,
    }
}

/// First-party `get_node` tool — given a `node_id` argument
/// (decimal string), returns kind / bounds / parent. LLM clients
/// use this after `get_document_info` + `list_pages` to drill into
/// the specific node they're about to modify. The snapshot pattern
/// pre-computes a map of every node id → its details so calls are
/// O(1) lookup; the host re-registers on document mutations.
pub struct GetNode {
    pub nodes: BTreeMap<u64, NodeRecord>,
}

#[derive(Debug, Clone)]
pub struct NodeRecord {
    pub kind: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub parent_id: u64,
}

impl McpTool for GetNode {
    fn name(&self) -> &str {
        "get_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let raw = match args.get("node_id") {
            Some(s) => s,
            None => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "node_id is required".into(),
                );
            }
        };
        let id: u64 = match raw.parse() {
            Ok(n) => n,
            Err(_) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a decimal u64, got {raw:?}"),
                );
            }
        };
        let Some(rec) = self.nodes.get(&id) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("node {id} not found"),
            );
        };
        let mut out = BTreeMap::new();
        out.insert("kind".into(), rec.kind.clone());
        out.insert("name".into(), rec.name.clone());
        out.insert("x".into(), rec.x.to_string());
        out.insert("y".into(), rec.y.to_string());
        out.insert("width".into(), rec.width.to_string());
        out.insert("height".into(), rec.height.to_string());
        out.insert("parent_id".into(), rec.parent_id.to_string());
        ToolOutcome::Ok(out)
    }
}

pub fn get_node_snapshot(doc: &crate::document::Document) -> GetNode {
    let mut nodes: BTreeMap<u64, NodeRecord> = BTreeMap::new();
    for page in &doc.pages {
        for node in &page.children {
            walk_node(node, 0, &mut nodes);
        }
    }
    GetNode { nodes }
}

fn walk_node(
    node: &crate::document::Node,
    parent_id: u64,
    out: &mut BTreeMap<u64, NodeRecord>,
) {
    let bounds = node.aggregate_bounds();
    let kind_label = match &node.kind {
        crate::document::NodeKind::Frame => "frame",
        crate::document::NodeKind::Group => "group",
        crate::document::NodeKind::Rect => "rect",
        crate::document::NodeKind::Ellipse => "ellipse",
        crate::document::NodeKind::Polygon => "polygon",
        crate::document::NodeKind::Line => "line",
        crate::document::NodeKind::Text => "text",
        crate::document::NodeKind::Path => "path",
        crate::document::NodeKind::Other(_) => "other",
    };
    out.insert(
        node.id.raw(),
        NodeRecord {
            kind: kind_label.into(),
            name: node.name.clone(),
            x: bounds.origin.x as i32,
            y: bounds.origin.y as i32,
            width: bounds.size.x as i32,
            height: bounds.size.y as i32,
            parent_id,
        },
    );
    for child in &node.children {
        walk_node(child, node.id.raw(), out);
    }
}
