//! Read-tool additions that didn't fit in `tools.rs`. Each tool is an
//! `EditorState` snapshot taken at registration time, mirroring the
//! spine in `tools.rs`.

use std::collections::BTreeMap;

use op_editor_core::geometry::aggregate_bounds;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;
use jian_ops_schema::node::PenNode;

use super::tools::kind_label;
use super::{McpTool, ToolErrorCode, ToolOutcome};

/// Immediate-children listing for a single container node, keyed by
/// parent id. Built at snapshot time so the registered tool stays
/// `&self`-only (same discipline as `GetNode`).
///
/// `known_ids` tracks every node id that exists in the document so
/// callers can tell an unknown id apart from a known node that just
/// has no children — `count=0` for the latter, `ToolFailed` for the
/// former.
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
        let id: &str = raw.as_str();
        if !self.known_ids.contains(id) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("node {id} not found"),
            );
        }
        // Empty children + known id ⇒ count=0 (NOT an error).
        let empty: Vec<ChildRecord> = Vec::new();
        let records: &Vec<ChildRecord> = self.children.get(id).unwrap_or(&empty);
        let mut out = BTreeMap::new();
        out.insert("count".into(), records.len().to_string());
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

pub fn get_node_children_snapshot(state: &EditorState) -> GetNodeChildren {
    let mut children: BTreeMap<String, Vec<ChildRecord>> = BTreeMap::new();
    let mut known_ids: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    match state.doc.pages.as_ref() {
        Some(pages) => {
            for page in pages {
                for node in &page.children {
                    walk(node, &mut children, &mut known_ids);
                }
            }
        }
        None => {
            for node in &state.doc.children {
                walk(node, &mut children, &mut known_ids);
            }
        }
    }
    GetNodeChildren {
        children,
        known_ids,
    }
}

fn walk(
    node: &PenNode,
    out: &mut BTreeMap<String, Vec<ChildRecord>>,
    known: &mut std::collections::BTreeSet<String>,
) {
    known.insert(node.id_str().to_string());
    let Some(node_children) = node.children() else {
        return;
    };
    if node_children.is_empty() {
        return;
    }
    let parent_id = node.id_str().to_string();
    let mut records = Vec::with_capacity(node_children.len());
    for child in node_children {
        let b = aggregate_bounds(child);
        records.push(ChildRecord {
            id: child.id_str().to_string(),
            kind: kind_label(child).into(),
            name: child.base().name.clone().unwrap_or_default(),
            x: b.x as i32,
            y: b.y as i32,
            width: b.w as i32,
            height: b.h as i32,
        });
        walk(child, out, known);
    }
    out.insert(parent_id, records);
}
