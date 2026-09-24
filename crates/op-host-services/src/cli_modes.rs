//! Shared argv dispatcher for the headless CLI server modes.
//!
//! Both binaries expose the same three modes — `--mcp <path>` (JSON-RPC
//! stdio MCP server), `--mcp-http <port> <path>` (Streamable-HTTP MCP
//! server), and `--serve-web <port> [doc] [--host <addr>]` (headless
//! web-canvas daemon) — differing only in the program name their
//! diagnostics carry. `op-host-desktop` falls back to the GUI on an
//! unknown mode; `op-host-web-server` treats it as a usage error. Both
//! decisions stay with the caller.
//!
//! The two MCP modes also accept `--mcp-profile <full|lean>` anywhere after
//! the mode (or `OPENPENCIL_MCP_PROFILE`; the flag wins) to choose the tool
//! catalog — see `mcp_serve::tool_catalog`.

use std::path::PathBuf;

use crate::mcp_serve::tool_catalog::{self, McpToolCatalog};

/// Run the headless server mode named by `mode` (the first argv token),
/// consuming the remaining `args`.
///
/// - `None` — `mode` is not one of the three server modes; the caller
///   decides what an unknown mode means.
/// - `Some(0)` — the mode ran to completion.
/// - `Some(code)` — the mode failed; the diagnostic (prefixed with
///   `"{prog} {mode}: "`) has already been printed to stderr and the
///   caller should exit with `code` (2 = malformed invocation, 1 =
///   runtime failure).
pub fn run_cli_mode(prog: &str, mode: &str, args: impl Iterator<Item = String>) -> Option<i32> {
    match mode {
        "--mcp" | "--mcp-http" => {
            let env = std::env::var(tool_catalog::MCP_PROFILE_ENV).ok();
            let (args, catalog) = match parse_mcp_mode_args(args, env) {
                Ok(parsed) => parsed,
                Err(e) => {
                    eprintln!("{prog} {mode}: {e}");
                    return Some(2);
                }
            };
            if catalog == McpToolCatalog::Lean {
                eprintln!("{prog} {mode}: serving the lean MCP tool profile");
            }
            let args = args.into_iter();
            if mode == "--mcp" {
                run_mcp_stdio(prog, args, catalog)
            } else {
                run_mcp_http(prog, args, catalog)
            }
        }
        "--serve-web" => serve_web(prog, args),
        _ => None,
    }
}

/// Split the tool-catalog selection out of an `--mcp` / `--mcp-http` argv.
/// Returns the positional arguments (in order) and the resolved catalog;
/// `env` is the `OPENPENCIL_MCP_PROFILE` value, passed in so tests never
/// touch process environment.
pub fn parse_mcp_mode_args(
    args: impl Iterator<Item = String>,
    env: Option<String>,
) -> Result<(Vec<String>, McpToolCatalog), tool_catalog::McpProfileError> {
    let (positional, flag) = tool_catalog::extract_profile_flag(args)?;
    let catalog = tool_catalog::resolve_catalog(flag.as_deref(), env.as_deref())?;
    Ok((positional, catalog))
}

fn run_mcp_stdio(
    prog: &str,
    mut args: impl Iterator<Item = String>,
    catalog: McpToolCatalog,
) -> Option<i32> {
    let Some(path) = args.next() else {
        eprintln!("{prog} --mcp: missing <path> arg");
        return Some(2);
    };
    crate::user_scene_template_store::initialize_user_scene_templates_once();
    if let Err(e) = crate::mcp_serve::run(PathBuf::from(path), catalog) {
        eprintln!("{prog} --mcp: {e}");
        return Some(1);
    }
    Some(0)
}

fn run_mcp_http(
    prog: &str,
    mut args: impl Iterator<Item = String>,
    catalog: McpToolCatalog,
) -> Option<i32> {
    let Some(port_arg) = args.next() else {
        eprintln!("{prog} --mcp-http: missing <port> arg");
        return Some(2);
    };
    let Ok(port) = port_arg.parse::<u16>() else {
        eprintln!("{prog} --mcp-http: <port> must be a u16, got {port_arg:?}");
        return Some(2);
    };
    let Some(path) = args.next() else {
        eprintln!("{prog} --mcp-http: missing <path> arg");
        return Some(2);
    };
    crate::user_scene_template_store::initialize_user_scene_templates_once();
    if let Err(e) = crate::mcp_serve::run_http(PathBuf::from(path), port, catalog) {
        eprintln!("{prog} --mcp-http: {e}");
        return Some(1);
    }
    Some(0)
}

fn serve_web(prog: &str, args: impl Iterator<Item = String>) -> Option<i32> {
    // `--serve-web <port> [doc] [--host <addr>]`: doc optional (empty
    // document otherwise); `--host` opts in to a non-loopback bind.
    // The same parser also accepts the managed flag form
    // (`--managed --port <n> ...`) for a supervising process that
    // wants the handshake-JSON / stdin-EOF contract.
    let options = match crate::web_canvas_server::parse_serve_web_args(args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("{prog} --serve-web: {e}");
            return Some(2);
        }
    };
    // The online daemon is multi-tenant and must not install host-user
    // material into a process-global registry. Its MCP profile also
    // refuses user-template tools; local and managed daemons load the
    // standard native collection before accepting any request.
    if !options.online {
        crate::user_scene_template_store::initialize_user_scene_templates_once();
    }
    if let Err(e) = crate::web_canvas_server::run_web_canvas(options) {
        eprintln!("{prog} --serve-web: {e}");
        return Some(1);
    }
    Some(0)
}

#[cfg(test)]
#[path = "cli_modes_tests.rs"]
mod tests;
