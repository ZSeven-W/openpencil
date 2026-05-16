//! Component MCP tools — `instantiate_component` / `create_component`
//! / `delete_component` / `rename_component` — plus `set_node_collapsed`.
//!
//! ## Component-command gap (Phase 5 Task 5.1)
//!
//! `op-editor-core` has no component registry yet (it was a shell-core
//! `Document` concern; the canonical-schema component model is a later
//! task). `EditorState::apply` REJECTS the four `*Component` variants
//! and `NodeFlag::Collapsed` (`Collapsed` has no canonical-schema
//! field — it was editor-chrome-only state).
//!
//! Rather than queue an `EditorCommand` the applier would silently
//! reject — which would surface to the client as a generic `Internal`
//! "host rejected command" error — these five tools return a clean,
//! self-describing `ToolFailed` error directly at call time. The tools
//! stay registered (so `tools/list` is honest about the catalog) and
//! the wire contract still validates arguments; only the apply step is
//! turned into an explicit "not supported yet" failure.
//!
//! The remaining write tools that lived here in shell-core (pages /
//! selection / viewport / tool / flags / undo) moved to `page_tools.rs`.

use std::collections::BTreeMap;

use super::{McpTool, ToolErrorCode, ToolOutcome};

/// Shared error body for the component-registry gap. The message names
/// the tool + says explicitly it is an unimplemented capability so an
/// LLM client doesn't retry it as a transient failure.
fn component_gap_error(tool: &str) -> ToolOutcome {
    ToolOutcome::Err(
        ToolErrorCode::ToolFailed,
        format!(
            "{tool} is not supported yet: op-editor-core has no component \
             registry (known gap — the canonical-schema component model is a \
             later task)"
        ),
    )
}

/// First-party `instantiate_component` tool. **Gap** — no component
/// registry in `op-editor-core`; surfaces a clean `ToolFailed`.
pub struct InstantiateComponent;

impl McpTool for InstantiateComponent {
    fn name(&self) -> &str {
        "instantiate_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        // Still validate the argument shape so the contract is honest.
        if args.get("component_id").map(String::is_empty).unwrap_or(true) {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "component_id is required".into(),
            );
        }
        component_gap_error("instantiate_component")
    }
}

pub fn instantiate_component_snapshot() -> InstantiateComponent {
    InstantiateComponent
}

/// First-party `create_component` tool. **Gap** — see module docs.
pub struct CreateComponent;

impl McpTool for CreateComponent {
    fn name(&self) -> &str {
        "create_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if args.get("node_id").map(String::is_empty).unwrap_or(true) {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        }
        if args.get("name").is_none() {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required".into(),
            );
        }
        component_gap_error("create_component")
    }
}

pub fn create_component_snapshot() -> CreateComponent {
    CreateComponent
}

/// First-party `delete_component` tool. **Gap** — see module docs.
pub struct DeleteComponent;

impl McpTool for DeleteComponent {
    fn name(&self) -> &str {
        "delete_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if args.get("component_id").map(String::is_empty).unwrap_or(true) {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "component_id is required".into(),
            );
        }
        component_gap_error("delete_component")
    }
}

pub fn delete_component_snapshot() -> DeleteComponent {
    DeleteComponent
}

/// First-party `rename_component` tool. **Gap** — see module docs.
pub struct RenameComponent;

impl McpTool for RenameComponent {
    fn name(&self) -> &str {
        "rename_component"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if args.get("component_id").map(String::is_empty).unwrap_or(true) {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "component_id is required".into(),
            );
        }
        match args.get("name") {
            None => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "name is required".into(),
                );
            }
            Some(name) if name.trim().is_empty() => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "name must not be empty / whitespace-only".into(),
                );
            }
            Some(_) => {}
        }
        component_gap_error("rename_component")
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
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
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
