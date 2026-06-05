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
pub(crate) struct PageNodes {
    pub(crate) id: String,
    pub(crate) roots: Vec<PenNode>,
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
        // Match TS read-nodes EXACTLY: { nodes, variables?, themes? } with
        // NATIVE JSON values and NO Rust-only `count` key (TS ReadNodesResult
        // has none; cf. batch_get which also dropped `count`). The old path
        // stringified the nodes/variables/themes arrays into String values.
        let mut out = serde_json::Map::new();
        out.insert("nodes".into(), Value::Array(nodes));
        if include_variables {
            let variables: Value = serde_json::from_str(&self.variables_json)
                .unwrap_or_else(|_| serde_json::json!({}));
            let themes: Value =
                serde_json::from_str(&self.themes_json).unwrap_or_else(|_| serde_json::json!([]));
            out.insert("variables".into(), variables);
            out.insert("themes".into(), themes);
        }
        ToolOutcome::OkJson(Value::Object(out).to_string())
    }
}

pub fn read_nodes_snapshot(state: &EditorState) -> ReadNodes {
    let (pages, active_page_id) = page_nodes_snapshots(state);
    // Match TS read-nodes EXACTLY (handleReadNodes): variables = doc.variables
    // ?? {} (absent → empty OBJECT); themes = doc.themes ?? [] (absent → empty
    // ARRAY, NOT an empty object). `unwrap_or_default()` would have collapsed an
    // absent themes map to `{}`, diverging from TS's `[]` for variable-only docs.
    let variables_json = match state.doc.variables.as_ref() {
        Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "{}".into()),
        None => "{}".into(),
    };
    let themes_json = match state.doc.themes.as_ref() {
        Some(t) => serde_json::to_string(t).unwrap_or_else(|_| "[]".into()),
        None => "[]".into(),
    };
    ReadNodes {
        pages,
        active_page_id,
        variables_json,
        themes_json,
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

pub(crate) fn page_nodes_snapshots(state: &EditorState) -> (Vec<PageNodes>, String) {
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

pub(crate) fn node_snapshot_value(node: &PenNode, depth: i32) -> Value {
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
