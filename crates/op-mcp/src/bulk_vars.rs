//! Bulk variable/theme MCP tools that mirror the TypeScript
//! `get_variables`, `set_variables`, and `set_themes` routes.

use std::collections::BTreeMap;

use jian_ops_schema::variable::VariableDefinition;
use op_editor_core::EditorState;

use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};

pub struct GetVariables {
    pub variables_json: String,
    pub themes_json: String,
    pub variable_count: usize,
    pub theme_axis_count: usize,
}

impl McpTool for GetVariables {
    fn name(&self) -> &str {
        "get_variables"
    }

    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        out.insert("variables".into(), self.variables_json.clone());
        out.insert("themes".into(), self.themes_json.clone());
        out.insert("variable_count".into(), self.variable_count.to_string());
        out.insert("theme_axis_count".into(), self.theme_axis_count.to_string());
        ToolOutcome::Ok(out)
    }
}

pub fn get_variables_snapshot(state: &EditorState) -> GetVariables {
    let variables = state.doc.variables.clone().unwrap_or_default();
    let themes = state.doc.themes.clone().unwrap_or_default();
    GetVariables {
        variables_json: serde_json::to_string(&variables).unwrap_or_else(|_| "{}".into()),
        themes_json: serde_json::to_string(&themes).unwrap_or_else(|_| "{}".into()),
        variable_count: variables.len(),
        theme_axis_count: themes.len(),
    }
}

pub struct SetVariables;

impl McpTool for SetVariables {
    fn name(&self) -> &str {
        "set_variables"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("variables") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "variables is required".into(),
            );
        };
        let variables: BTreeMap<String, VariableDefinition> = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("variables must be a JSON object of variable definitions: {e}"),
                );
            }
        };
        let replace = match parse_replace(args) {
            Ok(v) => v,
            Err(msg) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg),
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        out.insert("variable_count".into(), variables.len().to_string());
        ToolOutcome::OkWithCommand(out, EditorCommand::SetVariables { variables, replace })
    }
}

pub fn set_variables_snapshot() -> SetVariables {
    SetVariables
}

pub struct SetThemes;

impl McpTool for SetThemes {
    fn name(&self) -> &str {
        "set_themes"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw) = args.get("themes") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "themes is required".into());
        };
        let themes: BTreeMap<String, Vec<String>> = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("themes must be a JSON object of axis names to string arrays: {e}"),
                );
            }
        };
        let replace = match parse_replace(args) {
            Ok(v) => v,
            Err(msg) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg),
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        out.insert("theme_axis_count".into(), themes.len().to_string());
        ToolOutcome::OkWithCommand(out, EditorCommand::SetThemes { themes, replace })
    }
}

pub fn set_themes_snapshot() -> SetThemes {
    SetThemes
}

fn parse_replace(args: &BTreeMap<String, String>) -> Result<bool, String> {
    match args.get("replace").map(String::as_str) {
        None => Ok(false),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(other) => Err(format!("replace must be true or false, got {other:?}")),
    }
}
