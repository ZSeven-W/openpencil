//! Headless OpenPencil web / MCP server binary.
//!
//! Links only `op-host-services` (the extracted headless daemon) — no
//! winit / glutin / muda / accesskit adapters / skia-GL — so the
//! container / server image stays GUI-free. It mirrors the `--serve-web`
//! / `--mcp` / `--mcp-http` argv arms the desktop binary's
//! `run_cli_if_requested` dispatcher routes, but here that IS the whole
//! program: there is no GUI fallback, so an unknown / missing mode is a
//! usage error rather than "open the editor window".

use std::path::PathBuf;
use std::process::exit;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(first) = args.next() else {
        eprintln!(
            "op-host-web-server: missing mode. Usage:\n  \
             --serve-web <port> [doc] [--host <addr>]   headless web-canvas daemon\n  \
             --mcp <path>                               JSON-RPC stdio MCP server\n  \
             --mcp-http <port> <path>                   Streamable-HTTP MCP server"
        );
        exit(2);
    };
    match first.as_str() {
        "--serve-web" => {
            // `--serve-web <port> [doc] [--host <addr>]`: doc optional
            // (empty document otherwise); `--host` opts in to a
            // non-loopback bind.
            let (port, path, host) = op_host_services::web_canvas_server::parse_serve_web_args(
                args,
            )
            .unwrap_or_else(|e| {
                eprintln!("op-host-web-server --serve-web: {e}");
                exit(2);
            });
            if let Err(e) = op_host_services::web_canvas_server::run_web_canvas(path, port, &host) {
                eprintln!("op-host-web-server --serve-web: {e}");
                exit(1);
            }
        }
        "--mcp" => {
            let Some(path) = args.next() else {
                eprintln!("op-host-web-server --mcp: missing <path> arg");
                exit(2);
            };
            if let Err(e) = op_host_services::mcp_serve::run(PathBuf::from(path)) {
                eprintln!("op-host-web-server --mcp: {e}");
                exit(1);
            }
        }
        "--mcp-http" => {
            let Some(port_arg) = args.next() else {
                eprintln!("op-host-web-server --mcp-http: missing <port> arg");
                exit(2);
            };
            let Ok(port) = port_arg.parse::<u16>() else {
                eprintln!("op-host-web-server --mcp-http: <port> must be a u16, got {port_arg:?}");
                exit(2);
            };
            let Some(path) = args.next() else {
                eprintln!("op-host-web-server --mcp-http: missing <path> arg");
                exit(2);
            };
            if let Err(e) = op_host_services::mcp_serve::run_http(PathBuf::from(path), port) {
                eprintln!("op-host-web-server --mcp-http: {e}");
                exit(1);
            }
        }
        other => {
            eprintln!(
                "op-host-web-server: unknown mode {other:?} \
                 (expected --serve-web / --mcp / --mcp-http)"
            );
            exit(2);
        }
    }
}
