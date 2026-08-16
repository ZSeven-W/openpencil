//! Alias → `Command` mappers for the `op` CLI.
//!
//! `command_from_positionals` in `main.rs` routes each TS-compatible
//! alias here; every mapper turns positionals + flags into a `Command`.
//! Carved off `main.rs` to keep both files under the 800-line cap, plus
//! the shared arg-resolution helpers the mappers lean on
//! (`resolve_arg` / `parse_json_object` / `data_pairs` / `required_pos`).

use serde_json::Value;
use std::fs;
use std::io::{self, Read};

use crate::cli_error::CliError;
use crate::command_helpers::{flag_value, pair, push_file_path, tool_call};
use crate::mcp_http_cli::json_escape;
use crate::path_args::resolve_file_path_value;
use crate::{Command, Flags};

pub(crate) fn map_get(flags: &Flags) -> Result<Command, CliError> {
    let mut pairs = Vec::new();
    if let Some(id) = flag_value(flags, "id") {
        pairs.push(pair("nodeIds", format!(r#"["{}"]"#, json_escape(&id))));
    }
    if flag_value(flags, "type").is_some() || flag_value(flags, "name").is_some() {
        let mut fields = Vec::new();
        if let Some(kind) = flag_value(flags, "type") {
            fields.push(format!(r#""type":"{}""#, json_escape(&kind)));
        }
        if let Some(name) = flag_value(flags, "name") {
            fields.push(format!(r#""name":"{}""#, json_escape(&name)));
        }
        pairs.push(pair("patterns", format!("[{{{}}}]", fields.join(","))));
    }
    if let Some(parent) = flag_value(flags, "parent") {
        pairs.push(pair("parentId", parent));
    }
    if let Some(depth) = flag_value(flags, "depth") {
        pairs.push(pair("readDepth", depth));
    }
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    push_file_path(&mut pairs, flags);
    tool_call("batch_get", pairs)
}

pub(crate) fn map_selection(flags: &Flags) -> Result<Command, CliError> {
    let mut pairs = Vec::new();
    if let Some(depth) = flag_value(flags, "depth") {
        pairs.push(pair("readDepth", depth));
    }
    push_file_path(&mut pairs, flags);
    tool_call("get_selection", pairs)
}

pub(crate) fn map_insert(positionals: &[String], flags: &Flags) -> Result<Command, CliError> {
    let raw = resolve_arg(positionals.get(1).map(String::as_str))?;
    let mut pairs = data_pairs(&raw)?;
    if let Some(parent) = flag_value(flags, "parent") {
        pairs.push(pair("parent", parent));
    }
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    if flags.contains_key("post-process") {
        pairs.push(pair("postProcess", "true"));
    }
    push_file_path(&mut pairs, flags);
    tool_call("insert_node", pairs)
}

pub(crate) fn map_update(positionals: &[String], flags: &Flags) -> Result<Command, CliError> {
    let node_id = required_pos(positionals, 1, "Usage: op update <node-id> <json>")?;
    let raw = resolve_arg(positionals.get(2).map(String::as_str))?;
    let mut pairs = vec![pair("nodeId", node_id)];
    pairs.extend(data_pairs(&raw)?);
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    if flags.contains_key("post-process") {
        pairs.push(pair("postProcess", "true"));
    }
    push_file_path(&mut pairs, flags);
    tool_call("update_node", pairs)
}

pub(crate) fn map_replace(positionals: &[String], flags: &Flags) -> Result<Command, CliError> {
    let node_id = required_pos(positionals, 1, "Usage: op replace <node-id> <json>")?;
    let raw = resolve_arg(positionals.get(2).map(String::as_str))?;
    let mut pairs = vec![pair("nodeId", node_id)];
    pairs.extend(data_pairs(&raw)?);
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    if flags.contains_key("post-process") {
        pairs.push(pair("postProcess", "true"));
    }
    push_file_path(&mut pairs, flags);
    tool_call("replace_node", pairs)
}

pub(crate) fn map_delete(positionals: &[String], flags: &Flags) -> Result<Command, CliError> {
    let id = required_pos(positionals, 1, "Usage: op delete <node-id>")?;
    let mut pairs = vec![pair("nodeId", id)];
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    push_file_path(&mut pairs, flags);
    tool_call("delete_node", pairs)
}

pub(crate) fn map_read_nodes(positionals: &[String], flags: &Flags) -> Result<Command, CliError> {
    let mut pairs = Vec::new();
    if positionals.len() > 1 {
        // Canonical nodeIds shape matches map_get / batch_get: a JSON string
        // array. Comma-joined positionals (`op read-nodes "n10,n11"`) are
        // split client-side so they resolve to the same array.
        let ids: Vec<String> = positionals[1..]
            .iter()
            .flat_map(|raw| raw.split(','))
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .collect();
        if !ids.is_empty() {
            let json = ids
                .iter()
                .map(|id| format!("\"{}\"", json_escape(id)))
                .collect::<Vec<_>>()
                .join(",");
            pairs.push(pair("nodeIds", format!("[{json}]")));
        }
    }
    if let Some(depth) = flag_value(flags, "depth") {
        pairs.push(pair("depth", depth));
    }
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    if flags.contains_key("vars") {
        pairs.push(pair("includeVariables", "true"));
    }
    push_file_path(&mut pairs, flags);
    tool_call("read_nodes", pairs)
}

pub(crate) fn map_reparent(
    tool: &str,
    positionals: &[String],
    flags: &Flags,
) -> Result<Command, CliError> {
    let id = required_pos(
        positionals,
        1,
        "Usage: op move/copy <node-id> [--parent <parent-id>] [--page PAGE]",
    )?;
    let id_key = if tool == "copy_node" {
        "sourceId"
    } else {
        "nodeId"
    };
    let parent = flag_value(flags, "parent").unwrap_or_else(|| "null".into());
    let mut pairs = vec![pair(id_key, id), pair("parent", parent)];
    if tool == "move_node" {
        if let Some(index) = flag_value(flags, "index") {
            pairs.push(pair("index", index));
        }
    }
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    push_file_path(&mut pairs, flags);
    tool_call(tool, pairs)
}

pub(crate) fn map_design_like(
    tool: &str,
    payload: Option<&String>,
    flags: &Flags,
    default_post_process: bool,
) -> Result<Command, CliError> {
    let raw = resolve_arg(payload.map(String::as_str))?;
    let trimmed = raw.trim();
    let script_requested = flags.contains_key("script")
        || payload
            .map(String::as_str)
            .is_some_and(|p| p.starts_with('@') && (p.ends_with(".js") || p.ends_with(".mjs")));
    let mut pairs = Vec::new();
    if script_requested {
        pairs.push(pair("script", trimmed));
    } else if trimmed.starts_with('[') {
        pairs.push(pair("nodes_json", trimmed));
    } else {
        pairs.push(pair("operations", trimmed));
    }
    if default_post_process {
        pairs.push(pair("postProcess", "true"));
    }
    if let Some(canvas_width) = flag_value(flags, "canvas-width") {
        pairs.push(pair("canvasWidth", canvas_width));
    }
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    push_file_path(&mut pairs, flags);
    tool_call(tool, pairs)
}

pub(crate) fn map_design_content(
    positionals: &[String],
    flags: &Flags,
) -> Result<Command, CliError> {
    let section_id = required_pos(
        positionals,
        1,
        "Usage: op design:content <section-id> <json|@file|->",
    )?;
    let raw = resolve_arg(positionals.get(2).map(String::as_str))?;
    let payload: Value = serde_json::from_str(raw.trim())
        .map_err(|e| CliError::Payload(format!("invalid design:content JSON payload: {e}")))?;
    let children = payload.get("children").ok_or_else(|| {
        CliError::Payload("design:content JSON payload must contain a children array".into())
    })?;
    if !children.is_array() {
        return Err(CliError::Payload(
            "design:content children must be an array".into(),
        ));
    }

    let mut args = serde_json::Map::new();
    args.insert("sectionId".into(), Value::String(section_id));
    args.insert("children".into(), children.clone());
    if let Some(canvas_width) = flag_value(flags, "canvas-width") {
        let width = canvas_width.parse::<i64>().map_err(|_| {
            CliError::Usage(format!(
                "--canvas-width must be an integer, got {canvas_width:?}"
            ))
        })?;
        args.insert("canvasWidth".into(), Value::Number(width.into()));
    }
    if let Some(page) = flag_value(flags, "page") {
        args.insert("pageId".into(), Value::String(page));
    }
    if let Some(file) = flag_value(flags, "file") {
        args.insert("filePath".into(), resolve_file_path_value(&file));
    }
    let args_json = serde_json::to_string(&Value::Object(args))
        .map_err(|e| CliError::Payload(format!("cannot serialize design:content args: {e}")))?;
    Ok(Command::ToolCallJson {
        tool: "design_content".into(),
        args_json,
    })
}

pub(crate) fn map_design_skeleton(
    positionals: &[String],
    flags: &Flags,
) -> Result<Command, CliError> {
    let raw = resolve_arg(positionals.get(1).map(String::as_str))?;
    let payload: Value = serde_json::from_str(raw.trim())
        .map_err(|e| CliError::Payload(format!("invalid design:skeleton JSON payload: {e}")))?;
    let root_frame = payload.get("rootFrame").ok_or_else(|| {
        CliError::Payload("design:skeleton JSON payload must contain rootFrame".into())
    })?;
    if !root_frame.is_object() {
        return Err(CliError::Payload(
            "design:skeleton rootFrame must be an object".into(),
        ));
    }
    let sections = payload.get("sections").ok_or_else(|| {
        CliError::Payload("design:skeleton JSON payload must contain sections".into())
    })?;
    if !sections.is_array() {
        return Err(CliError::Payload(
            "design:skeleton sections must be an array".into(),
        ));
    }

    let mut args = serde_json::Map::new();
    args.insert("rootFrame".into(), root_frame.clone());
    args.insert("sections".into(), sections.clone());
    if let Some(style_guide) = payload.get("styleGuide") {
        if !style_guide.is_object() {
            return Err(CliError::Payload(
                "design:skeleton styleGuide must be an object".into(),
            ));
        }
        args.insert("styleGuide".into(), style_guide.clone());
    }
    if let Some(canvas_width) = flag_value(flags, "canvas-width") {
        let width = canvas_width.parse::<i64>().map_err(|_| {
            CliError::Usage(format!(
                "--canvas-width must be an integer, got {canvas_width:?}"
            ))
        })?;
        args.insert("canvasWidth".into(), Value::Number(width.into()));
    }
    if let Some(page) = flag_value(flags, "page") {
        args.insert("pageId".into(), Value::String(page));
    }
    if let Some(file) = flag_value(flags, "file") {
        args.insert("filePath".into(), resolve_file_path_value(&file));
    }
    let args_json = serde_json::to_string(&Value::Object(args))
        .map_err(|e| CliError::Payload(format!("cannot serialize design:skeleton args: {e}")))?;
    Ok(Command::ToolCallJson {
        tool: "design_skeleton".into(),
        args_json,
    })
}

pub(crate) fn map_design_refine(flags: &Flags) -> Result<Command, CliError> {
    let root_id = flag_value(flags, "root-id")
        .ok_or_else(|| CliError::usage("Usage: op design:refine --root-id <id>"))?;
    let mut pairs = vec![pair("rootId", root_id)];
    if let Some(canvas_width) = flag_value(flags, "canvas-width") {
        pairs.push(pair("canvasWidth", canvas_width));
    }
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    push_file_path(&mut pairs, flags);
    tool_call("design_refine", pairs)
}

pub(crate) fn map_layout(flags: &Flags) -> Result<Command, CliError> {
    let mut pairs = Vec::new();
    if let Some(parent) = flag_value(flags, "parent") {
        pairs.push(pair("parentId", parent));
    }
    if let Some(depth) = flag_value(flags, "depth") {
        pairs.push(pair("maxDepth", depth));
    }
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    push_file_path(&mut pairs, flags);
    tool_call("snapshot_layout", pairs)
}

pub(crate) fn map_find_space(flags: &Flags) -> Result<Command, CliError> {
    let direction = flag_value(flags, "direction").unwrap_or_else(|| "right".into());
    let width = flag_value(flags, "width").unwrap_or_else(|| "400".into());
    let height = flag_value(flags, "height").unwrap_or_else(|| "300".into());
    let mut pairs = vec![
        pair("direction", direction),
        pair("width", width),
        pair("height", height),
    ];
    if let Some(padding) = flag_value(flags, "padding") {
        pairs.push(pair("padding", padding));
    }
    if let Some(node_id) = flag_value(flags, "node") {
        pairs.push(pair("nodeId", node_id));
    }
    if let Some(node_id) = flag_value(flags, "node-id") {
        pairs.push(pair("nodeId", node_id));
    }
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    push_file_path(&mut pairs, flags);
    tool_call("find_empty_space", pairs)
}

pub(crate) fn generic_tool_call(
    tool: &str,
    rest: &[String],
    flags: &Flags,
) -> Result<Command, CliError> {
    let mut pairs = Vec::new();
    for kv in rest {
        let (k, v) = kv
            .split_once('=')
            .ok_or_else(|| CliError::Usage(format!("argument must be key=value, got {kv:?}")))?;
        if k.is_empty() {
            return Err(CliError::Usage(format!(
                "argument has an empty key: {kv:?}"
            )));
        }
        pairs.push(pair(k, v));
    }
    push_file_path(&mut pairs, flags);
    for (k, v) in flags {
        if is_global_compat_flag(k) {
            continue;
        }
        pairs.push(pair(k, v.clone().unwrap_or_else(|| "true".into())));
    }
    tool_call(tool, pairs)
}

pub(crate) fn is_global_compat_flag(key: &str) -> bool {
    ["file", "page", "post-process", "canvas-width", "depth"].contains(&key)
}

pub(crate) fn parse_json_object(raw: &str) -> Result<Value, CliError> {
    let value: Value = serde_json::from_str(raw.trim())
        .map_err(|e| CliError::Payload(format!("invalid JSON payload: {e}")))?;
    if !value.is_object() {
        return Err(CliError::Payload("JSON payload must be an object".into()));
    }
    Ok(value)
}

pub(crate) fn data_pairs(raw: &str) -> Result<Vec<(String, String)>, CliError> {
    parse_json_object(raw)?;
    Ok(vec![pair("data", raw.trim())])
}

pub(crate) fn resolve_arg(arg: Option<&str>) -> Result<String, CliError> {
    match arg {
        Some("-") => {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(|e| CliError::Io(format!("stdin read failed: {e}")))?;
            if input.trim().is_empty() {
                Err(CliError::usage("No data received from stdin"))
            } else {
                Ok(input.trim().to_string())
            }
        }
        Some(path) if path.starts_with('@') => {
            let path = &path[1..];
            fs::read_to_string(path)
                .map(|s| s.trim().to_string())
                .map_err(|e| CliError::Io(format!("cannot read file {path:?}: {e}")))
        }
        Some(value) => Ok(value.to_string()),
        None => Err(CliError::usage(
            "No data provided. Pass as argument, @filepath, or '-' for stdin",
        )),
    }
}

pub(crate) fn required_pos(
    positionals: &[String],
    index: usize,
    usage: &str,
) -> Result<String, CliError> {
    positionals
        .get(index)
        .cloned()
        .ok_or_else(|| CliError::usage(usage))
}
