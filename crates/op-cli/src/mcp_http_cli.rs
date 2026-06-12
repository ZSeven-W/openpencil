use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MCP_PATH: &str = "/mcp";

/// OpenPencil MCP identity marker reported in the `ping` reply's `result`
/// (mirrors `op-host-desktop` `MCP_SERVER_NAME`). Used to confirm a port
/// is really our server before routing tool calls / killing a pid.
const MCP_SERVER_NAME: &str = "openpencil-mcp";

/// Read deadline for a normal tool call. Generous headroom over the live
/// server's per-apply UI ack budget so a big batch design doesn't time out
/// CLI-side before the editor finishes.
const POST_TIMEOUT: Duration = Duration::from_secs(60);
/// Short deadline for the liveness/identity probe — `ping` is stateless
/// and never touches the UI thread, so it must return quickly.
const PING_TIMEOUT: Duration = Duration::from_secs(2);
/// Deadline for a graceful shutdown ack. A little patient so a headless
/// server briefly busy with an in-flight op (its accept loop is serial)
/// still processes the request rather than appearing unresponsive.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) fn status_json(port: u16) -> String {
    // Verify it's genuinely an OpenPencil MCP server (identity ping —
    // `result.server == "openpencil-mcp"`), not merely any TCP listener on
    // the port. Mirrors TS `op status`, which reports running only when the
    // OpenPencil endpoint actually responds. A stale/foreign listener now
    // correctly reports `running:false`.
    let running = ping_result(port).is_some();
    // TS `op status` returns {running, port, pid, url, uptime}. When the live
    // discovery file is present AND advertises this port, surface pid +
    // uptime (seconds since the host wrote `timestamp`). Otherwise fall back
    // to the discovery-free shape.
    if running {
        if let Some((file_port, pid, timestamp)) = crate::app_control_cli::live_status_fields() {
            if file_port == port {
                let uptime = now_millis().saturating_sub(timestamp) / 1000;
                return format!(
                    r#"{{"running":true,"port":{port},"pid":{pid},"url":"http://127.0.0.1:{port}","uptime":{uptime}}}"#
                );
            }
        }
    }
    status_json_from_running(port, running)
}

pub(crate) fn status_json_from_running(port: u16, running: bool) -> String {
    if running {
        format!(r#"{{"running":true,"port":{port},"url":"http://127.0.0.1:{port}"}}"#)
    } else {
        r#"{"running":false}"#.to_string()
    }
}

/// Wall-clock milliseconds since the Unix epoch (for `op status` uptime).
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// JSON-RPC body for `tools/list`.
pub(crate) fn tools_list_body() -> String {
    r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#.to_string()
}

/// JSON-RPC body for a `tools/call` of `tool` with the already-built
/// `arguments` object JSON.
pub(crate) fn tool_call_body(tool: &str, args_json: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{}","arguments":{}}}}}"#,
        json_escape(tool),
        args_json
    )
}

/// Build a JSON object from `key=value` pairs. MCP tool arguments are
/// scalar string-typed, so every value is emitted as a JSON string.
pub(crate) fn args_to_json(pairs: &[(String, String)]) -> String {
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
pub(crate) fn json_escape(s: &str) -> String {
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

pub(crate) fn http_request(body: &str) -> String {
    format!(
        "POST {MCP_PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

/// POST `body` to `127.0.0.1:port/mcp` with connect + read/write deadlines;
/// return `(http_status, body)`. `http_status` is 0 when no recognizable
/// status line was returned. The deadlines stop a stale port whose
/// service accepts but never replies from hanging the CLI indefinitely.
fn post_raw(port: u16, body: &str, timeout: Duration) -> Result<(u16, String), String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&addr, timeout).map_err(|e| {
        format!(
            "cannot reach the editor on 127.0.0.1:{port}: {e}\n\
             start the OpenPencil MCP server and point clients at http://127.0.0.1:{port}/mcp"
        )
    })?;
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    stream
        .write_all(http_request(body).as_bytes())
        .map_err(|e| format!("http write: {e}"))?;
    stream.flush().ok();
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("http read: {e}"))?;
    let status = parse_status_code(&response).unwrap_or(0);
    let body = match response.split_once("\r\n\r\n") {
        Some((_, body)) => body.trim().to_string(),
        None => response.trim().to_string(),
    };
    Ok((status, body))
}

/// Parse the numeric status from an HTTP status line (`HTTP/1.1 200 OK`).
fn parse_status_code(response: &str) -> Option<u16> {
    let mut parts = response.lines().next()?.split_whitespace();
    let _http_version = parts.next()?;
    parts.next()?.parse::<u16>().ok()
}

/// POST `body` to the HTTP MCP server on `127.0.0.1:port` and return the
/// tool result for display. A non-2xx HTTP status (e.g. a 404 from the wrong
/// service, or a 500 from a failed live UI request) is an error. The MCP
/// `tools/call` content envelope is unwrapped to the raw tool result, and an
/// `isError` tool result (or a JSON-RPC transport error) is surfaced as an
/// error — so `op` exits non-zero on a failed tool, like the TS CLI.
pub(crate) fn post(port: u16, body: &str) -> Result<String, String> {
    let (status, reply) = post_raw(port, body, POST_TIMEOUT)?;
    if !(200..300).contains(&status) {
        return Err(format!(
            "MCP server on 127.0.0.1:{port} returned HTTP {status}: {reply}"
        ));
    }
    unwrap_mcp_reply(&reply)
}

/// Unwrap an MCP JSON-RPC reply for human/agent-facing output, matching the
/// TS CLI (which prints the raw tool result, not the wire envelope):
/// - a `tools/call` content envelope → the inner `text` (the tool's data);
///   an `isError:true` result → `Err(text)`.
/// - a JSON-RPC transport `error` → `Err(message)`.
/// - anything else (e.g. a `tools/list` reply) → the raw reply unchanged.
fn unwrap_mcp_reply(reply: &str) -> Result<String, String> {
    let Ok(value) = serde_json::from_str::<Value>(reply) else {
        return Ok(reply.to_string());
    };
    if let Some(message) = value
        .get("error")
        .and_then(|err| err.get("message"))
        .and_then(Value::as_str)
    {
        return Err(message.to_string());
    }
    let Some(result) = value.get("result") else {
        return Ok(reply.to_string());
    };
    if let Some(content) = result.get("content").and_then(Value::as_array) {
        let text = content
            .iter()
            .filter_map(|block| block.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
        if result.get("isError").and_then(Value::as_bool) == Some(true) {
            return Err(text);
        }
        return Ok(text);
    }
    Ok(reply.to_string())
}

/// Plain HTTP `GET` against `127.0.0.1:port{path}` with the same deadline
/// discipline as [`post_raw`]; returns `(http_status, body)`.
fn http_get_raw(port: u16, path: &str, timeout: Duration) -> Result<(u16, String), String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|e| format!("connect 127.0.0.1:{port}: {e}"))?;
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("http write: {e}"))?;
    stream.flush().ok();
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("http read: {e}"))?;
    let status = parse_status_code(&response).unwrap_or(0);
    let body = match response.split_once("\r\n\r\n") {
        Some((_, body)) => body.trim().to_string(),
        None => response.trim().to_string(),
    };
    Ok((status, body))
}

/// True when `127.0.0.1:port` is the `--serve-web` web-canvas daemon —
/// `GET /api/mcp/server` reports our identity AND `mode:"web-canvas"`. Used
/// by `op start --web` so it never "reuses" a plain `--mcp-http` server
/// (which answers the token ping but serves no browser editor).
pub(crate) fn web_daemon_running(port: u16) -> bool {
    match http_get_raw(port, "/api/mcp/server", PING_TIMEOUT) {
        Ok((status, body)) => (200..300).contains(&status) && is_web_canvas_health(&body),
        Err(_) => false,
    }
}

/// Pure predicate over the `/api/mcp/server` health body (testable without
/// a socket): the OpenPencil identity marker plus the web-canvas mode.
fn is_web_canvas_health(body: &str) -> bool {
    serde_json::from_str::<Value>(body).is_ok_and(|v| {
        v.get("server").and_then(Value::as_str) == Some(MCP_SERVER_NAME)
            && v.get("mode").and_then(Value::as_str) == Some("web-canvas")
    })
}

/// Send a JSON-RPC `ping` and return the parsed `result` object when the
/// reply is an OpenPencil MCP server (`result.server == "openpencil-mcp"`).
/// This distinguishes our `/mcp` JSON-RPC server from a stale TCP listener,
/// a third-party JSON-RPC server, or a TS editor (which serves REST at
/// `/api/mcp/*`, not `/mcp`) — so discovery never mis-routes tool calls.
fn ping_result(port: u16) -> Option<Value> {
    let body = r#"{"jsonrpc":"2.0","id":0,"method":"ping"}"#;
    let (status, body) = post_raw(port, body, PING_TIMEOUT).ok()?;
    if !(200..300).contains(&status) {
        return None;
    }
    let value = serde_json::from_str::<Value>(&body).ok()?;
    let result = value.get("result")?;
    if result.get("server").and_then(Value::as_str) == Some(MCP_SERVER_NAME) {
        Some(result.clone())
    } else {
        None
    }
}

/// True when `127.0.0.1:port` is the *live* OpenPencil editor that
/// published `token` (`result.mode == "live"` and `result.token` matches).
/// Distinguishes the live canvas from a headless server squatting on a
/// reused port, and proves the server owns the discovery file we read.
pub(crate) fn mcp_ping_live(port: u16, token: &str) -> bool {
    ping_result(port).is_some_and(|result| {
        result.get("mode").and_then(Value::as_str) == Some("live")
            && result.get("token").and_then(Value::as_str) == Some(token)
    })
}

/// True when `127.0.0.1:port` is the CLI-spawned *headless* server that
/// reported this exact `token` — proving the pid in our manager file owns
/// this port, so `op stop` / reuse never act on a recycled pid. An empty or
/// mismatched token is treated as UNVERIFIED (returns false): we'd rather
/// re-launch / report "not running" than risk killing an unrelated process.
pub(crate) fn mcp_ping_headless(port: u16, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    ping_result(port)
        .is_some_and(|result| result.get("token").and_then(Value::as_str) == Some(token))
}

/// Ask the server on `127.0.0.1:port` to shut itself down (`op stop`),
/// authenticated by `token`. Returns true when the server acked
/// `shuttingDown`. This avoids signalling a pid entirely — so there's no
/// race where a recycled pid is killed — and lets the live editor quit
/// cleanly (saving state). An empty token never authenticates.
pub(crate) fn request_shutdown(port: u16, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let body = format!(
        r#"{{"jsonrpc":"2.0","id":0,"method":"openpencil/shutdown","params":{{"token":"{}"}}}}"#,
        json_escape(token)
    );
    match post_raw(port, &body, SHUTDOWN_TIMEOUT) {
        Ok((status, body)) if (200..300).contains(&status) => serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| {
                v.get("result")
                    .and_then(|r| r.get("shuttingDown"))
                    .and_then(Value::as_bool)
            })
            .unwrap_or(false),
        _ => false,
    }
}

pub(crate) fn pretty_json(raw: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwrap_extracts_tool_content_text() {
        let reply = r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"{\"ok\":\"true\"}"}]}}"#;
        assert_eq!(unwrap_mcp_reply(reply).unwrap(), r#"{"ok":"true"}"#);
    }

    #[test]
    fn unwrap_surfaces_iserror_result_as_err() {
        let reply = r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"Error: boom"}],"isError":true}}"#;
        assert_eq!(unwrap_mcp_reply(reply), Err("Error: boom".to_string()));
    }

    #[test]
    fn unwrap_surfaces_transport_error_as_err() {
        let reply = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32602,"message":"bad args"}}"#;
        assert_eq!(unwrap_mcp_reply(reply), Err("bad args".to_string()));
    }

    #[test]
    fn unwrap_passes_through_non_content_reply() {
        // A tools/list reply (result.tools, no content) is returned as-is.
        let reply = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        assert_eq!(unwrap_mcp_reply(reply).unwrap(), reply);
    }

    #[test]
    fn web_canvas_health_requires_identity_and_web_mode() {
        // The `--serve-web` daemon's health shape (extra fields tolerated).
        assert!(is_web_canvas_health(
            r#"{"running":true,"port":3100,"localIp":"127.0.0.1","server":"openpencil-mcp","mode":"web-canvas"}"#
        ));
        // A plain `--mcp-http` server (no health route → would 404 anyway),
        // a foreign service, or a missing mode must all be rejected.
        assert!(!is_web_canvas_health(
            r#"{"running":true,"port":3100,"server":"openpencil-mcp"}"#
        ));
        assert!(!is_web_canvas_health(
            r#"{"server":"someone-else","mode":"web-canvas"}"#
        ));
        assert!(!is_web_canvas_health("not json"));
    }
}
