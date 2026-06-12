//! GitHub Copilot CLI bridge — adapts the official
//! `github-copilot-sdk` to the shell-core [`ChatProvider`] trait.
//!
//! The SDK manages the `copilot --server --stdio` process and
//! speaks `Content-Length`-framed JSON-RPC. Events reach us
//! through a [`SessionHandler`] callback rather than a broadcast
//! subscription: `on_event` forwards each `SessionEvent` into the
//! turn's `ChatDelta` channel.
//!
//! One client is started per `send` (the handler binds the turn's
//! channel at session-creation/resume time). Chat-tracked providers
//! ([`CopilotProvider::for_chat`]) keep one Copilot **conversation**
//! alive across turns through the process-wide resume slot: the CLI
//! persists session state on disk, so the next turn resumes it via
//! `Client::resume_session` and follow-ups see the full multi-turn
//! context server-side — the same shape as the Claude Code adapter's
//! `--resume` slot (`chat_claude.rs`).
//!
//! Deliberate divergence from TS: the TS Copilot path
//! (`apps/web/server/api/ai/chat.ts:899-972`, `streamViaCopilot`)
//! creates a fresh session per request and never resumes — multi-turn
//! context there rides only the prompt. The Rust shell upgrades that
//! to real server-side session reuse (campaign item #31 deferred
//! "Copilot 跨轮 session 复用"); the no-session fallback (fresh
//! session + history digest folded into the prompt) keeps at least
//! the TS baseline when no resumable session exists.
//!
//! Event mapping:
//! - `assistant.message_delta` (`deltaContent`) → `TextDelta`
//! - `session.error` (`message`) → `Error`
//! - turn completion (`send_and_wait` returns) → `Done { EndTurn }`

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use github_copilot_sdk::handler::{
    HandlerEvent, HandlerResponse, PermissionResult, SessionHandler,
};
use github_copilot_sdk::types::{
    Attachment, MessageOptions, ResumeSessionConfig, SessionConfig, SessionEvent, SetModelOptions,
};
use github_copilot_sdk::{Client, ClientOptions};
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason};
use tokio::sync::mpsc;

use crate::chat_runtime::{prompt_with_system_prompt, shared_runtime, BlockingRecvIter};

/// How long to let a single Copilot turn run before the SDK times
/// the wait out.
const COPILOT_TURN_TIMEOUT: Duration = Duration::from_secs(180);

/// Process-wide chat-session resume slot. Each chat turn starts a
/// fresh `copilot --server --stdio` process; the slot carries the
/// previous turn's CLI session id forward so the next turn resumes it
/// (the CLI persists conversation state on disk). New Chat clears the
/// slot (`reset_copilot_chat_session`) so a fresh transcript starts a
/// fresh CLI session. Single-window desktop app — one slot is enough.
/// Mirrors `chat_claude::chat_resume_slot`.
fn copilot_resume_slot() -> &'static Mutex<Option<String>> {
    static SLOT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Forget the resumable Copilot chat session. Called when the user
/// starts a New Chat so stale context can't leak into it.
pub(crate) fn reset_copilot_chat_session() {
    if let Ok(mut slot) = copilot_resume_slot().lock() {
        *slot = None;
    }
}

/// `ChatProvider` impl backed by the GitHub Copilot CLI through the
/// official `github-copilot-sdk`.
pub struct CopilotProvider {
    label: String,
    /// When true, this provider participates in the process-wide chat
    /// resume slot: it resumes the stored session and stores the
    /// session id each turn reports. The design / codegen paths
    /// construct untracked providers so their sub-requests never join
    /// (or pollute) the user's chat conversation.
    track_chat_session: bool,
}

impl CopilotProvider {
    /// Build a Copilot provider. The CLI process is not spawned
    /// until the first `send`.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            label: "GitHub Copilot".into(),
            track_chat_session: false,
        }
    }

    /// Build a chat-panel provider that resumes (and records) the
    /// process-wide chat session, so consecutive chat turns share one
    /// Copilot CLI conversation.
    pub fn for_chat() -> Self {
        Self {
            label: "GitHub Copilot".into(),
            track_chat_session: true,
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
        // Chat-tracked turns resume the previous turn's CLI session so
        // cross-turn context lives server-side (mirrors the Claude
        // adapter's `--resume`). Only sessionless turns fold a compact
        // history digest in front of the user message — at least the
        // TS parity baseline (system prompt + last message,
        // `chat.ts:933-934`), plus the digest for cross-turn context
        // when e.g. the user switched providers mid-conversation.
        let resume = if self.track_chat_session {
            copilot_resume_slot().lock().ok().and_then(|s| s.clone())
        } else {
            None
        };
        let mut request = request;
        if resume.is_none() {
            let digest = op_ai::chat_history::history_digest(
                &request.history,
                op_ai::chat_history::DEFAULT_DIGEST_CHARS,
            );
            if !digest.is_empty() {
                request.user_message = format!("{digest}\n\n{}", request.user_message);
            }
        }
        request.user_message = prompt_with_system_prompt(
            &request.system_prompt,
            std::mem::take(&mut request.user_message),
        );
        let track_session = self.track_chat_session;
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            match run_turn(request, guard, tx.clone(), resume, track_session).await {
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

/// Session config for one turn: streaming on, the per-turn reasoning
/// effort, and the selected model when the request carries one (TS
/// parity: `chat.ts` spreads `model` into `createSession` only when
/// present — no selection keeps the CLI's default model).
fn session_config(request: &ChatRequest) -> SessionConfig {
    let mut config = SessionConfig::default();
    config.streaming = Some(true);
    config.reasoning_effort = Some(reasoning_effort_str(request.effort).to_string());
    if let Some(model) = request.model_id() {
        config.model = Some(model.to_string());
    }
    config
}

/// Resume config for a chat-tracked follow-up turn: streaming on,
/// handler installed by the caller. Model + reasoning effort are NOT
/// part of the resume payload (the wire has no fields for them) —
/// they ride a follow-up `session.set_model` when the request selects
/// a model.
fn resume_config(session_id: &str) -> ResumeSessionConfig {
    let mut config = ResumeSessionConfig::new(session_id.into());
    config.streaming = Some(true);
    config
}

/// Run one Copilot turn: start the CLI, resume the previous chat
/// session (or create a fresh streaming one) with a handler that
/// funnels events into `tx`, send the prompt, wait for completion,
/// then tear the session + client down. The CLI keeps session state
/// on disk, so `destroy` (wire `session.destroy`) only disconnects —
/// the recorded id stays resumable by the next turn.
///
/// The per-turn effort drives the session's `reasoning_effort`;
/// staged attachments spill to temp files passed as `File`
/// attachments. (Copilot has no separate thinking-mode knob — effort
/// is its single reasoning dial.)
async fn run_turn(
    request: ChatRequest,
    guard: Option<crate::chat_attachment::TempGuard>,
    tx: mpsc::Sender<ChatDelta>,
    resume: Option<String>,
    track_session: bool,
) -> Result<(), github_copilot_sdk::Error> {
    let client = Client::start(ClientOptions::default()).await?;
    let handler: Arc<dyn SessionHandler> = Arc::new(StreamHandler { tx });
    let mut resumed = false;
    let session = match resume {
        Some(id) => {
            match client
                .resume_session(resume_config(&id).with_handler(handler.clone()))
                .await
            {
                Ok(session) => {
                    resumed = true;
                    session
                }
                // The CLI may have GC'd the on-disk session (or the
                // copilot home moved). Fall back to a fresh session so
                // the turn still runs — context degrades to the prompt
                // for this turn and recovers via the freshly-recorded
                // session id on the next one.
                Err(_) => {
                    let config = session_config(&request).with_handler(handler);
                    client.create_session(config).await?
                }
            }
        }
        None => {
            let config = session_config(&request).with_handler(handler);
            client.create_session(config).await?
        }
    };
    // A resumed session keeps its previous model; apply this turn's
    // selection (and its effort) when one is present. Best-effort —
    // a failed switch must not abort the turn (the session's prior
    // model still answers).
    if resumed {
        if let Some(model) = request.model_id() {
            let opts = SetModelOptions::default()
                .with_reasoning_effort(reasoning_effort_str(request.effort));
            session.set_model(model, Some(opts)).await.ok();
        }
    }
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
    // Chat-tracked turns record the CLI session id so the NEXT turn
    // resumes this conversation (the CLI may reassign the id on
    // resume — always store what the live session reports).
    if track_session {
        if let Ok(mut slot) = copilot_resume_slot().lock() {
            *slot = Some(session.id().to_string());
        }
    }
    // Temp files are no longer needed once the turn is done.
    drop(guard);
    // Best-effort teardown — a failed cleanup must not mask a
    // successful turn. `destroy` is the wire `session.destroy`, which
    // DISCONNECTS but preserves on-disk session state for resume.
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

    #[test]
    fn session_config_sets_selected_model() {
        let req = ChatRequest {
            model: Some("claude-sonnet-4".into()),
            ..Default::default()
        };
        let config = session_config(&req);
        assert_eq!(config.model.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(config.streaming, Some(true));
    }

    #[test]
    fn session_config_without_model_keeps_cli_default() {
        // No selection → leave `model` unset so the CLI picks its own
        // default; blank ids are treated as unset too.
        let config = session_config(&ChatRequest::default());
        assert!(config.model.is_none());
        let config = session_config(&ChatRequest {
            model: Some("  ".into()),
            ..Default::default()
        });
        assert!(config.model.is_none());
    }

    #[test]
    fn resume_config_carries_session_id_and_streaming() {
        let config = resume_config("copilot-sess-42");
        assert_eq!(config.session_id.to_string(), "copilot-sess-42");
        assert_eq!(
            config.streaming,
            Some(true),
            "resumed turns must keep delta streaming on"
        );
        // Resume never silently swaps models — selection rides a
        // follow-up set_model instead.
        assert!(config.system_message.is_none());
    }

    #[test]
    fn resume_slot_round_trips_and_resets() {
        reset_copilot_chat_session();
        assert!(copilot_resume_slot().lock().unwrap().is_none());
        *copilot_resume_slot().lock().unwrap() = Some("sess-1".into());
        assert_eq!(
            copilot_resume_slot().lock().unwrap().as_deref(),
            Some("sess-1")
        );
        // New Chat forgets the conversation so stale context can't
        // leak into a fresh transcript.
        reset_copilot_chat_session();
        assert!(copilot_resume_slot().lock().unwrap().is_none());
    }

    #[test]
    fn for_chat_opts_into_session_tracking() {
        assert!(CopilotProvider::for_chat().track_chat_session);
        // Design / codegen sub-requests stay out of the user's chat
        // conversation.
        assert!(!CopilotProvider::new().track_chat_session);
    }
}
