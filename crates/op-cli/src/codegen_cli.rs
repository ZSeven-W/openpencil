use serde_json::Value;

use super::{flag_value, json_escape, pair, required_pos, resolve_arg, tool_call, Command, Flags};

pub(super) fn map_codegen(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    match positionals[0].as_str() {
        "codegen:plan" => map_codegen_plan(positionals, flags),
        "codegen:submit" => map_codegen_submit(positionals, flags),
        "codegen:assemble" => map_codegen_assemble(positionals, flags),
        "codegen:clean" => map_codegen_clean(positionals),
        _ => unreachable!("caller guards the codegen command set"),
    }
}

fn map_codegen_plan(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let raw =
        resolve_codegen_payload(positionals, 1, "Usage: op codegen:plan <plan-json|@file|->")?;
    let plan = compact_json_object(&raw, "plan-json")?;
    let mut fields = Vec::new();
    if let Some(file_path) = flag_value(flags, "file") {
        fields.push(json_string_field("filePath", &file_path));
    }
    if let Some(page_id) = flag_value(flags, "page") {
        fields.push(json_string_field("pageId", &page_id));
    }
    fields.push(format!(r#""plan":{plan}"#));
    raw_json_tool_call("codegen_plan", fields)
}

fn map_codegen_submit(positionals: &[String], flags: &Flags) -> Result<Command, String> {
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

fn map_codegen_assemble(positionals: &[String], flags: &Flags) -> Result<Command, String> {
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

fn map_codegen_clean(positionals: &[String]) -> Result<Command, String> {
    let plan_id = required_pos(positionals, 1, "Usage: op codegen:clean <planId>")?;
    tool_call("codegen_clean", vec![pair("planId", plan_id)])
}

fn resolve_codegen_payload(
    positionals: &[String],
    index: usize,
    usage: &str,
) -> Result<String, String> {
    let arg = positionals
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| usage.to_string())?;
    resolve_arg(Some(arg))
}

fn compact_json_object(raw: &str, label: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(raw.trim())
        .map_err(|e| format!("invalid {label} JSON payload: {e}"))?;
    if !value.is_object() {
        return Err(format!("{label} must be a JSON object"));
    }
    serde_json::to_string(&value).map_err(|e| format!("cannot serialize {label}: {e}"))
}

fn json_string_field(key: &str, value: &str) -> String {
    format!(r#""{}":"{}""#, json_escape(key), json_escape(value))
}

fn raw_json_tool_call(tool: &str, fields: Vec<String>) -> Result<Command, String> {
    Ok(Command::ToolCallJson {
        tool: tool.to_string(),
        args_json: format!("{{{}}}", fields.join(",")),
    })
}
