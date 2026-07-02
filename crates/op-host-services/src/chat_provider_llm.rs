//! `ChatProvider` → `LlmClient` adapter.
//!
//! Lets the orchestrator reuse whichever chat-panel agent the user
//! already selected (Claude Code / Copilot / Gemini / …) as its LLM
//! transport, instead of forcing a separate Anthropic API key.
//!
//! The CLI agents (`ClaudeCodeProvider`, `CopilotProvider`,
//! `SubprocessProvider`) manage their own auth — `claude` is logged in
//! by the user, Copilot rides GitHub auth, Gemini rides `gcloud`.
//! `agent::Provider` (the `QueryEngine`-facing trait) is a different
//! shape and only `AnthropicProvider` implements it, hence the original
//! Anthropic-key requirement. This adapter eliminates that
//! requirement by turning any `ChatProvider` into the `LlmClient` the
//! orchestrator wants.
//!
//! Each `LlmClient::call` spawns one `std::thread` that drains
//! `provider.send(req)` (a *blocking* iterator) into a futures mpsc
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

        // `provider.send` returns a *blocking* iterator. Drain it on a
        // dedicated thread; the LLM call's `BoxStream` is the receive
        // half of the futures mpsc channel.
        let provider = self.provider.clone();
        thread::spawn(move || {
            for delta in provider.send(chat_req) {
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
                    // Tool calls aren't routed through the orchestrator
                    // — it expects a single text completion per call.
                    // If a CLI agent decides to invoke an MCP tool
                    // mid-turn the result text follows in subsequent
                    // `TextDelta`s anyway.
                    ChatDelta::ToolUse { .. } => None,
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
    use std::sync::Mutex;
    use std::time::Duration;

    /// Captures the `ChatRequest` the adapter builds so the thinking
    /// policy is assertable without a real provider.
    struct CapturingProvider {
        seen: Arc<Mutex<Vec<ChatRequest>>>,
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
        futures::executor::block_on(async {
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
    }
}
