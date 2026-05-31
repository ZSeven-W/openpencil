use serde_json::Value;
use std::io::{Read, Write};

const MCP_PATH: &str = "/mcp";

pub(crate) fn status_json(port: u16) -> String {
    status_json_from_running(
        port,
        std::net::TcpStream::connect(("127.0.0.1", port)).is_ok(),
    )
}

pub(crate) fn status_json_from_running(port: u16, running: bool) -> String {
    if running {
        format!(r#"{{"running":true,"port":{port},"url":"http://127.0.0.1:{port}"}}"#)
    } else {
        r#"{"running":false}"#.to_string()
    }
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

/// POST `body` to the HTTP MCP server on `127.0.0.1:port` and return
/// the response body (the JSON-RPC reply).
pub(crate) fn post(port: u16, body: &str) -> Result<String, String> {
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

pub(crate) fn pretty_json(raw: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| raw.to_string())
}
