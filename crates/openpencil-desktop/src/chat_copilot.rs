//! GitHub Copilot CLI bridge — adapts the official
//! `github-copilot-sdk` to the shell-core [`ChatProvider`] trait.
//!
//! The SDK manages the `copilot --server --stdio` process and
//! speaks `Content-Length`-framed JSON-RPC. Events reach us
//! through a [`SessionHandler`] callback rather than a broadcast
//! subscription: `on_event` forwards each `SessionEvent` into the
//! turn's `ChatDelta` channel.
//!
//! One client + session is started per `send` (the handler binds
//! the turn's channel at session-creation time, so it can't be
//! reused across turns). The ~1 s CLI spawn per turn is the cost
//! of that simplicity; session reuse with a swappable sink is a
//! possible later optimisation.
//!
//! Event mapping:
//! - `assistant.message_delta` (`deltaContent`) → `TextDelta`
//! - `session.error` (`message`) → `Error`
//! - turn completion (`send_and_wait` returns) → `Done { EndTurn }`

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use github_copilot_sdk::handler::{
    HandlerEvent, HandlerResponse, PermissionResult, SessionHandler,
};
use github_copilot_sdk::types::{MessageOptions, SessionConfig, SessionEvent};
use github_copilot_sdk::{Client, ClientOptions};
use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, StopReason,
};
use tokio::sync::mpsc;

use crate::chat_runtime::{shared_runtime, BlockingRecvIter};

/// How long to let a single Copilot turn run before the SDK times
/// the wait out.
const COPILOT_TURN_TIMEOUT: Duration = Duration::from_secs(180);

/// `ChatProvider` impl backed by the GitHub Copilot CLI through the
/// official `github-copilot-sdk`.
pub struct CopilotProvider {
    label: String,
}

impl CopilotProvider {
    /// Build a Copilot provider. The CLI process is not spawned
    /// until the first `send`.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
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
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            match run_turn(prompt, tx.clone()).await {
                Ok(()) => {
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::EndTurn,
                        })
                        .await;
                }
                Err(e) => {
                    let _ = tx.send(ChatDelta::Error(format!("copilot: {e}"))).await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                }
            }
        });
        Box::new(BlockingRecvIter::new(rx))
    }
}

/// Run one Copilot turn: start the CLI, create a streaming session
/// whose handler funnels events into `tx`, send the prompt, wait
/// for completion, then tear the session + client down.
async fn run_turn(
    prompt: String,
    tx: mpsc::Sender<ChatDelta>,
) -> Result<(), github_copilot_sdk::Error> {
    let client = Client::start(ClientOptions::default()).await?;
    let mut config = SessionConfig::default();
    config.streaming = Some(true);
    let config = config.with_handler(Arc::new(StreamHandler { tx }));
    let session = client.create_session(config).await?;
    session
        .send_and_wait(
            MessageOptions::new(prompt).with_wait_timeout(COPILOT_TURN_TIMEOUT),
        )
        .await?;
    // Best-effort teardown — a failed cleanup must not mask a
    // successful turn.
    session.destroy().await.ok();
    client.stop().await.ok();
    Ok(())
}

/// `SessionHandler` that forwards one turn's session events into a
/// `ChatDelta` channel and auto-approves permission prompts so an
/// unattended chat turn isn't blocked waiting for a click.
struct StreamHandler {
    tx: mpsc::Sender<ChatDelta>,
}

#[async_trait]
impl SessionHandler for StreamHandler {
    async fn on_event(&self, event: HandlerEvent) -> HandlerResponse {
        match event {
            HandlerEvent::SessionEvent { event, .. } => {
                forward_session_event(&event, &self.tx).await;
                HandlerResponse::Ok
            }
            HandlerEvent::PermissionRequest { .. } => {
                HandlerResponse::Permission(PermissionResult::Approved)
            }
            _ => HandlerResponse::Ok,
        }
    }
}

/// Translate one `SessionEvent` into a `ChatDelta`. Unhandled
/// event types are dropped — only streamed text + errors surface
/// to the chat widget today.
async fn forward_session_event(event: &SessionEvent, tx: &mpsc::Sender<ChatDelta>) {
    match event.event_type.as_str() {
        "assistant.message_delta" => {
            if let Some(text) = event.data.get("deltaContent").and_then(|c| c.as_str())
            {
                let _ = tx.send(ChatDelta::TextDelta(text.to_string())).await;
            }
        }
        "session.error" => {
            let msg = event
                .data
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error")
                .to_string();
            let _ = tx.send(ChatDelta::Error(msg)).await;
        }
        _ => {}
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
