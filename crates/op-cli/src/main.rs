//! `op` - the OpenPencil CLI.
//! Keeps common TS `op` aliases while preserving low-level `op <tool> key=value`.

use std::collections::BTreeMap;

mod app_control_cli;
mod cli_conversion;
mod cli_error;
mod codegen_cli;
mod command_helpers;
mod command_mappers;
mod export_cli;
mod figma_cli;
mod html_cli;
mod mcp_http_cli;
mod page_theme_cli;
mod path_args;
mod skill_export_cli;
mod skill_install_cli;
mod skill_install_error;
mod template_cli;

use cli_error::CliError;

use command_helpers::{
    flag_value, pair, push_file_path, tool_call, tool_call_with_file, version_json,
};
use command_mappers::*;
use mcp_http_cli::{
    args_to_json, json_escape, post, pretty_json, status_json, tool_call_body, tools_list_body,
};
use path_args::resolve_file_path_arg;

#[cfg(test)]
use mcp_http_cli::{http_request, status_json_from_running};

#[cfg(test)]
use figma_cli::figma_default_out_path;

/// Default HTTP MCP port — the workspace-wide shared const.
const DEFAULT_PORT: u16 = op_editor_core::DEFAULT_MCP_PORT;

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
fn run(args: &[String]) -> Result<String, CliError> {
    let Parsed {
        port,
        port_explicit,
        pretty,
        command,
    } = parse_args(args)?;
    // For commands that talk to a running server, resolve the live
    // editor's published port (`~/.openpencil/.op-mcp-port`) unless the user
    // pinned `--port` explicitly. `op start` keeps the requested port.
    let needs_server = command_needs_server(&command);
    // The live MCP endpoint authenticates every stateful call, so the token
    // has to be resolved alongside the port rather than after it.
    let (target_port, target_token) = if port_explicit || !needs_server {
        (port, app_control_cli::token_for_port(port))
    } else {
        app_control_cli::discover_running_endpoint()
            .unwrap_or_else(|| (port, app_control_cli::token_for_port(port)))
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
        Command::ToolsList => post(target_port, &target_token, &tools_list_body())?,
        Command::ImportFigma { fig_path, out_path } => {
            figma_cli::run_import_figma(&fig_path, &out_path)?
        }
        Command::ImportHtml {
            html_path,
            out_path,
            viewport_height,
        } => html_cli::run_import_html(&html_path, &out_path, viewport_height.as_deref())?,
        Command::ImportSnapshot {
            json_path,
            out_path,
        } => html_cli::run_import_snapshot(&json_path, &out_path)?,
        Command::ToolCall { tool, args } => post(
            target_port,
            &target_token,
            &tool_call_body(&tool, &args_to_json(&args)),
        )?,
        Command::ToolCallJson { tool, args_json } => post(
            target_port,
            &target_token,
            &tool_call_body(&tool, &args_json),
        )?,
        Command::ExportDeck { output, format } => {
            export_cli::run_export_deck(target_port, &target_token, &output, &format)?
        }
        Command::Templates { scene, tag } => template_cli::run_templates(
            target_port,
            &target_token,
            scene.as_deref(),
            tag.as_deref(),
        )?,
        Command::UseTemplate { template_id } => {
            template_cli::run_use_template(target_port, &target_token, &template_id)?
        }
        Command::ExportFrames { output_dir, format } => {
            export_cli::run_export_frames(target_port, &target_token, &output_dir, &format)?
        }
        Command::Styles { id, tag, platform } => template_cli::run_styles(
            target_port,
            &target_token,
            id.as_deref(),
            tag.as_deref(),
            platform.as_deref(),
        )?,
        Command::Export {
            item_id,
            selection: _,
            output,
            format,
            scale,
        } => export_cli::run_export(
            target_port,
            &target_token,
            item_id.as_deref(),
            &output,
            &format,
            scale.as_deref(),
        )?,
    };
    Ok(if pretty { pretty_json(&out) } else { out })
}

/// Whether a parsed command talks to an already-running MCP endpoint.
///
/// Keep the dedicated aliases here alongside the generic tool-call variants:
/// they bypass `Command::ToolCall` at dispatch time but still need the same
/// live-port and instance-token discovery.
fn command_needs_server(command: &Command) -> bool {
    matches!(
        command,
        Command::Status
            | Command::ToolsList
            | Command::ToolCall { .. }
            | Command::ToolCallJson { .. }
            | Command::ExportDeck { .. }
            | Command::ExportFrames { .. }
            | Command::Templates { .. }
            | Command::UseTemplate { .. }
            | Command::Styles { .. }
            | Command::Export { .. }
    )
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
    ImportHtml {
        html_path: String,
        out_path: String,
        /// Optional `--viewport-height` override for the import viewport,
        /// kept as the validated raw text so `Command` stays `Eq`.
        viewport_height: Option<String>,
    },
    ImportSnapshot {
        json_path: String,
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
    Export {
        item_id: Option<String>,
        selection: bool,
        output: String,
        format: String,
        scale: Option<String>,
    },
    ExportDeck {
        output: String,
        format: String,
    },
    Templates {
        scene: Option<String>,
        tag: Option<String>,
    },
    UseTemplate {
        template_id: String,
    },
    Styles {
        id: Option<String>,
        tag: Option<String>,
        platform: Option<String>,
    },
    ExportFrames {
        output_dir: String,
        format: String,
    },
}

type Flags = BTreeMap<String, Option<String>>;

/// Parse command-line args. `--port`, `--pretty`, `--help`, and
/// `--version` are global; the rest are left for command aliases or
/// low-level MCP tool arguments.
fn parse_args(args: &[String]) -> Result<Parsed, CliError> {
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
                            let next = args.get(i + 1).ok_or_else(|| {
                                CliError::usage("--port needs a value (e.g. --port 3100)")
                            })?;
                            i += 1;
                            next.clone()
                        }
                    };
                    port = raw_port.parse::<u16>().map_err(|_| {
                        CliError::Usage(format!("--port must be a u16, got {raw_port:?}"))
                    })?;
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

fn command_from_positionals(positionals: &[String], flags: &Flags) -> Result<Command, CliError> {
    match positionals[0].as_str() {
        "help" | "-h" | "--help" => Ok(Command::Help),
        "version" => Ok(Command::Version),
        "tools" => Ok(Command::ToolsList),
        "status" => Ok(Command::Status),
        "start" => {
            let headless = flags.contains_key("headless");
            let web = flags.contains_key("web");
            if headless && web {
                return Err(CliError::usage(
                    "--headless and --web are mutually exclusive",
                ));
            }
            let host = flag_value(flags, "host");
            if host.is_some() && !web {
                return Err(CliError::usage(
                    "--host requires --web (only the web daemon binds non-loopback)",
                ));
            }
            Ok(Command::StartMcp {
                document_path: flag_value(flags, "file"),
                headless,
                web,
                host,
            })
        }
        "stop" => Ok(Command::StopMcp),
        "export" => export_cli::map_export(flags),
        "export-deck" => export_cli::map_export_deck(flags),
        "templates" => template_cli::map_templates(flags),
        "use-template" => template_cli::map_use_template(flags, positionals),
        "styles" => template_cli::map_styles(flags, positionals),
        "export-frames" => export_cli::map_export_frames(flags),
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
        "import:svg" => html_cli::map_import_svg(positionals, flags),
        "import:html" => html_cli::map_import_html(positionals, flags),
        "import:snapshot" => html_cli::map_import_snapshot(positionals, flags),
        "import:figma" => figma_cli::map_import_figma(positionals, flags),
        "codegen:plan" | "codegen:submit" | "codegen:assemble" | "codegen:clean" => {
            codegen_cli::map_codegen(positionals, flags)
        }
        tool => generic_tool_call(tool, &positionals[1..], flags),
    }
}

#[cfg(test)]
mod cli_conversion_tests;
#[cfg(test)]
mod cli_design_tests;
#[cfg(test)]
mod cli_export_tests;
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
