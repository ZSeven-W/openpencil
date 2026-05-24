//! Desktop `LlmClient` adapter for `op_orchestrator`.
//!
//! [`DesktopLlmClient`] bridges `agent::Provider` (the host's LLM
//! interface) to `op_orchestrator::LlmClient` (the orchestrator's
//! transport-free seam). Each `call` spins up a fresh `QueryEngine` so
//! planner / sub-agent / cleanup turns get independent context —
//! distinct from `BuiltInProvider` in `chat_runtime.rs`, which shares
//! one engine across a chat thread to accumulate user history.
//!
//! Live caller: [`design_session::DesignSession::start`], which owns
//! the worker thread that drives `Orchestrator::run` against this
//! adapter + a `RemoteDocSink`. Previous `DesktopDocSink` + entry-point
//! `run_design_request` were removed once `DesignSession` replaced
//! them with the actor-channel model (no UI freeze; ID-remapped state
//! mirrored back to the worker via ack snapshots).

use std::sync::Arc;

use agent::abort::AbortController;
use agent::provider::Provider;
use agent::query::QueryEngine;
use agent::stream::Event;
use futures::channel::mpsc;
use futures::StreamExt;
use op_orchestrator::{CallRequest, LlmChunk, LlmClient, LlmError};

/// `LlmClient` 的 desktop 实现。每次 `call` 新建一个 `QueryEngine`
/// —— 规划与各 sub-agent 因此拿到互相隔离的对话上下文(不复用
/// `BuiltInProvider` 那个累积历史的共享引擎)。
pub struct DesktopLlmClient {
    provider: Arc<dyn Provider>,
    /// 缺省模型 —— `CallRequest.model` 为 `None` 时用它。
    default_model: String,
}

impl DesktopLlmClient {
    pub fn new(provider: Arc<dyn Provider>, default_model: impl Into<String>) -> Self {
        Self {
            provider,
            default_model: default_model.into(),
        }
    }
}

impl LlmClient for DesktopLlmClient {
    fn call(
        &self,
        req: CallRequest,
    ) -> futures::stream::BoxStream<'static, Result<LlmChunk, LlmError>> {
        let (tx, rx) = mpsc::unbounded::<Result<LlmChunk, LlmError>>();

        // 调用前已中止 —— 直接给一个 aborted 错误流。
        if req.abort.is_set() {
            let _ = tx.unbounded_send(Err(LlmError {
                message: "aborted".into(),
                aborted: true,
            }));
            return Box::pin(rx);
        }

        let provider = self.provider.clone();
        let model = req
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());
        let system = req.system_prompt.clone();
        let user = req.user_prompt.clone();

        crate::chat_runtime::shared_runtime().spawn(async move {
            let engine = QueryEngine::new(provider, model).with_system(system);
            let abort = AbortController::new();
            let stream = match engine.run(user, abort).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.unbounded_send(Err(LlmError {
                        message: e.to_string(),
                        aborted: false,
                    }));
                    return;
                }
            };
            let mut stream = stream;
            while let Some(item) = stream.next().await {
                let sent = match item {
                    Ok(Event::TextDelta { delta }) => tx.unbounded_send(Ok(LlmChunk::Text(delta))),
                    Ok(Event::Thinking { delta }) => {
                        tx.unbounded_send(Ok(LlmChunk::Thinking(delta)))
                    }
                    Ok(Event::Result { .. }) => break,
                    Ok(Event::Error { code, message }) => tx.unbounded_send(Err(LlmError {
                        message: format!("{code}: {message}"),
                        aborted: false,
                    })),
                    // ToolUse / ToolResult / Usage / 其它 —— 编排器
                    // 只要文本,静默跳过。
                    Ok(_) => Ok(()),
                    Err(e) => tx.unbounded_send(Err(LlmError {
                        message: e.to_string(),
                        aborted: false,
                    })),
                };
                if sent.is_err() {
                    break; // 接收端已丢弃
                }
            }
        });

        Box::pin(rx)
    }
}
