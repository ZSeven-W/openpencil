//! Cancellable built-in provider transport for native mobile code generation.
//!
//! Mobile codegen deliberately accepts only the model row the user selected
//! from a ready, saved built-in provider. Phones cannot spawn desktop CLI or
//! ACP agents, and silently falling back to one would send paid work to a
//! different model than the Code panel names.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use op_ai::chat_provider::{
    ChatDelta, ChatHistoryRole, ChatProvider, ChatRequest, StopReason, ThinkingMode,
};
use op_ai::chat_sse::{parse_anthropic_sse_data, parse_openai_sse_data, provider_endpoint};
use op_chat_agent::backoff::{
    apply_reasoning_wire_control, apply_reasoning_wire_control_anthropic, builtin_http_client,
    builtin_http_min_gap, send_with_backoff, BUILTIN_HTTP_MAX_RETRIES,
};
use op_chat_agent::chat_builtin_http::BuiltinHttpError;
use op_editor_core::{BuiltinAgentKind, EditorState};
use serde_json::{json, Value};
use tokio::task::AbortHandle;
use zeroize::Zeroizing;

use crate::editor_chat_turn::{pump_sse_response, MobileChatTurnError};

const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Configuration failures resolved synchronously on the engine thread.
/// These are rendered into `CodegenState::error` only at the host boundary.
#[derive(Debug)]
pub(crate) enum MobileBuiltinProviderError {
    NoSelectedModel,
    ExternalModel { label: String },
    MissingProvider,
    ProviderNotReady { label: String },
    InvalidCatalogModel,
    ModelNotSaved { label: String, model: String },
    InvalidEndpoint { message: String },
    UnsupportedEndpointScheme { scheme: String },
    EndpointMissingHost,
    EndpointHasUserInfo,
    EndpointHasQueryOrFragment,
    ClientUnavailable { message: String },
}

impl fmt::Display for MobileBuiltinProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSelectedModel => formatter.write_str(
                "Select a ready built-in model before generating code on mobile",
            ),
            Self::ExternalModel { label } => write!(
                formatter,
                "Code generation on mobile requires a built-in API-key model; {label} is not available",
            ),
            Self::MissingProvider => formatter.write_str(
                "The selected built-in provider is no longer configured. Choose another model in the chat model picker",
            ),
            Self::ProviderNotReady { label } => write!(
                formatter,
                "Connect {label} and save its API key and model before generating code",
            ),
            Self::InvalidCatalogModel => formatter.write_str(
                "The selected built-in model entry is invalid. Choose the model again",
            ),
            Self::ModelNotSaved { label, model } => write!(
                formatter,
                "Model {model} is no longer saved for {label}. Choose a saved model before generating code",
            ),
            Self::InvalidEndpoint { message } => {
                write!(formatter, "The selected provider endpoint is invalid: {message}")
            }
            Self::UnsupportedEndpointScheme { scheme } => write!(
                formatter,
                "The selected provider endpoint uses unsupported scheme {scheme:?}; use http or https",
            ),
            Self::EndpointMissingHost => {
                formatter.write_str("The selected provider endpoint has no host")
            }
            Self::EndpointHasUserInfo => formatter.write_str(
                "The selected provider endpoint must not contain a username or password",
            ),
            Self::EndpointHasQueryOrFragment => formatter.write_str(
                "The selected provider endpoint must not contain a query string or fragment",
            ),
            Self::ClientUnavailable { message } => write!(
                formatter,
                "Provider HTTP client is unavailable: {message}",
            ),
        }
    }
}

impl std::error::Error for MobileBuiltinProviderError {}

/// Plain, tool-free provider used by the shared code-generation pipeline.
/// The credential is a zeroizing snapshot and never appears in `Debug`.
pub(crate) struct MobileBuiltinProvider {
    kind: BuiltinAgentKind,
    api_key: Zeroizing<String>,
    model: String,
    base_url: String,
    label: String,
    client: reqwest::Client,
}

impl MobileBuiltinProvider {
    /// Resolve exactly the selected catalog row and matching saved config.
    pub(crate) fn from_selected_model(
        state: &EditorState,
    ) -> Result<Self, MobileBuiltinProviderError> {
        let entry = state
            .chat
            .selected_model_entry()
            .ok_or(MobileBuiltinProviderError::NoSelectedModel)?;
        let provider_id = entry.builtin_provider_id.as_deref().ok_or_else(|| {
            MobileBuiltinProviderError::ExternalModel {
                label: entry.display_name.clone(),
            }
        })?;
        let model = entry
            .builtin_model_id()
            .ok_or(MobileBuiltinProviderError::InvalidCatalogModel)?;
        let config = state
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .find(|candidate| candidate.id == provider_id)
            .ok_or(MobileBuiltinProviderError::MissingProvider)?;
        let label = if config.display_name.trim().is_empty() {
            entry
                .builtin_provider_display_name
                .as_deref()
                .unwrap_or(model)
                .to_string()
        } else {
            config.display_name.trim().to_string()
        };
        if !config.ready() {
            return Err(MobileBuiltinProviderError::ProviderNotReady { label });
        }
        if !config.has_model(model) {
            return Err(MobileBuiltinProviderError::ModelNotSaved {
                label,
                model: model.to_string(),
            });
        }

        let configured_base = if config.base_url.trim().is_empty() {
            config.kind.default_base_url()
        } else {
            config.base_url.trim()
        };
        let base_url = normalize_provider_base_url(configured_base)?;
        let client = builtin_http_client().map_err(|error| {
            MobileBuiltinProviderError::ClientUnavailable {
                message: error.to_string(),
            }
        })?;
        Ok(Self {
            kind: config.kind,
            api_key: Zeroizing::new(config.api_key.trim().to_string()),
            model: model.to_string(),
            base_url,
            label,
            client,
        })
    }

    fn send_inner(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        if cancel.load(Ordering::Acquire) {
            return Box::new(std::iter::empty());
        }
        if !request.attachments.is_empty() {
            return immediate_error("Mobile code generation accepts text-only model requests");
        }

        let turn = MobileProviderTurn {
            kind: self.kind,
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            base_url: self.base_url.clone(),
            client: self.client.clone(),
            request,
        };
        let (tx, rx) = mpsc::channel();
        let task = op_chat_agent::runtime::shared_runtime().spawn(run_turn(turn, tx));
        Box::new(CancellableRecvIter::new(rx, cancel, task.abort_handle()))
    }
}

impl fmt::Debug for MobileBuiltinProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MobileBuiltinProvider")
            .field("kind", &self.kind)
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("label", &self.label)
            .finish()
    }
}

impl ChatProvider for MobileBuiltinProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, Arc::new(AtomicBool::new(false)))
    }

    fn supports_cancellable_send(&self) -> bool {
        true
    }

    fn supports_evidence_only_send(&self) -> bool {
        true
    }

    fn send_cancellable(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, cancel)
    }
}

struct MobileProviderTurn {
    kind: BuiltinAgentKind,
    api_key: Zeroizing<String>,
    model: String,
    base_url: String,
    client: reqwest::Client,
    request: ChatRequest,
}

async fn run_turn(turn: MobileProviderTurn, tx: Sender<ChatDelta>) {
    let outcome = match turn.kind {
        BuiltinAgentKind::Anthropic => run_anthropic_request(&turn, &tx).await,
        BuiltinAgentKind::OpenAiCompat => run_openai_request(&turn, &tx).await,
    };
    match outcome {
        Ok(true) => {}
        Ok(false) => {
            let _ = tx.send(ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            });
        }
        Err(error) => {
            let _ = tx.send(ChatDelta::Error(error.to_string()));
            let _ = tx.send(ChatDelta::Done {
                stop_reason: StopReason::Aborted,
            });
        }
    }
}

async fn run_openai_request(
    turn: &MobileProviderTurn,
    tx: &Sender<ChatDelta>,
) -> Result<bool, BuiltinHttpError> {
    let url = provider_endpoint(&turn.base_url, "/chat/completions");
    let body = openai_request_body(&turn.request, &turn.model);
    let response = send_with_backoff(
        "openai-compatible",
        &url,
        BUILTIN_HTTP_MAX_RETRIES,
        builtin_http_min_gap(),
        || {
            turn.client
                .post(&url)
                .bearer_auth(turn.api_key.as_str())
                .json(&body)
        },
    )
    .await?;
    pump_shared_sse(response, tx, parse_openai_sse_data).await
}

async fn run_anthropic_request(
    turn: &MobileProviderTurn,
    tx: &Sender<ChatDelta>,
) -> Result<bool, BuiltinHttpError> {
    let url = provider_endpoint(&turn.base_url, "/v1/messages");
    let body = anthropic_request_body(&turn.request, &turn.model);
    let response = send_with_backoff(
        "anthropic",
        &url,
        BUILTIN_HTTP_MAX_RETRIES,
        builtin_http_min_gap(),
        || {
            turn.client
                .post(&url)
                .header("x-api-key", turn.api_key.as_str())
                .header("anthropic-version", "2023-06-01")
                .json(&body)
        },
    )
    .await?;
    pump_shared_sse(response, tx, parse_anthropic_sse_data).await
}

fn history_messages(request: &ChatRequest) -> Vec<Value> {
    let mut messages = Vec::with_capacity(request.history.len() + 1);
    messages.extend(
        request
            .history
            .iter()
            .map(|(role, text)| json!({ "role": chat_role(*role), "content": text })),
    );
    messages
}

fn openai_request_body(request: &ChatRequest, model: &str) -> Value {
    let mut messages = history_messages(request);
    if !request.system_prompt.trim().is_empty() {
        messages.insert(
            0,
            json!({ "role": "system", "content": request.system_prompt }),
        );
    }
    messages.push(json!({ "role": "user", "content": request.user_message }));
    let mut body = json!({
        "model": model,
        "stream": true,
        "max_tokens": request.max_output_tokens.max(1),
        "messages": messages,
    });
    apply_reasoning_wire_control(&mut body, model, reduce_reasoning(request));
    body
}

fn anthropic_request_body(request: &ChatRequest, model: &str) -> Value {
    let mut messages = history_messages(request);
    messages.push(json!({ "role": "user", "content": request.user_message }));
    let mut body = json!({
        "model": model,
        "max_tokens": request.max_output_tokens.max(1),
        "stream": true,
        "messages": messages,
    });
    if !request.system_prompt.trim().is_empty() {
        body.as_object_mut()
            .expect("anthropic request body is an object")
            .insert("system".into(), json!(request.system_prompt));
    }
    apply_reasoning_wire_control_anthropic(&mut body, model, reduce_reasoning(request));
    body
}

fn chat_role(role: ChatHistoryRole) -> &'static str {
    role.as_str()
}

fn reduce_reasoning(request: &ChatRequest) -> bool {
    request.thinking == ThinkingMode::Disabled
}

async fn pump_shared_sse(
    response: reqwest::Response,
    tx: &Sender<ChatDelta>,
    parse: fn(&str) -> Option<ChatDelta>,
) -> Result<bool, BuiltinHttpError> {
    pump_sse_response(response, tx, parse)
        .await
        .map_err(|error| match error {
            MobileChatTurnError::SseStream { message } => BuiltinHttpError::SseStream { message },
            other => BuiltinHttpError::SseStream {
                message: other.to_string(),
            },
        })
}

fn normalize_provider_base_url(base_url: &str) -> Result<String, MobileBuiltinProviderError> {
    let url = reqwest::Url::parse(base_url).map_err(|error| {
        MobileBuiltinProviderError::InvalidEndpoint {
            message: error.to_string(),
        }
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(MobileBuiltinProviderError::UnsupportedEndpointScheme {
            scheme: url.scheme().to_string(),
        });
    }
    if url.host_str().is_none() {
        return Err(MobileBuiltinProviderError::EndpointMissingHost);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(MobileBuiltinProviderError::EndpointHasUserInfo);
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(MobileBuiltinProviderError::EndpointHasQueryOrFragment);
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn immediate_error(message: &str) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
    Box::new(
        [
            ChatDelta::Error(message.to_string()),
            ChatDelta::Done {
                stop_reason: StopReason::Aborted,
            },
        ]
        .into_iter(),
    )
}

/// Sync bridge for `ChatProvider`, with a hard abort for a silent HTTP task.
struct CancellableRecvIter<T> {
    rx: Receiver<T>,
    cancel: Arc<AtomicBool>,
    abort: Option<AbortHandle>,
}

impl<T> CancellableRecvIter<T> {
    fn new(rx: Receiver<T>, cancel: Arc<AtomicBool>, abort: AbortHandle) -> Self {
        Self {
            rx,
            cancel,
            abort: Some(abort),
        }
    }

    fn abort(&mut self) {
        if let Some(abort) = self.abort.take() {
            abort.abort();
        }
    }
}

impl<T> Iterator for CancellableRecvIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.cancel.load(Ordering::Acquire) {
                self.abort();
                return None;
            }
            match self.rx.recv_timeout(CANCEL_POLL_INTERVAL) {
                Ok(value) => {
                    if self.cancel.load(Ordering::Acquire) {
                        self.abort();
                        return None;
                    }
                    return Some(value);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.abort.take();
                    return None;
                }
            }
        }
    }
}

impl<T> Drop for CancellableRecvIter<T> {
    fn drop(&mut self) {
        self.abort();
    }
}

#[cfg(test)]
#[path = "editor_builtin_provider_tests.rs"]
mod tests;
