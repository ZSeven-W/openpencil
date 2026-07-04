//! `op` - the OpenPencil CLI.
//! Keeps common TS `op` aliases while preserving low-level `op <tool> key=value`.

use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};

mod app_control_cli;
mod cli_conversion;
mod codegen_cli;
mod figma_cli;
mod mcp_http_cli;
mod page_theme_cli;
mod path_args;
mod skill_export_cli;
mod skill_install_cli;

use mcp_http_cli::{
    args_to_json, json_escape, post, pretty_json, status_json, tool_call_body, tools_list_body,
};
use path_args::{resolve_file_path_arg, resolve_file_path_value};

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
        port_explicit,
        pretty,
        command,
    } = parse_args(args)?;
    // For commands that talk to a running server, resolve the live
    // editor's published port (`~/.openpencil/.op-mcp-port`) unless the user
    // pinned `--port` explicitly. `op start` keeps the requested port.
    let needs_server = matches!(
        command,
        Command::Status
            | Command::ToolsList
            | Command::ToolCall { .. }
            | Command::ToolCallJson { .. }
    );
    let target_port = if port_explicit || !needs_server {
        port
    } else {
        app_control_cli::discover_running_port().unwrap_or(port)
    };
    let out = match command {
        Command::Help => USAGE.to_string(),
        Command::Version => version_json(),
        Command::Status => status_json(target_port),
        Command::StartMcp {
            document_path,
            headless,
            web,
            host,
        } => app_control_cli::run_start(
            port,
            document_path.as_deref(),
            headless,
            web,
            host.as_deref(),
        )?,
        Command::StopMcp => app_control_cli::run_stop()?,
        Command::SkillExport { name, out_dir } => {
            skill_export_cli::run_export(&name, out_dir.as_deref())?
        }
        Command::InstallSkill { target } => skill_install_cli::run_install(target.as_deref())?,
        Command::UninstallSkill { target } => skill_install_cli::run_uninstall(target.as_deref())?,
        Command::ToolsList => post(target_port, &tools_list_body())?,
        Command::ImportFigma { fig_path, out_path } => {
            figma_cli::run_import_figma(&fig_path, &out_path)?
        }
        Command::ToolCall { tool, args } => {
            post(target_port, &tool_call_body(&tool, &args_to_json(&args)))?
        }
        Command::ToolCallJson { tool, args_json } => {
            post(target_port, &tool_call_body(&tool, &args_json))?
        }
    };
    Ok(if pretty { pretty_json(&out) } else { out })
}

#[derive(Debug, PartialEq, Eq)]
struct Parsed {
    port: u16,
    /// Whether `--port` was passed explicitly. When false, server-bound
    /// commands resolve the running editor's port via discovery instead
    /// of assuming the default.
    port_explicit: bool,
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
        headless: bool,
        /// `--web`: serve the browser editor (wasm bundle) instead of the
        /// desktop GUI / windowless file server.
        web: bool,
        /// `--host` bind address for `--web` (e.g. `0.0.0.0` for LAN/Docker).
        host: Option<String>,
    },
    StopMcp,
    SkillExport {
        name: String,
        out_dir: Option<String>,
    },
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
    let mut port_explicit = false;
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
                    port_explicit = true;
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
        port_explicit,
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
        "start" => {
            let headless = flags.contains_key("headless");
            let web = flags.contains_key("web");
            if headless && web {
                return Err("--headless and --web are mutually exclusive".into());
            }
            let host = flag_value(flags, "host");
            if host.is_some() && !web {
                return Err(
                    "--host requires --web (only the web daemon binds non-loopback)".into(),
                );
            }
            Ok(Command::StartMcp {
                document_path: flag_value(flags, "file"),
                headless,
                web,
                host,
            })
        }
        "stop" => Ok(Command::StopMcp),
        "skill:export" => Ok(Command::SkillExport {
            name: required_pos(
                positionals,
                1,
                "Usage: op skill:export <skill-name> [--out .claude/skills]",
            )?,
            out_dir: flag_value(flags, "out"),
        }),
        "install" => Ok(Command::InstallSkill {
            target: flag_value(flags, "target"),
        }),
        "uninstall" => Ok(Command::UninstallSkill {
            target: flag_value(flags, "target"),
        }),
        "open" => {
            let args = flag_value(flags, "file")
                .or_else(|| positionals.get(1).cloned())
                .map(|path| vec![pair("filePath", resolve_file_path_arg(&path))])
                .unwrap_or_default();
            tool_call("open_document", args)
        }
        "save" => {
            let file_path =
                resolve_file_path_arg(&required_pos(positionals, 1, "Usage: op save <file.op>")?);
            let mut args = vec![pair("filePath", file_path)];
            if let Some(source) = flag_value(flags, "file") {
                args.push(pair("sourceFilePath", resolve_file_path_arg(&source)));
            }
            tool_call("save_document", args)
        }
        "get" => map_get(flags),
        "selection" => map_selection(flags),
        "insert" => map_insert(positionals, flags),
        "update" => map_update(positionals, flags),
        "delete" => map_delete(positionals, flags),
        "read-nodes" => map_read_nodes(positionals, flags),
        "move" => map_reparent("move_node", positionals, flags),
        "copy" => map_reparent("copy_node", positionals, flags),
        "replace" => map_replace(positionals, flags),
        "design" => map_design_like("batch_design", positionals.get(1), flags, true),
        "design:upsert-vars"
        | "design:upsert-component"
        | "design:upsert-screen"
        | "design:status"
        | "design:lint" => cli_conversion::map_design_conversion(positionals, flags),
        "design:skeleton" => map_design_skeleton(positionals, flags),
        "design:content" => map_design_content(positionals, flags),
        "design:refine" => map_design_refine(flags),
        "page" => page_theme_cli::map_page(positionals, flags),
        "vars" => tool_call_with_file("get_variables", flags),
        "vars:set" => page_theme_cli::map_vars_set(positionals, flags),
        "themes" => tool_call_with_file("get_variables", flags),
        "themes:set" => page_theme_cli::map_themes_set(positionals, flags),
        "theme:save" => page_theme_cli::map_theme_save(positionals, flags),
        "theme:load" => page_theme_cli::map_theme_load(positionals, flags),
        "theme:list" => page_theme_cli::map_theme_list(positionals),
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
    push_file_path(&mut pairs, flags);
    tool_call("batch_get", pairs)
}

fn map_selection(flags: &Flags) -> Result<Command, String> {
    let mut pairs = Vec::new();
    if let Some(depth) = flag_value(flags, "depth") {
        pairs.push(pair("readDepth", depth));
    }
    push_file_path(&mut pairs, flags);
    tool_call("get_selection", pairs)
}

fn map_insert(positionals: &[String], flags: &Flags) -> Result<Command, String> {
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

fn map_update(positionals: &[String], flags: &Flags) -> Result<Command, String> {
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

fn map_replace(positionals: &[String], flags: &Flags) -> Result<Command, String> {
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

fn map_delete(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let id = required_pos(positionals, 1, "Usage: op delete <node-id>")?;
    let mut pairs = vec![pair("nodeId", id)];
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    push_file_path(&mut pairs, flags);
    tool_call("delete_node", pairs)
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
    push_file_path(&mut pairs, flags);
    tool_call("read_nodes", pairs)
}

fn map_reparent(tool: &str, positionals: &[String], flags: &Flags) -> Result<Command, String> {
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

fn map_design_like(
    tool: &str,
    payload: Option<&String>,
    flags: &Flags,
    default_post_process: bool,
) -> Result<Command, String> {
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

fn map_design_content(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let section_id = required_pos(
        positionals,
        1,
        "Usage: op design:content <section-id> <json|@file|->",
    )?;
    let raw = resolve_arg(positionals.get(2).map(String::as_str))?;
    let payload: Value = serde_json::from_str(raw.trim())
        .map_err(|e| format!("invalid design:content JSON payload: {e}"))?;
    let children = payload
        .get("children")
        .ok_or("design:content JSON payload must contain a children array")?;
    if !children.is_array() {
        return Err("design:content children must be an array".into());
    }

    let mut args = serde_json::Map::new();
    args.insert("sectionId".into(), Value::String(section_id));
    args.insert("children".into(), children.clone());
    if let Some(canvas_width) = flag_value(flags, "canvas-width") {
        let width = canvas_width
            .parse::<i64>()
            .map_err(|_| format!("--canvas-width must be an integer, got {canvas_width:?}"))?;
        args.insert("canvasWidth".into(), Value::Number(width.into()));
    }
    if let Some(page) = flag_value(flags, "page") {
        args.insert("pageId".into(), Value::String(page));
    }
    if let Some(file) = flag_value(flags, "file") {
        args.insert("filePath".into(), resolve_file_path_value(&file));
    }
    let args_json = serde_json::to_string(&Value::Object(args))
        .map_err(|e| format!("cannot serialize design:content args: {e}"))?;
    Ok(Command::ToolCallJson {
        tool: "design_content".into(),
        args_json,
    })
}

fn map_design_skeleton(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let raw = resolve_arg(positionals.get(1).map(String::as_str))?;
    let payload: Value = serde_json::from_str(raw.trim())
        .map_err(|e| format!("invalid design:skeleton JSON payload: {e}"))?;
    let root_frame = payload
        .get("rootFrame")
        .ok_or("design:skeleton JSON payload must contain rootFrame")?;
    if !root_frame.is_object() {
        return Err("design:skeleton rootFrame must be an object".into());
    }
    let sections = payload
        .get("sections")
        .ok_or("design:skeleton JSON payload must contain sections")?;
    if !sections.is_array() {
        return Err("design:skeleton sections must be an array".into());
    }

    let mut args = serde_json::Map::new();
    args.insert("rootFrame".into(), root_frame.clone());
    args.insert("sections".into(), sections.clone());
    if let Some(style_guide) = payload.get("styleGuide") {
        if !style_guide.is_object() {
            return Err("design:skeleton styleGuide must be an object".into());
        }
        args.insert("styleGuide".into(), style_guide.clone());
    }
    if let Some(canvas_width) = flag_value(flags, "canvas-width") {
        let width = canvas_width
            .parse::<i64>()
            .map_err(|_| format!("--canvas-width must be an integer, got {canvas_width:?}"))?;
        args.insert("canvasWidth".into(), Value::Number(width.into()));
    }
    if let Some(page) = flag_value(flags, "page") {
        args.insert("pageId".into(), Value::String(page));
    }
    if let Some(file) = flag_value(flags, "file") {
        args.insert("filePath".into(), resolve_file_path_value(&file));
    }
    let args_json = serde_json::to_string(&Value::Object(args))
        .map_err(|e| format!("cannot serialize design:skeleton args: {e}"))?;
    Ok(Command::ToolCallJson {
        tool: "design_skeleton".into(),
        args_json,
    })
}

fn map_design_refine(flags: &Flags) -> Result<Command, String> {
    let root_id = flag_value(flags, "root-id").ok_or("Usage: op design:refine --root-id <id>")?;
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
    push_file_path(&mut pairs, flags);
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
    push_file_path(&mut pairs, flags);
    tool_call("find_empty_space", pairs)
}

fn map_import_svg(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let path = required_pos(
        positionals,
        1,
        "Usage: op import:svg <file.svg> [--x N] [--y N] [--parent P] [--page PAGE]",
    )?;
    let mut pairs = vec![pair("svgPath", path)];
    if let Some(x) = flag_value(flags, "x") {
        pairs.push(pair("x", x));
    }
    if let Some(y) = flag_value(flags, "y") {
        pairs.push(pair("y", y));
    }
    if let Some(parent) = flag_value(flags, "parent") {
        pairs.push(pair("parent", parent));
    }
    if let Some(page) = flag_value(flags, "page") {
        pairs.push(pair("pageId", page));
    }
    push_file_path(&mut pairs, flags);
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
    push_file_path(&mut pairs, flags);
    for (k, v) in flags {
        if is_global_compat_flag(k) {
            continue;
        }
        pairs.push(pair(k, v.clone().unwrap_or_else(|| "true".into())));
    }
    tool_call(tool, pairs)
}

fn is_global_compat_flag(key: &str) -> bool {
    ["file", "page", "post-process", "canvas-width", "depth"].contains(&key)
}

fn parse_json_object(raw: &str) -> Result<Value, String> {
    let value: Value =
        serde_json::from_str(raw.trim()).map_err(|e| format!("invalid JSON payload: {e}"))?;
    if !value.is_object() {
        return Err("JSON payload must be an object".into());
    }
    Ok(value)
}

fn data_pairs(raw: &str) -> Result<Vec<(String, String)>, String> {
    parse_json_object(raw)?;
    Ok(vec![pair("data", raw.trim())])
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

fn push_file_path(pairs: &mut Vec<(String, String)>, flags: &Flags) {
    if let Some(file) = flag_value(flags, "file") {
        pairs.push(pair("filePath", resolve_file_path_arg(&file)));
    }
}

fn pair(k: impl Into<String>, v: impl Into<String>) -> (String, String) {
    (k.into(), v.into())
}

fn tool_call_with_file(tool: &str, flags: &Flags) -> Result<Command, String> {
    let mut pairs = Vec::new();
    push_file_path(&mut pairs, flags);
    tool_call(tool, pairs)
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
mod cli_conversion_tests;
#[cfg(test)]
mod cli_design_tests;
#[cfg(test)]
mod cli_file_flag_tests;
#[cfg(test)]
mod cli_import_tests;
#[cfg(test)]
mod cli_node_tests;
#[cfg(test)]
mod cli_selection_tests;
#[cfg(test)]
mod cli_start_tests;
#[cfg(test)]
mod tests;
