//! Real `ChatProvider` impls — the desktop-side companion to the
//! abstraction in `op_ai::chat_provider`. Shell-core
//! intentionally stays wasm32-clean (no tokio / reqwest / process), so
//! transports live here on the native binary.
//!
//! Backends shipped from this module:
//!
//! - [`BuiltInProvider`] — wraps `agent::QueryEngine` in-process. The
//!   default `BuiltIn` kind that runs through one of agent-rs's
//!   Provider impls (Anthropic today; OpenAI-compat / Ollama land as
//!   their feature flags flip on).
//! - Subprocess(CliName) / HttpServer(CliName) / Acp bridges land in
//!   follow-up commits — they need process-spawn + line-delim JSON-RPC
//!   plumbing that's its own self-contained module per transport.
//!
//! Async → sync bridge: agent-rs is built around an async
//! `EventStream`, but `ChatProvider::send` returns a plain
//! `Iterator<Item = ChatDelta>` so widget code stays free of futures
//! plumbing. The bridge spawns a tokio task on a process-wide runtime
//! that pumps `Event`s into a `std::sync::mpsc::channel`; the returned
//! iterator drains the receiver until it goes idle.

use std::sync::{Arc, OnceLock};

use agent::abort::AbortController;
use agent::provider::Provider;
use agent::query::QueryEngine;
use agent::stream::Event;
use futures::StreamExt;
use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, StopReason,
};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;

/// Process-wide tokio runtime used for every BuiltIn chat turn. We
/// own a single multi-thread runtime instead of spinning one up per
/// provider so abort controllers + reqwest connection pools stay
/// shared. Initialized lazily on first chat send so cold startup
/// (open file menu, draw chrome) doesn't pay the spawn cost.
pub(crate) fn shared_runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .enable_all()
            .thread_name("op-chat")
            .build()
            .expect("chat runtime build")
    })
}

/// `ChatProvider` impl that drives `agent::QueryEngine` directly. The
/// engine is built once at construct time and reused across every
/// `send` call so its `MessageStore` (agent-rs's internal conversation
/// history) persists across turns. This is critical: the LLM sees the
/// full multi-turn history, not just each user message in isolation
/// (codex BLOCK — per-turn engine rebuild discarded history).
///
/// Trade-off: `ChatRequest.system_prompt` + `max_output_tokens` are
/// **not** honored per-turn — they're set once via the constructor
/// and any per-request override is ignored. The settings modal carries
/// these fields, so per-turn overrides aren't a real use case (the TS
/// app behaves the same way: system prompt + max tokens are settings,
/// not per-message inputs). If a future flow needs per-turn override,
/// the right fix lands in agent-rs (a `QueryEngine::with_overrides`
/// method on a turn-scoped builder), not by tearing down the store.
pub struct BuiltInProvider {
    engine: Arc<QueryEngine>,
    label: String,
}

impl BuiltInProvider {
    /// Build a BuiltIn provider from a hand-rolled `Provider` impl.
    /// Used by tests + future settings tabs that need a custom backend
    /// (e.g., a local proxy or a mocked HTTP server). `system_prompt`
    /// + `max_output_tokens` apply to every turn for the lifetime of
    /// the provider; rebuild a new `BuiltInProvider` to change them.
    #[allow(dead_code)]
    pub fn from_provider(
        provider: Arc<dyn Provider>,
        model: impl Into<String>,
        system_prompt: Option<String>,
        max_output_tokens: u32,
        label: impl Into<String>,
    ) -> Self {
        let mut engine = QueryEngine::new(provider, model)
            .with_max_output_tokens(max_output_tokens.max(1));
        if let Some(sys) = system_prompt {
            engine = engine.with_system(sys);
        }
        Self {
            engine: Arc::new(engine),
            label: label.into(),
        }
    }
}

impl ChatProvider for BuiltInProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn send(
        &self,
        request: ChatRequest,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        // Engine handle is shared across turns so the MessageStore
        // accumulates history; `Arc` clone is cheap. Per-turn
        // overrides on `request.system_prompt` / `max_output_tokens`
        // are documented as no-ops (see struct comment).
        let _ = request.system_prompt;
        let _ = request.max_output_tokens;
        let engine = self.engine.clone();
        let abort = AbortController::new();
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            let stream = match engine.run(request.user_message, abort).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(ChatDelta::Error(e.to_string())).await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    return;
                }
            };
            let mut stream = stream;
            let mut emitted_done = false;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(Event::TextDelta { delta }) => {
                        if tx.send(ChatDelta::TextDelta(delta)).await.is_err() {
                            return;
                        }
                    }
                    Ok(Event::Thinking { delta }) => {
                        if tx.send(ChatDelta::Thinking(delta)).await.is_err() {
                            return;
                        }
                    }
                    Ok(Event::ToolUse { name, input, .. }) => {
                        let args = input.to_string();
                        if tx
                            .send(ChatDelta::ToolUse { name, args })
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                    Ok(Event::Result { data }) => {
                        let reason = map_stop_reason(data.stop_reason.as_deref());
                        let _ = tx.send(ChatDelta::Done { stop_reason: reason }).await;
                        emitted_done = true;
                        break;
                    }
                    Ok(Event::Error { code, message }) => {
                        let _ = tx
                            .send(ChatDelta::Error(format!("{code}: {message}")))
                            .await;
                        // Always send a terminal `Done` after `Error`
                        // so consumers can distinguish a completed
                        // error path from a silently-dropped channel
                        // (codex CONCERN 2).
                        let _ = tx
                            .send(ChatDelta::Done {
                                stop_reason: StopReason::Aborted,
                            })
                            .await;
                        emitted_done = true;
                        break;
                    }
                    Ok(_) => {} // ToolResult / Usage / Notice / Unknown — silent
                    Err(e) => {
                        let _ = tx.send(ChatDelta::Error(e.to_string())).await;
                        let _ = tx
                            .send(ChatDelta::Done {
                                stop_reason: StopReason::Aborted,
                            })
                            .await;
                        emitted_done = true;
                        break;
                    }
                }
            }
            if !emitted_done {
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::EndTurn,
                    })
                    .await;
            }
        });
        Box::new(BlockingRecvIter::new(rx))
    }
}

/// Adapter that turns a `tokio::sync::mpsc::Receiver<ChatDelta>` into
/// a sync `Iterator<Item = ChatDelta>`. The receiver's
/// `blocking_recv` blocks the calling thread until a value arrives or
/// the channel closes — exactly the contract `ChatProvider::send`'s
/// iterator return needs. Sharing this helper across both BuiltIn +
/// Subprocess (and the future HttpServer / Acp) keeps the async ↔
/// sync bridge in one place.
pub(crate) struct BlockingRecvIter<T> {
    rx: mpsc::Receiver<T>,
}

impl<T> BlockingRecvIter<T> {
    pub(crate) fn new(rx: mpsc::Receiver<T>) -> Self {
        Self { rx }
    }
}

impl<T> Iterator for BlockingRecvIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.rx.blocking_recv()
    }
}

fn map_stop_reason(s: Option<&str>) -> StopReason {
    match s.unwrap_or("") {
        "end_turn" | "stop_sequence" => StopReason::EndTurn,
        "max_tokens" => StopReason::MaxTokens,
        "tool_use" => StopReason::ToolUse,
        "aborted" | "user_abort" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent::error::AgentError;
    use agent::provider::{Provider, ProviderCapabilities, StreamRequest};
    use agent::stream::{Event, EventStream, ResultData};
    use async_trait::async_trait;
    use futures::stream;

    /// Test double Provider — emits a fixed Event script.
    #[derive(Debug)]
    struct ScriptedProvider {
        script: Vec<Event>,
    }

    #[async_trait]
    impl Provider for ScriptedProvider {
        fn id(&self) -> &str {
            "scripted"
        }
        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
        async fn stream(
            &self,
            _req: StreamRequest,
            _abort: AbortController,
        ) -> Result<Box<dyn EventStream>, AgentError> {
            let items: Vec<Result<Event, AgentError>> =
                self.script.iter().cloned().map(Ok).collect();
            Ok(Box::new(stream::iter(items)))
        }
    }

    #[test]
    fn builtin_provider_streams_text_deltas_through_iterator() {
        let provider = Arc::new(ScriptedProvider {
            script: vec![
                Event::TextDelta {
                    delta: "Hello".into(),
                },
                Event::TextDelta {
                    delta: ", world".into(),
                },
                Event::Result {
                    data: ResultData {
                        stop_reason: Some("end_turn".into()),
                        ..Default::default()
                    },
                },
            ],
        });
        let bridge = BuiltInProvider::from_provider(
            provider,
            "test-model",
            None,
            1024,
            "scripted",
        );
        let req = ChatRequest {
            system_prompt: String::new(),
            user_message: "hi".into(),
            max_output_tokens: 1024,
        };
        let deltas: Vec<ChatDelta> = bridge.send(req).collect();
        assert!(
            matches!(deltas.first(), Some(ChatDelta::TextDelta(s)) if s == "Hello"),
            "first delta should be text 'Hello', got {:?}",
            deltas.first()
        );
        let done = deltas.last().unwrap();
        assert!(
            matches!(done, ChatDelta::Done { stop_reason: StopReason::EndTurn }),
            "last delta should be Done EndTurn, got {:?}",
            done
        );
    }

    #[test]
    fn builtin_provider_surfaces_event_error() {
        let provider = Arc::new(ScriptedProvider {
            script: vec![Event::Error {
                code: "rate_limited".into(),
                message: "slow down".into(),
            }],
        });
        let bridge = BuiltInProvider::from_provider(
            provider,
            "m",
            None,
            64,
            "err",
        );
        let deltas: Vec<ChatDelta> = bridge
            .send(ChatRequest {
                system_prompt: String::new(),
                user_message: "x".into(),
                max_output_tokens: 64,
            })
            .collect();
        assert!(
            deltas
                .iter()
                .any(|d| matches!(d, ChatDelta::Error(s) if s.contains("rate_limited") && s.contains("slow down"))),
            "expected Error delta carrying code + message, got {:?}",
            deltas
        );
    }

    #[test]
    fn map_stop_reason_table() {
        assert!(matches!(
            map_stop_reason(Some("end_turn")),
            StopReason::EndTurn
        ));
        assert!(matches!(
            map_stop_reason(Some("max_tokens")),
            StopReason::MaxTokens
        ));
        assert!(matches!(
            map_stop_reason(Some("tool_use")),
            StopReason::ToolUse
        ));
        assert!(matches!(
            map_stop_reason(Some("aborted")),
            StopReason::Aborted
        ));
        assert!(matches!(
            map_stop_reason(Some("user_abort")),
            StopReason::Aborted
        ));
        // Unknown values map to EndTurn (sensible default — turn over).
        assert!(matches!(
            map_stop_reason(Some("rocket_launch")),
            StopReason::EndTurn
        ));
        assert!(matches!(map_stop_reason(None), StopReason::EndTurn));
    }
}
