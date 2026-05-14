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

