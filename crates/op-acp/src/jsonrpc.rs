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

/// Backpressure ceiling for the outbound-frame queue (requests we send plus
/// the auto-generated replies to agent → client requests). Deliberately
/// generous: a healthy session never queues more than a handful of frames,
/// so hitting this means the writer (or the agent reading it) has stalled.
pub const OUTBOUND_CAPACITY: usize = 1024;

/// Backpressure ceiling for buffered `session/update` notifications. A chatty
/// or malicious agent can stream these faster than the UI drains them; the
/// bound turns unbounded memory growth into a fail-closed connection error.
pub const NOTIFICATION_CAPACITY: usize = 1024;

/// Map of in-flight request id → response waiter.
type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, AcpError>>>>>;

/// Non-blocking enqueue for the reader task. Awaiting here would deadlock the
/// reader against the queues whose responses it must continue dispatching.
/// Overflow is therefore a connection failure, never a silently truncated
/// successful turn.
fn offer<T>(tx: &mpsc::Sender<T>, message: T, what: &str) -> Result<(), AcpError> {
    match tx.try_send(message) {
        Ok(()) => Ok(()),
        Err(mpsc::error::TrySendError::Full(_)) => Err(AcpError::Transport(format!(
            "{what} queue overflow; closing ACP connection"
        ))),
        Err(mpsc::error::TrySendError::Closed(_)) => Err(AcpError::Closed),
    }
}

/// Shared JSON-RPC engine — cloned between the connection handle and
/// the background reader task.
#[derive(Clone)]
pub struct JsonRpcEngine {
    out_tx: mpsc::Sender<Value>,
    pending: Pending,
    next_id: Arc<AtomicU64>,
    failure: Arc<Mutex<Option<AcpError>>>,
}

impl JsonRpcEngine {
    /// Build an engine that writes outbound frames to `out_tx`.
    pub fn new(out_tx: mpsc::Sender<Value>) -> Self {
        Self {
            out_tx,
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            failure: Arc::new(Mutex::new(None)),
        }
    }

    fn current_failure(&self) -> Option<AcpError> {
        self.failure
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Permanently fail this engine and wake every request currently waiting
    /// on the reader. Future calls fail immediately with the same cause.
    pub fn fail(&self, error: AcpError) {
        let error = {
            let mut failure = self
                .failure
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            failure.get_or_insert(error).clone()
        };
        let waiters: Vec<_> = self
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .drain()
            .map(|(_, waiter)| waiter)
            .collect();
        for waiter in waiters {
            let _ = waiter.send(Err(error.clone()));
        }
    }

    /// The shared pending-request map (the reader task resolves it).
    pub fn pending(&self) -> Pending {
        self.pending.clone()
    }

    /// A clone of the outbound-frame sender.
    pub fn out_tx(&self) -> mpsc::Sender<Value> {
        self.out_tx.clone()
    }

    /// Send a request and await its correlated response, up to
    /// `timeout`.
    ///
    /// `timeout` is a deadline for the WHOLE call, enqueue included: the
    /// outbound queue is bounded, so a stalled writer must not silently double
    /// the caller's budget. Awaiting the send is safe here — `call` runs on the
    /// caller's task, never on the writer task that drains the queue, so it
    /// cannot block its own drain.
    pub async fn call(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, AcpError> {
        if let Some(error) = self.current_failure() {
            return Err(error);
        }
        let deadline = tokio::time::Instant::now() + timeout;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id, tx);
        if let Some(error) = self.current_failure() {
            self.pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&id);
            return Err(error);
        }

        let req = JsonRpcRequest::new(id, method, params);
        let frame = serde_json::to_value(&req).map_err(|e| AcpError::Protocol(e.to_string()))?;
        let forget = || {
            self.pending
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .remove(&id);
        };
        match tokio::time::timeout_at(deadline, self.out_tx.send(frame)).await {
            Ok(Ok(())) => {}
            // The writer task dropped the receiver — connection died.
            Ok(Err(_)) => {
                forget();
                return Err(AcpError::Closed);
            }
            Err(_) => {
                forget();
                return Err(AcpError::Transport(format!(
                    "request '{method}' timed out queueing for the agent"
                )));
            }
        }

        match tokio::time::timeout_at(deadline, rx).await {
            Ok(Ok(result)) => result,
            // The reader task dropped the sender — connection died.
            Ok(Err(_)) => Err(AcpError::Closed),
            Err(_) => {
                forget();
                Err(AcpError::Transport(format!("request '{method}' timed out")))
            }
        }
    }

    /// Send a JSON-RPC notification with a deadline covering both
    /// serialization and queueing. Notifications have no response, so a
    /// successful enqueue is the complete operation from the client's side.
    pub async fn notify(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<(), AcpError> {
        if let Some(error) = self.current_failure() {
            return Err(error);
        }
        let frame = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        match tokio::time::timeout(timeout, self.out_tx.send(frame)).await {
            Ok(Ok(())) => match self.current_failure() {
                Some(error) => Err(error),
                None => Ok(()),
            },
            Ok(Err(_)) => Err(AcpError::Closed),
            Err(_) => Err(AcpError::Transport(format!(
                "notification '{method}' timed out queueing for the agent"
            ))),
        }
    }
}

/// Choose an explicit allow option from a `session/request_permission`
/// request. If the agent offers no allow choice, cancel instead of inventing
/// an option id that was never advertised.
pub fn auto_approve_permission(params: &Value) -> Value {
    let parsed: RequestPermissionParams = serde_json::from_value(params.clone())
        .unwrap_or(RequestPermissionParams { options: vec![] });
    let chosen = parsed.options.iter().find(|o| {
        matches!(o.kind.as_deref(), Some("allow_once") | Some("allow_always"))
            || o.option_id.starts_with("allow")
    });
    match chosen {
        Some(option) => serde_json::json!({
            "outcome": { "outcome": "selected", "optionId": option.option_id }
        }),
        None => serde_json::json!({ "outcome": { "outcome": "cancelled" } }),
    }
}

fn validate_envelope(value: &Value) -> Result<(), AcpError> {
    let object = value
        .as_object()
        .ok_or_else(|| AcpError::Protocol("JSON-RPC batch entry must be an object".into()))?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(AcpError::Protocol(
            "JSON-RPC envelope must declare jsonrpc=2.0".into(),
        ));
    }
    if let Some(method) = object.get("method") {
        if method.as_str().is_none_or(str::is_empty) {
            return Err(AcpError::Protocol(
                "JSON-RPC method must be a non-empty string".into(),
            ));
        }
        if object.contains_key("result") || object.contains_key("error") {
            return Err(AcpError::Protocol(
                "JSON-RPC request cannot contain result or error".into(),
            ));
        }
        if let Some(id) = object.get("id") {
            if id.is_null() || !(id.is_string() || id.is_number()) {
                return Err(AcpError::Protocol(
                    "JSON-RPC request id must be a string or number".into(),
                ));
            }
        }
        return Ok(());
    }

    if object.get("id").and_then(Value::as_u64).is_none() {
        return Err(AcpError::Protocol(
            "JSON-RPC response id must match a non-negative client request id".into(),
        ));
    }
    let has_result = object.contains_key("result");
    let has_error = object.contains_key("error");
    if has_result == has_error {
        return Err(AcpError::Protocol(
            "JSON-RPC response must contain exactly one of result or error".into(),
        ));
    }
    if has_error && serde_json::from_value::<JsonRpcError>(object["error"].clone()).is_err() {
        return Err(AcpError::Protocol(
            "JSON-RPC response contains an invalid error object".into(),
        ));
    }
    Ok(())
}

fn dispatch_one(
    value: Value,
    pending: &Pending,
    notif_tx: &mpsc::Sender<SessionNotification>,
) -> Result<Option<Value>, AcpError> {
    validate_envelope(&value)?;
    match classify_inbound(&value) {
        Inbound::Response { id, result, error } => {
            if let Some(tx) = pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&id)
            {
                let resolved = match error {
                    Some(error) => Err(AcpError::Rpc {
                        code: error.code,
                        message: error.message,
                    }),
                    None => Ok(result.unwrap_or(Value::Null)),
                };
                let _ = tx.send(resolved);
            }
            Ok(None)
        }
        Inbound::Request { id, method, params } => {
            let response = if method == METHOD_REQUEST_PERMISSION {
                JsonRpcResponse::ok(id, auto_approve_permission(&params))
            } else {
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
            serde_json::to_value(response)
                .map(Some)
                .map_err(|error| AcpError::Protocol(error.to_string()))
        }
        Inbound::Notification { method, params } => {
            if method == METHOD_SESSION_UPDATE {
                let notification =
                    serde_json::from_value::<SessionNotification>(params).map_err(|error| {
                        AcpError::Protocol(format!("invalid session/update: {error}"))
                    })?;
                offer(notif_tx, notification, "session/update notification")?;
            }
            Ok(None)
        }
        Inbound::Unknown => Err(AcpError::Protocol(
            "unrecognized canonical JSON-RPC envelope".into(),
        )),
    }
}

/// Route one inbound JSON frame: resolve a response waiter, forward a
/// `session/update` notification, or reply to an agent → client
/// request.
///
/// Batch entries are dispatched in source order and request replies are
/// collected into one response array, matching JSON-RPC 2.0 and the official
/// ACP SDK transport. Queue overflow and non-canonical envelopes fail closed.
pub fn dispatch_inbound(
    value: Value,
    pending: &Pending,
    notif_tx: &mpsc::Sender<SessionNotification>,
    out_tx: &mpsc::Sender<Value>,
) -> Result<(), AcpError> {
    match value {
        Value::Array(entries) => {
            if entries.is_empty() {
                return Err(AcpError::Protocol(
                    "JSON-RPC batch must contain at least one entry".into(),
                ));
            }
            let mut replies = Vec::new();
            for entry in entries {
                if let Some(reply) = dispatch_one(entry, pending, notif_tx)? {
                    replies.push(reply);
                }
            }
            if !replies.is_empty() {
                offer(out_tx, Value::Array(replies), "outbound batch reply")?;
            }
            Ok(())
        }
        frame => {
            if let Some(reply) = dispatch_one(frame, pending, notif_tx)? {
                offer(out_tx, reply, "outbound reply")?;
            }
            Ok(())
        }
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
    fn permission_without_an_allow_option_is_cancelled() {
        let params = serde_json::json!({
            "options": [{ "optionId": "first", "kind": "custom" }]
        });
        let custom = auto_approve_permission(&params);
        assert_eq!(custom["outcome"]["outcome"], "cancelled");
        assert!(custom["outcome"].get("optionId").is_none());
        let empty = serde_json::json!({ "options": [] });
        assert_eq!(
            auto_approve_permission(&empty)["outcome"]["outcome"],
            "cancelled"
        );
    }

    #[tokio::test]
    async fn call_correlates_a_response() {
        let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let engine = JsonRpcEngine::new(out_tx);
        let (notif_tx, _notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending = engine.pending();
        let reply_tx = engine.out_tx();

        // Background "agent": read the request, echo a response.
        tokio::spawn(async move {
            let req = out_rx.recv().await.unwrap();
            let id = req["id"].as_u64().unwrap();
            let response = serde_json::json!({
                "jsonrpc": "2.0", "id": id, "result": { "pong": true }
            });
            dispatch_inbound(response, &pending, &notif_tx, &reply_tx).unwrap();
        });

        let result = engine
            .call("ping", serde_json::json!({}), Duration::from_secs(2))
            .await
            .unwrap();
        assert_eq!(result["pong"], true);
    }

    #[tokio::test]
    async fn call_surfaces_rpc_error() {
        let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let engine = JsonRpcEngine::new(out_tx);
        let (notif_tx, _notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending = engine.pending();
        let reply_tx = engine.out_tx();
        tokio::spawn(async move {
            let req = out_rx.recv().await.unwrap();
            let id = req["id"].as_u64().unwrap();
            let err = serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": -32000, "message": "boom" }
            });
            dispatch_inbound(err, &pending, &notif_tx, &reply_tx).unwrap();
        });
        let err = engine
            .call("fail", serde_json::json!({}), Duration::from_secs(2))
            .await
            .unwrap_err();
        assert!(matches!(err, AcpError::Rpc { code: -32000, .. }));
    }

    #[tokio::test]
    async fn notification_has_no_id_and_uses_canonical_envelope() {
        let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let engine = JsonRpcEngine::new(out_tx);
        engine
            .notify(
                "session/cancel",
                serde_json::json!({"sessionId":"s1"}),
                Duration::from_secs(1),
            )
            .await
            .unwrap();
        let frame = out_rx.recv().await.unwrap();
        assert_eq!(frame["jsonrpc"], "2.0");
        assert_eq!(frame["method"], "session/cancel");
        assert_eq!(frame["params"]["sessionId"], "s1");
        assert!(frame.get("id").is_none());
    }

    /// Build one `session/update` frame carrying `session` as its id.
    fn update_frame(session: &str) -> Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": session,
                "update": { "sessionUpdate": "agent_message_chunk",
                            "content": { "type": "text", "text": "spam" } }
            }
        })
    }

    #[tokio::test]
    async fn a_flooding_agent_cannot_grow_the_notification_queue_without_bound() {
        let (out_tx, _out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let (notif_tx, mut notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));

        // Filling the queue succeeds; the first excess update fails closed
        // without blocking or silently dropping a streamed delta.
        for index in 0..NOTIFICATION_CAPACITY {
            dispatch_inbound(
                update_frame(&format!("s{index}")),
                &pending,
                &notif_tx,
                &out_tx,
            )
            .unwrap();
        }
        let overflow =
            dispatch_inbound(update_frame("overflow"), &pending, &notif_tx, &out_tx).unwrap_err();
        assert!(overflow.to_string().contains("queue overflow"));

        let mut buffered = 0;
        while notif_rx.try_recv().is_ok() {
            buffered += 1;
        }
        assert_eq!(
            buffered, NOTIFICATION_CAPACITY,
            "queue must cap at its capacity, not grow to the flood size"
        );
    }

    #[tokio::test]
    async fn canonical_batch_responses_are_dispatched_in_source_order() {
        let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let engine = JsonRpcEngine::new(out_tx);
        let (notif_tx, _notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending = engine.pending();
        let reply_tx = engine.out_tx();

        tokio::spawn(async move {
            let first = out_rx.recv().await.unwrap();
            let second = out_rx.recv().await.unwrap();
            dispatch_inbound(
                serde_json::json!([
                    { "jsonrpc": "2.0", "id": first["id"], "result": "first" },
                    { "jsonrpc": "2.0", "id": second["id"], "result": "second" }
                ]),
                &pending,
                &notif_tx,
                &reply_tx,
            )
            .unwrap();
        });
        let (first, second) = tokio::join!(
            engine.call("first", Value::Null, Duration::from_secs(2)),
            engine.call("second", Value::Null, Duration::from_secs(2))
        );
        assert_eq!(first.unwrap(), "first");
        assert_eq!(second.unwrap(), "second");
    }

    #[tokio::test]
    async fn a_full_outbound_queue_never_blocks_the_reader() {
        // Permission auto-replies are produced by the reader task itself. If
        // the writer stalled, an awaited send here would deadlock the reader
        // against the queue it is the only producer for.
        let (out_tx, _out_rx) = mpsc::channel::<Value>(1);
        let (notif_tx, _notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let permission = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "session/request_permission",
            "params": { "options": [{ "optionId": "allow-1", "kind": "allow_once" }] }
        });
        dispatch_inbound(permission.clone(), &pending, &notif_tx, &out_tx).unwrap();
        let error = dispatch_inbound(permission, &pending, &notif_tx, &out_tx).unwrap_err();
        assert!(error.to_string().contains("queue overflow"));
    }

    #[tokio::test]
    async fn call_fails_fast_when_the_outbound_queue_is_closed() {
        let (out_tx, out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        drop(out_rx);
        let engine = JsonRpcEngine::new(out_tx);
        let err = engine
            .call("ping", serde_json::json!({}), Duration::from_secs(5))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AcpError::Closed),
            "expected Closed, got {err:?}"
        );
    }

    #[tokio::test]
    async fn notification_reaches_the_channel() {
        let (out_tx, _out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let (notif_tx, mut notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
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
        dispatch_inbound(note, &pending, &notif_tx, &out_tx).unwrap();
        let received = notif_rx.recv().await.unwrap();
        assert_eq!(received.session_id.as_deref(), Some("s1"));
    }

    #[tokio::test]
    async fn batch_permission_replies_are_emitted_as_one_response_array() {
        let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let (notif_tx, _notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        dispatch_inbound(
            serde_json::json!([
                {
                    "jsonrpc": "2.0", "id": "p1",
                    "method": "session/request_permission",
                    "params": { "options": [{ "optionId": "yes", "kind": "allow_once" }] }
                },
                {
                    "jsonrpc": "2.0", "id": "p2",
                    "method": "session/request_permission",
                    "params": { "options": [] }
                }
            ]),
            &pending,
            &notif_tx,
            &out_tx,
        )
        .unwrap();
        let replies = out_rx.recv().await.unwrap();
        assert_eq!(replies.as_array().unwrap().len(), 2);
        assert_eq!(replies[0]["result"]["outcome"]["optionId"], "yes");
        assert_eq!(replies[1]["result"]["outcome"]["outcome"], "cancelled");
    }

    #[test]
    fn noncanonical_and_empty_batch_frames_are_rejected() {
        let (out_tx, _out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let (notif_tx, _notif_rx) = mpsc::channel(NOTIFICATION_CAPACITY);
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        for invalid in [
            serde_json::json!([]),
            serde_json::json!({ "id": 1, "result": null }),
            serde_json::json!({ "jsonrpc": "1.0", "id": 1, "result": null }),
            serde_json::json!({ "jsonrpc": "2.0", "id": 1, "result": null, "error": null }),
        ] {
            assert!(dispatch_inbound(invalid, &pending, &notif_tx, &out_tx).is_err());
        }
    }
}
