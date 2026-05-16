//! Read-tool additions that didn't fit in `tools.rs` (already over
//! the 800-line cap). Each tool is a Document snapshot taken at
//! registration time, mirroring the spine in `tools.rs`.

use std::collections::BTreeMap;

use super::{McpTool, ToolErrorCode, ToolOutcome};

/// Immediate-children listing for a single container node, keyed
/// by parent id. Built at snapshot time so the registered tool
/// stays `&self`-only (same discipline as `GetNode`).
///
/// `known_ids` tracks every node id that exists in the document
/// (including leaves and empty containers) so callers can tell an
/// unknown id apart from a known node that just has no children —
/// the wire response is `count=0` for the latter, `ToolFailed` for
/// the former (codex stop-gate: previous impl conflated the two).
pub struct GetNodeChildren {
    pub children: BTreeMap<String, Vec<ChildRecord>>,
    pub known_ids: std::collections::BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct ChildRecord {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl McpTool for GetNodeChildren {
    fn name(&self) -> &str {
        "get_node_children"
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
        // `node_id` is the canonical `.op` schema string id.
        let id: &str = raw.as_str();
        if !self.known_ids.contains(id) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("node {id} not found"),
            );
        }
        // Empty children + known id ⇒ count=0 (NOT an error).
        // Codex stop-gate: previously this branch returned ToolFailed
        // for any node without children, conflating "leaf / empty
        // container" with "unknown id". LLM callers couldn't tell
        // the two apart and were forced into defensive double-checks.
        let empty: Vec<ChildRecord> = Vec::new();
        let records: &Vec<ChildRecord> = self.children.get(id).unwrap_or(&empty);
        let mut out = BTreeMap::new();
        out.insert("count".into(), records.len().to_string());
        // Comma-separated id list keeps the response wire-compatible
        // with the rest of the read tools (flat BTreeMap<String,
        // String> shape). LLMs can split + parse downstream.
        let ids: Vec<String> = records.iter().map(|r| r.id.to_string()).collect();
        out.insert("ids".into(), ids.join(","));
        for (i, rec) in records.iter().enumerate() {
            let prefix = format!("child_{i}");
            out.insert(format!("{prefix}_id"), rec.id.to_string());
            out.insert(format!("{prefix}_kind"), rec.kind.clone());
            out.insert(format!("{prefix}_name"), rec.name.clone());
            out.insert(format!("{prefix}_x"), rec.x.to_string());
            out.insert(format!("{prefix}_y"), rec.y.to_string());
            out.insert(format!("{prefix}_width"), rec.width.to_string());
            out.insert(format!("{prefix}_height"), rec.height.to_string());
        }
        ToolOutcome::Ok(out)
    }
}

pub fn get_node_children_snapshot(doc: &crate::document::Document) -> GetNodeChildren {
    let mut children: BTreeMap<String, Vec<ChildRecord>> = BTreeMap::new();
    let mut known_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for page in &doc.pages {
        for node in &page.children {
            walk(node, &mut children, &mut known_ids);
        }
    }
    GetNodeChildren {
        children,
        known_ids,
    }
}

fn walk(
    node: &crate::document::Node,
    out: &mut BTreeMap<String, Vec<ChildRecord>>,
    known: &mut std::collections::BTreeSet<String>,
) {
    known.insert(node.id.as_str().to_string());
    if node.children.is_empty() {
        return;
    }
    let parent_id = node.id.as_str().to_string();
    let mut records = Vec::with_capacity(node.children.len());
    for child in &node.children {
        let bounds = child.aggregate_bounds();
        records.push(ChildRecord {
            id: child.id.as_str().to_string(),
            kind: kind_label(&child.kind).into(),
            name: child.name.clone(),
            x: bounds.origin.x as i32,
            y: bounds.origin.y as i32,
            width: bounds.size.x as i32,
            height: bounds.size.y as i32,
        });
        walk(child, out, known);
    }
    out.insert(parent_id, records);
}

fn kind_label(kind: &crate::document::NodeKind) -> &'static str {
    match kind {
        crate::document::NodeKind::Frame => "frame",
        crate::document::NodeKind::Group => "group",
        crate::document::NodeKind::Rect => "rect",
        crate::document::NodeKind::Ellipse => "ellipse",
        crate::document::NodeKind::Polygon => "polygon",
        crate::document::NodeKind::Line => "line",
        crate::document::NodeKind::Text => "text",
        crate::document::NodeKind::Path => "path",
        crate::document::NodeKind::Other(_) => "other",
    }
}
