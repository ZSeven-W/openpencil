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
use github_copilot_sdk::types::{Attachment, MessageOptions, SessionConfig, SessionEvent};
use github_copilot_sdk::{Client, ClientOptions};
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason};
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

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        // Stage attachments up front so a write failure aborts the
        // turn with an error instead of silently dropping them.
        let guard = if request.attachments.is_empty() {
            None
        } else {
            match crate::chat_attachment::write_temp_attachments(&request.attachments) {
                Ok(g) => Some(g),
                Err(e) => return crate::chat_attachment::attachment_error_turn(e),
            }
        };
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            match run_turn(request, guard, tx.clone()).await {
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

/// Map the chat panel's effort level onto Copilot's `reasoningEffort`
/// string. Copilot's CLI names its top tier `xhigh` (TS parity:
/// `chat.ts` maps `'max'` → `'xhigh'`).
fn reasoning_effort_str(level: EffortLevel) -> &'static str {
    match level {
        EffortLevel::Low => "low",
        EffortLevel::Medium => "medium",
        EffortLevel::High => "high",
        EffortLevel::Max => "xhigh",
    }
}

/// Run one Copilot turn: start the CLI, create a streaming session
/// whose handler funnels events into `tx`, send the prompt, wait
/// for completion, then tear the session + client down.
///
/// The per-turn effort drives the session's `reasoning_effort`;
/// staged attachments spill to temp files passed as `File`
/// attachments. (Copilot has no separate thinking-mode knob — effort
/// is its single reasoning dial.)
async fn run_turn(
    request: ChatRequest,
    guard: Option<crate::chat_attachment::TempGuard>,
    tx: mpsc::Sender<ChatDelta>,
) -> Result<(), github_copilot_sdk::Error> {
    let client = Client::start(ClientOptions::default()).await?;
    let mut config = SessionConfig::default();
    config.streaming = Some(true);
    config.reasoning_effort = Some(reasoning_effort_str(request.effort).to_string());
    let config = config.with_handler(Arc::new(StreamHandler { tx }));
    let session = client.create_session(config).await?;
    // `guard` holds the staged attachment temp files (written before
    // the turn was spawned); Copilot reads them as `File` attachments.
    let mut opts =
        MessageOptions::new(request.user_message).with_wait_timeout(COPILOT_TURN_TIMEOUT);
    if let Some(g) = &guard {
        let files: Vec<Attachment> = g
            .paths()
            .iter()
            .zip(request.attachments.iter())
            .map(|(path, att)| Attachment::File {
                path: path.clone(),
                display_name: Some(att.name.clone()),
                line_range: None,
            })
            .collect();
        if !files.is_empty() {
            opts = opts.with_attachments(files);
        }
    }
    session.send_and_wait(opts).await?;
    // Temp files are no longer needed once the turn is done.
    drop(guard);
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
            if let Some(text) = event.data.get("deltaContent").and_then(|c| c.as_str()) {
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

    #[test]
    fn reasoning_effort_maps_max_to_xhigh() {
        assert_eq!(reasoning_effort_str(EffortLevel::Low), "low");
        assert_eq!(reasoning_effort_str(EffortLevel::Medium), "medium");
        assert_eq!(reasoning_effort_str(EffortLevel::High), "high");
        // Copilot's top tier is "xhigh", not "max" (TS parity).
        assert_eq!(reasoning_effort_str(EffortLevel::Max), "xhigh");
    }
}
