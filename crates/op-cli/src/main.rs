//! `op` - the OpenPencil command-line tool.
//!
//! The Rust CLI talks to the live Rust MCP HTTP server and keeps the
//! common command surface aligned with the TypeScript `op` CLI where
//! the Rust MCP tool set already supports the behavior. Unknown
//! commands still fall back to the low-level `op <tool> key=value`
//! form so new MCP tools remain immediately scriptable.

use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};

mod app_control_cli;
mod codegen_cli;
mod figma_cli;
mod mcp_http_cli;
mod skill_install_cli;

use mcp_http_cli::{
    args_to_json, json_escape, post, pretty_json, status_json, tool_call_body, tools_list_body,
};

#[cfg(test)]
use mcp_http_cli::{http_request, status_json_from_running};

#[cfg(test)]
use figma_cli::figma_default_out_path;

/// Default HTTP MCP port, matching TS `@zseven-w/pen-mcp`.
const DEFAULT_PORT: u16 = 3100;

const USAGE: &str = include_str!("usage.txt");

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            println!("{out}");
        }
        Err(e) => {
            eprintln!("op: {e}");
            std::process::exit(1);
        }
    }
}

/// Parse `args`, perform the request, return the text to print.
fn run(args: &[String]) -> Result<String, String> {
    let Parsed {
        port,
        pretty,
        command,
    } = parse_args(args)?;
    let out = match command {
        Command::Help => USAGE.to_string(),
        Command::Version => version_json(),
        Command::Status => status_json(port),
        Command::StartMcp { document_path } => {
            app_control_cli::run_start(port, document_path.as_deref())?
        }
        Command::StopMcp => app_control_cli::run_stop()?,
        Command::InstallSkill { target } => skill_install_cli::run_install(target.as_deref())?,
        Command::UninstallSkill { target } => skill_install_cli::run_uninstall(target.as_deref())?,
        Command::ToolsList => post(port, &tools_list_body())?,
        Command::ImportFigma { fig_path, out_path } => {
            figma_cli::run_import_figma(&fig_path, &out_path)?
        }
        Command::ToolCall { tool, args } => {
            post(port, &tool_call_body(&tool, &args_to_json(&args)))?
        }
        Command::ToolCallJson { tool, args_json } => {
            post(port, &tool_call_body(&tool, &args_json))?
        }
    };
    Ok(if pretty { pretty_json(&out) } else { out })
}

#[derive(Debug, PartialEq, Eq)]
struct Parsed {
    port: u16,
    pretty: bool,
    command: Command,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Version,
    Status,
    StartMcp {
        document_path: Option<String>,
    },
    StopMcp,
    InstallSkill {
        target: Option<String>,
    },
    UninstallSkill {
        target: Option<String>,
    },
    ToolsList,
    ImportFigma {
        fig_path: String,
        out_path: String,
    },
    ToolCall {
        tool: String,
        args: Vec<(String, String)>,
    },
    ToolCallJson {
        tool: String,
        args_json: String,
    },
}

type Flags = BTreeMap<String, Option<String>>;

/// Parse command-line args. `--port`, `--pretty`, `--help`, and
/// `--version` are global; the rest are left for command aliases or
/// low-level MCP tool arguments.
fn parse_args(args: &[String]) -> Result<Parsed, String> {
    let mut port = DEFAULT_PORT;
    let mut pretty = false;
    let mut positionals = Vec::new();
    let mut flags: Flags = BTreeMap::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--" {
            positionals.extend(args[i + 1..].iter().cloned());
            break;
        }
        if arg == "-h" {
            flags.insert("help".into(), None);
            i += 1;
            continue;
        }
        if arg == "-V" {
            flags.insert("version".into(), None);
            i += 1;
            continue;
        }
        if let Some(raw) = arg.strip_prefix("--") {
            let (key, inline_value) = match raw.split_once('=') {
                Some((k, v)) => (k.to_string(), Some(v.to_string())),
                None => (raw.to_string(), None),
            };
            match key.as_str() {
                "port" => {
                    let raw_port = match inline_value {
                        Some(v) => v,
                        None => {
                            let next = args
                                .get(i + 1)
                                .ok_or("--port needs a value (e.g. --port 3100)")?;
                            i += 1;
                            next.clone()
                        }
                    };
                    port = raw_port
                        .parse::<u16>()
                        .map_err(|_| format!("--port must be a u16, got {raw_port:?}"))?;
                }
                "pretty" => pretty = true,
                "help" => {
                    flags.insert("help".into(), None);
                }
                "version" => {
                    flags.insert("version".into(), None);
                }
                _ => {
                    let value = match inline_value {
                        Some(v) => Some(v),
                        None if args.get(i + 1).is_some_and(|next| !next.starts_with("--")) => {
                            i += 1;
                            Some(args[i].clone())
                        }
                        None => None,
                    };
                    flags.insert(key, value);
                }
            }
        } else {
            positionals.push(arg.clone());
        }
        i += 1;
    }

    let command = if flags.contains_key("help") {
        Command::Help
    } else if flags.contains_key("version") {
        Command::Version
    } else if positionals.is_empty() {
        Command::Help
    } else {
        command_from_positionals(&positionals, &flags)?
    };
    Ok(Parsed {
        port,
        pretty,
        command,
    })
}

fn command_from_positionals(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    match positionals[0].as_str() {
        "help" | "-h" | "--help" => Ok(Command::Help),
        "version" => Ok(Command::Version),
        "tools" => Ok(Command::ToolsList),
        "status" => Ok(Command::Status),
        "start" => Ok(Command::StartMcp {
            document_path: flag_value(flags, "file"),
        }),
        "stop" => Ok(Command::StopMcp),
        "install" => Ok(Command::InstallSkill {
            target: flag_value(flags, "target"),
        }),
        "uninstall" => Ok(Command::UninstallSkill {
            target: flag_value(flags, "target"),
        }),
        "open" => {
            let args = flag_value(flags, "file")
                .or_else(|| positionals.get(1).cloned())
                .map(|path| vec![pair("filePath", path)])
                .unwrap_or_default();
            tool_call("open_document", args)
        }
        "save" => {
            let file_path = required_pos(positionals, 1, "Usage: op save <file.op>")?;
            tool_call("save_document", vec![pair("filePath", file_path)])
        }
        "get" => map_get(flags),
        "selection" => tool_call("get_selection", vec![]),
        "insert" => map_insert(positionals),
        "update" => map_update(positionals),
        "delete" => map_one_id("delete_node", positionals),
        "read-nodes" => map_read_nodes(positionals, flags),
        "move" => map_reparent("move_node", positionals, flags),
        "copy" => map_reparent("copy_node", positionals, flags),
        "replace" => map_replace(positionals),
        "design" => map_design_like("batch_design", positionals.get(1)),
        "design:skeleton" => map_design_like("design_skeleton", positionals.get(1)),
        "design:content" => {
            let payload = positionals.get(2).or_else(|| positionals.get(1));
            map_design_like("design_content", payload)
        }
        "design:refine" => map_design_like("design_refine", positionals.get(1)),
        "page" => map_page(positionals, flags),
        "vars" => tool_call("get_variables", vec![]),
        "vars:set" => map_vars_set(positionals, flags),
        "themes" => tool_call("get_active_theme", vec![]),
        "themes:set" => map_themes_set(positionals, flags),
        "theme:save" => map_theme_save(positionals, flags),
        "theme:load" => map_theme_load(positionals),
        "theme:list" => map_theme_list(positionals),
        "layout" => map_layout(flags),
        "find-space" => map_find_space(flags),
        "import:svg" => map_import_svg(positionals, flags),
        "import:figma" => figma_cli::map_import_figma(positionals, flags),
        "codegen:plan" | "codegen:submit" | "codegen:assemble" | "codegen:clean" => {
            codegen_cli::map_codegen(positionals, flags)
        }
        tool => generic_tool_call(tool, &positionals[1..], flags),
    }
}

fn map_get(flags: &Flags) -> Result<Command, String> {
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
    tool_call("batch_get", pairs)
}

fn map_insert(positionals: &[String]) -> Result<Command, String> {
    let raw = resolve_arg(positionals.get(1).map(String::as_str))?;
    tool_call("insert_node", insert_pairs(&raw)?)
}

fn map_update(positionals: &[String]) -> Result<Command, String> {
    let node_id = required_pos(positionals, 1, "Usage: op update <node-id> <json>")?;
    let raw = resolve_arg(positionals.get(2).map(String::as_str))?;
    let mut pairs = vec![pair("node_id", node_id)];
    pairs.extend(update_pairs(&raw)?);
    tool_call("update_node", pairs)
}

fn map_replace(positionals: &[String]) -> Result<Command, String> {
    let node_id = required_pos(positionals, 1, "Usage: op replace <node-id> <json>")?;
    let raw = resolve_arg(positionals.get(2).map(String::as_str))?;
    let mut pairs = vec![pair("node_id", node_id)];
    pairs.extend(replace_pairs(&raw)?);
    tool_call("replace_node", pairs)
}

fn map_one_id(tool: &str, positionals: &[String]) -> Result<Command, String> {
    let id = required_pos(positionals, 1, "Usage: op delete <node-id>")?;
    tool_call(tool, vec![pair("node_id", id)])
}

fn map_read_nodes(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let mut pairs = Vec::new();
    if let Some(ids) = positionals.get(1) {
        pairs.push(pair("nodeIds", ids.clone()));
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
    tool_call("read_nodes", pairs)
}

fn map_reparent(tool: &str, positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let id = required_pos(
        positionals,
        1,
        "Usage: op move/copy <node-id> [--parent <parent-id>]",
    )?;
    let parent = flag_value(flags, "parent").unwrap_or_default();
    tool_call(
        tool,
        vec![pair("node_id", id), pair("target_parent_id", parent)],
    )
}

fn map_design_like(tool: &str, payload: Option<&String>) -> Result<Command, String> {
    let raw = resolve_arg(payload.map(String::as_str))?;
    let trimmed = raw.trim();
    if !trimmed.starts_with('[') {
        return Err(format!(
            "{tool} in Rust MCP currently accepts a JSON array payload (nodes_json), not the TS batch DSL"
        ));
    }
    tool_call(tool, vec![pair("nodes_json", trimmed)])
}

fn map_page(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let sub = positionals
        .get(1)
        .map(String::as_str)
        .ok_or("Usage: op page list|add|remove|rename|reorder|duplicate ...")?;
    match sub {
        "list" => tool_call("list_pages", vec![]),
        "add" => {
            let name = flag_value(flags, "name").or_else(|| positionals.get(2).cloned());
            let mut pairs = Vec::new();
            if let Some(name) = name {
                pairs.push(pair("name", name));
            }
            tool_call("add_page", pairs)
        }
        "remove" | "delete" => {
            let page_id = required_pos(positionals, 2, "Usage: op page remove <page-id>")?;
            tool_call("remove_page", vec![pair("pageId", page_id)])
        }
        "rename" => {
            let page_id = required_pos(positionals, 2, "Usage: op page rename <page-id> <name>")?;
            let name = required_pos(positionals, 3, "Usage: op page rename <page-id> <name>")?;
            tool_call(
                "rename_page",
                vec![pair("pageId", page_id), pair("name", name)],
            )
        }
        "reorder" => {
            let page_id = required_pos(positionals, 2, "Usage: op page reorder <page-id> <index>")?;
            let index = required_pos(positionals, 3, "Usage: op page reorder <page-id> <index>")?;
            tool_call(
                "reorder_page",
                vec![pair("pageId", page_id), pair("index", index)],
            )
        }
        "duplicate" => {
            let page_id = required_pos(positionals, 2, "Usage: op page duplicate <page-id>")?;
            let mut pairs = vec![pair("pageId", page_id)];
            if let Some(name) = flag_value(flags, "name") {
                pairs.push(pair("name", name));
            }
            tool_call("duplicate_page", pairs)
        }
        _ => Err(format!("unknown page subcommand {sub:?}")),
    }
}

fn map_vars_set(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let raw = resolve_arg(positionals.get(1).map(String::as_str))?;
    let mut pairs = vec![pair("variables", raw)];
    if flags.contains_key("replace") {
        pairs.push(pair("replace", "true"));
    }
    tool_call("set_variables", pairs)
}

fn map_themes_set(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let raw = resolve_arg(positionals.get(1).map(String::as_str))?;
    let mut pairs = vec![pair("themes", raw)];
    if flags.contains_key("replace") {
        pairs.push(pair("replace", "true"));
    }
    tool_call("set_themes", pairs)
}

fn map_theme_save(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let preset_path = required_pos(positionals, 1, "Usage: op theme:save <file.optheme>")?;
    let mut pairs = vec![pair("presetPath", preset_path)];
    if let Some(name) = flag_value(flags, "name") {
        pairs.push(pair("name", name));
    }
    tool_call("save_theme_preset", pairs)
}

fn map_theme_load(positionals: &[String]) -> Result<Command, String> {
    let preset_path = required_pos(positionals, 1, "Usage: op theme:load <file.optheme>")?;
    tool_call("load_theme_preset", vec![pair("presetPath", preset_path)])
}

fn map_theme_list(positionals: &[String]) -> Result<Command, String> {
    let directory = required_pos(positionals, 1, "Usage: op theme:list <directory>")?;
    tool_call("list_theme_presets", vec![pair("directory", directory)])
}

fn map_layout(flags: &Flags) -> Result<Command, String> {
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
    tool_call("snapshot_layout", pairs)
}

fn map_find_space(flags: &Flags) -> Result<Command, String> {
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
    tool_call("find_empty_space", pairs)
}

fn map_import_svg(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let path = required_pos(
        positionals,
        1,
        "Usage: op import:svg <file.svg> [--x N] [--y N]",
    )?;
    let svg =
        fs::read_to_string(&path).map_err(|e| format!("cannot read SVG file {path:?}: {e}"))?;
    let mut pairs = vec![pair("svg", svg)];
    if let Some(x) = flag_value(flags, "x") {
        pairs.push(pair("x", x));
    }
    if let Some(y) = flag_value(flags, "y") {
        pairs.push(pair("y", y));
    }
    tool_call("import_svg", pairs)
}

fn generic_tool_call(tool: &str, rest: &[String], flags: &Flags) -> Result<Command, String> {
    let mut pairs = Vec::new();
    for kv in rest {
        let (k, v) = kv
            .split_once('=')
            .ok_or_else(|| format!("argument must be key=value, got {kv:?}"))?;
        if k.is_empty() {
            return Err(format!("argument has an empty key: {kv:?}"));
        }
        pairs.push(pair(k, v));
    }
    for (k, v) in flags {
        if is_global_compat_flag(k) {
            continue;
        }
        pairs.push(pair(k, v.clone().unwrap_or_else(|| "true".into())));
    }
    tool_call(tool, pairs)
}

fn is_global_compat_flag(key: &str) -> bool {
    matches!(
        key,
        "file" | "page" | "post-process" | "canvas-width" | "depth"
    )
}

fn insert_pairs(raw: &str) -> Result<Vec<(String, String)>, String> {
    let value = parse_json_object(raw)?;
    let obj = value.as_object().expect("validated JSON object");
    let kind =
        normalize_kind(string_field(obj, &["kind", "type"]).ok_or("insert JSON needs kind/type")?);
    let name = string_field(obj, &["name", "content"]).unwrap_or_else(|| kind.clone());
    let mut pairs = vec![
        pair("kind", kind),
        pair("name", name),
        pair("x", dimension_field(obj, "x", Some("0"))?),
        pair("y", dimension_field(obj, "y", Some("0"))?),
        pair("width", dimension_field(obj, "width", Some("100"))?),
        pair("height", dimension_field(obj, "height", Some("100"))?),
    ];
    if let Some(fill_hex) = fill_hex_field(obj)? {
        pairs.push(pair("fill_hex", fill_hex));
    }
    Ok(pairs)
}

fn update_pairs(raw: &str) -> Result<Vec<(String, String)>, String> {
    let value = parse_json_object(raw)?;
    let obj = value.as_object().expect("validated JSON object");
    let mut pairs = Vec::new();
    if let Some(name) = string_field(obj, &["name"]) {
        pairs.push(pair("name", name));
    }
    for key in ["x", "y", "width", "height"] {
        if obj.contains_key(key) {
            pairs.push(pair(key, dimension_field(obj, key, None)?));
        }
    }
    if let Some(fill_hex) = fill_hex_field(obj)? {
        pairs.push(pair("fill_hex", fill_hex));
    }
    if pairs.is_empty() {
        return Err("update JSON must include at least one of x/y/width/height/name/fill".into());
    }
    Ok(pairs)
}

fn replace_pairs(raw: &str) -> Result<Vec<(String, String)>, String> {
    let value = parse_json_object(raw)?;
    let obj = value.as_object().expect("validated JSON object");
    let kind =
        normalize_kind(string_field(obj, &["kind", "type"]).ok_or("replace JSON needs kind/type")?);
    let name = string_field(obj, &["name", "content"]).unwrap_or_else(|| kind.clone());
    let mut pairs = vec![
        pair("kind", kind),
        pair("name", name),
        pair("x", dimension_field(obj, "x", None)?),
        pair("y", dimension_field(obj, "y", None)?),
        pair("width", dimension_field(obj, "width", None)?),
        pair("height", dimension_field(obj, "height", None)?),
    ];
    if let Some(fill_hex) = fill_hex_field(obj)? {
        pairs.push(pair("fill_hex", fill_hex));
    }
    if let Some(v) = string_field(obj, &["drop_children", "dropChildren"]) {
        pairs.push(pair("drop_children", v));
    }
    Ok(pairs)
}

fn parse_json_object(raw: &str) -> Result<Value, String> {
    let value: Value =
        serde_json::from_str(raw.trim()).map_err(|e| format!("invalid JSON payload: {e}"))?;
    if !value.is_object() {
        return Err("JSON payload must be an object".into());
    }
    Ok(value)
}

fn string_field(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| scalar_to_string(obj.get(*key)?))
}

fn dimension_field(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    default: Option<&str>,
) -> Result<String, String> {
    let Some(value) = obj.get(key) else {
        return default
            .map(str::to_string)
            .ok_or_else(|| format!("{key} is required and must be a numeric doc-px value"));
    };
    let Some(s) = scalar_to_string(value) else {
        return Err(format!("{key} must be a string or number"));
    };
    match s.parse::<i32>() {
        Ok(_) => Ok(s),
        Err(_) => Err(format!(
            "{key} must be a decimal i32 for Rust MCP, got {s:?}"
        )),
    }
}

fn fill_hex_field(obj: &serde_json::Map<String, Value>) -> Result<Option<String>, String> {
    if let Some(v) = obj.get("fill_hex").or_else(|| obj.get("fillHex")) {
        return scalar_to_string(v)
            .map(Some)
            .ok_or_else(|| "fill_hex must be a string".into());
    }
    let Some(fill) = obj.get("fill") else {
        return Ok(None);
    };
    match fill {
        Value::String(s) => Ok(Some(s.clone())),
        Value::Array(items) => {
            for item in items {
                if let Value::Object(fill_obj) = item {
                    if let Some(color) = string_field(fill_obj, &["color"]) {
                        return Ok(Some(color));
                    }
                }
            }
            Ok(None)
        }
        Value::Object(fill_obj) => Ok(string_field(fill_obj, &["color"])),
        _ => Err("fill must be a hex string, fill object, or fill array".into()),
    }
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(v) => Some(v.to_string()),
        _ => None,
    }
}

fn normalize_kind(kind: String) -> String {
    match kind.as_str() {
        "rectangle" => "rect".into(),
        other => other.into(),
    }
}

fn resolve_arg(arg: Option<&str>) -> Result<String, String> {
    match arg {
        Some("-") => {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(|e| format!("stdin read failed: {e}"))?;
            if input.trim().is_empty() {
                Err("No data received from stdin".into())
            } else {
                Ok(input.trim().to_string())
            }
        }
        Some(path) if path.starts_with('@') => {
            let path = &path[1..];
            fs::read_to_string(path)
                .map(|s| s.trim().to_string())
                .map_err(|e| format!("cannot read file {path:?}: {e}"))
        }
        Some(value) => Ok(value.to_string()),
        None => Err("No data provided. Pass as argument, @filepath, or '-' for stdin".into()),
    }
}

fn required_pos(positionals: &[String], index: usize, usage: &str) -> Result<String, String> {
    positionals
        .get(index)
        .cloned()
        .ok_or_else(|| usage.to_string())
}

fn flag_value(flags: &Flags, key: &str) -> Option<String> {
    flags.get(key).and_then(Clone::clone)
}

fn pair(k: impl Into<String>, v: impl Into<String>) -> (String, String) {
    (k.into(), v.into())
}

fn tool_call(tool: &str, args: Vec<(String, String)>) -> Result<Command, String> {
    Ok(Command::ToolCall {
        tool: tool.to_string(),
        args,
    })
}

fn version_json() -> String {
    format!(r#"{{"version":"{}"}}"#, env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests;
