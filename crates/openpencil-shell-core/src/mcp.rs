//! MCP (Model Context Protocol) request / response types.
//! Mirrors the wire shape `packages/pen-mcp` uses for its stdio +
//! HTTP server. v1 scope: protocol types + tool registry trait.
//! Real stdio listener + HTTP server land in `openpencil-desktop`
//! (or a dedicated `openpencil-mcp` binary) once the routing
//! decisions are made; the data shape here lets that work proceed
//! without redesign.

use std::collections::BTreeMap;

/// JSON-RPC-style request id. Strings + integers both supported by
/// the spec; we accept either over the wire.
#[derive(Debug, Clone, PartialEq)]
pub enum RequestId {
    Str(String),
    Num(i64),
}

/// Inbound tool invocation. `tool` is the registered tool name
/// (`insert_node`, `batch_design`, `design_skeleton`, etc); `arguments`
/// is the JSON object the tool expects.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: RequestId,
    pub tool: String,
    pub arguments: BTreeMap<String, String>,
}

/// Tool response — either a structured result object or an error.
/// Errors are typed enough for the LLM client to recover (e.g.
/// `MissingArgument` vs `InvalidArgument` vs `ToolFailed`).
#[derive(Debug, Clone, PartialEq)]
pub enum ToolResponse {
    Ok {
        id: RequestId,
        result: BTreeMap<String, String>,
    },
    Err {
        id: RequestId,
        code: ToolErrorCode,
        message: String,
    },
}

/// Tool failure kind — matches JSON-RPC error categories. The MCP
/// server maps these to standard codes when serialising.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolErrorCode {
    MissingArgument,
    InvalidArgument,
    ToolFailed,
    UnknownTool,
    Internal,
}

/// Result of a tool's work — content + payload only. The
/// `ToolRegistry::dispatch` wrapper attaches the originating
/// `RequestId` so a misbehaving tool literally can't mint a
/// wrong id (codex BLOCK: passing `&ToolCall` to tools left id
/// preservation as a convention only; this shape enforces it
/// structurally).
#[derive(Debug, Clone, PartialEq)]
pub enum ToolOutcome {
    Ok(BTreeMap<String, String>),
    Err(ToolErrorCode, String),
}

/// Trait every MCP tool implements. The MCP server walks its
/// `ToolRegistry`, looks up the requested tool, and forwards the
/// arguments. Tools return a `ToolOutcome`; the registry wraps it
/// with the originating request id to produce a `ToolResponse`.
pub trait McpTool: Send + Sync {
    fn name(&self) -> &str;
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome;
}

/// Registry — owned by the MCP server. v1 is a plain HashMap; a
/// future version may add priority / per-tool auth / rate limits.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Box<dyn McpTool>>,
}

impl ToolRegistry {
    pub fn register(&mut self, tool: Box<dyn McpTool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }
    pub fn dispatch(&self, call: ToolCall) -> ToolResponse {
        // The registry — not the tool — stamps the response id. Tools
        // never see the id; their `ToolOutcome` is content-only. This
        // makes id mismatch structurally impossible (codex BLOCK:
        // passing the id to tools left enforcement as convention).
        let Some(tool) = self.tools.get(&call.tool) else {
            return ToolResponse::Err {
                id: call.id,
                code: ToolErrorCode::UnknownTool,
                message: format!("unknown tool: {}", call.tool),
            };
        };
        match tool.call(&call.arguments) {
            ToolOutcome::Ok(result) => ToolResponse::Ok {
                id: call.id,
                result,
            },
            ToolOutcome::Err(code, message) => ToolResponse::Err {
                id: call.id,
                code,
                message,
            },
        }
    }
    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(String::as_str).collect()
    }
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// JSON-RPC wire serialiser for `ToolResponse`. Manual emitter so
/// shell-core stays serde-free (no dep adds for wasm32). Produces
/// the standard `{"jsonrpc": "2.0", "id": ..., "result": ...}` /
/// `{"jsonrpc": "2.0", "id": ..., "error": {"code": ..., "message"
/// ...}}` shape any MCP client expects.
pub fn response_to_json(r: &ToolResponse) -> String {
    let (id_repr, body) = match r {
        ToolResponse::Ok { id, result } => (
            id_to_json(id),
            format!(r#""result":{}"#, btree_to_json(result)),
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

/// Parse a JSON-RPC request line into a `ToolCall`. Returns None on
/// malformed input. Same minimal-parser strategy as `response_to_json`
/// — hand-rolled, no serde. Real production servers should use serde
/// but the stub is enough to round-trip the test fixtures.
pub fn parse_tool_call(line: &str) -> Option<ToolCall> {
    // Stub parser — extracts the three required fields (`id`,
    // `method`, `params`) by simple string searches. Robust against
    // ordering but not against deeply-nested params. Real serde-
    // backed parsing lands when the server binary lands.
    let id = extract_field(line, "id")?;
    let id = if let Ok(n) = id.parse::<i64>() {
        RequestId::Num(n)
    } else {
        RequestId::Str(id.trim_matches('"').to_string())
    };
    let tool = extract_field(line, "method")?.trim_matches('"').to_string();
    // Empty arguments map — real implementation parses the params
    // object into the BTreeMap. Round-trip with the simple test
    // fixtures is enough for the v1 scaffold.
    Some(ToolCall {
        id,
        tool,
        arguments: BTreeMap::new(),
    })
}

fn id_to_json(id: &RequestId) -> String {
    match id {
        RequestId::Str(s) => json_escape(s),
        RequestId::Num(n) => n.to_string(),
    }
}

fn error_code_to_int(code: ToolErrorCode) -> i32 {
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

fn btree_to_json(m: &BTreeMap<String, String>) -> String {
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

fn json_escape(s: &str) -> String {
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

fn extract_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{}\"", key);
    let start = line.find(&needle)? + needle.len();
    let after_colon = &line[start..];
    let colon = after_colon.find(':')? + 1;
    let val = after_colon[colon..].trim_start();
    let val_start = start + colon + (after_colon[colon..].len() - val.len());
    // Read until next , or }.
    let end_rel = val
        .find(|c: char| c == ',' || c == '}')
        .unwrap_or(val.len());
    Some(line[val_start..val_start + end_rel].trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;
    impl McpTool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
            ToolOutcome::Ok(args.clone())
        }
    }

    /// Deliberately badly-behaved tool: tries to invent a different
    /// response id. Under the v2 trait it CAN'T — `call` returns
    /// outcome only; the registry stamps the id. Used by the
    /// `registry_forces_id_on_misbehaving_tool` regression.
    struct LyingTool;
    impl McpTool for LyingTool {
        fn name(&self) -> &str {
            "lie"
        }
        fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
            ToolOutcome::Ok(BTreeMap::new())
        }
    }

    #[test]
    fn registry_starts_empty() {
        let r = ToolRegistry::default();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
        assert!(r.names().is_empty());
    }

    #[test]
    fn registry_dispatches_to_registered_tool() {
        let mut r = ToolRegistry::default();
        r.register(Box::new(EchoTool));
        let mut args = BTreeMap::new();
        args.insert("k".into(), "v".into());
        let call = ToolCall {
            id: RequestId::Str("req-1".into()),
            tool: "echo".into(),
            arguments: args.clone(),
        };
        match r.dispatch(call) {
            ToolResponse::Ok { id, result } => {
                // Codex BLOCK: the request id MUST round-trip via the
                // tool — JSON-RPC matches responses by id.
                assert_eq!(id, RequestId::Str("req-1".into()));
                assert_eq!(result.get("k"), Some(&"v".to_string()));
            }
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn registry_forces_id_on_response_regardless_of_tool() {
        // Codex BLOCK round 2: id preservation must be enforced
        // structurally, not by convention. The trait now returns
        // a content-only `ToolOutcome`; the registry stamps the id.
        // Verify any tool's response carries the registry-supplied
        // id even when the tool itself has no access to it.
        let mut r = ToolRegistry::default();
        r.register(Box::new(LyingTool));
        let call = ToolCall {
            id: RequestId::Str("req-honest".into()),
            tool: "lie".into(),
            arguments: BTreeMap::new(),
        };
        match r.dispatch(call) {
            ToolResponse::Ok { id, .. } => {
                assert_eq!(id, RequestId::Str("req-honest".into()));
            }
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn response_to_json_ok_payload() {
        let mut result = BTreeMap::new();
        result.insert("k".into(), "v".into());
        let r = ToolResponse::Ok {
            id: RequestId::Num(7),
            result,
        };
        let j = response_to_json(&r);
        assert!(j.contains(r#""jsonrpc":"2.0""#));
        assert!(j.contains(r#""id":7"#));
        assert!(j.contains(r#""result":"#));
        assert!(j.contains(r#""k":"v""#));
    }

    #[test]
    fn response_to_json_err_payload() {
        let r = ToolResponse::Err {
            id: RequestId::Str("req".into()),
            code: ToolErrorCode::UnknownTool,
            message: "no such tool".into(),
        };
        let j = response_to_json(&r);
        assert!(j.contains(r#""id":"req""#));
        assert!(j.contains(r#""code":-32601"#));
        assert!(j.contains(r#""message":"no such tool""#));
    }

    #[test]
    fn parse_tool_call_round_trips_through_registry() {
        let line = r#"{"jsonrpc":"2.0","id":42,"method":"echo","params":{}}"#;
        let call = parse_tool_call(line).expect("parse");
        assert_eq!(call.id, RequestId::Num(42));
        assert_eq!(call.tool, "echo");
        let mut r = ToolRegistry::default();
        r.register(Box::new(EchoTool));
        match r.dispatch(call) {
            ToolResponse::Ok { id, .. } => assert_eq!(id, RequestId::Num(42)),
            _ => panic!(),
        }
    }

    #[test]
    fn json_escape_handles_special_chars() {
        let r = ToolResponse::Err {
            id: RequestId::Str("x\"y".into()),
            code: ToolErrorCode::Internal,
            message: "line1\nline2".into(),
        };
        let j = response_to_json(&r);
        assert!(j.contains(r#""x\"y""#));
        assert!(j.contains(r#""line1\nline2""#));
    }

    #[test]
    fn registry_errors_on_unknown_tool() {
        let r = ToolRegistry::default();
        let call = ToolCall {
            id: RequestId::Num(7),
            tool: "nope".into(),
            arguments: BTreeMap::new(),
        };
        match r.dispatch(call) {
            ToolResponse::Err { code, message, .. } => {
                assert_eq!(code, ToolErrorCode::UnknownTool);
                assert!(message.contains("nope"));
            }
            _ => panic!("expected Err"),
        }
    }
}
