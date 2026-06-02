//! MCP write tools. Each returns `ToolOutcome::OkWithCommand(...)` so
//! the host applies the mutation against the live editor state via
//! `EditorState::apply`.
//!
//! Ported off shell-core's `McpCommand` onto `op_editor_core::
//! EditorCommand`. The biggest model change: node ids are now the
//! canonical `.op` schema's string ids (`NodeId`), not the old `u64`.
//! `parse_node_id` accepts any non-empty string.

use std::collections::BTreeMap;

use jian_ops_schema::variable::VariableKind;
use op_editor_core::EditorState;
use op_editor_core::NodeId;

use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};

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

/// First-party `set_variable_color` tool — validates that the variable
/// exists + is Color-kind + the hex parses, then returns
/// `OkWithCommand(SetVariableColor)`.
pub struct SetVariableColor {
    /// Snapshot of which Color variables exist. Validation only.
    pub known_colors: BTreeMap<String, ()>,
}

impl McpTool for SetVariableColor {
    fn name(&self) -> &str {
        "set_variable_color"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
        };
        let Some(hex) = args.get("hex") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "hex is required".into());
        };
        if !self.known_colors.contains_key(name) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("variable {name:?} not found or not Color-kind"),
            );
        }
        if !validate_hex(hex) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("hex must be #rgb/#rrggbb/#rrggbbaa, got {hex:?}"),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::SetVariableColor {
                name: name.clone(),
                hex: hex.clone(),
            },
        )
    }
}

/// First-party `set_active_axis_value` tool — pins an axis to a value.
pub struct SetActiveAxisValue {
    /// Snapshot of axis → allowed-values. Validation only.
    pub axes: BTreeMap<String, Vec<String>>,
}

impl McpTool for SetActiveAxisValue {
    fn name(&self) -> &str {
        "set_active_axis_value"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(axis) = args.get("axis") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "axis is required".into());
        };
        let Some(value) = args.get("value") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "value is required".into());
        };
        let Some(allowed) = self.axes.get(axis) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("axis {axis:?} not defined in themes"),
            );
        };
        if !allowed.iter().any(|v| v == value) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!(
                    "value {value:?} not in axis {axis:?}; allowed: {}",
                    allowed.join(", ")
                ),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::SetActiveAxisValue {
                axis: axis.clone(),
                value: value.clone(),
            },
        )
    }
}

/// First-party `insert_node` tool — creates a fresh node on the active
/// page. The applier allocates a non-colliding id.
pub struct InsertNode;

impl McpTool for InsertNode {
    fn name(&self) -> &str {
        "insert_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let kind = match args.get("kind") {
            Some(s) => s.clone(),
            None => {
                return ToolOutcome::Err(ToolErrorCode::MissingArgument, "kind is required".into());
            }
        };
        if !ALLOWED_KINDS.iter().any(|k| *k == kind) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!(
                    "kind {kind:?} not supported; allowed: {}",
                    ALLOWED_KINDS.join(", ")
                ),
            );
        }
        let name = match args.get("name") {
            Some(s) => s.clone(),
            None => {
                return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
            }
        };
        let x = match parse_i32_arg(args, "x") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let y = match parse_i32_arg(args, "y") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let width = match parse_i32_arg(args, "width") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let height = match parse_i32_arg(args, "height") {
            Ok(v) => v,
            Err(e) => return e,
        };
        if width < 0 || height < 0 {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "width / height must be non-negative".into(),
            );
        }
        let fill_hex = match args.get("fill_hex") {
            None => None,
            Some(s) if !validate_hex(s) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {s:?}"),
                );
            }
            Some(s) => Some(s.clone()),
        };
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
    "frame", "group", "rect", "ellipse", "polygon", "line", "text", "path",
];

// `ToolOutcome` is the shared MCP outcome type — see `parse_node_id`.
#[allow(clippy::result_large_err)]
fn parse_i32_arg(args: &BTreeMap<String, String>, key: &str) -> Result<i32, ToolOutcome> {
    let Some(raw) = args.get(key) else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::MissingArgument,
            format!("{key} is required"),
        ));
    };
    raw.parse::<i32>().map_err(|_| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a decimal i32, got {raw:?}"),
        )
    })
}

pub fn insert_node_snapshot() -> InsertNode {
    InsertNode
}

/// First-party `update_node` tool — patch fields on an existing node.
pub struct UpdateNode;

impl McpTool for UpdateNode {
    fn name(&self) -> &str {
        "update_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let x = parse_opt_i32(args, "x");
        let y = parse_opt_i32(args, "y");
        let width = parse_opt_i32(args, "width");
        let height = parse_opt_i32(args, "height");
        for (lab, v) in [("x", &x), ("y", &y), ("width", &width), ("height", &height)] {
            if let Err(e) = v {
                return ToolOutcome::Err(ToolErrorCode::InvalidArgument, format!("{lab}: {e}"));
            }
        }
        let name = args.get("name").cloned();
        let fill_hex = match args.get("fill_hex") {
            None => None,
            Some(s) if !validate_hex(s) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {s:?}"),
                );
            }
            Some(s) => Some(s.clone()),
        };
        let x = x.unwrap();
        let y = y.unwrap();
        let width = width.unwrap();
        let height = height.unwrap();
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
            },
        )
    }
}

pub fn update_node_snapshot() -> UpdateNode {
    UpdateNode
}

/// Parse an optional i32 arg. `Ok(None)` when absent, `Ok(Some)` on a
/// successful parse, `Err` on present-but-malformed input.
fn parse_opt_i32(args: &BTreeMap<String, String>, key: &str) -> Result<Option<i32>, String> {
    match args.get(key) {
        None => Ok(None),
        Some(s) => s
            .parse::<i32>()
            .map(Some)
            .map_err(|_| format!("expected decimal i32, got {s:?}")),
    }
}

/// First-party `delete_node` tool — removes a node + descendants.
pub struct DeleteNode;

impl McpTool for DeleteNode {
    fn name(&self) -> &str {
        "delete_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::DeleteNode { node_id })
    }
}

pub fn delete_node_snapshot() -> DeleteNode {
    DeleteNode
}

/// First-party `move_node` tool — reparent a node. An empty
/// `target_parent_id` reparents to the active page root.
pub struct MoveNode;

impl McpTool for MoveNode {
    fn name(&self) -> &str {
        "move_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        // `target_parent_id` is required; an empty string ("" or "0")
        // means "the active page root" (the NONE sentinel).
        let Some(raw_target) = args.get("target_parent_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "target_parent_id is required (\"\" or \"0\" = page root)".into(),
            );
        };
        let target_parent = root_or_node_id(raw_target);
        if target_parent.is_real() && target_parent == node_id {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "node_id and target_parent_id must differ".into(),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::MoveNode {
                node_id,
                target_parent,
            },
        )
    }
}

pub fn move_node_snapshot() -> MoveNode {
    MoveNode
}

/// Resolve a `target_parent_id`-style arg. The legacy wire used `"0"`
/// for "page root"; the canonical model uses the empty `NodeId::NONE`
/// sentinel. Both `""` and `"0"` map to `NONE` so older clients keep
/// working. `"root"` is also accepted because the generated tool
/// schema uses that wording for page-root inserts.
fn root_or_node_id(raw: &str) -> NodeId {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "0" || trimmed.eq_ignore_ascii_case("root") {
        NodeId::NONE
    } else {
        NodeId::new(trimmed)
    }
}

/// First-party `copy_node` tool — deep-clone a node + subtree under a
/// new parent.
pub struct CopyNode;

impl McpTool for CopyNode {
    fn name(&self) -> &str {
        "copy_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let Some(raw_target) = args.get("target_parent_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "target_parent_id is required (\"\" or \"0\" = page root)".into(),
            );
        };
        let target_parent = root_or_node_id(raw_target);
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::CopyNode {
                node_id,
                target_parent,
            },
        )
    }
}

pub fn copy_node_snapshot() -> CopyNode {
    CopyNode
}

/// First-party `replace_node` tool — swap an existing node for a
/// freshly-built one at the same parent slot.
pub struct ReplaceNode;

impl McpTool for ReplaceNode {
    fn name(&self) -> &str {
        "replace_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let kind = match args.get("kind") {
            Some(s) => s.clone(),
            None => {
                return ToolOutcome::Err(ToolErrorCode::MissingArgument, "kind is required".into());
            }
        };
        if !ALLOWED_KINDS.iter().any(|k| *k == kind) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!(
                    "kind {kind:?} not supported; allowed: {}",
                    ALLOWED_KINDS.join(", ")
                ),
            );
        }
        let name = match args.get("name") {
            Some(s) => s.clone(),
            None => {
                return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
            }
        };
        let x = match parse_i32_arg(args, "x") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let y = match parse_i32_arg(args, "y") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let width = match parse_i32_arg(args, "width") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let height = match parse_i32_arg(args, "height") {
            Ok(v) => v,
            Err(e) => return e,
        };
        if width < 0 || height < 0 {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "width / height must be non-negative".into(),
            );
        }
        let fill_hex = match args.get("fill_hex") {
            None => None,
            Some(s) if !validate_hex(s) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("fill_hex must be #rgb/#rrggbb/#rrggbbaa, got {s:?}"),
                );
            }
            Some(s) => Some(s.clone()),
        };
        let drop_children = match args.get("drop_children") {
            None => false,
            Some(s) if s == "true" => true,
            Some(s) if s == "false" => false,
            Some(s) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("drop_children must be \"true\" or \"false\", got {s:?}"),
                );
            }
        };
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
            },
        )
    }
}

pub fn replace_node_snapshot() -> ReplaceNode {
    ReplaceNode
}

pub fn set_active_axis_value_snapshot(state: &EditorState) -> SetActiveAxisValue {
    let axes = state
        .doc
        .themes
        .as_ref()
        .map(|themes| {
            themes
                .iter()
                .map(|(name, values)| (name.clone(), values.clone()))
                .collect()
        })
        .unwrap_or_default();
    SetActiveAxisValue { axes }
}

pub fn set_variable_color_snapshot(state: &EditorState) -> SetVariableColor {
    let known_colors = state
        .doc
        .variables
        .as_ref()
        .map(|vars| {
            vars.iter()
                .filter(|(_, def)| matches!(def.kind, VariableKind::Color))
                .map(|(name, _)| (name.clone(), ()))
                .collect()
        })
        .unwrap_or_default();
    SetVariableColor { known_colors }
}

/// First-party `import_svg` tool — parse an SVG document + insert the
/// resulting nodes on the active page. `x` / `y` (optional, default 0)
/// offset the imported nodes in doc-px.
pub struct ImportSvg;

impl McpTool for ImportSvg {
    fn name(&self) -> &str {
        "import_svg"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(svg) = args.get("svg") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "svg is required (an SVG document string)".into(),
            );
        };
        if svg.trim().is_empty() {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "svg must not be empty".into(),
            );
        }
        // `x` / `y` are optional doc-px offsets — absent ⇒ 0, a
        // malformed value rejects so the LLM sees a typed error.
        let x = match parse_opt_i32(args, "x") {
            Ok(v) => v.unwrap_or(0),
            Err(e) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, format!("x: {e}")),
        };
        let y = match parse_opt_i32(args, "y") {
            Ok(v) => v.unwrap_or(0),
            Err(e) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, format!("y: {e}")),
        };
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
            EditorCommand::ImportSvg {
                svg: svg.clone(),
                x,
                y,
                target_parent,
                page_id,
            },
        )
    }
}

pub fn import_svg_snapshot() -> ImportSvg {
    ImportSvg
}

/// `#rgb`, `#rrggbb`, `#rrggbbaa` — requires the leading `#`.
pub(super) fn validate_hex(s: &str) -> bool {
    let Some(rest) = s.trim().strip_prefix('#') else {
        return false;
    };
    matches!(rest.len(), 3 | 6 | 8) && rest.chars().all(|c| c.is_ascii_hexdigit())
}
