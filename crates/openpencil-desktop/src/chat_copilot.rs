//! GitHub Copilot CLI IPC bridge — adapts `copilot_sdk` (the
//! in-workspace fork of copilot-community-sdk/copilot-sdk-rust) to
//! the shell-core [`ChatProvider`] trait.
//!
//! Wire: the Copilot CLI speaks LSP-style Content-Length-framed
//! JSON-RPC over stdio. `copilot_sdk::Client` owns the long-lived
//! subprocess; `Session` lives inside the client and is reused
//! across turns (multi-turn conversation history is the client's
//! responsibility, not ours). Our bridge keeps one `Client` + one
//! `Session` per-`CopilotProvider`, starts them lazily on first
//! `send`, and translates SDK events → `ChatDelta`:
//!
//! - `AssistantMessageDelta { delta_content }` → `TextDelta`
//! - `AssistantReasoningDelta { ... }` → `Thinking`
//! - `ToolExecutionStart { tool_name, ... }` → `ToolUse`
//! - `SessionError` / `Abort` → `Error` + `Done { Aborted }`
//! - `AssistantTurnEnd` or `SessionIdle` → `Done { EndTurn }`
//!
//! `AssistantMessage` (the final batched message) is intentionally
//! ignored when deltas are streaming — emitting both would duplicate
//! the text in the chat panel. Future work: dedup heuristics that
//! degrade gracefully when delta events were missed.

use std::sync::Arc;

use copilot_sdk::{Client, Session, SessionConfig, SessionEventData};
use openpencil_shell_core::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, StopReason,
};
use tokio::sync::{mpsc, Mutex};

use crate::chat_runtime::{shared_runtime, BlockingRecvIter};

/// `ChatProvider` impl backed by the GitHub Copilot CLI via
/// `copilot-sdk`. The Client + Session are constructed once on first
/// `send` and reused across subsequent calls (so the model sees
/// multi-turn history through the SDK's session abstraction).
pub struct CopilotProvider {
    /// Lazy-init: `None` until the first `send` constructs the
    /// client. Boxed `Mutex` so multiple concurrent `send` calls
    /// race onto the init path safely (only one wins).
    client_session: Arc<Mutex<Option<Arc<ClientSession>>>>,
    label: String,
}

struct ClientSession {
    _client: Arc<Client>, // kept alive for the duration of the session
    session: Arc<Session>,
}

impl CopilotProvider {
    /// Build a Copilot provider that picks up the `gh copilot` CLI
    /// from PATH (or the SDK's fallback search paths). The Client +
    /// Session don't actually spin up until the first `send`.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            client_session: Arc::new(Mutex::new(None)),
            label: "GitHub Copilot".into(),
        }
    }
}

impl Default for CopilotProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatProvider for CopilotProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn send(
        &self,
        request: ChatRequest,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let prompt = request.user_message;
        let slot = self.client_session.clone();
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            // Resolve (or create) the long-lived Client + Session.
            let cs = match ensure_session(&slot).await {
                Ok(cs) => cs,
                Err(e) => {
                    let _ = tx.send(ChatDelta::Error(format!("copilot init: {e}"))).await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    return;
                }
            };

            let session = cs.session.clone();
            // Subscribe BEFORE sending so we don't miss the first
            // delta (the SDK fires events on a broadcast channel).
            let mut events = session.subscribe();

            if let Err(e) = session.send(prompt).await {
                let _ = tx.send(ChatDelta::Error(format!("copilot send: {e}"))).await;
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::Aborted,
                    })
                    .await;
                return;
            }

            let mut emitted_done = false;
            loop {
                if tx.is_closed() {
                    break;
                }
                let event = match events.recv().await {
                    Ok(e) => e,
                    Err(_) => break, // subscription closed
                };
                let outcome = dispatch_event(event.data, &tx).await;
                match outcome {
                    DispatchOutcome::Continue => {}
                    DispatchOutcome::Done(reason) => {
                        let _ = tx.send(ChatDelta::Done { stop_reason: reason }).await;
                        emitted_done = true;
                        break;
                    }
                    DispatchOutcome::Errored(message) => {
                        let _ = tx.send(ChatDelta::Error(message)).await;
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

async fn ensure_session(
    slot: &Arc<Mutex<Option<Arc<ClientSession>>>>,
) -> copilot_sdk::Result<Arc<ClientSession>> {
    let mut guard = slot.lock().await;
    if let Some(cs) = guard.as_ref() {
        return Ok(cs.clone());
    }
    let client = Arc::new(Client::builder().build()?);
    client.start().await?;
    let session = client.create_session(SessionConfig::default()).await?;
    let cs = Arc::new(ClientSession {
        _client: client,
        session,
    });
    *guard = Some(cs.clone());
    Ok(cs)
}

enum DispatchOutcome {
    Continue,
    Done(StopReason),
    Errored(String),
}

async fn dispatch_event(data: SessionEventData, tx: &mpsc::Sender<ChatDelta>) -> DispatchOutcome {
    match data {
        SessionEventData::AssistantMessageDelta(d) => {
            let _ = tx.send(ChatDelta::TextDelta(d.delta_content)).await;
            DispatchOutcome::Continue
        }
        SessionEventData::AssistantReasoningDelta(d) => {
            let _ = tx.send(ChatDelta::Thinking(d.delta_content)).await;
            DispatchOutcome::Continue
        }
        SessionEventData::ToolExecutionStart(t) => {
            let args = serde_json::to_string(&t.arguments)
                .unwrap_or_else(|_| "{}".into());
            let _ = tx
                .send(ChatDelta::ToolUse {
                    name: t.tool_name,
                    args,
                })
                .await;
            DispatchOutcome::Continue
        }
        SessionEventData::AssistantTurnEnd(_) | SessionEventData::SessionIdle(_) => {
            DispatchOutcome::Done(StopReason::EndTurn)
        }
        SessionEventData::SessionError(e) => DispatchOutcome::Errored(e.message),
        SessionEventData::Abort(_) => DispatchOutcome::Done(StopReason::Aborted),
        // Other event types (turn start, intent, usage, hooks,
        // tool progress / complete, custom agents, etc.) aren't
        // surfaced to the chat widget today. Adding them is a
        // matter of taste — the SDK keeps the original payload so
        // we can promote any of them later.
        _ => DispatchOutcome::Continue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_label_is_human_readable() {
        assert_eq!(CopilotProvider::new().provider_label(), "GitHub Copilot");
    }

    #[test]
    fn provider_constructs_as_chat_provider_trait_object() {
        let _: Arc<dyn ChatProvider> = Arc::new(CopilotProvider::new());
    }
}
