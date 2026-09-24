use serde_json::Value;

use super::{
    flag_value, json_escape, pair, required_pos, resolve_arg, resolve_file_path_arg, tool_call,
    Command, Flags,
};
use crate::cli_error::CliError;

pub(super) fn map_codegen(positionals: &[String], flags: &Flags) -> Result<Command, CliError> {
    match positionals[0].as_str() {
        "codegen:plan" => map_codegen_plan(positionals, flags),
        "codegen:submit" => map_codegen_submit(positionals, flags),
        "codegen:assemble" => map_codegen_assemble(positionals, flags),
        "codegen:clean" => map_codegen_clean(positionals),
        "codegen:export" => map_codegen_export(flags),
        _ => unreachable!("caller guards the codegen command set"),
    }
}

/// `op codegen:export [--framework F] [--nodes ids] [--page P] [--out DIR]`
/// — the deterministic `codegen_export` tool; `--out` writes every
/// returned file under DIR instead of printing the JSON result.
fn map_codegen_export(flags: &Flags) -> Result<Command, CliError> {
    let mut arguments = serde_json::Map::new();
    let pairs = [
        ("framework", "framework"),
        ("nodes", "nodeIds"),
        ("page", "pageId"),
    ];
    for (flag, key) in pairs {
        if let Some(value) = flag_value(flags, flag) {
            arguments.insert(key.into(), Value::String(value));
        }
    }
    if let Some(file_path) = flag_value(flags, "file") {
        arguments.insert(
            "filePath".into(),
            Value::String(resolve_file_path_arg(&file_path)),
        );
    }
    Ok(Command::CodegenExport {
        args_json: Value::Object(arguments).to_string(),
        out_dir: flag_value(flags, "out"),
    })
}

/// Write each `files` entry of a `codegen_export` result under `out_dir`,
/// returning a JSON summary. Entry paths come from the server, so any
/// absolute or parent-escaping path is refused rather than written.
pub(super) fn write_codegen_export(response: &str, out_dir: &str) -> Result<String, CliError> {
    let value: Value = serde_json::from_str(response).map_err(|error| {
        CliError::Payload(format!("codegen_export returned invalid JSON: {error}"))
    })?;
    let files = value
        .get("files")
        .and_then(Value::as_object)
        .ok_or_else(|| CliError::Payload("codegen_export response is missing files".into()))?;
    let root = std::path::Path::new(out_dir);
    let mut written = Vec::with_capacity(files.len());
    for (relative, content) in files {
        let path = std::path::Path::new(relative);
        let safe = path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));
        let Some(content) = content.as_str().filter(|_| safe) else {
            return Err(CliError::Payload(format!(
                "codegen_export returned an unsafe file entry {relative:?}"
            )));
        };
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                CliError::Io(format!("cannot create {}: {error}", parent.display()))
            })?;
        }
        std::fs::write(&target, content)
            .map_err(|error| CliError::Io(format!("cannot write {}: {error}", target.display())))?;
        written.push(Value::String(target.display().to_string()));
    }
    Ok(serde_json::json!({
        "framework": value.get("framework").cloned().unwrap_or(Value::Null),
        "files": written,
    })
    .to_string())
}

fn map_codegen_plan(positionals: &[String], flags: &Flags) -> Result<Command, CliError> {
    let raw =
        resolve_codegen_payload(positionals, 1, "Usage: op codegen:plan <plan-json|@file|->")?;
    let plan = compact_json_object(&raw, "plan-json")?;
    let mut fields = Vec::new();
    if let Some(file_path) = flag_value(flags, "file") {
        let resolved_file_path = resolve_file_path_arg(&file_path);
        fields.push(json_string_field("filePath", &resolved_file_path));
    }
    if let Some(page_id) = flag_value(flags, "page") {
        fields.push(json_string_field("pageId", &page_id));
    }
    fields.push(format!(r#""plan":{plan}"#));
    raw_json_tool_call("codegen_plan", fields)
}

fn map_codegen_submit(positionals: &[String], flags: &Flags) -> Result<Command, CliError> {
    let plan_id = required_pos(
        positionals,
        1,
        "Usage: op codegen:submit <planId> <chunk-result|@file|->",
    )?;
    let raw = resolve_codegen_payload(
        positionals,
        2,
        "Usage: op codegen:submit <planId> <chunk-result|@file|->",
    )?;
    let result = compact_json_object(&raw, "chunk-result")?;
    let mut fields = vec![
        json_string_field("planId", &plan_id),
        format!(r#""result":{result}"#),
    ];
    if let Some(status) = flag_value(flags, "status") {
        fields.push(json_string_field("status", &status));
    }
    raw_json_tool_call("codegen_submit_chunk", fields)
}

fn map_codegen_assemble(positionals: &[String], flags: &Flags) -> Result<Command, CliError> {
    let plan_id = required_pos(
        positionals,
        1,
        "Usage: op codegen:assemble <planId> [--framework react]",
    )?;
    let framework = flag_value(flags, "framework").unwrap_or_else(|| "react".into());
    tool_call(
        "codegen_assemble",
        vec![pair("planId", plan_id), pair("framework", framework)],
    )
}

fn map_codegen_clean(positionals: &[String]) -> Result<Command, CliError> {
    let plan_id = required_pos(positionals, 1, "Usage: op codegen:clean <planId>")?;
    tool_call("codegen_clean", vec![pair("planId", plan_id)])
}

fn resolve_codegen_payload(
    positionals: &[String],
    index: usize,
    usage: &str,
) -> Result<String, CliError> {
    let arg = positionals
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| CliError::usage(usage))?;
    resolve_arg(Some(arg))
}

fn compact_json_object(raw: &str, label: &str) -> Result<String, CliError> {
    let value: Value = serde_json::from_str(raw.trim())
        .map_err(|e| CliError::Payload(format!("invalid {label} JSON payload: {e}")))?;
    if !value.is_object() {
        return Err(CliError::Payload(format!("{label} must be a JSON object")));
    }
    serde_json::to_string(&value)
        .map_err(|e| CliError::Payload(format!("cannot serialize {label}: {e}")))
}

fn json_string_field(key: &str, value: &str) -> String {
    format!(r#""{}":"{}""#, json_escape(key), json_escape(value))
}

fn raw_json_tool_call(tool: &str, fields: Vec<String>) -> Result<Command, CliError> {
    Ok(Command::ToolCallJson {
        tool: tool.to_string(),
        args_json: format!("{{{}}}", fields.join(",")),
    })
}
