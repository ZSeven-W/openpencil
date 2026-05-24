//! Component MCP tools — `instantiate_component` / `create_component`
//! / `delete_component` / `rename_component` — plus `set_node_collapsed`.
//!
//! The remaining write tools that lived here in shell-core (pages /
//! selection / viewport / tool / flags / undo) moved to `page_tools.rs`.

use std::collections::BTreeMap;

use op_editor_core::NodeId;

use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};

fn wrote() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    out.insert("wrote".into(), "true".into());
    out
}

/// First-party `instantiate_component` tool.
pub struct InstantiateComponent;

impl McpTool for InstantiateComponent {
    fn name(&self) -> &str {
        "instantiate_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        // Still validate the argument shape so the contract is honest.
        if args
            .get("component_id")
            .map(String::is_empty)
            .unwrap_or(true)
        {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "component_id is required".into(),
            );
        }
        ToolOutcome::OkWithCommand(
            wrote(),
            EditorCommand::InstantiateComponent {
                component_id: NodeId::new(args["component_id"].clone()),
            },
        )
    }
}

pub fn instantiate_component_snapshot() -> InstantiateComponent {
    InstantiateComponent
}

/// First-party `create_component` tool.
pub struct CreateComponent;

impl McpTool for CreateComponent {
    fn name(&self) -> &str {
        "create_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if args.get("node_id").map(String::is_empty).unwrap_or(true) {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "node_id is required".into());
        }
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
        };
        if name.trim().is_empty() {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "name must not be empty / whitespace-only".into(),
            );
        }
        ToolOutcome::OkWithCommand(
            wrote(),
            EditorCommand::CreateComponent {
                node_id: NodeId::new(args["node_id"].clone()),
                name: name.clone(),
            },
        )
    }
}

pub fn create_component_snapshot() -> CreateComponent {
    CreateComponent
}

/// First-party `delete_component` tool.
pub struct DeleteComponent;

impl McpTool for DeleteComponent {
    fn name(&self) -> &str {
        "delete_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if args
            .get("component_id")
            .map(String::is_empty)
            .unwrap_or(true)
        {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "component_id is required".into(),
            );
        }
        ToolOutcome::OkWithCommand(
            wrote(),
            EditorCommand::DeleteComponent {
                component_id: NodeId::new(args["component_id"].clone()),
            },
        )
    }
}

pub fn delete_component_snapshot() -> DeleteComponent {
    DeleteComponent
}

/// First-party `rename_component` tool.
pub struct RenameComponent;

impl McpTool for RenameComponent {
    fn name(&self) -> &str {
        "rename_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if args
            .get("component_id")
            .map(String::is_empty)
            .unwrap_or(true)
        {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "component_id is required".into(),
            );
        }
        match args.get("name") {
            None => {
                return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
            }
            Some(name) if name.trim().is_empty() => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "name must not be empty / whitespace-only".into(),
                );
            }
            Some(_) => {}
        }
        ToolOutcome::OkWithCommand(
            wrote(),
            EditorCommand::RenameComponent {
                component_id: NodeId::new(args["component_id"].clone()),
                name: args["name"].clone(),
            },
        )
    }
}

pub fn rename_component_snapshot() -> RenameComponent {
    RenameComponent
}

/// First-party `set_node_collapsed` tool — toggle a node's layer-panel
/// disclosure state.
///
/// **Gap** — the layer-panel `collapsed` flag is editor-chrome-only
/// state; the canonical `PenNodeBase` has no `collapsed` field, so
/// `EditorState::apply` rejects `NodeFlag::Collapsed`. The tool returns
/// a clean `ToolFailed` rather than queueing a doomed command.
pub struct SetNodeCollapsed;

impl McpTool for SetNodeCollapsed {
    fn name(&self) -> &str {
        "set_node_collapsed"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if args.get("node_id").map(String::is_empty).unwrap_or(true) {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "node_id is required".into());
        }
        match args.get("value").map(String::as_str) {
            Some("true") | Some("false") => {}
            Some(other) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("value must be \"true\" or \"false\", got {other:?}"),
                );
            }
            None => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "value is required (\"true\" or \"false\")".into(),
                );
            }
        }
        ToolOutcome::Err(
            ToolErrorCode::ToolFailed,
            "set_node_collapsed is not supported: the layer-panel `collapsed` \
             flag is editor-chrome-only state with no canonical-schema field \
             (known gap)"
                .into(),
        )
    }
}

pub fn set_node_collapsed_snapshot() -> SetNodeCollapsed {
    SetNodeCollapsed
}
