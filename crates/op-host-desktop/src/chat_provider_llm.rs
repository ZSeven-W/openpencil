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
}

impl ChatProviderLlmClient {
    pub fn new(provider: Arc<dyn ChatProvider>) -> Self {
        Self { provider }
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
        let chat_req = ChatRequest {
            // Kept empty deliberately — see the prepend above. Any CLI
            // that grows a real system-prompt channel later should
            // also unwrap this back into the field.
            system_prompt: String::new(),
            user_message,
            // The orchestrator's prompts can run long (planner system
            // is ~12 KB, sub-agents emit dense JSON). Give them room.
            max_output_tokens: 8192,
            thinking: ThinkingMode::Disabled,
            effort: EffortLevel::Low,
            attachments: vec![],
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
