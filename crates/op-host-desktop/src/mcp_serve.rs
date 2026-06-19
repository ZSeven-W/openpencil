//! Argv dispatcher for the headless CLI server modes.
//!
//! The MCP stdio / HTTP server runners live in
//! [`op_web_daemon::mcp_serve`] and the headless web-canvas daemon in
//! `crate::web_canvas_server`; this residual just inspects `argv` and
//! routes `--mcp` / `--mcp-http` / `--serve-web` to them. `main` calls
//! it before opening the GUI window.

use std::path::PathBuf;

use op_web_daemon::mcp_serve::{run, run_http};

/// If argv requests a headless server mode, run it and return `true` —
/// the caller (`main`) should then exit. Returns `false` for normal
/// GUI mode. Exits the process on a malformed invocation.
///
/// - `--mcp <path>` — JSON-RPC stdio MCP server backed by `<path>`.
/// - `--mcp-http <port> <path>` — Streamable-HTTP MCP server.
/// - `--serve-web <port> [doc] [--host <addr>]` — headless web-canvas daemon.
///
/// External CLIs (Claude Code / Codex / Gemini / Copilot) spawn the
/// binary in these modes to drive the Rust editor the same way they
/// drive TS pen-mcp.
pub fn run_cli_if_requested() -> bool {
    let mut args = std::env::args().skip(1);
    let Some(first) = args.next() else {
        return false;
    };
    if first == "--mcp" {
        let Some(path) = args.next() else {
            eprintln!("openpencil-desktop --mcp: missing <path> arg");
            std::process::exit(2);
        };
        if let Err(e) = run(PathBuf::from(path)) {
            eprintln!("openpencil-desktop --mcp: {e}");
            std::process::exit(1);
        }
        return true;
    }
    if first == "--mcp-http" {
        let Some(port_arg) = args.next() else {
            eprintln!("openpencil-desktop --mcp-http: missing <port> arg");
            std::process::exit(2);
        };
        let Ok(port) = port_arg.parse::<u16>() else {
            eprintln!("openpencil-desktop --mcp-http: <port> must be a u16, got {port_arg:?}");
            std::process::exit(2);
        };
        let Some(path) = args.next() else {
            eprintln!("openpencil-desktop --mcp-http: missing <path> arg");
            std::process::exit(2);
        };
        if let Err(e) = run_http(PathBuf::from(path), port) {
            eprintln!("openpencil-desktop --mcp-http: {e}");
            std::process::exit(1);
        }
        return true;
    }
    if first == "--serve-web" {
        // `--serve-web <port> [doc] [--host <addr>]`: doc optional (empty
        // document otherwise); `--host` opts in to a non-loopback bind.
        let (port, path, host) = crate::web_canvas_server::parse_serve_web_args(args)
            .unwrap_or_else(|e| {
                eprintln!("openpencil-desktop --serve-web: {e}");
                std::process::exit(2);
            });
        if let Err(e) = crate::web_canvas_server::run_web_canvas(path, port, &host) {
            eprintln!("openpencil-desktop --serve-web: {e}");
            std::process::exit(1);
        }
        return true;
    }
    // Unknown leading arg → fall through to GUI mode for now.
    false
}
