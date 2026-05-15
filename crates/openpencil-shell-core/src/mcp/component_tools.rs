//! Components CRUD + active-page MCP write tools. Carved off
//! `write_tools.rs` to stay under the 800-line cap.

use std::collections::BTreeMap;

use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};

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

/// First-party `create_component` tool — promote an existing
/// Frame or Group on the active page to a registered component.
/// Required args: `node_id`, `name`. The applier rejects non-
/// container kinds + unknown ids.
pub struct CreateComponent;

impl McpTool for CreateComponent {
    fn name(&self) -> &str {
        "create_component"
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
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required".into(),
            );
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::CreateComponent {
                node_id,
                name: name.clone(),
            },
        )
    }
}

pub fn create_component_snapshot() -> CreateComponent {
    CreateComponent
}

/// First-party `delete_component` tool — remove a component from
/// the registry by id. Live instances already on the page are
/// not affected (they're independent clones).
pub struct DeleteComponent;

impl McpTool for DeleteComponent {
    fn name(&self) -> &str {
        "delete_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("component_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "component_id is required".into(),
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
            McpCommand::DeleteComponent { component_id },
        )
    }
}

pub fn delete_component_snapshot() -> DeleteComponent {
    DeleteComponent
}

/// First-party `rename_component` tool — change a registered
/// component's display name. The applier rejects unknown ids
/// and empty / whitespace-only names.
pub struct RenameComponent;

impl McpTool for RenameComponent {
    fn name(&self) -> &str {
        "rename_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("component_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "component_id is required".into(),
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
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required".into(),
            );
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
            McpCommand::RenameComponent {
                component_id,
                name: name.clone(),
            },
        )
    }
}

pub fn rename_component_snapshot() -> RenameComponent {
    RenameComponent
}

/// First-party `set_active_page` tool — switch which page is the
/// active target for subsequent inserts / batch_design / design_*
/// commands. The applier rejects out-of-range indices.
pub struct SetActivePage;

impl McpTool for SetActivePage {
    fn name(&self) -> &str {
        "set_active_page"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("index") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "index is required (0-based page index)".into(),
            );
        };
        let index: u32 = match raw.parse() {
            Ok(n) => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("index must be a u32, got {raw:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, McpCommand::SetActivePage { index })
    }
}

pub fn set_active_page_snapshot() -> SetActivePage {
    SetActivePage
}

/// First-party `add_page` tool — append a fresh empty page +
/// switch the active page to it. No args. The applier returns
/// false on id-space exhaustion at `max_node_id() + 1`.
pub struct AddPage;

impl McpTool for AddPage {
    fn name(&self) -> &str {
        "add_page"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, McpCommand::AddPage)
    }
}

pub fn add_page_snapshot() -> AddPage {
    AddPage
}

/// First-party `rename_page` tool — set a page's display name.
/// Rejects out-of-range indices + empty / whitespace-only names.
pub struct RenamePage;

impl McpTool for RenamePage {
    fn name(&self) -> &str {
        "rename_page"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("index") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "index is required (0-based page index)".into(),
            );
        };
        let index: u32 = match raw.parse() {
            Ok(n) => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("index must be a u32, got {raw:?}"),
                );
            }
        };
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required".into(),
            );
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
            McpCommand::RenamePage {
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
        let Some(raw) = args.get("index") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "index is required (0-based page index)".into(),
            );
        };
        let index: u32 = match raw.parse() {
            Ok(n) => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("index must be a u32, got {raw:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, McpCommand::DeletePage { index })
    }
}

pub fn delete_page_snapshot() -> DeletePage {
    DeletePage
}

/// First-party `duplicate_page` tool — clone the page at `index`
/// and switch the active page to the clone.
pub struct DuplicatePage;

impl McpTool for DuplicatePage {
    fn name(&self) -> &str {
        "duplicate_page"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("index") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "index is required (0-based page index)".into(),
            );
        };
        let index: u32 = match raw.parse() {
            Ok(n) => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("index must be a u32, got {raw:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, McpCommand::DuplicatePage { index })
    }
}

pub fn duplicate_page_snapshot() -> DuplicatePage {
    DuplicatePage
}

/// First-party `reorder_page` tool — move a page from one index
/// to another. The applier clamps `to` into `[0, page_count)`.
pub struct ReorderPage;

impl McpTool for ReorderPage {
    fn name(&self) -> &str {
        "reorder_page"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let from: u32 = match args.get("from").map(|s| s.parse::<u32>()) {
            Some(Ok(n)) => n,
            Some(_) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "from must be a u32".into(),
                );
            }
            None => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "from is required (0-based source page index)".into(),
                );
            }
        };
        let to: u32 = match args.get("to").map(|s| s.parse::<u32>()) {
            Some(Ok(n)) => n,
            Some(_) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "to must be a u32".into(),
                );
            }
            None => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "to is required (0-based target page index)".into(),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, McpCommand::ReorderPage { from, to })
    }
}

pub fn reorder_page_snapshot() -> ReorderPage {
    ReorderPage
}

/// First-party `clear_selection` tool — drop the current multi-
/// select. No args.
pub struct ClearSelection;

impl McpTool for ClearSelection {
    fn name(&self) -> &str {
        "clear_selection"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, McpCommand::ClearSelection)
    }
}

pub fn clear_selection_snapshot() -> ClearSelection {
    ClearSelection
}

/// First-party `set_selection` tool — set the selection to a
/// single node by id. The applier rejects ids that don't
/// resolve on any page.
pub struct SetSelection;

impl McpTool for SetSelection {
    fn name(&self) -> &str {
        "set_selection"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
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
        ToolOutcome::OkWithCommand(out, McpCommand::SetSelection { node_id })
    }
}

pub fn set_selection_snapshot() -> SetSelection {
    SetSelection
}

/// First-party `set_selection_set` tool — replace the
/// multi-selection (`Document.selected_set`) with the supplied
/// `node_ids` list (comma-separated u64s). Empty list clears the
/// selection. Unknown ids are dropped at apply time so a panel
/// race with a delete doesn't fail the whole call.
pub struct SetSelectionSet;

impl McpTool for SetSelectionSet {
    fn name(&self) -> &str {
        "set_selection_set"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let raw = args.get("node_ids").map(String::as_str).unwrap_or("");
        let mut node_ids: Vec<u64> = Vec::new();
        for piece in raw.split(',') {
            let trimmed = piece.trim();
            if trimmed.is_empty() {
                continue;
            }
            match trimmed.parse::<u64>() {
                Ok(n) if n > 0 => node_ids.push(n),
                _ => {
                    return ToolOutcome::Err(
                        ToolErrorCode::InvalidArgument,
                        format!(
                            "node_ids must be comma-separated positive u64 ids, \
                             got {trimmed:?}"
                        ),
                    );
                }
            }
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, McpCommand::SetSelectionSet { node_ids })
    }
}

pub fn set_selection_set_snapshot() -> SetSelectionSet {
    SetSelectionSet
}

/// First-party `toggle_node_selection` tool — Shift-click parity.
/// Toggles `node_id` in the multi-selection: removes it if already
/// selected (re-anchors to the new last), adds it as the new
/// anchor otherwise. Lets LLM clients build up multi-selection
/// incrementally without sending the full id list each step.
pub struct ToggleNodeSelection;

impl McpTool for ToggleNodeSelection {
    fn name(&self) -> &str {
        "toggle_node_selection"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
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
        ToolOutcome::OkWithCommand(out, McpCommand::ToggleNodeSelection { node_id })
    }
}

pub fn toggle_node_selection_snapshot() -> ToggleNodeSelection {
    ToggleNodeSelection
}

/// First-party `cycle_active_axis_value` tool — advance the
/// active value for a theme axis to its next entry (wrapping
/// back to the first). Pairs with `set_active_axis_value` so an
/// LLM can either jump straight to a known value or just say
/// "next" without knowing the values list.
///
/// Carries a snapshot of every theme axis that has at least one
/// value, so unknown / empty axes are rejected at call time as
/// `ToolFailed` instead of falling through to the applier and
/// surfacing as a misleading `Internal` error (codex stop-gate:
/// previously the tool was zero-state and any bad axis got the
/// generic "host rejected command" demotion).
pub struct CycleActiveAxisValue {
    pub axes_with_values: std::collections::BTreeSet<String>,
}

impl McpTool for CycleActiveAxisValue {
    fn name(&self) -> &str {
        "cycle_active_axis_value"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(axis) = args.get("axis") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "axis is required".into(),
            );
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
                format!(
                    "axis {axis:?} not defined in themes (or has no values to cycle)"
                ),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::CycleActiveAxisValue {
                axis: axis.clone(),
            },
        )
    }
}

pub fn cycle_active_axis_value_snapshot(
    doc: &crate::document::Document,
) -> CycleActiveAxisValue {
    let axes_with_values = doc
        .var_table
        .themes
        .iter()
        .filter(|t| !t.values.is_empty())
        .map(|t| t.name.clone())
        .collect();
    CycleActiveAxisValue { axes_with_values }
}

/// First-party `set_viewport` tool — set canvas pan + zoom.
/// Any subset of args can be omitted (the applier only writes
/// the ones that are present + valid). zoom_percent clamps into
/// [10, 2000] (10% .. 2000%) at apply time.
pub struct SetViewport;

impl McpTool for SetViewport {
    fn name(&self) -> &str {
        "set_viewport"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
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
            McpCommand::SetViewport {
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

/// Internal helper that builds a `SetNodeFlag` command for the
/// three flag write tools below. Single point for the arg parse
/// + flag validation.
fn build_set_node_flag(
    args: &BTreeMap<String, String>,
    flag: crate::mcp::NodeFlag,
) -> ToolOutcome {
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
        McpCommand::SetNodeFlag {
            node_id,
            flag,
            value,
        },
    )
}

/// First-party `set_node_hidden` tool — toggle a node's
/// visibility (the layer-panel eye icon).
pub struct SetNodeHidden;
impl McpTool for SetNodeHidden {
    fn name(&self) -> &str {
        "set_node_hidden"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        build_set_node_flag(args, crate::mcp::NodeFlag::Hidden)
    }
}
pub fn set_node_hidden_snapshot() -> SetNodeHidden {
    SetNodeHidden
}

/// First-party `set_node_locked` tool — toggle a node's lock
/// (the layer-panel padlock icon).
pub struct SetNodeLocked;
impl McpTool for SetNodeLocked {
    fn name(&self) -> &str {
        "set_node_locked"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        build_set_node_flag(args, crate::mcp::NodeFlag::Locked)
    }
}
pub fn set_node_locked_snapshot() -> SetNodeLocked {
    SetNodeLocked
}

/// First-party `set_node_collapsed` tool — toggle a node's
/// layer-panel disclosure state.
pub struct SetNodeCollapsed;
impl McpTool for SetNodeCollapsed {
    fn name(&self) -> &str {
        "set_node_collapsed"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        build_set_node_flag(args, crate::mcp::NodeFlag::Collapsed)
    }
}
pub fn set_node_collapsed_snapshot() -> SetNodeCollapsed {
    SetNodeCollapsed
}

/// First-party `set_active_tool` tool — change the active
/// canvas tool. Accepts: select / rect / ellipse / polygon /
/// line / pen / text / frame / hand.
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
        if !ALLOWED_TOOLS.iter().any(|t| *t == tool.as_str()) {
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
        ToolOutcome::OkWithCommand(out, McpCommand::SetActiveTool { tool: tool.clone() })
    }
}

pub fn set_active_tool_snapshot() -> SetActiveTool {
    SetActiveTool
}

/// First-party `undo` tool — pop the last history snapshot.
/// No args. Returns false at apply time when past stack is empty.
pub struct Undo;

impl McpTool for Undo {
    fn name(&self) -> &str {
        "undo"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, McpCommand::Undo)
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
        ToolOutcome::OkWithCommand(out, McpCommand::Redo)
    }
}

pub fn redo_snapshot() -> Redo {
    Redo
}
