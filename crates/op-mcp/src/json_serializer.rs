//! Manual JSON-RPC wire serializer for `ToolResponse` + internal
//! helpers. Hand-rolled so shell-core stays serde-free (wasm32
//! bundle size). Carved off `mcp.rs` to stay under the 800-line cap.

use std::collections::BTreeMap;

use super::{RequestId, ToolErrorCode, ToolResponse};

/// JSON-RPC wire serialiser for `ToolResponse`. Produces the
/// standard `{"jsonrpc": "2.0", "id": ..., "result": ...}` /
/// `{"jsonrpc": "2.0", "id": ..., "error": {"code": ..., "message":
/// ...}}` shape any MCP client expects.
pub fn response_to_json(r: &ToolResponse) -> String {
    let (id_repr, body) = match r {
        ToolResponse::Ok {
            id, result, json, ..
        } => (
            id_to_json(id),
            match json {
                // Nested-JSON read result: embed verbatim as the wire result.
                Some(raw) => format!(r#""result":{raw}"#),
                None => format!(r#""result":{}"#, btree_to_json(result)),
            },
        ),
        ToolResponse::Err { id, code, message } => (
            id_to_json(id),
            format!(
                r#""error":{{"code":{},"message":{}}}"#,
                error_code_to_int(*code),
                json_escape(message),
            ),
        ),
    };
    format!(r#"{{"jsonrpc":"2.0","id":{},{}}}"#, id_repr, body)
}

/// JSON-RPC serializer for a *tool* result in the MCP-spec `tools/call`
/// shape, matching TS `pen-mcp` (`server.ts`): the tool's data rides inside
/// `result.content` as a single `text` block (the flat result map serialized
/// to a JSON string), and a tool-level failure becomes `isError:true` in the
/// result — NOT a JSON-RPC `error` (those are reserved for transport/parse
/// failures, still emitted via [`response_to_json`]). External MCP clients
/// (Claude Code / Codex) require this envelope.
pub fn tool_response_to_json(r: &ToolResponse) -> String {
    let (id_repr, body) = match r {
        ToolResponse::Ok {
            id, result, json, ..
        } => {
            // Nested-JSON read result rides verbatim in the text block;
            // otherwise the flat string-map is encoded as the text.
            let text = match json {
                Some(raw) => json_escape(raw),
                None => json_escape(&btree_to_json(result)),
            };
            (
                id_to_json(id),
                format!(r#""result":{{"content":[{{"type":"text","text":{text}}}]}}"#),
            )
        }
        ToolResponse::Err { id, message, .. } => (
            id_to_json(id),
            format!(
                r#""result":{{"content":[{{"type":"text","text":{}}}],"isError":true}}"#,
                json_escape(&format!("Error: {message}")),
            ),
        ),
    };
    format!(r#"{{"jsonrpc":"2.0","id":{},{}}}"#, id_repr, body)
}

pub(super) fn id_to_json(id: &RequestId) -> String {
    match id {
        RequestId::Str(s) => json_escape(s),
        RequestId::Num(n) => n.to_string(),
    }
}

pub(super) fn error_code_to_int(code: ToolErrorCode) -> i32 {
    // JSON-RPC reserves -32600..-32603 for transport-level errors;
    // tool errors live in the application range (-32000..-32099).
    match code {
        ToolErrorCode::MissingArgument => -32_001,
        ToolErrorCode::InvalidArgument => -32_602,
        ToolErrorCode::ToolFailed => -32_002,
        ToolErrorCode::UnknownTool => -32_601,
        ToolErrorCode::Internal => -32_603,
    }
}

pub(super) fn btree_to_json(m: &BTreeMap<String, String>) -> String {
    let mut out = String::from("{");
    let mut first = true;
    for (k, v) in m {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&format!("{}:{}", json_escape(k), json_escape(v)));
    }
    out.push('}');
    out
}

pub(super) fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
