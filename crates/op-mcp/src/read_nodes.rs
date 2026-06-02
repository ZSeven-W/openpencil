//! TS-compatible `read_nodes` tool for codegen and inspection flows.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::walkers::find_node;
use op_editor_core::{EditorState, NodeId};
use serde_json::Value;

use super::{McpTool, ToolErrorCode, ToolOutcome};

type ToolCallError = (ToolErrorCode, String);

pub struct ReadNodes {
    pages: Vec<PageNodes>,
    active_page_id: String,
    variables_json: String,
    themes_json: String,
}

#[derive(Clone)]
struct PageNodes {
    id: String,
    roots: Vec<PenNode>,
}

impl McpTool for ReadNodes {
    fn name(&self) -> &str {
        "read_nodes"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let page = match self.page_nodes(args) {
            Ok(page) => page,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let depth = match parse_depth(args) {
            Ok(depth) => depth,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let include_variables = match parse_include_variables(args) {
            Ok(v) => v,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let node_ids = match parse_node_ids(args) {
            Ok(ids) => ids,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };

        let selected: Vec<&PenNode> = match node_ids {
            Some(ids) if !ids.is_empty() => ids
                .iter()
                .filter_map(|id| NodeId::new_opt(id).and_then(|nid| find_node(&page.roots, &nid)))
                .collect(),
            _ => page.roots.iter().collect(),
        };
        let nodes: Vec<Value> = selected
            .iter()
            .map(|node| node_snapshot_value(node, depth))
            .collect();
        let nodes_json = match serde_json::to_string(&nodes) {
            Ok(json) => json,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("serialize nodes failed: {e}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("count".into(), nodes.len().to_string());
        out.insert("nodes".into(), nodes_json);
        if include_variables {
            out.insert("variables".into(), self.variables_json.clone());
            out.insert("themes".into(), self.themes_json.clone());
        }
        ToolOutcome::Ok(out)
    }
}

pub fn read_nodes_snapshot(state: &EditorState) -> ReadNodes {
    let (pages, active_page_id) = page_nodes_snapshots(state);
    let variables = state.doc.variables.clone().unwrap_or_default();
    let themes = state.doc.themes.clone().unwrap_or_default();
    ReadNodes {
        pages,
        active_page_id,
        variables_json: serde_json::to_string(&variables).unwrap_or_else(|_| "{}".into()),
        themes_json: serde_json::to_string(&themes).unwrap_or_else(|_| "{}".into()),
    }
}

impl ReadNodes {
    fn page_nodes(&self, args: &BTreeMap<String, String>) -> Result<&PageNodes, ToolCallError> {
        let target =
            arg_alias(args, &["pageId", "page_id", "page"]).unwrap_or(&self.active_page_id);
        self.pages
            .iter()
            .find(|page| page.id == target)
            .ok_or_else(|| {
                (
                    ToolErrorCode::ToolFailed,
                    format!("page not found: {target}"),
                )
            })
    }
}

fn page_nodes_snapshots(state: &EditorState) -> (Vec<PageNodes>, String) {
    match state.doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => {
            let active_idx = state
                .ui
                .active_page_index
                .min(pages.len().saturating_sub(1));
            let active_page_id = pages[active_idx].id.clone();
            let pages = pages
                .iter()
                .map(|page| PageNodes {
                    id: page.id.clone(),
                    roots: page.children.clone(),
                })
                .collect();
            (pages, active_page_id)
        }
        _ => (
            vec![PageNodes {
                id: "0".into(),
                roots: state.doc.children.clone(),
            }],
            "0".into(),
        ),
    }
}

fn parse_depth(args: &BTreeMap<String, String>) -> Result<i32, ToolCallError> {
    match args.get("depth") {
        None => Ok(-1),
        Some(raw) => raw.parse::<i32>().map_err(|_| {
            (
                ToolErrorCode::InvalidArgument,
                format!("depth must be an i32, got {raw:?}"),
            )
        }),
    }
}

fn parse_include_variables(args: &BTreeMap<String, String>) -> Result<bool, ToolCallError> {
    match arg_alias(args, &["includeVariables", "include_variables", "vars"]) {
        None => Ok(false),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(raw) => Err((
            ToolErrorCode::InvalidArgument,
            format!("includeVariables must be true or false, got {raw:?}"),
        )),
    }
}

fn parse_node_ids(args: &BTreeMap<String, String>) -> Result<Option<Vec<String>>, ToolCallError> {
    let Some(raw) = arg_alias(args, &["nodeIds", "node_ids", "ids"]) else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Some(Vec::new()));
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<String>>(trimmed)
            .map(Some)
            .map_err(|e| {
                (
                    ToolErrorCode::InvalidArgument,
                    format!("nodeIds must be a JSON string array: {e}"),
                )
            });
    }
    Ok(Some(
        trimmed
            .split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .collect(),
    ))
}

fn node_snapshot_value(node: &PenNode, depth: i32) -> Value {
    let mut value = serde_json::to_value(node).unwrap_or(Value::Null);
    if depth != -1 {
        truncate_children(&mut value, depth);
    }
    value
}

fn truncate_children(value: &mut Value, depth: i32) {
    let Value::Object(map) = value else {
        return;
    };
    let Some(children) = map.get_mut("children") else {
        return;
    };
    let Value::Array(items) = children else {
        return;
    };
    if items.is_empty() {
        return;
    }
    if depth <= 0 {
        *children = Value::String("...".into());
    } else {
        for child in items {
            truncate_children(child, depth - 1);
        }
    }
}

fn arg_alias<'a>(args: &'a BTreeMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| args.get(*key).map(String::as_str))
}
