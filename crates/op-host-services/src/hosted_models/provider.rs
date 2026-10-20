use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, StopReason};
use op_editor_core::{HostedModelRoute, HostedProtocol};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use super::{HostedRuntimeError, Session};

pub(super) struct HostedProvider {
    pub session: Session,
    pub epoch: u64,
    pub subject: String,
    pub route: HostedModelRoute,
}

impl ChatProvider for HostedProvider {
    fn provider_label(&self) -> &str {
        self.route.display_name()
    }
    fn hosted_session_epoch(&self) -> Option<u64> {
        Some(self.session.auth_epoch)
    }
    fn supports_cancellable_send(&self) -> bool {
        true
    }
    fn supports_evidence_only_send(&self) -> bool {
        true
    }
    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_cancellable(request, Arc::new(AtomicBool::new(false)))
    }
    fn send_cancellable(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        if cancel.load(Ordering::Acquire) {
            return Box::new(
                [ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                }]
                .into_iter(),
            );
        }
        let session = self.session.clone();
        let epoch = self.epoch;
        let subject = self.subject.clone();
        let route = self.route.clone();
        let (tx, rx) = mpsc::channel(64);
        let worker_cancel = cancel.clone();
        let task = crate::chat_runtime::shared_runtime().spawn(async move {
            let operation = async {
                if worker_cancel.load(Ordering::Acquire) {
                    return Err(HostedRuntimeError::Cancelled);
                }
                let access = super::access_for(&session, epoch).await?;
                if access.subject() != subject {
                    return Err(HostedRuntimeError::SessionChanged);
                }
                super::check_session(&session, epoch)?;
                // Catalog membership is checked again immediately before the paid request.
                {
                    let state = super::STATE.lock().unwrap_or_else(|p| p.into_inner());
                    if !state.routes.iter().any(|candidate| candidate == &route) {
                        return Err(HostedRuntimeError::RouteUnavailable);
                    }
                }
                let (path, body) = request_body(&route, request)?;
                if worker_cancel.load(Ordering::Acquire) {
                    return Err(HostedRuntimeError::Cancelled);
                }
                let client = super::client()?;
                let mut builder = client
                    .post(format!("{}{path}", access.gateway_base_url()))
                    .bearer_auth(access.bearer())
                    .json(&body);
                if route.protocol() == HostedProtocol::AnthropicMessages {
                    builder = builder.header("anthropic-version", "2023-06-01");
                }
                let response = builder
                    .send()
                    .await
                    .map_err(|_| HostedRuntimeError::Network)?;
                if !response.status().is_success() {
                    return Err(HostedRuntimeError::GatewayRejected);
                }
                super::stream::pump(response, &tx, route.protocol()).await
            };
            let result = tokio::select! {
                result = operation => result,
                _ = tx.closed() => return,
                error = watch_session(&session, epoch) => Err(error),
                _ = watch_cancel(&worker_cancel) => Err(HostedRuntimeError::Cancelled),
            };
            match result {
                Ok(true) => {}
                Ok(false) => {
                    let _ = tx
                        .send(ChatDelta::Error(
                            "Hosted gateway stream ended before completion".into(),
                        ))
                        .await;
                }
                Err(error) => {
                    let _ = tx.send(ChatDelta::Error(error.to_string())).await;
                }
            }
        });
        Box::new(crate::chat_runtime::BlockingRecvIter::cancellable(
            rx, cancel, task,
        ))
    }
}

async fn watch_cancel(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

pub(super) async fn watch_session(session: &Session, epoch: u64) -> HostedRuntimeError {
    loop {
        if let Err(error) = super::check_session(session, epoch) {
            return error;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

pub(super) fn request_body(
    route: &HostedModelRoute,
    request: ChatRequest,
) -> Result<(&'static str, Value), HostedRuntimeError> {
    if !request.attachments.is_empty() {
        return Err(HostedRuntimeError::UnsupportedAttachments);
    }
    let mut messages: Vec<_> = request
        .history
        .iter()
        .map(|(role, content)| json!({"role":role.as_str(),"content":content}))
        .collect();
    messages.push(json!({ "role": "user", "content": request.user_message }));
    let mut body = json!({ "model": route.model_id(), "stream": true,
        "max_tokens": request.max_output_tokens.clamp(1, 65536), "messages": messages });
    let path = match route.protocol() {
        HostedProtocol::AnthropicMessages => {
            if !request.system_prompt.trim().is_empty() {
                body["system"] = json!(request.system_prompt);
            }
            "/messages"
        }
        HostedProtocol::OpenAiCompatible => {
            body["stream_options"] = json!({"include_usage": true});
            if !request.system_prompt.trim().is_empty() {
                body["messages"]
                    .as_array_mut()
                    .unwrap()
                    .insert(0, json!({"role":"system","content":request.system_prompt}));
            }
            crate::chat_builtin_http::apply_reasoning_wire_control(
                &mut body,
                route.model_id(),
                request.thinking == op_ai::chat_provider::ThinkingMode::Disabled,
            );
            "/chat/completions"
        }
        HostedProtocol::OpenAiResponses => return Err(HostedRuntimeError::UnsupportedProtocol),
    };
    Ok((path, body))
}
