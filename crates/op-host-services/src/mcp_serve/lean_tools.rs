//! The two lean-profile tools that compose several existing tools.
//!
//! The lean catalog keeps six names, so two of them absorb capabilities that
//! the full catalog spreads across separate tools. Neither re-implements
//! anything: each holds the existing tools' snapshots and calls them, then
//! merges their outcomes. A behaviour change in `read_nodes`, `list_pages`,
//! `export_nodes`, … therefore reaches the lean profile for free.
//!
//! - [`LeanEditorState`] — `get_editor_state` + `get_document_info` +
//!   `list_pages` + `list_variables` + `get_active_theme`, and `read_nodes`
//!   when the caller asks for node JSON.
//! - [`LeanScreenshot`] — `get_screenshot`, or `export_nodes` when the caller
//!   names an export `format`.

use std::collections::BTreeMap;

use op_editor_core::EditorState;
use op_editor_ui::layout_scene::LayoutScene;
use op_mcp::{
    document_info_snapshot, get_active_theme_snapshot, get_editor_state_snapshot,
    list_pages_snapshot, list_variables_snapshot, read_nodes_snapshot, McpTool, ToolErrorCode,
    ToolOutcome,
};
use serde_json::{Map, Value};

use super::export_tool::export_nodes_from_scene;
use super::screenshot_tool::{get_screenshot_from_scene, scene_root_node_id};

/// Arguments that make the lean `get_editor_state` include node JSON.
const NODE_READ_ARGS: &[&str] = &["nodeIds", "node_ids", "ids", "depth"];
/// Arguments forwarded verbatim to `read_nodes`.
const READ_NODES_ARGS: &[&str] = &["nodeIds", "node_ids", "ids", "depth", "pageId", "page_id"];

type Failure = (ToolErrorCode, String);

/// Turn one delegated tool's outcome into a JSON value for the merged reply.
fn outcome_value(tool: &str, outcome: ToolOutcome) -> Result<Value, Failure> {
    match outcome {
        ToolOutcome::Ok(map) => Ok(Value::Object(
            map.into_iter()
                .map(|(key, value)| (key, Value::String(value)))
                .collect(),
        )),
        ToolOutcome::OkJson(json) => serde_json::from_str(&json).map_err(|error| {
            (
                ToolErrorCode::Internal,
                format!("{tool} returned invalid JSON: {error}"),
            )
        }),
        ToolOutcome::Err(code, message) => Err((code, message)),
        // Every delegate here is a read tool; a command or an image from one
        // would be a contract change upstream, not something to merge.
        other => Err((
            ToolErrorCode::Internal,
            format!("{tool} returned an unexpected outcome: {other:?}"),
        )),
    }
}

// --- get_editor_state (lean) -------------------------------------------

/// Lean `get_editor_state`: one read that answers "where am I" for a design
/// task, plus optional node reads so the lean catalog needs no separate
/// `read_nodes` / `batch_get`.
pub(crate) struct LeanEditorState {
    editor: Box<dyn McpTool>,
    document: Box<dyn McpTool>,
    pages: Box<dyn McpTool>,
    variables: Box<dyn McpTool>,
    theme: Box<dyn McpTool>,
    nodes: Box<dyn McpTool>,
}

impl LeanEditorState {
    pub(crate) fn snapshot(state: &EditorState) -> Self {
        Self {
            editor: Box::new(get_editor_state_snapshot(state)),
            document: Box::new(document_info_snapshot(state)),
            pages: Box::new(list_pages_snapshot(state)),
            variables: Box::new(list_variables_snapshot(state)),
            theme: Box::new(get_active_theme_snapshot(state)),
            nodes: Box::new(read_nodes_snapshot(state)),
        }
    }

    fn merged(&self, args: &BTreeMap<String, String>) -> Result<Value, Failure> {
        let none = BTreeMap::new();
        let mut out = Map::new();
        for (key, tool) in [
            ("editor", &self.editor),
            ("document", &self.document),
            ("pages", &self.pages),
            ("variables", &self.variables),
            ("theme", &self.theme),
        ] {
            out.insert(key.into(), outcome_value(tool.name(), tool.call(&none))?);
        }
        if NODE_READ_ARGS.iter().any(|key| args.contains_key(*key)) {
            let forwarded: BTreeMap<String, String> = args
                .iter()
                .filter(|(key, _)| READ_NODES_ARGS.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            let read = outcome_value(self.nodes.name(), self.nodes.call(&forwarded))?;
            out.insert(
                "nodes".into(),
                read.get("nodes")
                    .cloned()
                    .unwrap_or(Value::Array(Vec::new())),
            );
        }
        Ok(Value::Object(out))
    }
}

impl McpTool for LeanEditorState {
    fn name(&self) -> &str {
        "get_editor_state"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        match self.merged(args) {
            Ok(value) => ToolOutcome::OkJson(value.to_string()),
            Err((code, message)) => ToolOutcome::Err(code, message),
        }
    }
}

// --- get_screenshot (lean) ---------------------------------------------

/// Lean `get_screenshot`: an inline PNG for visual verification, or — when
/// `format` is given — the `export_nodes` file bytes for that node.
pub(crate) struct LeanScreenshot {
    scene: LayoutScene,
}

impl LeanScreenshot {
    pub(crate) fn snapshot(state: &EditorState) -> Self {
        // One layout derive serves both paths; each call builds the tool it
        // needs over a clone, which is far cheaper than a second derive.
        Self {
            scene: op_pen_loader::editor_state_to_active_page_layout_scene(state),
        }
    }

    fn export(&self, args: &BTreeMap<String, String>, format: &str) -> ToolOutcome {
        let raw_id = match args.get("nodeId").map(|id| id.trim()) {
            Some(id) if !id.is_empty() => id,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "nodeId is required (pass a node id or \"root\")".into(),
                )
            }
        };
        let node_id = if raw_id == "root" {
            match scene_root_node_id(&self.scene) {
                Some(id) => id,
                None => {
                    return ToolOutcome::Err(
                        ToolErrorCode::ToolFailed,
                        "active page is empty — no root node to export".into(),
                    )
                }
            }
        } else {
            raw_id.to_string()
        };
        let mut forwarded = BTreeMap::new();
        forwarded.insert(
            "nodeIds".to_string(),
            Value::from(vec![node_id]).to_string(),
        );
        forwarded.insert("format".to_string(), format.to_string());
        if let Some(scale) = args.get("scale") {
            forwarded.insert("scale".to_string(), scale.clone());
        }
        export_nodes_from_scene(self.scene.clone()).call(&forwarded)
    }
}

impl McpTool for LeanScreenshot {
    fn name(&self) -> &str {
        "get_screenshot"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        match args
            .get("format")
            .map(|format| format.trim())
            .filter(|format| !format.is_empty())
        {
            Some(format) => self.export(args, format),
            None => get_screenshot_from_scene(self.scene.clone()).call(args),
        }
    }
}
