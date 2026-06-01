//! MCP move/copy tools.

use std::collections::BTreeMap;

use op_editor_core::NodeId;

use crate::write_tools::root_or_node_id;
use crate::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};

/// First-party `move_node` tool — reparent a node. An empty
/// `target_parent_id` reparents to the active page root.
pub struct MoveNode;

impl McpTool for MoveNode {
    fn name(&self) -> &str {
        "move_node"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id_any(args, &["node_id", "nodeId"], "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let target_parent = match parse_target_parent_arg(args) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if target_parent.is_real() && target_parent == node_id {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "node_id and target_parent_id must differ".into(),
            );
        }
        let index = match parse_optional_usize_arg(args, "index") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let page_id = optional_page_id(args);
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::MoveNode {
                node_id,
                target_parent,
                page_id,
                index,
            },
        )
    }
}

pub fn move_node_snapshot() -> MoveNode {
    MoveNode
}

/// First-party `copy_node` tool — deep-clone a node + subtree under a
/// new parent.
pub struct CopyNode;

impl McpTool for CopyNode {
    fn name(&self) -> &str {
        "copy_node"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id_any(args, &["sourceId", "node_id", "nodeId"], "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let target_parent = match parse_target_parent_arg(args) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let page_id = optional_page_id(args);
        let overrides_json = args
            .get("overrides")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::CopyNode {
                node_id,
                target_parent,
                overrides_json,
                page_id,
            },
        )
    }
}

pub fn copy_node_snapshot() -> CopyNode {
    CopyNode
}

#[allow(clippy::result_large_err)]
fn parse_node_id_any(
    args: &BTreeMap<String, String>,
    keys: &[&str],
    message_key: &str,
) -> Result<NodeId, ToolOutcome> {
    for key in keys {
        if let Some(raw) = args.get(*key) {
            return NodeId::new_opt(raw.as_str()).ok_or_else(|| {
                ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("{key} must be a non-empty node id"),
                )
            });
        }
    }
    Err(ToolOutcome::Err(
        ToolErrorCode::MissingArgument,
        format!("{message_key} is required"),
    ))
}

#[allow(clippy::result_large_err)]
fn parse_target_parent_arg(args: &BTreeMap<String, String>) -> Result<NodeId, ToolOutcome> {
    let Some(raw_target) = args
        .get("parent")
        .or_else(|| args.get("parent_id"))
        .or_else(|| args.get("target_parent_id"))
    else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::MissingArgument,
            "target_parent_id is required (\"\" or \"0\" = page root)".into(),
        ));
    };
    Ok(root_or_node_id(raw_target))
}

#[allow(clippy::result_large_err)]
fn parse_optional_usize_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<usize>, ToolOutcome> {
    let Some(raw) = args.get(key) else {
        return Ok(None);
    };
    raw.parse::<usize>().map(Some).map_err(|_| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a non-negative integer, got {raw:?}"),
        )
    })
}

fn optional_page_id(args: &BTreeMap<String, String>) -> Option<String> {
    args.get("pageId")
        .or_else(|| args.get("page_id"))
        .or_else(|| args.get("page"))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}
