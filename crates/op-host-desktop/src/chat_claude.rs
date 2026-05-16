//! Claude Code IPC bridge — adapts `anthropic_agent_sdk::query` to
//! the shell-core [`ChatProvider`] trait.
//!
//! The vendored `vendor/anthropic-agent-sdk` crate carries
//! the full Claude Code CLI subprocess transport: spawn the `claude`
//! binary with `--print --verbose --output-format stream-json --`,
//! parse the line-delimited stream-JSON envelope, surface
//! `Message::Assistant` / `Result` / `System` / `User` /
//! `StreamEvent` shapes. This module is the thin mapping layer that
//! collapses those into the OP chat panel's `ChatDelta` vocabulary.
//!
//! Why a separate adapter instead of inlining into chat_subprocess.rs:
//! the SDK owns ~30 fields of CLI options (system prompts, MCP
//! servers, allowed tools, sandbox config, ...) that the bridge
//! eventually wires through. Keeping each provider in its own file
//! gives that surface room to grow without busting the 800-line cap.

use std::sync::Arc;

use anthropic_agent_sdk::{
    types::{ContentBlock, Message},
    ClaudeAgentOptions, StreamExt,
};
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, StopReason};
use tokio::sync::mpsc;

use crate::chat_runtime::{shared_runtime, BlockingRecvIter};

/// `ChatProvider` impl that drives Claude Code via the
/// `anthropic-agent-sdk` Rust client. Single-shot per send today
/// (each call spawns a fresh `claude --print` subprocess); multi-turn
/// via `--resume <session_id>` lands in a follow-up once the OP
/// settings panel exposes session-pinning.
pub struct ClaudeCodeProvider {
    /// Optional CLI-options bundle the SDK forwards to `claude`.
    /// Cloned per-`send`; `None` falls through to the SDK's defaults.
    options: Option<ClaudeAgentOptions>,
    label: String,
}

impl ClaudeCodeProvider {
    /// Build a Claude Code provider with no extra options (SDK
    /// defaults). The CLI is discovered via the SDK's own `find_cli`
    /// (PATH + npm-global / yarn / Linux package locations) — no
    /// binary path needed up front.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            options: None,
            label: "Claude Code".into(),
        }
    }

    /// Build a Claude Code provider with a pre-configured
    /// `ClaudeAgentOptions` (system prompt, model selection, tool
    /// allowlist, MCP servers, sandbox config, etc.). The settings
    /// modal calls this when the user has tuned options away from
    /// defaults.
    #[allow(dead_code)]
    pub fn with_options(options: ClaudeAgentOptions) -> Self {
        Self {
            options: Some(options),
            label: "Claude Code".into(),
        }
    }
}

impl Default for ClaudeCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatProvider for ClaudeCodeProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let prompt = request.user_message;
        let options = self.options.clone();
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            let stream = match anthropic_agent_sdk::query(prompt, options).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx
                        .send(ChatDelta::Error(format!("claude query: {e}")))
                        .await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    return;
                }
            };
            let mut stream = Box::pin(stream);
            let mut emitted_done = false;
            while let Some(msg_result) = stream.next().await {
                // Drop-out the moment the chat panel goes away.
                if tx.is_closed() {
                    break;
                }
                let msg = match msg_result {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = tx
                            .send(ChatDelta::Error(format!("claude stream: {e}")))
                            .await;
                        let _ = tx
                            .send(ChatDelta::Done {
                                stop_reason: StopReason::Aborted,
                            })
                            .await;
                        emitted_done = true;
                        break;
                    }
                };
                if let Some((stop, last)) = handle_message(msg, &tx).await {
                    emitted_done = true;
                    if stop {
                        let _ = tx.send(ChatDelta::Done { stop_reason: last }).await;
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

/// Dispatch one SDK `Message` into `ChatDelta`s sent over `tx`.
/// Returns `Some((true, reason))` when this message is the turn-
/// terminating `Result` (caller should emit terminal Done +
/// break), `Some((false, _))` when emitted-but-not-terminal,
/// `None` when the message was ignored. Marking these explicitly
/// keeps the caller's loop centralized.
async fn handle_message(msg: Message, tx: &mpsc::Sender<ChatDelta>) -> Option<(bool, StopReason)> {
    match msg {
        Message::Assistant { message, .. } => {
            // Stream each ContentBlock as the right ChatDelta variant.
            // Claude Code groups multiple blocks (text + tool_use +
            // thinking) into one assistant message; we surface each
            // individually so the chat panel can render them in turn.
            for block in &message.content {
                match block {
                    ContentBlock::Text { text } => {
                        let _ = tx.send(ChatDelta::TextDelta(text.clone())).await;
                    }
                    ContentBlock::Thinking { thinking, .. } => {
                        let _ = tx.send(ChatDelta::Thinking(thinking.clone())).await;
                    }
                    ContentBlock::ToolUse { name, input, .. } => {
                        let _ = tx
                            .send(ChatDelta::ToolUse {
                                name: name.clone(),
                                args: input.to_string(),
                            })
                            .await;
                    }
                    ContentBlock::ToolResult { .. } => {
                        // Tool results are part of the conversation
                        // history the CLI already shows the model;
                        // the chat widget doesn't render them
                        // separately today.
                    }
                }
            }
            Some((false, StopReason::EndTurn))
        }
        Message::Result {
            subtype, is_error, ..
        } => {
            let reason = if is_error {
                StopReason::Aborted
            } else {
                map_result_subtype(&subtype)
            };
            Some((true, reason))
        }
        Message::System { .. } | Message::User { .. } | Message::StreamEvent { .. } => {
            // Init / context / partial-stream events — the chat
            // widget doesn't surface them today; silent.
            None
        }
    }
}

fn map_result_subtype(s: &str) -> StopReason {
    match s {
        "success" => StopReason::EndTurn,
        "error_max_turns" => StopReason::MaxTokens,
        "error_during_execution" | "error" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

// Keep the import surface alive so callers from main.rs can
// reach the SDK without re-importing.
#[allow(unused_imports)]
pub use anthropic_agent_sdk::{ClaudeAgentOptions as ClaudeOptions, ClaudeSDKClient};

/// Returned by `ClaudeCodeProvider::new` / `with_options` chain
/// for the rare smoke test where we want to confirm the wiring
/// compiles end-to-end without a live `claude` binary on PATH.
#[doc(hidden)]
#[allow(dead_code)]
pub fn _smoke_sdk_arc_send() -> Arc<dyn ChatProvider> {
    Arc::new(ClaudeCodeProvider::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_result_subtype_table() {
        assert!(matches!(map_result_subtype("success"), StopReason::EndTurn));
        assert!(matches!(
            map_result_subtype("error_max_turns"),
            StopReason::MaxTokens
        ));
        assert!(matches!(
            map_result_subtype("error_during_execution"),
            StopReason::Aborted
        ));
        assert!(matches!(map_result_subtype("error"), StopReason::Aborted));
        // Unknown subtypes default to EndTurn so a future SDK addition
        // doesn't break the chat widget.
        assert!(matches!(map_result_subtype("rocket"), StopReason::EndTurn));
    }

    #[test]
    fn provider_label_is_human_readable() {
        let p = ClaudeCodeProvider::new();
        assert_eq!(p.provider_label(), "Claude Code");
    }

    #[test]
    fn provider_constructs_as_chat_provider_trait_object() {
        // Type-system check: ClaudeCodeProvider satisfies the
        // ChatProvider trait bounds (Send + Sync) so it can live
        // behind an `Arc<dyn ChatProvider>` in the widget host.
        let _: Arc<dyn ChatProvider> = Arc::new(ClaudeCodeProvider::new());
    }
}
