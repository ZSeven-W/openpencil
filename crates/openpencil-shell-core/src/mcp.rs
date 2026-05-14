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

/// Trait every MCP tool implements. The MCP server walks its
/// `ToolRegistry`, looks up the requested tool, and forwards the
/// full `ToolCall` (id + args) so the tool can mint a response
/// carrying the originating request id — JSON-RPC requires every
/// response to echo it back (codex BLOCK: prior signature dropped
/// the id, forcing tools to invent fakes).
pub trait McpTool: Send + Sync {
    fn name(&self) -> &str;
    fn call(&self, request: &ToolCall) -> ToolResponse;
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
        if let Some(tool) = self.tools.get(&call.tool) {
            tool.call(&call)
        } else {
            ToolResponse::Err {
                id: call.id,
                code: ToolErrorCode::UnknownTool,
                message: format!("unknown tool: {}", call.tool),
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;
    impl McpTool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn call(&self, request: &ToolCall) -> ToolResponse {
            ToolResponse::Ok {
                id: request.id.clone(),
                result: request.arguments.clone(),
            }
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
