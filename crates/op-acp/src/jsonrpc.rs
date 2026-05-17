//! Minimal JSON-RPC 2.0 engine over the ndJSON transport.
//!
//! [`JsonRpcEngine`] allocates request ids, correlates responses
//! through per-id oneshot channels, and — via [`dispatch_inbound`] —
//! routes inbound frames: responses to their waiter, `session/update`
//! notifications to a channel, and `session/request_permission`
//! requests to an auto-approval reply (TS parity — the user already
//! trusted the agent by configuring it).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use crate::protocol::{
    classify_inbound, Inbound, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
    RequestPermissionParams, SessionNotification, METHOD_REQUEST_PERMISSION, METHOD_SESSION_UPDATE,
};
use crate::types::AcpError;

/// Map of in-flight request id → response waiter.
type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, AcpError>>>>>;

/// Shared JSON-RPC engine — cloned between the connection handle and
/// the background reader task.
#[derive(Clone)]
pub struct JsonRpcEngine {
    out_tx: mpsc::UnboundedSender<Value>,
    pending: Pending,
    next_id: Arc<AtomicU64>,
}

impl JsonRpcEngine {
    /// Build an engine that writes outbound frames to `out_tx`.
    pub fn new(out_tx: mpsc::UnboundedSender<Value>) -> Self {
        Self {
            out_tx,
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// The shared pending-request map (the reader task resolves it).
    pub fn pending(&self) -> Pending {
        self.pending.clone()
    }

    /// A clone of the outbound-frame sender.
    pub fn out_tx(&self) -> mpsc::UnboundedSender<Value> {
        self.out_tx.clone()
    }

    /// Send a request and await its correlated response, up to
    /// `timeout`.
    pub async fn call(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, AcpError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);

        let req = JsonRpcRequest::new(id, method, params);
        let frame =
            serde_json::to_value(&req).map_err(|e| AcpError::Protocol(e.to_string()))?;
        if self.out_tx.send(frame).is_err() {
            self.pending.lock().unwrap().remove(&id);
            return Err(AcpError::Closed);
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            // The reader task dropped the sender — connection died.
            Ok(Err(_)) => Err(AcpError::Closed),
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                Err(AcpError::Transport(format!(
                    "request '{method}' timed out"
                )))
            }
        }
    }
}

/// Choose an "allow" option from a `session/request_permission`
/// request and build the JSON-RPC result that selects it. Mirrors the
/// TS `requestPermission` handler.
pub fn auto_approve_permission(params: &Value) -> Value {
    let parsed: RequestPermissionParams =
        serde_json::from_value(params.clone()).unwrap_or(RequestPermissionParams { options: vec![] });
    let chosen = parsed
        .options
        .iter()
        .find(|o| {
            matches!(o.kind.as_deref(), Some("allow_once") | Some("allow_always"))
                || o.option_id.starts_with("allow")
        })
        .or_else(|| parsed.options.first());
    let option_id = chosen
        .map(|o| o.option_id.clone())
        .unwrap_or_else(|| "allow".to_string());
    serde_json::json!({
        "outcome": { "outcome": "selected", "optionId": option_id }
    })
}

/// Route one inbound JSON frame: resolve a response waiter, forward a
/// `session/update` notification, or reply to an agent → client
/// request.
pub fn dispatch_inbound(
    value: Value,
    pending: &Pending,
    notif_tx: &mpsc::UnboundedSender<SessionNotification>,
    out_tx: &mpsc::UnboundedSender<Value>,
) {
    match classify_inbound(&value) {
        Inbound::Response { id, result, error } => {
            if let Some(tx) = pending.lock().unwrap().remove(&id) {
                let resolved = match error {
                    Some(e) => Err(AcpError::Rpc {
                        code: e.code,
                        message: e.message,
                    }),
                    None => Ok(result.unwrap_or(Value::Null)),
                };
                let _ = tx.send(resolved);
            }
        }
        Inbound::Request { id, method, params } => {
            let response = if method == METHOD_REQUEST_PERMISSION {
                JsonRpcResponse::ok(id, auto_approve_permission(&params))
            } else {
                // Unsupported agent → client request — reply with a
                // JSON-RPC "method not found" so the agent fails fast
                // instead of hanging.
                JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32601,
                        message: format!("method '{method}' not supported"),
                        data: None,
                    }),
                }
            };
            if let Ok(frame) = serde_json::to_value(&response) {
                let _ = out_tx.send(frame);
            }
        }
        Inbound::Notification { method, params } => {
            if method == METHOD_SESSION_UPDATE {
                if let Ok(note) = serde_json::from_value::<SessionNotification>(params) {
                    let _ = notif_tx.send(note);
                }
            }
            // Other notifications are not surfaced.
        }
        Inbound::Unknown => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_approve_prefers_allow_option() {
        let params = serde_json::json!({
            "options": [
                { "optionId": "reject-1", "kind": "reject_once" },
                { "optionId": "ok-1", "kind": "allow_always" }
            ]
        });
        let out = auto_approve_permission(&params);
        assert_eq!(out["outcome"]["outcome"], "selected");
        assert_eq!(out["outcome"]["optionId"], "ok-1");
    }

    #[test]
    fn auto_approve_falls_back_to_first_option() {
        let params = serde_json::json!({
            "options": [{ "optionId": "first", "kind": "custom" }]
        });
        assert_eq!(
            auto_approve_permission(&params)["outcome"]["optionId"],
            "first"
        );
        // No options at all → the generic "allow" sentinel.
        let empty = serde_json::json!({ "options": [] });
        assert_eq!(
            auto_approve_permission(&empty)["outcome"]["optionId"],
            "allow"
        );
    }

    #[tokio::test]
    async fn call_correlates_a_response() {
        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Value>();
        let engine = JsonRpcEngine::new(out_tx);
        let (notif_tx, _notif_rx) = mpsc::unbounded_channel();
        let pending = engine.pending();
        let reply_tx = engine.out_tx();

        // Background "agent": read the request, echo a response.
        tokio::spawn(async move {
            let req = out_rx.recv().await.unwrap();
            let id = req["id"].as_u64().unwrap();
            let response = serde_json::json!({
                "jsonrpc": "2.0", "id": id, "result": { "pong": true }
            });
            dispatch_inbound(response, &pending, &notif_tx, &reply_tx);
        });

        let result = engine
            .call("ping", serde_json::json!({}), Duration::from_secs(2))
            .await
            .unwrap();
        assert_eq!(result["pong"], true);
    }

    #[tokio::test]
    async fn call_surfaces_rpc_error() {
        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Value>();
        let engine = JsonRpcEngine::new(out_tx);
        let (notif_tx, _notif_rx) = mpsc::unbounded_channel();
        let pending = engine.pending();
        let reply_tx = engine.out_tx();
        tokio::spawn(async move {
            let req = out_rx.recv().await.unwrap();
            let id = req["id"].as_u64().unwrap();
            let err = serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": -32000, "message": "boom" }
            });
            dispatch_inbound(err, &pending, &notif_tx, &reply_tx);
        });
        let err = engine
            .call("fail", serde_json::json!({}), Duration::from_secs(2))
            .await
            .unwrap_err();
        assert!(matches!(err, AcpError::Rpc { code: -32000, .. }));
    }

    #[tokio::test]
    async fn notification_reaches_the_channel() {
        let (out_tx, _out_rx) = mpsc::unbounded_channel::<Value>();
        let (notif_tx, mut notif_rx) = mpsc::unbounded_channel();
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let note = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": "s1",
                "update": { "sessionUpdate": "agent_message_chunk",
                            "content": { "type": "text", "text": "hi" } }
            }
        });
        dispatch_inbound(note, &pending, &notif_tx, &out_tx);
        let received = notif_rx.recv().await.unwrap();
        assert_eq!(received.session_id.as_deref(), Some("s1"));
    }
}
