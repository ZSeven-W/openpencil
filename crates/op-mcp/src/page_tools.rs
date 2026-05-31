//! Page / selection / viewport / tool / flag / history MCP write
//! tools. Carved off `component_tools.rs` so each file stays under the
//! 800-line cap once the component tools were collapsed to the gap
//! errors.
//!
//! Ported off shell-core's `McpCommand` onto `op_editor_core::
//! EditorCommand`.

use std::collections::BTreeMap;

use op_editor_core::EditorState;
use op_editor_core::NodeId;

use super::write_tools::parse_node_id;
use super::{EditorCommand, McpTool, NodeFlag, ToolErrorCode, ToolOutcome};

/// First-party `set_active_page` tool — switch the active page.
pub struct SetActivePage;

impl McpTool for SetActivePage {
    fn name(&self) -> &str {
        "set_active_page"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let index = match parse_u32_arg(args, "index") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::SetActivePage { index })
    }
}

pub fn set_active_page_snapshot() -> SetActivePage {
    SetActivePage
}

// `ToolOutcome` is the shared MCP outcome type — boxing it broadly to
// shrink the `Err` variant would destabilize every tool signature.
#[allow(clippy::result_large_err)]
fn parse_u32_arg(args: &BTreeMap<String, String>, key: &str) -> Result<u32, ToolOutcome> {
    let Some(raw) = args.get(key) else {
        return Err(ToolOutcome::Err(
            ToolErrorCode::MissingArgument,
            format!("{key} is required"),
        ));
    };
    raw.parse::<u32>().map_err(|_| {
        ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a u32, got {raw:?}"),
        )
    })
}

/// First-party `add_page` tool — append a fresh empty page.
pub struct AddPage;

impl McpTool for AddPage {
    fn name(&self) -> &str {
        "add_page"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let name = match args.get("name") {
            Some(name) if name.trim().is_empty() => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "name must not be empty / whitespace-only".into(),
                );
            }
            Some(name) => Some(name.clone()),
            None => None,
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::AddPage { name })
    }
}

pub fn add_page_snapshot() -> AddPage {
    AddPage
}

/// First-party `rename_page` tool — set a page's display name.
pub struct RenamePage;

impl McpTool for RenamePage {
    fn name(&self) -> &str {
        "rename_page"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let index = match parse_u32_arg(args, "index") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
        };
        if name.trim().is_empty() {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "name must not be empty / whitespace-only".into(),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::RenamePage {
                index,
                name: name.clone(),
            },
        )
    }
}

pub fn rename_page_snapshot() -> RenamePage {
    RenamePage
}

/// First-party `delete_page` tool — remove a page by index.
pub struct DeletePage;

impl McpTool for DeletePage {
    fn name(&self) -> &str {
        "delete_page"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let index = match parse_u32_arg(args, "index") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::DeletePage { index })
    }
}

pub fn delete_page_snapshot() -> DeletePage {
    DeletePage
}

/// First-party `duplicate_page` tool — clone the page at `index`.
pub struct DuplicatePage;

impl McpTool for DuplicatePage {
    fn name(&self) -> &str {
        "duplicate_page"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let index = match parse_u32_arg(args, "index") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::DuplicatePage { index })
    }
}

pub fn duplicate_page_snapshot() -> DuplicatePage {
    DuplicatePage
}

/// First-party `reorder_page` tool — move a page from one index to
/// another.
pub struct ReorderPage;

impl McpTool for ReorderPage {
    fn name(&self) -> &str {
        "reorder_page"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let from = match parse_u32_arg(args, "from") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let to = match parse_u32_arg(args, "to") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::ReorderPage { from, to })
    }
}

pub fn reorder_page_snapshot() -> ReorderPage {
    ReorderPage
}

/// First-party `clear_selection` tool — drop the current multi-select.
pub struct ClearSelection;

impl McpTool for ClearSelection {
    fn name(&self) -> &str {
        "clear_selection"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::ClearSelection)
    }
}

pub fn clear_selection_snapshot() -> ClearSelection {
    ClearSelection
}

/// First-party `set_selection` tool — set the selection to a single
/// node by id.
pub struct SetSelection;

impl McpTool for SetSelection {
    fn name(&self) -> &str {
        "set_selection"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::SetSelection { node_id })
    }
}

pub fn set_selection_snapshot() -> SetSelection {
    SetSelection
}

/// First-party `set_selection_set` tool — replace the multi-selection
/// with the supplied comma-separated node ids. Empty list clears it.
pub struct SetSelectionSet;

impl McpTool for SetSelectionSet {
    fn name(&self) -> &str {
        "set_selection_set"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let raw = args.get("node_ids").map(String::as_str).unwrap_or("");
        let mut node_ids: Vec<NodeId> = Vec::new();
        for piece in raw.split(',') {
            let trimmed = piece.trim();
            if trimmed.is_empty() {
                continue;
            }
            node_ids.push(NodeId::new(trimmed));
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::SetSelectionSet { node_ids })
    }
}

pub fn set_selection_set_snapshot() -> SetSelectionSet {
    SetSelectionSet
}

/// First-party `toggle_node_selection` tool — Shift-click parity.
pub struct ToggleNodeSelection;

impl McpTool for ToggleNodeSelection {
    fn name(&self) -> &str {
        "toggle_node_selection"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::ToggleNodeSelection { node_id })
    }
}

pub fn toggle_node_selection_snapshot() -> ToggleNodeSelection {
    ToggleNodeSelection
}

/// First-party `cycle_active_axis_value` tool — advance a theme axis to
/// its next declared value.
pub struct CycleActiveAxisValue {
    pub axes_with_values: std::collections::BTreeSet<String>,
}

impl McpTool for CycleActiveAxisValue {
    fn name(&self) -> &str {
        "cycle_active_axis_value"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(axis) = args.get("axis") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "axis is required".into());
        };
        if axis.trim().is_empty() {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "axis must not be empty".into(),
            );
        }
        if !self.axes_with_values.contains(axis) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("axis {axis:?} not defined in themes (or has no values to cycle)"),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::CycleActiveAxisValue { axis: axis.clone() },
        )
    }
}

pub fn cycle_active_axis_value_snapshot(state: &EditorState) -> CycleActiveAxisValue {
    let axes_with_values = state
        .doc
        .themes
        .as_ref()
        .map(|themes| {
            themes
                .iter()
                .filter(|(_, values)| !values.is_empty())
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default();
    CycleActiveAxisValue { axes_with_values }
}

/// First-party `set_viewport` tool — set canvas pan + zoom.
pub struct SetViewport;

impl McpTool for SetViewport {
    fn name(&self) -> &str {
        "set_viewport"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        // `ToolOutcome` is the shared MCP outcome type — see `parse_u32_arg`.
        #[allow(clippy::result_large_err)]
        fn parse_opt_i32(
            args: &BTreeMap<String, String>,
            key: &str,
        ) -> Result<Option<i32>, ToolOutcome> {
            match args.get(key) {
                None => Ok(None),
                Some(s) => match s.parse::<i32>() {
                    Ok(n) => Ok(Some(n)),
                    Err(_) => Err(ToolOutcome::Err(
                        ToolErrorCode::InvalidArgument,
                        format!("{key} must be an i32, got {s:?}"),
                    )),
                },
            }
        }
        let pan_x = match parse_opt_i32(args, "pan_x") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let pan_y = match parse_opt_i32(args, "pan_y") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let zoom_percent = match parse_opt_i32(args, "zoom_percent") {
            Ok(v) => v,
            Err(e) => return e,
        };
        if pan_x.is_none() && pan_y.is_none() && zoom_percent.is_none() {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "at least one of pan_x / pan_y / zoom_percent must be set".into(),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::SetViewport {
                pan_x,
                pan_y,
                zoom_percent,
            },
        )
    }
}

pub fn set_viewport_snapshot() -> SetViewport {
    SetViewport
}

/// Build a `SetNodeFlag` command. Single point for arg parse + flag
/// validation.
fn build_set_node_flag(args: &BTreeMap<String, String>, flag: NodeFlag) -> ToolOutcome {
    let node_id = match parse_node_id(args, "node_id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let Some(raw_value) = args.get("value") else {
        return ToolOutcome::Err(
            ToolErrorCode::MissingArgument,
            "value is required (\"true\" or \"false\")".into(),
        );
    };
    let value = match raw_value.as_str() {
        "true" => true,
        "false" => false,
        _ => {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("value must be \"true\" or \"false\", got {raw_value:?}"),
            );
        }
    };
    let mut out = BTreeMap::new();
    out.insert("wrote".into(), "true".into());
    ToolOutcome::OkWithCommand(
        out,
        EditorCommand::SetNodeFlag {
            node_id,
            flag,
            value,
        },
    )
}

/// First-party `set_node_hidden` tool — toggle a node's visibility.
pub struct SetNodeHidden;
impl McpTool for SetNodeHidden {
    fn name(&self) -> &str {
        "set_node_hidden"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        build_set_node_flag(args, NodeFlag::Hidden)
    }
}
pub fn set_node_hidden_snapshot() -> SetNodeHidden {
    SetNodeHidden
}

/// First-party `set_node_locked` tool — toggle a node's lock.
pub struct SetNodeLocked;
impl McpTool for SetNodeLocked {
    fn name(&self) -> &str {
        "set_node_locked"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        build_set_node_flag(args, NodeFlag::Locked)
    }
}
pub fn set_node_locked_snapshot() -> SetNodeLocked {
    SetNodeLocked
}

/// First-party `set_active_tool` tool — change the active canvas tool.
pub struct SetActiveTool;

impl McpTool for SetActiveTool {
    fn name(&self) -> &str {
        "set_active_tool"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(tool) = args.get("tool") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "tool is required (select / rect / ellipse / polygon / line / pen / text / frame / hand)".into(),
            );
        };
        const ALLOWED_TOOLS: &[&str] = &[
            "select", "rect", "ellipse", "polygon", "line", "pen", "text", "frame", "hand",
        ];
        if !ALLOWED_TOOLS.contains(&tool.as_str()) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!(
                    "tool {tool:?} not supported; allowed: {}",
                    ALLOWED_TOOLS.join(", ")
                ),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::SetActiveTool { tool: tool.clone() })
    }
}

pub fn set_active_tool_snapshot() -> SetActiveTool {
    SetActiveTool
}

/// First-party `undo` tool — pop the last history snapshot.
pub struct Undo;

impl McpTool for Undo {
    fn name(&self) -> &str {
        "undo"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::Undo)
    }
}

pub fn undo_snapshot() -> Undo {
    Undo
}

/// First-party `redo` tool — push the last undone snapshot back.
pub struct Redo;

impl McpTool for Redo {
    fn name(&self) -> &str {
        "redo"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::Redo)
    }
}

pub fn redo_snapshot() -> Redo {
    Redo
}
