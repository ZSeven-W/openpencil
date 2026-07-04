use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::variable::VariableDefinition;

use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};

pub struct UpsertVariablesTool;

impl McpTool for UpsertVariablesTool {
    fn name(&self) -> &str {
        "upsert_variables"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(key) = required(args, "key") else {
            return missing("key");
        };
        let Some(raw) = required(args, "variables") else {
            return missing("variables");
        };
        let variables: BTreeMap<String, VariableDefinition> = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("variables is not a valid JSON object of variable definitions: {e}"),
                );
            }
        };
        let mut out = wrote(key);
        out.insert("count".into(), variables.len().to_string());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::UpsertVariables {
                variables,
                key: key.clone(),
                source_path: args.get("sourcePath").cloned(),
                source_hash: args.get("sourceHash").cloned(),
            },
        )
    }
}

pub fn upsert_variables_snapshot() -> UpsertVariablesTool {
    UpsertVariablesTool
}

pub struct UpsertComponentTool;

impl McpTool for UpsertComponentTool {
    fn name(&self) -> &str {
        "upsert_component"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(key) = required(args, "key") else {
            return missing("key");
        };
        let Some(name) = required(args, "name") else {
            return missing("name");
        };
        let Some(raw) = required(args, "node_json") else {
            return missing("node_json");
        };
        let root = match parse_node(raw, "node_json") {
            Ok(root) => root,
            Err(message) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, message),
        };
        ToolOutcome::OkWithCommand(
            wrote(key),
            EditorCommand::UpsertComponent {
                key: key.clone(),
                name: name.clone(),
                root: Box::new(root),
                source_path: args.get("sourcePath").cloned(),
                source_hash: args.get("sourceHash").cloned(),
            },
        )
    }
}

pub fn upsert_component_snapshot() -> UpsertComponentTool {
    UpsertComponentTool
}

pub struct UpsertScreenTool;

impl McpTool for UpsertScreenTool {
    fn name(&self) -> &str {
        "upsert_screen"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(key) = required(args, "key") else {
            return missing("key");
        };
        let Some(raw) = required(args, "node_json") else {
            return missing("node_json");
        };
        let root = match parse_node(raw, "node_json") {
            Ok(root) => root,
            Err(message) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, message),
        };
        ToolOutcome::OkWithCommand(
            wrote(key),
            EditorCommand::UpsertScreen {
                key: key.clone(),
                root: Box::new(root),
                source_path: args.get("sourcePath").cloned(),
                source_hash: args.get("sourceHash").cloned(),
            },
        )
    }
}

pub fn upsert_screen_snapshot() -> UpsertScreenTool {
    UpsertScreenTool
}

fn required<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Option<&'a String> {
    args.get(key).filter(|value| !value.trim().is_empty())
}

fn missing(name: &str) -> ToolOutcome {
    ToolOutcome::Err(
        ToolErrorCode::MissingArgument,
        format!("{name} is required"),
    )
}

fn parse_node(raw: &str, arg_name: &str) -> Result<PenNode, String> {
    serde_json::from_str(raw).map_err(|e| format!("{arg_name} is not a valid PenNode: {e}"))
}

fn wrote(key: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    out.insert("wrote".into(), "true".into());
    out.insert("key".into(), key.to_string());
    out
}
