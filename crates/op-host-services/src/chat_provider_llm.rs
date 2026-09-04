//! `ChatProvider` → `LlmClient` adapter.
//!
//! Lets the orchestrator reuse whichever chat-panel agent the user
//! already selected (Claude Code / Copilot / Antigravity / …) as its LLM
//! transport, instead of forcing a separate Anthropic API key.
//!
//! The CLI agents (`ClaudeCodeProvider`, `CopilotProvider`,
//! `SubprocessProvider`) manage their own auth — `claude` is logged in
//! by the user, Copilot rides GitHub auth, and Antigravity manages its
//! Google account.
//! `agent::Provider` (the `QueryEngine`-facing trait) is a different
//! shape and only `AnthropicProvider` implements it, hence the original
//! Anthropic-key requirement. This adapter eliminates that
//! requirement by turning any `ChatProvider` into the `LlmClient` the
//! orchestrator wants.
//!
//! Each `LlmClient::call` spawns one `std::thread` that drains
//! `provider.send_cancellable(req, abort)` (a *blocking* iterator) into a futures mpsc
//! channel; the returned `BoxStream` is the receive half. This is the
//! same async↔sync bridge `BlockingRecvIter` uses in the opposite
//! direction in `chat_runtime.rs`.

use std::sync::Arc;
use std::thread;

use futures::channel::mpsc;
use futures::stream::BoxStream;
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, ThinkingMode};
use op_orchestrator::{CallRequest, LlmChunk, LlmClient, LlmError};

/// Wraps a `ChatProvider` so the orchestrator can call it as an
/// `LlmClient`. `Arc` so multiple concurrent `call()`s can share the
/// same provider (orchestrator may run planner + sub-agents in
/// parallel under `concurrency > 1`).
pub struct ChatProviderLlmClient {
    provider: Arc<dyn ChatProvider>,
    /// Model id the user picked in the chat model picker, forwarded
    /// into every `ChatRequest` this adapter builds so design turns
    /// honor the selection the same way chat turns do. `None` keeps
    /// the CLI's own default.
    model: Option<String>,
}

impl ChatProviderLlmClient {
    pub fn new(provider: Arc<dyn ChatProvider>) -> Self {
        Self {
            provider,
            model: None,
        }
    }

    /// Attach the selected model id (from
    /// `chat_session::selected_cli_model_id`) to every request this
    /// client issues.
    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }
}

impl LlmClient for ChatProviderLlmClient {
    fn call(&self, req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        let (tx, rx) = mpsc::unbounded::<Result<LlmChunk, LlmError>>();

        if req.abort.is_set() {
            let _ = tx.unbounded_send(Err(LlmError {
                message: "aborted".into(),
                aborted: true,
            }));
            return Box::pin(rx);
        }

        // **Inline the orchestrator's system prompt into the user
        // message.** The CLI-backed `ChatProvider` impls
        // (`ClaudeCodeProvider`, `CopilotProvider`,
        // `SubprocessProvider`) all *ignore* `ChatRequest.system_prompt`
        // — they drive their respective CLIs through subprocess /
        // SDK channels that don't expose a per-turn system slot.
        // Putting `req.system_prompt` into the field would silently
        // drop the orchestrator's planner / sub-agent role prompt,
        // leaving the LLM with only the bare user prompt — codex
        // stop-time review caught this regression. Follow the same
        // prepend pattern `BuiltInProvider` uses for its
        // generation-phase skill preamble (see
        // `chat_runtime.rs::ChatProvider::send` line 138).
        let user_message = if req.system_prompt.is_empty() {
            req.user_prompt.clone()
        } else {
            format!("{}\n\n---\n\n{}", req.system_prompt, req.user_prompt)
        };
        // MiniMax-M3 needs its reasoning: with thinking disabled it
        // answers design subtasks in ~10s with lazy minimal output
        // (ab-v9: 17% expected-shape vs 60% with thinking kept, at an
        // acceptable ~110s). The parse layer strips `<think>` blocks,
        // so M3 rides `Adaptive` (no disable field, no directive) with
        // room for the think tokens; every other model stays disabled —
        // cleaner and cheaper without.
        let m3_keeps_thinking = req
            .model
            .as_deref()
            .filter(|model| !op_orchestrator::is_acp_capability_marker(model))
            .map(|m| m.to_ascii_lowercase().contains("minimax-m3"))
            .unwrap_or(false);
        let chat_req = ChatRequest {
            // Kept empty deliberately — see the prepend above. Any CLI
            // that grows a real system-prompt channel later should
            // also unwrap this back into the field.
            system_prompt: String::new(),
            user_message,
            // The orchestrator composes its own per-request context;
            // chat-transcript history does not apply here.
            history: Vec::new(),
            // The orchestrator's prompts can run long (planner system
            // is ~12 KB, sub-agents emit dense JSON). Give them room —
            // and M3's think stream burns budget before the answer.
            // 16384 matches the headless harness value that ran a
            // 52-prompt corpus with ZERO truncated responses; the old
            // 8192 truncated a rich plan mid-JSON on the desktop
            // ("planning parse failure; using fallback plan" → a
            // skeleton design with a hero and three cards).
            max_output_tokens: if m3_keeps_thinking { 24576 } else { 16384 },
            thinking: if m3_keeps_thinking {
                ThinkingMode::Adaptive
            } else {
                ThinkingMode::Disabled
            },
            effort: EffortLevel::Low,
            attachments: vec![],
            model: self.model.clone(),
        };

        // `provider.send_cancellable` returns a *blocking* iterator. Drain it
        // on a dedicated thread; the LLM call's `BoxStream` is the receive
        // half of the futures mpsc channel. Sharing the orchestrator's abort
        // bit is important for silent CLI transports: Stop must be observable
        // even when there is no next delta on which to notice a dropped
        // receiver.
        let provider = self.provider.clone();
        let cancel = req.abort.shared_atomic();
        thread::spawn(move || {
            for delta in provider.send_cancellable(chat_req, cancel) {
                let chunk = match delta {
                    ChatDelta::TextDelta(s) => Some(Ok(LlmChunk::Text(s))),
                    ChatDelta::Thinking(s) => Some(Ok(LlmChunk::Thinking(s))),
                    ChatDelta::Error(msg) => Some(Err(LlmError {
                        message: msg,
                        aborted: false,
                    })),
                    // `Done` closes the stream by exiting the loop; the
                    // orchestrator parses the accumulated text and
                    // decides what to do.
                    ChatDelta::Done { .. } => break,
                    // The orchestrator expects exactly one text completion;
                    // ignoring a tool call can leave it parsing a truncated
                    // pre-tool response as a design script.
                    ChatDelta::ToolUse { name, .. } => Some(Err(LlmError {
                        message: format!(
                            "provider attempted unsupported tool `{name}` during design generation"
                        ),
                        aborted: false,
                    })),
                };
                if let Some(c) = chunk {
                    if tx.unbounded_send(c).is_err() {
                        // Receiver dropped — orchestrator turn aborted.
                        break;
                    }
                }
            }
        });

        Box::pin(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_orchestrator::AbortFlag;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc as std_mpsc, Mutex};
    use std::time::Duration;

    /// Captures the `ChatRequest` the adapter builds so the thinking
    /// policy is assertable without a real provider.
    struct CapturingProvider {
        seen: Arc<Mutex<Vec<ChatRequest>>>,
    }

    struct SilentCancellableProvider {
        started_tx: std_mpsc::Sender<()>,
        canceled_tx: std_mpsc::Sender<()>,
    }

    impl ChatProvider for SilentCancellableProvider {
        fn provider_label(&self) -> &str {
            "silent-cancellable"
        }

        fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            panic!("orchestrator adapter must use send_cancellable")
        }

        fn supports_cancellable_send(&self) -> bool {
            true
        }

        fn send_cancellable(
            &self,
            _request: ChatRequest,
            cancel: Arc<AtomicBool>,
        ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            let _ = self.started_tx.send(());
            Box::new(SilentCancelIter {
                cancel,
                canceled_tx: Some(self.canceled_tx.clone()),
            })
        }
    }

    struct SilentCancelIter {
        cancel: Arc<AtomicBool>,
        canceled_tx: Option<std_mpsc::Sender<()>>,
    }

    impl Iterator for SilentCancelIter {
        type Item = ChatDelta;

        fn next(&mut self) -> Option<Self::Item> {
            while !self.cancel.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(1));
            }
            if let Some(tx) = self.canceled_tx.take() {
                let _ = tx.send(());
            }
            None
        }
    }

    impl ChatProvider for CapturingProvider {
        fn provider_label(&self) -> &str {
            "capture"
        }
        fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            self.seen.lock().unwrap().push(request);
            Box::new(std::iter::once(ChatDelta::Done {
                stop_reason: op_ai::chat_provider::StopReason::EndTurn,
            }))
        }
    }

    fn call_with_model(model: Option<&str>) -> ChatRequest {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let client = ChatProviderLlmClient::new(Arc::new(CapturingProvider { seen: seen.clone() }));
        let req = CallRequest {
            system_prompt: "sys".into(),
            user_prompt: "user".into(),
            model: model.map(String::from),
            provider: None,
            timeout: Duration::from_secs(5),
            abort: AbortFlag::new(),
            no_text_timeout: None,
            first_text_timeout: None,
        };
        let mut stream = client.call(req);
        crate::chat_runtime::block_on_anywhere(async {
            use futures::StreamExt;
            while stream.next().await.is_some() {}
        });
        let captured = seen.lock().unwrap();
        captured.first().expect("provider was called").clone()
    }

    /// ab-v9: M3 with thinking disabled emits lazy minimal manifests
    /// (17% vs 60% with thinking kept) — M3 rides Adaptive with a
    /// bigger budget, everyone else stays Disabled.
    #[test]
    fn minimax_m3_keeps_thinking_others_disable_it() {
        let m3 = call_with_model(Some("MiniMax-M3"));
        assert_eq!(m3.thinking, ThinkingMode::Adaptive);
        assert_eq!(m3.max_output_tokens, 24576);

        let m27 = call_with_model(Some("MiniMax-M2.7"));
        assert_eq!(m27.thinking, ThinkingMode::Disabled);
        assert_eq!(m27.max_output_tokens, 16384);

        let unknown = call_with_model(None);
        assert_eq!(unknown.thinking, ThinkingMode::Disabled);

        let acp_marker = call_with_model(Some("acp:minimax-m3-wrapper"));
        assert_eq!(acp_marker.thinking, ThinkingMode::Disabled);
        assert_eq!(acp_marker.max_output_tokens, 16384);
        assert_eq!(
            acp_marker.model, None,
            "ACP capability identity must not become a transport model"
        );
    }

    #[test]
    fn abort_flag_cancels_a_silent_provider_transport() {
        let (started_tx, started_rx) = std_mpsc::channel();
        let (canceled_tx, canceled_rx) = std_mpsc::channel();
        let client = ChatProviderLlmClient::new(Arc::new(SilentCancellableProvider {
            started_tx,
            canceled_tx,
        }));
        let abort = AbortFlag::new();
        let req = CallRequest {
            system_prompt: "sys".into(),
            user_prompt: "user".into(),
            model: None,
            provider: None,
            timeout: Duration::from_secs(5),
            abort: abort.clone(),
            no_text_timeout: None,
            first_text_timeout: None,
        };

        let stream = client.call(req);
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("silent provider should start");

        abort.set();

        canceled_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("orchestrator Stop must reach a silent provider transport");
        crate::chat_runtime::block_on_anywhere(async {
            use futures::StreamExt;
            let _: Vec<_> = stream.collect().await;
        });
    }

    // ── Provider-shape agnosticism ───────────────────────────────────────
    //
    // The manual per-subtask retry (failed-subtask remediation, phase 2)
    // reuses `ChatProviderLlmClient` with WHATEVER `ChatProvider` the user
    // currently has selected — a CLI-backed agent (Claude Code /
    // Copilot adapters, or Codex / Antigravity / Grok Build via
    // `SubprocessProvider`) exactly as often as a builtin API-key
    // provider. `call()` above has no branch on the
    // concrete provider type anywhere — it only calls the `ChatProvider`
    // trait method — so these two tests prove that structurally, with two
    // stub providers shaped like the two real families instead of relying
    // on reading the source.

    /// Shaped like `SubprocessProvider` (CLI agents): plain text deltas,
    /// no thinking stream, `Done` ends the turn.
    struct SubprocessStyleProvider;
    impl ChatProvider for SubprocessStyleProvider {
        fn provider_label(&self) -> &str {
            "subprocess-style"
        }
        fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            Box::new(
                vec![
                    ChatDelta::TextDelta("I(null, ".into()),
                    ChatDelta::TextDelta("{\"type\":\"frame\"});".into()),
                    ChatDelta::Done {
                        stop_reason: op_ai::chat_provider::StopReason::EndTurn,
                    },
                ]
                .into_iter(),
            )
        }
    }

    /// Shaped like a builtin reasoning-model HTTP provider: a `Thinking`
    /// stream precedes the text — the adapter must forward `Thinking` as
    /// `LlmChunk::Thinking` (not drop or misfile it as text) exactly as it
    /// does for the subprocess shape's plain text.
    struct ReasoningStyleProvider;
    impl ChatProvider for ReasoningStyleProvider {
        fn provider_label(&self) -> &str {
            "reasoning-style"
        }
        fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            Box::new(
                vec![
                    ChatDelta::Thinking("considering the layout...".into()),
                    ChatDelta::TextDelta("I(null, {\"type\":\"frame\"});".into()),
                    ChatDelta::Done {
                        stop_reason: op_ai::chat_provider::StopReason::EndTurn,
                    },
                ]
                .into_iter(),
            )
        }
    }

    struct ToolEscapingProvider;
    impl ChatProvider for ToolEscapingProvider {
        fn provider_label(&self) -> &str {
            "tool-escaping"
        }

        fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            Box::new(
                vec![ChatDelta::ToolUse {
                    name: "write".into(),
                    args: "{}".into(),
                }]
                .into_iter(),
            )
        }
    }

    fn collect_chunks(client: ChatProviderLlmClient) -> Vec<Result<LlmChunk, LlmError>> {
        let req = CallRequest {
            system_prompt: "sys".into(),
            user_prompt: "user".into(),
            model: None,
            provider: None,
            timeout: Duration::from_secs(5),
            abort: AbortFlag::new(),
            no_text_timeout: None,
            first_text_timeout: None,
        };
        crate::chat_runtime::block_on_anywhere(async {
            use futures::StreamExt;
            client.call(req).collect().await
        })
    }

    #[test]
    fn works_for_a_subprocess_style_cli_provider() {
        let chunks = collect_chunks(ChatProviderLlmClient::new(Arc::new(
            SubprocessStyleProvider,
        )));
        let text: String = chunks
            .iter()
            .filter_map(|c| match c {
                Ok(LlmChunk::Text(t)) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "I(null, {\"type\":\"frame\"});");
        assert!(
            chunks
                .iter()
                .all(|c| !matches!(c, Ok(LlmChunk::Thinking(_)))),
            "the subprocess-style provider emits no thinking: {chunks:?}"
        );
    }

    #[test]
    fn works_for_a_reasoning_style_builtin_http_provider() {
        let chunks = collect_chunks(ChatProviderLlmClient::new(Arc::new(ReasoningStyleProvider)));
        let text: String = chunks
            .iter()
            .filter_map(|c| match c {
                Ok(LlmChunk::Text(t)) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        let thinking: String = chunks
            .iter()
            .filter_map(|c| match c {
                Ok(LlmChunk::Thinking(t)) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "I(null, {\"type\":\"frame\"});");
        assert_eq!(thinking, "considering the layout...");
    }

    #[test]
    fn tool_use_is_a_design_generation_error() {
        let chunks = collect_chunks(ChatProviderLlmClient::new(Arc::new(ToolEscapingProvider)));
        assert!(
            chunks.iter().any(|chunk| matches!(
                chunk,
                Err(error) if error.message.contains("unsupported tool `write`")
            )),
            "{chunks:?}"
        );
    }
}
