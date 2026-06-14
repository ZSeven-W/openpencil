//! ACP wire protocol — JSON-RPC 2.0 envelopes plus the subset of
//! Agent Client Protocol message shapes the client drives.
//!
//! Method names + the `protocolVersion` constant mirror the
//! `@agentclientprotocol/sdk` schema (`AGENT_METHODS` / `PROTOCOL_VERSION`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use op_rpc_transport::{
    classify_inbound, Inbound, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
};

/// ACP protocol version this client speaks (`PROTOCOL_VERSION` in the SDK).
pub const PROTOCOL_VERSION: u32 = 1;

/// `initialize` — handshake.
pub const METHOD_INITIALIZE: &str = "initialize";
/// `session/new` — open a session.
pub const METHOD_SESSION_NEW: &str = "session/new";
/// `session/prompt` — drive one turn.
pub const METHOD_SESSION_PROMPT: &str = "session/prompt";
/// `session/update` — streaming notification from the agent.
pub const METHOD_SESSION_UPDATE: &str = "session/update";
/// `session/request_permission` — agent asks the client to approve a tool.
pub const METHOD_REQUEST_PERMISSION: &str = "session/request_permission";

/// `initialize` result — only the fields the client reads.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    #[serde(default)]
    pub protocol_version: Option<u32>,
    #[serde(default)]
    pub agent_info: Option<AgentInfoWire>,
}

/// `agentInfo` block of an [`InitializeResult`].
#[derive(Debug, Default, Deserialize)]
pub struct AgentInfoWire {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

/// `session/new` result.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionResult {
    pub session_id: String,
}

/// One content block of a prompt / message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text.
    Text { text: String },
    /// Any other content type — preserved opaquely.
    #[serde(other)]
    Other,
}

/// The discriminated `update` payload of a `session/update`
/// notification (`sessionUpdate` tag).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "sessionUpdate", rename_all = "snake_case")]
pub enum SessionUpdate {
    /// A streamed chunk of the agent's reply.
    AgentMessageChunk { content: ContentBlock },
    /// A streamed chunk of the agent's private reasoning.
    AgentThoughtChunk { content: ContentBlock },
    /// The agent announced a tool call (display-only).
    ToolCall {
        #[serde(rename = "toolCallId", default)]
        tool_call_id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(rename = "rawInput", default)]
        raw_input: Value,
    },
    /// A tool call's status changed.
    ToolCallUpdate {
        #[serde(rename = "toolCallId", default)]
        tool_call_id: String,
        #[serde(default)]
        status: Option<String>,
        #[serde(rename = "rawOutput", default)]
        raw_output: Value,
        #[serde(default)]
        content: Value,
    },
    /// Any other update kind — ignored by the event adapter.
    #[serde(other)]
    Other,
}

/// A `session/update` notification's params.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNotification {
    #[serde(default)]
    pub session_id: Option<String>,
    pub update: SessionUpdate,
}

/// One option offered in a `session/request_permission` request.
#[derive(Debug, Clone, Deserialize)]
pub struct PermissionOption {
    #[serde(rename = "optionId")]
    pub option_id: String,
    #[serde(default)]
    pub kind: Option<String>,
}

/// `session/request_permission` request params (only `options`).
#[derive(Debug, Clone, Deserialize)]
pub struct RequestPermissionParams {
    #[serde(default)]
    pub options: Vec<PermissionOption>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_response_request_notification() {
        let resp = serde_json::json!({"jsonrpc":"2.0","id":7,"result":{"ok":true}});
        assert!(matches!(
            classify_inbound(&resp),
            Inbound::Response { id: 7, .. }
        ));
        let req = serde_json::json!({"jsonrpc":"2.0","id":3,"method":"session/request_permission","params":{}});
        assert!(matches!(classify_inbound(&req), Inbound::Request { .. }));
        let note = serde_json::json!({"jsonrpc":"2.0","method":"session/update","params":{}});
        assert!(matches!(
            classify_inbound(&note),
            Inbound::Notification { .. }
        ));
        assert!(matches!(
            classify_inbound(&serde_json::json!(5)),
            Inbound::Unknown
        ));
    }

    #[test]
    fn session_update_deserializes_known_variants() {
        let chunk = serde_json::json!({
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "hello" }
        });
        let u: SessionUpdate = serde_json::from_value(chunk).unwrap();
        match u {
            SessionUpdate::AgentMessageChunk {
                content: ContentBlock::Text { text },
            } => assert_eq!(text, "hello"),
            other => panic!("unexpected: {other:?}"),
        }
        // An unknown update kind falls through to `Other`.
        let weird = serde_json::json!({"sessionUpdate": "plan", "entries": []});
        assert!(matches!(
            serde_json::from_value::<SessionUpdate>(weird).unwrap(),
            SessionUpdate::Other
        ));
    }

    #[test]
    fn tool_call_update_carries_status() {
        let v = serde_json::json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": "t1",
            "status": "completed",
            "rawOutput": { "result": 42 }
        });
        let u: SessionUpdate = serde_json::from_value(v).unwrap();
        match u {
            SessionUpdate::ToolCallUpdate {
                tool_call_id,
                status,
                ..
            } => {
                assert_eq!(tool_call_id, "t1");
                assert_eq!(status.as_deref(), Some("completed"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn request_serializes_with_jsonrpc_envelope() {
        let req = JsonRpcRequest::new(1, METHOD_INITIALIZE, serde_json::json!({}));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
    }
}
