//! MCP write tools. Each returns `ToolOutcome::OkWithCommand(...)`
//! so the host applies the mutation against the live document via
//! `Document::apply_mcp_command`. Pulled out of the read-tool
//! `tools.rs` for file-size hygiene (800-line cap) as the write
//! surface grew past insert_node + update_node + delete_node.

use std::collections::BTreeMap;

use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};

/// First-party `set_variable_color` tool — the first write tool.
/// Validates that the variable exists + is Color-kind + the hex
/// parses, then returns `OkWithCommand(SetVariableColor)` so the
/// host applies the change against the live document. Reads the
/// snapshot lazily — tool validation is O(n) over the variables
/// vec; the apply path routes through `VariableTable::set_color_hex`
/// with the full correctness chain (subset / no-clobber / no-shadow).
///
/// Wire shape:
///   args   — { "name": "<variable>", "hex": "#rrggbb" }
///   result — { "wrote": "true" } when the command was queued
///   command — `McpCommand::SetVariableColor { name, hex }`
pub struct SetVariableColor {
    /// Snapshot of which variables exist + their kinds. Used for
    /// validation only — the host applies the write so this
    /// snapshot can lag a frame behind without breaking anything.
    pub known_colors: BTreeMap<String, ()>,
}

impl McpTool for SetVariableColor {
    fn name(&self) -> &str {
        "set_variable_color"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required".into(),
            );
        };
        let Some(hex) = args.get("hex") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "hex is required".into(),
            );
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
            McpCommand::SetVariableColor {
                name: name.clone(),
                hex: hex.clone(),
            },
        )
    }
}

/// First-party `set_active_axis_value` tool — pins an axis to a
/// value (vs `cycle_active_axis_value` which advances). Validates
/// axis exists + value is in `themes[axis].values`; the host
/// applier re-validates against live state.
pub struct SetActiveAxisValue {
    /// Snapshot of axis → allowed-values, mirroring
    /// `VariableTable::themes`. Validation only — the host re-runs
    /// the same check at apply time so a stale snapshot can't slip
    /// an unauthorized value through.
    pub axes: BTreeMap<String, Vec<String>>,
}

impl McpTool for SetActiveAxisValue {
    fn name(&self) -> &str {
        "set_active_axis_value"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(axis) = args.get("axis") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "axis is required".into(),
            );
        };
        let Some(value) = args.get("value") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "value is required".into(),
            );
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
            McpCommand::SetActiveAxisValue {
                axis: axis.clone(),
                value: value.clone(),
            },
        )
    }
}

/// First-party `insert_node` tool — creates a fresh node on the
/// active page. Args: kind / name / x / y / width / height +
/// optional fill_hex. The applier allocates a non-colliding id
/// past `max_node_id()` so the LLM never has to reason about id
/// space. Stateless — no document snapshot needed.
pub struct InsertNode;

impl McpTool for InsertNode {
    fn name(&self) -> &str {
        "insert_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        // Required args: kind, name, x, y, width, height.
        let kind = match args.get("kind") {
            Some(s) => s.clone(),
            None => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "kind is required".into(),
                );
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
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "name is required".into(),
                );
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
        // fill_hex is optional, but if present it must parse.
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
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::InsertNode {
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
            },
        )
    }
}

pub(super) const ALLOWED_KINDS: &[&str] = &[
    "frame", "group", "rect", "ellipse", "polygon", "line", "text", "path",
];

fn parse_i32_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<i32, ToolOutcome> {
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
    // Stateless — no document snapshot needed.
    InsertNode
}

/// First-party `update_node` tool — patch fields on an existing
/// node. Required arg: node_id. Optional: x, y, width, height,
/// name, fill_hex. At least one optional arg must be present; the
/// applier no-ops on an empty patch (returns false).
pub struct UpdateNode;

impl McpTool for UpdateNode {
    fn name(&self) -> &str {
        "update_node"
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
        let node_id: u64 = match raw.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw:?}"),
                );
            }
        };
        let x = parse_opt_i32(args, "x");
        let y = parse_opt_i32(args, "y");
        let width = parse_opt_i32(args, "width");
        let height = parse_opt_i32(args, "height");
        for (lab, v) in [("x", &x), ("y", &y), ("width", &width), ("height", &height)] {
            if let Err(e) = v {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("{lab}: {e}"),
                );
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
            McpCommand::UpdateNode {
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

/// Parse an optional i32 arg. Returns Ok(None) when absent, Ok(Some)
/// on a successful parse, Err on present-but-malformed input.
fn parse_opt_i32(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<i32>, String> {
    match args.get(key) {
        None => Ok(None),
        Some(s) => s
            .parse::<i32>()
            .map(Some)
            .map_err(|_| format!("expected decimal i32, got {s:?}")),
    }
}

/// First-party `delete_node` tool — removes a node + its descendants
/// from its parent. Required arg: node_id. The applier walks every
/// page recursively to find the right parent vec.
pub struct DeleteNode;

impl McpTool for DeleteNode {
    fn name(&self) -> &str {
        "delete_node"
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
        let node_id: u64 = match raw.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, McpCommand::DeleteNode { node_id })
    }
}

pub fn delete_node_snapshot() -> DeleteNode {
    DeleteNode
}

/// First-party `move_node` tool — reparent a node. Required args:
/// node_id, target_parent_id. `target_parent_id == "0"` reparents
/// to the active page root. Non-zero ids must resolve to an
/// existing node; the applier rejects cycles (target descendant
/// of source) at apply time.
pub struct MoveNode;

impl McpTool for MoveNode {
    fn name(&self) -> &str {
        "move_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(raw_target) = args.get("target_parent_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "target_parent_id is required (0 = page root)".into(),
            );
        };
        let target_parent_id: u64 = match raw_target.parse() {
            Ok(n) => n,
            Err(_) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("target_parent_id must be u64, got {raw_target:?}"),
                );
            }
        };
        if node_id == target_parent_id {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "node_id and target_parent_id must differ".into(),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::MoveNode {
                node_id,
                target_parent_id,
            },
        )
    }
}

pub fn move_node_snapshot() -> MoveNode {
    MoveNode
}

/// First-party `copy_node` tool — deep-clone a node + every
/// descendant under a new parent. Required args: node_id,
/// target_parent_id. `target_parent_id == "0"` puts the copy at
/// the active page root. Fresh ids are allocated by the applier
/// past `max_node_id()` so the clone never collides with any
/// live node. Allows `node_id == target_parent_id` (duplicating
/// a container's contents under itself is a valid LLM op).
pub struct CopyNode;

impl McpTool for CopyNode {
    fn name(&self) -> &str {
        "copy_node"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(raw_target) = args.get("target_parent_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "target_parent_id is required (0 = page root)".into(),
            );
        };
        let target_parent_id: u64 = match raw_target.parse() {
            Ok(n) => n,
            Err(_) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("target_parent_id must be u64, got {raw_target:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::CopyNode {
                node_id,
                target_parent_id,
            },
        )
    }
}

pub fn copy_node_snapshot() -> CopyNode {
    CopyNode
}

/// First-party `replace_node` tool — swap an existing node with a
/// freshly-built one at the same parent slot. Required args:
/// node_id, kind, name, x, y, width, height. Optional: fill_hex.
/// Bounded scope: only leaf-style fields are accepted today (the
/// children of the replacement default to empty). TS-equivalent
/// behavior for primitives; subtree-replacement requires a JSON
/// Node parser that doesn't live on this side yet.
///
/// **Destructive on containers**: replacing a Frame / Group / node
/// with children drops every descendant of the old node. The
/// applier refuses the swap when the target has children unless
/// the optional `drop_children` arg is the string `"true"` —
/// callers that genuinely want to discard the subtree must opt
/// in. Reserve `replace_node` for primitive → primitive swaps
/// where the loss is intended; use `update_node` to patch a
/// container in place.
pub struct ReplaceNode;

impl McpTool for ReplaceNode {
    fn name(&self) -> &str {
        "replace_node"
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
        let node_id: u64 = match raw.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw:?}"),
                );
            }
        };
        let kind = match args.get("kind") {
            Some(s) => s.clone(),
            None => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "kind is required".into(),
                );
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
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "name is required".into(),
                );
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
        // Optional opt-in for destructive swaps. The applier
        // refuses to drop a container's children unless this is
        // `"true"`. Missing → safe default false. Any other
        // value (including the empty string) is malformed and
        // rejected — silently treating "" as false would hide a
        // mis-serialized confirmation from the caller.
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
            McpCommand::ReplaceNode {
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

pub fn set_active_axis_value_snapshot(
    doc: &crate::document::Document,
) -> SetActiveAxisValue {
    let axes = doc
        .var_table
        .themes
        .iter()
        .map(|t| (t.name.clone(), t.values.clone()))
        .collect();
    SetActiveAxisValue { axes }
}

pub fn set_variable_color_snapshot(
    doc: &crate::document::Document,
) -> SetVariableColor {
    use crate::document::VariableKind;
    let known_colors = doc
        .var_table
        .variables
        .iter()
        .filter(|v| matches!(v.kind, VariableKind::Color))
        .map(|v| (v.name.clone(), ()))
        .collect();
    SetVariableColor { known_colors }
}

/// First-party `instantiate_component` tool — drop a clone of a
/// registered component's root subtree onto the active page.
/// Required arg: `component_id` (the id of the component's root,
/// returned by `list_components`). The applier handles fresh-id
/// allocation + history snapshot.
pub struct InstantiateComponent;

impl McpTool for InstantiateComponent {
    fn name(&self) -> &str {
        "instantiate_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("component_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "component_id is required (from list_components.components `name|id`)".into(),
            );
        };
        let component_id: u64 = match raw.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("component_id must be a positive u64, got {raw:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::InstantiateComponent { component_id },
        )
    }
}

pub fn instantiate_component_snapshot() -> InstantiateComponent {
    InstantiateComponent
}

/// `#rgb`, `#rrggbb`, `#rrggbbaa` — matches the format
/// `VariableTable::parse_hex_color` accepts. Lenient on case;
/// requires the leading `#`.
pub(super) fn validate_hex(s: &str) -> bool {
    let Some(rest) = s.trim().strip_prefix('#') else {
        return false;
    };
    matches!(rest.len(), 3 | 6 | 8)
        && rest.chars().all(|c| c.is_ascii_hexdigit())
}

