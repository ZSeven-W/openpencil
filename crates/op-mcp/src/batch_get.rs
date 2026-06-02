//! TS-compatible `batch_get` document read/search tool.

use std::collections::{BTreeMap, BTreeSet};

use jian_ops_schema::node::PenNode;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::walkers::find_node;
use op_editor_core::{EditorState, NodeId};
use regex::{Regex, RegexBuilder};
use serde_json::Value;

use super::read_nodes::{node_snapshot_value, page_nodes_snapshots, PageNodes};
use super::{McpTool, ToolErrorCode, ToolOutcome};

type ToolCallError = (ToolErrorCode, String);

pub struct BatchGet {
    pages: Vec<PageNodes>,
    active_page_id: String,
}

struct SearchPattern {
    node_type: Option<String>,
    name: Option<Regex>,
    reusable: Option<bool>,
}

impl McpTool for BatchGet {
    fn name(&self) -> &str {
        "batch_get"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if matches!(
            arg_alias(args, &["resolveRefs", "resolve_refs"]),
            Some("true")
        ) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "resolveRefs is not supported by Rust MCP batch_get yet".into(),
            );
        }
        let page = match self.page_nodes(args) {
            Ok(page) => page,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let read_depth = match parse_i32_arg(args, &["readDepth", "read_depth", "depth"], 1) {
            Ok(depth) => depth,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let search_depth = match parse_search_depth(args) {
            Ok(depth) => depth,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let patterns = match parse_patterns(args) {
            Ok(patterns) => patterns,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let node_ids = match parse_node_ids(args) {
            Ok(ids) => ids,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let parent_id = arg_alias(args, &["parentId", "parent_id", "parent"]);
        let search_roots = search_roots(&page.roots, parent_id);
        let mut nodes: Vec<&PenNode> = Vec::new();
        let mut seen = BTreeSet::new();

        if patterns.is_empty() && node_ids.is_empty() {
            nodes.extend(search_roots.iter());
        } else {
            for pattern in &patterns {
                match collect_matches(
                    search_roots,
                    pattern,
                    search_depth,
                    0,
                    &mut seen,
                    &mut nodes,
                ) {
                    Ok(()) => {}
                    Err((code, msg)) => return ToolOutcome::Err(code, msg),
                }
            }
            for id in &node_ids {
                if seen.contains(id) {
                    continue;
                }
                if let Some(nid) = NodeId::new_opt(id) {
                    if let Some(node) = find_node(&page.roots, &nid) {
                        seen.insert(id.clone());
                        nodes.push(node);
                    }
                }
            }
        }

        let values: Vec<Value> = nodes
            .iter()
            .map(|node| node_snapshot_value(node, read_depth))
            .collect();
        let nodes_json = match serde_json::to_string(&values) {
            Ok(json) => json,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("serialize nodes failed: {e}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("count".into(), values.len().to_string());
        out.insert("nodes".into(), nodes_json);
        ToolOutcome::Ok(out)
    }
}

pub fn batch_get_snapshot(state: &EditorState) -> BatchGet {
    let (pages, active_page_id) = page_nodes_snapshots(state);
    BatchGet {
        pages,
        active_page_id,
    }
}

impl BatchGet {
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

fn search_roots<'a>(roots: &'a [PenNode], parent_id: Option<&str>) -> &'a [PenNode] {
    let Some(parent_id) = parent_id else {
        return roots;
    };
    NodeId::new_opt(parent_id)
        .and_then(|id| find_node(roots, &id))
        .and_then(PenNodeExt::children)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn collect_matches<'a>(
    roots: &'a [PenNode],
    pattern: &SearchPattern,
    max_depth: Option<usize>,
    current_depth: usize,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<&'a PenNode>,
) -> Result<(), ToolCallError> {
    if max_depth.is_some_and(|max| current_depth > max) {
        return Ok(());
    }
    for node in roots {
        if matches_pattern(node, pattern) {
            let id = node.id_str().to_string();
            if seen.insert(id) {
                out.push(node);
            }
        }
        if let Some(children) = node.children() {
            collect_matches(children, pattern, max_depth, current_depth + 1, seen, out)?;
        }
    }
    Ok(())
}

fn matches_pattern(node: &PenNode, pattern: &SearchPattern) -> bool {
    if let Some(expected) = &pattern.node_type {
        if node_type(node) != normalize_node_type(expected) {
            return false;
        }
    }
    if let Some(name) = &pattern.name {
        if !name.is_match(node.base().name.as_deref().unwrap_or("")) {
            return false;
        }
    }
    if let Some(reusable) = pattern.reusable {
        if node_reusable(node) != reusable {
            return false;
        }
    }
    true
}

fn parse_patterns(args: &BTreeMap<String, String>) -> Result<Vec<SearchPattern>, ToolCallError> {
    let Some(raw) = arg_alias(args, &["patterns"]) else {
        return Ok(Vec::new());
    };
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        (
            ToolErrorCode::InvalidArgument,
            format!("patterns must be a JSON array: {e}"),
        )
    })?;
    let Value::Array(items) = value else {
        return Err((
            ToolErrorCode::InvalidArgument,
            "patterns must be a JSON array".into(),
        ));
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Value::Object(map) = item else {
            return Err((
                ToolErrorCode::InvalidArgument,
                "each pattern must be an object".into(),
            ));
        };
        let name = optional_string(&map, "name")?
            .map(|raw| RegexBuilder::new(&raw).case_insensitive(true).build())
            .transpose()
            .map_err(|e| {
                (
                    ToolErrorCode::InvalidArgument,
                    format!("pattern name must be a valid regex: {e}"),
                )
            })?;
        out.push(SearchPattern {
            node_type: optional_string(&map, "type")?,
            name,
            reusable: optional_bool(&map, "reusable")?,
        });
    }
    Ok(out)
}

fn parse_node_ids(args: &BTreeMap<String, String>) -> Result<Vec<String>, ToolCallError> {
    let Some(raw) = arg_alias(args, &["nodeIds", "node_ids", "ids"]) else {
        return Ok(Vec::new());
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<String>>(trimmed).map_err(|e| {
            (
                ToolErrorCode::InvalidArgument,
                format!("nodeIds must be a JSON string array: {e}"),
            )
        });
    }
    Ok(trimmed
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect())
}

fn parse_search_depth(args: &BTreeMap<String, String>) -> Result<Option<usize>, ToolCallError> {
    let Some(raw) = arg_alias(args, &["searchDepth", "search_depth"]) else {
        return Ok(None);
    };
    if raw.eq_ignore_ascii_case("infinity") {
        return Ok(None);
    }
    raw.parse::<usize>().map(Some).map_err(|_| {
        (
            ToolErrorCode::InvalidArgument,
            format!("searchDepth must be a non-negative integer, got {raw:?}"),
        )
    })
}

fn parse_i32_arg(
    args: &BTreeMap<String, String>,
    keys: &[&str],
    default: i32,
) -> Result<i32, ToolCallError> {
    let Some(raw) = arg_alias(args, keys) else {
        return Ok(default);
    };
    raw.parse::<i32>().map_err(|_| {
        (
            ToolErrorCode::InvalidArgument,
            format!("{} must be an i32, got {raw:?}", keys[0]),
        )
    })
}

fn optional_string(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<String>, ToolCallError> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        _ => Err((
            ToolErrorCode::InvalidArgument,
            format!("pattern {key} must be a string"),
        )),
    }
}

fn optional_bool(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<bool>, ToolCallError> {
    match map.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(v)) => Ok(Some(*v)),
        _ => Err((
            ToolErrorCode::InvalidArgument,
            format!("pattern {key} must be a boolean"),
        )),
    }
}

fn node_type(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text_input",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon_font",
        PenNode::Ref(_) => "ref",
    }
}

fn normalize_node_type(raw: &str) -> &str {
    match raw {
        "rect" => "rectangle",
        "textInput" => "text_input",
        "iconFont" => "icon_font",
        other => other,
    }
}

fn node_reusable(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(frame) if frame.reusable == Some(true))
}

fn arg_alias<'a>(args: &'a BTreeMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| args.get(*key).map(String::as_str))
}
