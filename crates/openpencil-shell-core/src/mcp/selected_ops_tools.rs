//! Selection-targeting MCP write tools (duplicate / delete / nudge /
//! group / ungroup / reorder / clipboard / align).
//!
//! Ported off shell-core's `McpCommand` onto `op_editor_core::
//! EditorCommand`.

use std::collections::BTreeMap;

use op_editor_core::ReorderDirection;

use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};

/// First-party `duplicate_selected` tool — Cmd+D equivalent.
pub struct DuplicateSelected;

impl McpTool for DuplicateSelected {
    fn name(&self) -> &str {
        "duplicate_selected"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let offset_px: i32 = match args.get("offset_px") {
            None => 10,
            Some(s) => match s.parse::<i32>() {
                Ok(n) => n,
                Err(_) => {
                    return ToolOutcome::Err(
                        ToolErrorCode::InvalidArgument,
                        format!("offset_px must be an i32, got {s:?}"),
                    );
                }
            },
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::DuplicateSelected { offset_px })
    }
}

pub fn duplicate_selected_snapshot() -> DuplicateSelected {
    DuplicateSelected
}

/// First-party `delete_selected` tool — Delete-key equivalent.
pub struct DeleteSelected;

impl McpTool for DeleteSelected {
    fn name(&self) -> &str {
        "delete_selected"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::DeleteSelected)
    }
}

pub fn delete_selected_snapshot() -> DeleteSelected {
    DeleteSelected
}

/// First-party `nudge_selected` tool — arrow-key nudge equivalent.
pub struct NudgeSelected;

impl McpTool for NudgeSelected {
    fn name(&self) -> &str {
        "nudge_selected"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        fn parse_required_i32(
            args: &BTreeMap<String, String>,
            key: &str,
        ) -> Result<i32, ToolOutcome> {
            let Some(s) = args.get(key) else {
                return Err(ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    format!("{key} is required"),
                ));
            };
            s.parse::<i32>().map_err(|_| {
                ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("{key} must be an i32, got {s:?}"),
                )
            })
        }
        let dx = match parse_required_i32(args, "dx") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let dy = match parse_required_i32(args, "dy") {
            Ok(v) => v,
            Err(e) => return e,
        };
        if dx == 0 && dy == 0 {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "dx and dy can't both be 0".into(),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::NudgeSelected { dx, dy })
    }
}

pub fn nudge_selected_snapshot() -> NudgeSelected {
    NudgeSelected
}

/// First-party `group_selected` tool — Cmd+G equivalent.
pub struct GroupSelected;

impl McpTool for GroupSelected {
    fn name(&self) -> &str {
        "group_selected"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::GroupSelected)
    }
}

pub fn group_selected_snapshot() -> GroupSelected {
    GroupSelected
}

/// First-party `ungroup_selected` tool — Cmd+Shift+G equivalent.
pub struct UngroupSelected;

impl McpTool for UngroupSelected {
    fn name(&self) -> &str {
        "ungroup_selected"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::UngroupSelected)
    }
}

pub fn ungroup_selected_snapshot() -> UngroupSelected {
    UngroupSelected
}

/// First-party `reorder_selected` tool — bring forward / send
/// backward. `direction` is "up" or "down".
pub struct ReorderSelected;

impl McpTool for ReorderSelected {
    fn name(&self) -> &str {
        "reorder_selected"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(direction) = args.get("direction") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "direction is required (\"up\" or \"down\")".into(),
            );
        };
        let dir = match direction.as_str() {
            "up" => ReorderDirection::Up,
            "down" => ReorderDirection::Down,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("direction must be \"up\" or \"down\", got {direction:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::ReorderSelected { direction: dir },
        )
    }
}

pub fn reorder_selected_snapshot() -> ReorderSelected {
    ReorderSelected
}

/// First-party `align_selected` tool — align / distribute the current
/// multi-selection.
pub struct AlignSelected;

const ALIGN_ACTIONS: &[&str] = &[
    "left",
    "center_h",
    "right",
    "top",
    "center_v",
    "bottom",
    "distribute_h",
    "distribute_v",
];

impl McpTool for AlignSelected {
    fn name(&self) -> &str {
        "align_selected"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(action) = args.get("action") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "action is required (one of \"left\", \"center_h\", \"right\", \
                 \"top\", \"center_v\", \"bottom\", \"distribute_h\", \"distribute_v\")"
                    .into(),
            );
        };
        if !ALIGN_ACTIONS.contains(&action.as_str()) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("action must be one of {ALIGN_ACTIONS:?}, got {action:?}"),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::AlignSelected {
                action: action.clone(),
            },
        )
    }
}

pub fn align_selected_snapshot() -> AlignSelected {
    AlignSelected
}

/// First-party `copy_selected` tool — Cmd+C parity.
pub struct CopySelected;

impl McpTool for CopySelected {
    fn name(&self) -> &str {
        "copy_selected"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::CopySelected)
    }
}

pub fn copy_selected_snapshot() -> CopySelected {
    CopySelected
}

/// First-party `cut_selected` tool — Cmd+X parity.
pub struct CutSelected;

impl McpTool for CutSelected {
    fn name(&self) -> &str {
        "cut_selected"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::CutSelected)
    }
}

pub fn cut_selected_snapshot() -> CutSelected {
    CutSelected
}

/// First-party `paste_clipboard` tool — Cmd+V parity.
pub struct PasteClipboard;

impl McpTool for PasteClipboard {
    fn name(&self) -> &str {
        "paste_clipboard"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let offset_px: i32 = match args.get("offset_px") {
            None => 10,
            Some(s) => match s.parse::<i32>() {
                Ok(n) => n,
                Err(_) => {
                    return ToolOutcome::Err(
                        ToolErrorCode::InvalidArgument,
                        format!("offset_px must be an i32, got {s:?}"),
                    );
                }
            },
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::PasteClipboard { offset_px })
    }
}

pub fn paste_clipboard_snapshot() -> PasteClipboard {
    PasteClipboard
}
