//! MCP write tools. Each returns `ToolOutcome::OkWithCommand(...)` so
//! the host applies the mutation against the live editor state via
//! `EditorState::apply`.
//!
//! Ported off shell-core's `McpCommand` onto `op_editor_core::
//! EditorCommand`. The biggest model change: node ids are now the
//! canonical `.op` schema's string ids (`NodeId`), not the old `u64`.
//! `parse_node_id` accepts any non-empty string.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::NodeId;
use serde_json::Value;

use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};
use crate::insert_node_args::{insert_node_params, InsertNodeParams};
use crate::insert_node_data::{ts_data_node, ts_replacement_data_node};
use crate::update_node_data::ts_update_patch_json;

pub use crate::write_tools_import_svg::{import_svg_snapshot, ImportSvg};
pub use crate::write_tools_variables::{
    set_active_axis_value_snapshot, set_variable_color_snapshot, SetActiveAxisValue,
    SetVariableColor,
};

/// Parse a `node_id`-style argument into a `NodeId`. Node ids are
/// canonical `.op` schema strings — any non-empty string is valid; an
/// empty string (the NONE sentinel) is rejected.
// `ToolOutcome` is the shared MCP outcome type — boxing it broadly to
// shrink the `Err` variant would destabilize every tool signature.
#[allow(clippy::result_large_err)]
pub(super) fn parse_node_id(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<NodeId, ToolOutcome> {
    let Some(raw) = args.get(key) else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::MissingArgument,
            format!("{key} is required"),
        ));
    };
    NodeId::new_opt(raw.as_str()).ok_or_else(|| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a non-empty node id"),
        )
    })
}

/// First-party `insert_node` tool — creates a fresh node on the active
/// page. The applier allocates a non-colliding id.
pub struct InsertNode;

impl McpTool for InsertNode {
    fn name(&self) -> &str {
        "insert_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        match ts_data_tree_command(args) {
            Ok(Some(command)) => {
                let mut out = BTreeMap::new();
                out.insert("wrote".into(), "true".into());
                out.insert("count".into(), "1".into());
                return ToolOutcome::OkWithCommand(out, command);
            }
            Ok(None) => {}
            Err(e) => return e,
        }
        let params = match insert_node_params(args) {
            Ok(params) => params,
            Err(e) => return e,
        };
        let InsertNodeParams {
            kind,
            name,
            x,
            y,
            width,
            height,
            fill_hex,
        } = params;
        if !ALLOWED_KINDS.iter().any(|k| *k == kind) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!(
                    "kind {kind:?} not supported; allowed: {}",
                    ALLOWED_KINDS.join(", ")
                ),
            );
        }
        if width < 0 || height < 0 {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "width / height must be non-negative".into(),
            );
        }
        if let Some(hex) = fill_hex.as_deref() {
            if !validate_hex(hex) {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {hex:?}"),
                );
            }
        }
        let target_parent = args
            .get("parent")
            .or_else(|| args.get("parent_id"))
            .or_else(|| args.get("target_parent_id"))
            .map(|s| root_or_node_id(s))
            .unwrap_or(NodeId::NONE);
        let page_id = args
            .get("pageId")
            .or_else(|| args.get("page_id"))
            .or_else(|| args.get("page"))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::InsertNode {
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
                target_parent,
                page_id,
            },
        )
    }
}

pub(super) const ALLOWED_KINDS: &[&str] = &[
    "frame",
    "group",
    "rect",
    "ellipse",
    "polygon",
    "line",
    "text",
    "path",
    // Interactive widget family (built by op-editor-core's widget factory).
    "text_input",
    "text_area",
    "number_input",
    "select",
    "radio_group",
    "switch",
    "checkbox",
    "slider",
    "progress",
    "tabs",
    // Component instance: a `ref` node points at a reusable master by id and
    // renders the master's subtree (resolved by `ref_resolve` before variables
    // + layout). Carries no fill/children of its own — those are inherited.
    "ref",
];

pub fn insert_node_snapshot() -> InsertNode {
    InsertNode
}

#[allow(clippy::result_large_err)]
fn ts_data_tree_command(
    args: &BTreeMap<String, String>,
) -> Result<Option<EditorCommand>, ToolOutcome> {
    let Some(node) = ts_data_tree_node(args)? else {
        return Ok(None);
    };
    let parent_id = args
        .get("parent")
        .or_else(|| args.get("parent_id"))
        .or_else(|| args.get("target_parent_id"))
        .map(|s| root_or_node_id(s))
        .unwrap_or(NodeId::NONE);
    let page_id = args
        .get("pageId")
        .or_else(|| args.get("page_id"))
        .or_else(|| args.get("page"))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let mut nodes = vec![node];
    let hoist = super::batch_design::hoist_generation_state(&mut nodes);
    Ok(Some(super::batch_design::with_hoisted_state(
        hoist,
        EditorCommand::InsertSubtree {
            nodes,
            parent_id,
            page_id,
        },
    )))
}

#[allow(clippy::result_large_err)]
fn ts_data_tree_node(args: &BTreeMap<String, String>) -> Result<Option<PenNode>, ToolOutcome> {
    ts_data_node(args)
}

/// First-party `update_node` tool — patch fields on an existing node.
pub struct UpdateNode;

impl McpTool for UpdateNode {
    fn name(&self) -> &str {
        "update_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id_alias(args, "node_id", "nodeId") {
            Ok(v) => v,
            Err(e) => return e,
        };
        match ts_update_patch_json(args) {
            Ok(Some(patch_json)) => {
                let page_id = args
                    .get("pageId")
                    .or_else(|| args.get("page_id"))
                    .or_else(|| args.get("page"))
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                let mut out = BTreeMap::new();
                out.insert("wrote".into(), "true".into());
                return ToolOutcome::OkWithCommand(
                    out,
                    EditorCommand::PatchNodeData {
                        node_id,
                        patch_json,
                        page_id,
                    },
                );
            }
            Ok(None) => {}
            Err(e) => return e,
        }
        let patch_args = match update_patch_args(args) {
            Ok(v) => v,
            Err(e) => return e,
        };
        // Parse each dimension, rejecting the first invalid one; collecting
        // the validated values here keeps the bindings below unwrap-free.
        let mut dims = [None, None, None, None];
        for (slot, key) in dims.iter_mut().zip(["x", "y", "width", "height"]) {
            match parse_opt_i32(&patch_args, key) {
                Ok(v) => *slot = v,
                Err(e) => {
                    return ToolOutcome::Err(ToolErrorCode::InvalidArgument, format!("{key}: {e}"))
                }
            }
        }
        let [x, y, width, height] = dims;
        let name = patch_args.get("name").cloned();
        let fill_hex = match patch_args.get("fill_hex") {
            None => None,
            Some(s) if !validate_hex(s) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {s:?}"),
                );
            }
            Some(s) => Some(s.clone()),
        };
        let page_id = args
            .get("pageId")
            .or_else(|| args.get("page_id"))
            .or_else(|| args.get("page"))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if x.is_none()
            && y.is_none()
            && width.is_none()
            && height.is_none()
            && name.is_none()
            && fill_hex.is_none()
        {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "at least one of x / y / width / height / name / fill_hex must be set".into(),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::UpdateNode {
                node_id,
                x,
                y,
                width,
                height,
                name,
                fill_hex,
                page_id,
            },
        )
    }
}

pub fn update_node_snapshot() -> UpdateNode {
    UpdateNode
}

#[allow(clippy::result_large_err)]
fn parse_node_id_alias(
    args: &BTreeMap<String, String>,
    snake_key: &str,
    camel_key: &str,
) -> Result<NodeId, ToolOutcome> {
    if args.contains_key(snake_key) {
        return parse_node_id(args, snake_key);
    }
    let Some(raw) = args.get(camel_key) else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::MissingArgument,
            format!("{snake_key} is required"),
        ));
    };
    NodeId::new_opt(raw.as_str()).ok_or_else(|| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{camel_key} must be a non-empty node id"),
        )
    })
}

#[allow(clippy::result_large_err)]
fn update_patch_args(
    args: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, ToolOutcome> {
    let Some(raw) = args.get("data") else {
        return Ok(args.clone());
    };
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("data must be a JSON object: {e}"),
        )
    })?;
    let Some(obj) = value.as_object() else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "data must be a JSON object".into(),
        ));
    };
    let mut out = BTreeMap::new();
    if let Some(name) = json_scalar_to_string(obj.get("name").or_else(|| obj.get("content"))) {
        out.insert("name".into(), name);
    }
    for key in ["x", "y", "width", "height"] {
        if let Some(value) = obj.get(key) {
            let Some(raw) = json_scalar_to_string(Some(value)) else {
                return Err(ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("data.{key} must be a string or number"),
                ));
            };
            out.insert(key.into(), raw);
        }
    }
    if let Some(fill_hex) = json_fill_hex_field(obj)? {
        out.insert("fill_hex".into(), fill_hex);
    }
    Ok(out)
}

#[allow(clippy::result_large_err)]
fn json_fill_hex_field(
    obj: &serde_json::Map<String, Value>,
) -> Result<Option<String>, ToolOutcome> {
    if let Some(v) = obj.get("fill_hex").or_else(|| obj.get("fillHex")) {
        return json_scalar_to_string(Some(v)).map(Some).ok_or_else(|| {
            ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "data.fill_hex must be a string".into(),
            )
        });
    }
    let Some(fill) = obj.get("fill") else {
        return Ok(None);
    };
    match fill {
        Value::String(s) => Ok(Some(s.clone())),
        Value::Object(fill_obj) => Ok(json_scalar_to_string(fill_obj.get("color"))),
        Value::Array(items) => {
            for item in items {
                if let Value::Object(fill_obj) = item {
                    if let Some(color) = json_scalar_to_string(fill_obj.get("color")) {
                        return Ok(Some(color));
                    }
                }
            }
            Ok(None)
        }
        _ => Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "data.fill must be a hex string, fill object, or fill array".into(),
        )),
    }
}

fn json_scalar_to_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(v) => Some(v.to_string()),
        _ => None,
    }
}

/// A geometry arg was present but is not a decimal `i32`.
///
/// One-variant enum on purpose: callers prefix it with the arg name
/// (`format!("{key}: {error}")`), so the payload stays the raw text and the
/// `Display` reproduces the previous message byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseI32Error {
    /// The raw arg value, rendered with `{:?}` in the message.
    pub raw: String,
}

impl std::fmt::Display for ParseI32Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = &self.raw;
        write!(f, "expected decimal i32, got {raw:?}")
    }
}

impl std::error::Error for ParseI32Error {}

/// Parse an optional i32 arg. `Ok(None)` when absent, `Ok(Some)` on a
/// successful parse, `Err` on present-but-malformed input.
pub(crate) fn parse_opt_i32(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<i32>, ParseI32Error> {
    match args.get(key) {
        None => Ok(None),
        Some(s) => s
            .parse::<i32>()
            .map(Some)
            .map_err(|_| ParseI32Error { raw: s.clone() }),
    }
}

/// First-party `delete_node` tool — removes a node + descendants.
pub struct DeleteNode;

impl McpTool for DeleteNode {
    fn name(&self) -> &str {
        "delete_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id_alias(args, "node_id", "nodeId") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let page_id = args
            .get("pageId")
            .or_else(|| args.get("page_id"))
            .or_else(|| args.get("page"))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::DeleteNode { node_id, page_id })
    }
}

pub fn delete_node_snapshot() -> DeleteNode {
    DeleteNode
}

/// Resolve a `target_parent_id`-style arg. The legacy wire used `"0"`
/// for "page root"; the canonical model uses the empty `NodeId::NONE`
/// sentinel. Both `""` and `"0"` map to `NONE` so older clients keep
/// working. `"root"` is also accepted because the generated tool
/// schema uses that wording for page-root inserts.
pub(super) fn root_or_node_id(raw: &str) -> NodeId {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed == "0"
        || trimmed.eq_ignore_ascii_case("root")
        || trimmed.eq_ignore_ascii_case("null")
    {
        NodeId::NONE
    } else {
        NodeId::new(trimmed)
    }
}

/// First-party `replace_node` tool — swap an existing node for a
/// freshly-built one at the same parent slot.
pub struct ReplaceNode;

impl McpTool for ReplaceNode {
    fn name(&self) -> &str {
        "replace_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id_alias(args, "node_id", "nodeId") {
            Ok(v) => v,
            Err(e) => return e,
        };
        match ts_replacement_data_node(args) {
            Ok(Some(mut node)) => {
                let drop_children = match parse_drop_children_arg(args) {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                let page_id = args
                    .get("pageId")
                    .or_else(|| args.get("page_id"))
                    .or_else(|| args.get("page"))
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                let mut out = BTreeMap::new();
                out.insert("wrote".into(), "true".into());
                // The replacement is GENERATED node JSON — hoist any
                // node-level `state` to the document root, same as the
                // insert paths and batch_program's R().
                let hoist =
                    super::batch_design::hoist_generation_state(std::slice::from_mut(&mut node));
                return ToolOutcome::OkWithCommand(
                    out,
                    super::batch_design::with_hoisted_state(
                        hoist,
                        EditorCommand::ReplaceSubtree {
                            node_id,
                            node: Box::new(node),
                            drop_children,
                            page_id,
                        },
                    ),
                );
            }
            Ok(None) => {}
            Err(e) => return e,
        }
        let params = match insert_node_params(args) {
            Ok(params) => params,
            Err(e) => return e,
        };
        let InsertNodeParams {
            kind,
            name,
            x,
            y,
            width,
            height,
            fill_hex,
        } = params;
        if !ALLOWED_KINDS.iter().any(|k| *k == kind) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!(
                    "kind {kind:?} not supported; allowed: {}",
                    ALLOWED_KINDS.join(", ")
                ),
            );
        }
        if width < 0 || height < 0 {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "width / height must be non-negative".into(),
            );
        }
        if let Some(hex) = fill_hex.as_deref() {
            if !validate_hex(hex) {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {hex:?}"),
                );
            }
        }
        let drop_children = match parse_drop_children_arg(args) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let page_id = args
            .get("pageId")
            .or_else(|| args.get("page_id"))
            .or_else(|| args.get("page"))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::ReplaceNode {
                node_id,
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
                drop_children,
                page_id,
            },
        )
    }
}

pub fn replace_node_snapshot() -> ReplaceNode {
    ReplaceNode
}

#[allow(clippy::result_large_err)]
fn parse_drop_children_arg(args: &BTreeMap<String, String>) -> Result<bool, ToolOutcome> {
    match args
        .get("drop_children")
        .or_else(|| args.get("dropChildren"))
    {
        None => Ok(false),
        Some(s) if s == "true" => Ok(true),
        Some(s) if s == "false" => Ok(false),
        Some(s) => Err(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("drop_children must be \"true\" or \"false\", got {s:?}"),
        )),
    }
}

/// `#rgb`, `#rrggbb`, `#rrggbbaa` — requires the leading `#`.
pub(super) fn validate_hex(s: &str) -> bool {
    let Some(rest) = s.trim().strip_prefix('#') else {
        return false;
    };
    matches!(rest.len(), 3 | 6 | 8) && rest.chars().all(|c| c.is_ascii_hexdigit())
}
