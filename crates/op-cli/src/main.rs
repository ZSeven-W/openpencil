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
use std::io::{self, Read, Write};

/// Default HTTP MCP port, matching TS `@zseven-w/pen-mcp`.
const DEFAULT_PORT: u16 = 3100;
const MCP_PATH: &str = "/mcp";

const USAGE: &str = "\
op - OpenPencil CLI (drives the editor over the HTTP MCP transport)

USAGE:
  op [--port N] tools                     list every MCP tool + input schema
  op [--port N] <tool> [key=value ...]    call one MCP tool with string args
  op [--port N] <command> [options]       TS-style command aliases
  op help                                 show this message

COMMON COMMANDS:
  op get [--id ID] [--name NAME]          read document, node, or named node
  op selection                            get current selection
  op insert <json|@file|->                insert a leaf node
  op update <id> <json|@file|->           patch x/y/width/height/name/fill
  op delete <id>                          delete a node
  op move <id> [--parent P]               reparent a node (empty parent = page root)
  op copy <id> [--parent P]               deep-copy a node
  op replace <id> <json|@file|->          replace with a leaf node
  op design <json-array|@file|->          call batch_design(nodes_json)
  op design:skeleton <json-array|@file|->
  op design:content [section] <json-array|@file|->
  op design:refine <json-array|@file|->
  op page list|add [--name N]|remove|rename|reorder|duplicate ...
  op vars                                 list variables
  op themes                               get active theme pins
  op layout                               snapshot top-level layout
  op import:svg <file.svg> [--x N] [--y N]

GLOBAL FLAGS:
  --port <n>      MCP HTTP port (default: 3100)
  --pretty        pretty-print JSON replies
  --file <path>   accepted for TS CLI compatibility; Rust MCP targets the
                  document already opened by the MCP server
  --page <id>     accepted for TS CLI compatibility; use set_active_page for
                  Rust MCP page selection by index

The server must be running at:
  http://127.0.0.1:3100/mcp

Low-level examples:
  op tools
  op insert_node kind=rect name=Box x=10 y=20 width=100 height=60
  op set_node_fill_hex node_id=n3 hex=#ff0000";

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
        Command::ToolsList => post(port, &tools_list_body())?,
        Command::ToolCall { tool, args } => {
            post(port, &tool_call_body(&tool, &args_to_json(&args)))?
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
    ToolsList,
    ToolCall {
        tool: String,
        args: Vec<(String, String)>,
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
        "status" => tool_call("get_document_info", vec![]),
        "open" if positionals.len() == 1 => tool_call("get_document_info", vec![]),
        "get" => map_get(flags),
        "selection" => tool_call("get_selection", vec![]),
        "insert" => map_insert(positionals),
        "update" => map_update(positionals),
        "delete" => map_one_id("delete_node", positionals),
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
        "vars" => tool_call("list_variables", vec![]),
        "themes" => tool_call("get_active_theme", vec![]),
        "layout" => tool_call("snapshot_layout", vec![]),
        "import:svg" => map_import_svg(positionals, flags),
        "start" | "stop" | "save" | "read-nodes" | "find-space" | "vars:set" | "themes:set"
        | "theme:save" | "theme:load" | "theme:list" | "import:figma" | "install" | "uninstall"
        | "codegen:plan" | "codegen:submit" | "codegen:assemble" | "codegen:clean" => Err(format!(
            "TS command {:?} is not implemented by the Rust HTTP MCP CLI yet",
            positionals[0]
        )),
        tool => generic_tool_call(tool, &positionals[1..], flags),
    }
}

fn map_get(flags: &Flags) -> Result<Command, String> {
    if let Some(id) = flag_value(flags, "id") {
        return tool_call("get_node", vec![pair("node_id", id)]);
    }
    if let Some(name) = flag_value(flags, "name") {
        return tool_call("find_node_by_name", vec![pair("name", name)]);
    }
    if flag_value(flags, "parent").is_some() {
        return Err("Rust MCP get currently supports --id and --name; use get_node_children directly for parent reads".into());
    }
    if flag_value(flags, "type").is_some() {
        return tool_call("list_node_kinds", vec![]);
    }
    tool_call("get_document_info", vec![])
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
            let index = required_pos(positionals, 2, "Usage: op page remove <index>")?;
            tool_call("delete_page", vec![pair("index", index)])
        }
        "rename" => {
            let index = required_pos(positionals, 2, "Usage: op page rename <index> <name>")?;
            let name = required_pos(positionals, 3, "Usage: op page rename <index> <name>")?;
            tool_call(
                "rename_page",
                vec![pair("index", index), pair("name", name)],
            )
        }
        "reorder" => {
            let from = required_pos(
                positionals,
                2,
                "Usage: op page reorder <from-index> <to-index>",
            )?;
            let to = required_pos(
                positionals,
                3,
                "Usage: op page reorder <from-index> <to-index>",
            )?;
            tool_call("reorder_page", vec![pair("from", from), pair("to", to)])
        }
        "duplicate" => {
            let index = required_pos(positionals, 2, "Usage: op page duplicate <index>")?;
            tool_call("duplicate_page", vec![pair("index", index)])
        }
        _ => Err(format!("unknown page subcommand {sub:?}")),
    }
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

fn pretty_json(raw: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| raw.to_string())
}

/// JSON-RPC body for `tools/list`.
fn tools_list_body() -> String {
    r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#.to_string()
}

/// JSON-RPC body for a `tools/call` of `tool` with the already-built
/// `arguments` object JSON.
fn tool_call_body(tool: &str, args_json: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{}","arguments":{}}}}}"#,
        json_escape(tool),
        args_json
    )
}

/// Build a JSON object from `key=value` pairs. MCP tool arguments are
/// scalar string-typed, so every value is emitted as a JSON string.
fn args_to_json(pairs: &[(String, String)]) -> String {
    let mut out = String::from("{");
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&json_escape(k));
        out.push_str("\":\"");
        out.push_str(&json_escape(v));
        out.push('"');
    }
    out.push('}');
    out
}

/// Escape a string for inclusion in a JSON string literal.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

fn http_request(body: &str) -> String {
    format!(
        "POST {MCP_PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

/// POST `body` to the HTTP MCP server on `127.0.0.1:port` and return
/// the response body (the JSON-RPC reply).
fn post(port: u16, body: &str) -> Result<String, String> {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).map_err(|e| {
        format!(
            "cannot reach the editor on 127.0.0.1:{port}: {e}\n\
             start the OpenPencil MCP server and point clients at http://127.0.0.1:{port}/mcp"
        )
    })?;
    stream
        .write_all(http_request(body).as_bytes())
        .map_err(|e| format!("http write: {e}"))?;
    stream.flush().ok();
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("http read: {e}"))?;
    Ok(match response.split_once("\r\n\r\n") {
        Some((_, body)) => body.trim().to_string(),
        None => response.trim().to_string(),
    })
}

#[cfg(test)]
mod tests;
