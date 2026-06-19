//! JSON-RPC wire-protocol helpers (handshake / ping / shutdown /
//! stateless classification). Split out of `mcp_serve.rs` to keep it
//! under the 800-line cap. Re-exported from `mcp_serve` so callers keep
//! using `crate::mcp_serve::*`.

use super::{sniff_id_raw, sniff_method};

pub fn initialize_response(id_raw: &str) -> String {
    // Spec: `initialize` returns protocolVersion + capabilities +
    // serverInfo. We declare only `tools` capabilities — no
    // resources / prompts / completion are exposed yet. serverInfo.version
    // sources CARGO_PKG_VERSION (the workspace version) so it matches the TS
    // pen-mcp serverInfo, which reads package.json — both report the product
    // version with no hand-maintained drift.
    let version = env!("CARGO_PKG_VERSION");
    format!(
        r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"protocolVersion":"2024-11-05","capabilities":{{"tools":{{"listChanged":false}}}},"serverInfo":{{"name":"{MCP_SERVER_NAME}","version":"{version}"}}}}}}"#
    )
}

pub fn ping_response(id_raw: &str, token: Option<&str>) -> String {
    // Carry the OpenPencil identity marker so the `op` CLI can confirm a
    // port is really our MCP server (not a stale listener or a third-party
    // JSON-RPC server) before routing tool calls to it. A CLI-spawned
    // headless server also echoes its per-instance `token` so `op stop` can
    // confirm the pid in its manager file owns this port. The token is
    // passed in (sourced from the env at the call site) so this stays a
    // pure formatter — testable without mutating process-global env.
    match token {
        Some(token) => format!(
            r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"server":"{MCP_SERVER_NAME}","mode":"headless","token":"{token}"}}}}"#
        ),
        None => {
            format!(
                r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"server":"{MCP_SERVER_NAME}"}}}}"#
            )
        }
    }
}

/// The CLI passes a per-instance token to a spawned `--mcp-http` server via
/// `OPENPENCIL_MCP_TOKEN`; reporting it in `ping` lets the CLI confirm the
/// pid in its manager file belongs to the server actually serving the port
/// (defeats stale-pid / port-reuse confusion on `op stop` / reuse).
/// Restricted to safe chars so it embeds in the JSON reply without escaping;
/// anything else is treated as absent.
pub fn headless_token_from_env() -> Option<String> {
    sanitize_token(std::env::var("OPENPENCIL_MCP_TOKEN").ok()?)
}

/// Accept a candidate token only when it is non-empty and `[A-Za-z0-9_-]`,
/// so it embeds in the JSON ping reply without escaping. Pure (no env), so
/// tests exercise it directly. Anything else ⇒ `None`.
pub fn sanitize_token(token: String) -> Option<String> {
    let safe = !token.is_empty()
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    safe.then_some(token)
}

/// OpenPencil MCP `serverInfo.name` + ping identity marker.
pub const MCP_SERVER_NAME: &str = "openpencil-mcp";

/// A JSON-RPC method that needs no document state, so it can be answered
/// without a UI-thread snapshot. Lets the live server (`mcp_live`) reply
/// to handshake/liveness probes instantly instead of round-tripping the
/// UI thread (which would make `op`'s `ping` slow / false-negative under
/// editor load).
pub enum Stateless {
    /// Canned response ready to send (e.g. `initialize`).
    Respond(String),
    /// A notification — swallow with no body (HTTP 202).
    Swallow,
    /// A `ping` carrying this raw JSON-RPC id; the caller builds the body
    /// (the live server adds its identity token).
    Ping(String),
    /// Needs document state — fall through to the snapshot/dispatch path.
    NeedsState,
}

/// If `line` is a token-authed `openpencil/shutdown` whose `params.token`
/// matches `token` (non-empty), return its raw JSON-RPC id (for the ack).
/// This lets the `op` CLI ask the server to quit ITSELF — no pid-kill, so
/// there's no signal-the-wrong-process race. An empty/mismatched token is
/// rejected so a random client can't shut the server down.
pub fn shutdown_request_id(line: &str, token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("method").and_then(serde_json::Value::as_str) != Some("openpencil/shutdown") {
        return None;
    }
    let req_token = value
        .get("params")
        .and_then(|params| params.get("token"))
        .and_then(serde_json::Value::as_str)?;
    if req_token == token {
        Some(
            value
                .get("id")
                .map(|id| id.to_string())
                .unwrap_or_else(|| "null".to_string()),
        )
    } else {
        None
    }
}

/// JSON-RPC ack for an accepted `openpencil/shutdown`.
pub fn shutdown_ok_response(id_raw: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id_raw},"result":{{"ok":true,"shuttingDown":true}}}}"#)
}

/// Classify a JSON-RPC line as a stateless handshake method or not.
pub fn classify_stateless(line: &str) -> Stateless {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Stateless::Swallow;
    }
    match sniff_method(trimmed).as_deref() {
        Some("initialize") => match sniff_id_raw(trimmed) {
            Some(id) => Stateless::Respond(initialize_response(&id)),
            None => Stateless::Swallow,
        },
        Some("ping") => match sniff_id_raw(trimmed) {
            Some(id) => Stateless::Ping(id),
            None => Stateless::Swallow,
        },
        Some("notifications/initialized") | Some("initialized") => Stateless::Swallow,
        _ => Stateless::NeedsState,
    }
}
