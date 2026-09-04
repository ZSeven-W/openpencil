//! GitHub Copilot CLI bridge — adapts the official
//! `github-copilot-sdk` to the shell-core [`ChatProvider`] trait.
//!
//! OpenPencil owns the `copilot --server --stdio` process lifetime while the
//! SDK speaks `Content-Length`-framed JSON-RPC. Events reach us through an SDK
//! [`EventSubscription`](github_copilot_sdk::EventSubscription); each
//! `SessionEvent` is forwarded into the turn's `ChatDelta` channel.
//!
//! One client and session are started per `send` on a disposable runtime; the
//! turn subscribes before sending. Multi-turn context rides the request's
//! history digest, matching the TS Copilot path. A process-global SDK resume
//! slot is deliberately avoided because OpenPencil supports multiple chat tabs.
//!
//! Event mapping:
//! - `assistant.message_delta` (`deltaContent`) → `TextDelta`
//! - SDK terminal errors → one `Error` from the worker boundary
//! - turn completion (`send_and_wait` returns) → `Done { EndTurn }`

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;

use github_copilot_sdk::subscription::RecvErrorKind;
use github_copilot_sdk::types::{
    Attachment, MessageOptions, SessionConfig, SessionEvent, SessionId,
};
use github_copilot_sdk::{Client, Error, ErrorKind};
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason};
use tokio::sync::mpsc;

use crate::chat_runtime::{prompt_with_system_prompt, BlockingRecvIter};

/// How long to let a single Copilot turn run before the SDK times
/// the wait out.
const COPILOT_TURN_TIMEOUT: Duration = Duration::from_secs(180);
const COPILOT_TOTAL_TIMEOUT: Duration = Duration::from_secs(195);
const COPILOT_GRACEFUL_STOP_TIMEOUT: Duration = Duration::from_secs(2);
const COPILOT_SESSION_TEARDOWN_TIMEOUT: Duration = Duration::from_secs(2);
static NEXT_COPILOT_SESSION: AtomicU64 = AtomicU64::new(1);

/// Retained for the host's provider-reset fanout. Copilot turns are stateless
/// at the SDK-session layer, so there is no cross-tab resume state to clear.
pub fn reset_copilot_chat_session() {}

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

    /// Build the chat-panel provider. Context is supplied by `ChatRequest`
    /// history, keeping each editor tab isolated.
    pub fn for_chat() -> Self {
        Self::new()
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

    fn supports_cancellable_send(&self) -> bool {
        true
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, None)
    }

    fn send_cancellable(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, Some(cancel))
    }
}

impl CopilotProvider {
    fn send_inner(
        &self,
        request: ChatRequest,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
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
        let mut request = request;
        let digest = op_ai::chat_history::history_digest(
            &request.history,
            op_ai::chat_history::DEFAULT_DIGEST_CHARS,
        );
        if !digest.is_empty() {
            request.user_message = format!("{digest}\n\n{}", request.user_message);
        }
        request.user_message = prompt_with_system_prompt(
            &request.system_prompt,
            std::mem::take(&mut request.user_message),
        );
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        let worker_tx = tx.clone();
        let worker = std::thread::Builder::new()
            .name("openpencil-copilot-turn".to_string())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                match runtime {
                    Ok(runtime) => runtime.block_on(async move {
                        match run_turn(request, guard, worker_tx.clone()).await {
                            Ok(()) => {
                                let _ = worker_tx
                                    .send(ChatDelta::Done {
                                        stop_reason: StopReason::EndTurn,
                                    })
                                    .await;
                            }
                            Err(error) => {
                                let _ = worker_tx
                                    .send(ChatDelta::Error(format!("copilot: {error}")))
                                    .await;
                                let _ = worker_tx
                                    .send(ChatDelta::Done {
                                        stop_reason: StopReason::Aborted,
                                    })
                                    .await;
                            }
                        }
                    }),
                    Err(error) => {
                        let _ = worker_tx
                            .blocking_send(ChatDelta::Error(format!("copilot runtime: {error}")));
                        let _ = worker_tx.blocking_send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        });
                    }
                }
            });
        if let Err(error) = worker {
            let _ = tx.try_send(ChatDelta::Error(format!("copilot worker: {error}")));
            let _ = tx.try_send(ChatDelta::Done {
                stop_reason: StopReason::Aborted,
            });
        }
        match cancel {
            Some(cancel) => Box::new(BlockingRecvIter::cooperative(rx, cancel)),
            None => Box::new(BlockingRecvIter::new(rx)),
        }
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
fn session_config(request: &ChatRequest, session_id: &SessionId) -> SessionConfig {
    let mut config = SessionConfig::default();
    config.session_id = Some(session_id.clone());
    config.streaming = Some(true);
    // OpenPencil supplies the owning tab's history on every turn and never
    // uses Copilot's cross-session search/retrieval store. Keep that store
    // isolated; bounded session.delete below remains the persistence cleanup.
    config.enable_session_store = Some(false);
    if let Some(model) = request.model_id() {
        config.model = Some(model.to_string());
        // The synthetic auto model chooses both a concrete model and its
        // effort. Current CLIs reject an explicit reasoningEffort with auto.
        if model != "auto" {
            config.reasoning_effort = Some(reasoning_effort_str(request.effort).to_string());
        }
    }
    config.approve_all_permissions()
}

fn reasoning_effort_is_unsupported(error: &Error) -> bool {
    error
        .to_string()
        .to_ascii_lowercase()
        .contains("reasoning effort is not supported")
}

async fn create_compatible_session(
    client: &Client,
    request: &ChatRequest,
    session_id: &SessionId,
) -> Result<github_copilot_sdk::session::Session, Error> {
    match client
        .create_session(session_config(request, session_id))
        .await
    {
        Err(error) if reasoning_effort_is_unsupported(&error) => {
            // Capability metadata evolves independently from the app. Retry
            // only the explicit server verdict without the optional effort.
            let mut fallback = session_config(request, session_id);
            fallback.reasoning_effort = None;
            client.create_session(fallback).await
        }
        result => result,
    }
}

fn new_turn_session_id() -> SessionId {
    let serial = NEXT_COPILOT_SESSION.fetch_add(1, Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    SessionId::new(format!(
        "openpencil-{}-{timestamp}-{serial}",
        std::process::id()
    ))
}

fn cancellation_error() -> Error {
    Error::with_message(ErrorKind::Io, "Copilot turn cancelled")
}

async fn delete_turn_session(client: &Client, session_id: &SessionId) -> Result<(), Error> {
    match tokio::time::timeout(
        COPILOT_GRACEFUL_STOP_TIMEOUT,
        client.delete_session(session_id),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(Error::with_message(
            ErrorKind::Io,
            format!("Copilot session cleanup timed out for {session_id}"),
        )),
    }
}

fn merge_turn_and_cleanup(
    turn: Result<(), Error>,
    cleanup: Result<(), Error>,
) -> Result<(), Error> {
    match (turn, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(turn_error), Err(cleanup_error)) => Err(Error::with_message(
            ErrorKind::Io,
            format!("{turn_error}; Copilot session cleanup also failed: {cleanup_error}"),
        )),
    }
}

/// Run one Copilot turn: start the guarded CLI, verify the protocol, run the
/// session on a bounded future, then delete the per-turn session and
/// deterministically reap the process. Multi-turn context is supplied by the
/// owning tab's request history; no SDK session id is resumed by a later turn.
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
    let exe = crate::model_discovery::resolve_cli("copilot").ok_or_else(|| {
        Error::with_message(
            ErrorKind::BinaryNotFound {
                name: "copilot".to_string(),
                hint: Some("Install GitHub Copilot CLI and connect it in Agents".to_string()),
            },
            "GitHub Copilot CLI was not found",
        )
    })?;
    let mut server = crate::copilot_sdk_probe::CopilotServer::spawn(&exe)
        .await
        .map_err(|error| Error::with_message(ErrorKind::Io, error.to_string()))?;
    let client = server.client().clone();
    let result = run_turn_on_client(&client, request, guard, tx).await;
    let _ = tokio::time::timeout(COPILOT_GRACEFUL_STOP_TIMEOUT, client.stop()).await;
    server.shutdown().await;
    result
}

async fn run_turn_on_client(
    client: &Client,
    request: ChatRequest,
    guard: Option<crate::chat_attachment::TempGuard>,
    tx: mpsc::Sender<ChatDelta>,
) -> Result<(), github_copilot_sdk::Error> {
    let session_id = new_turn_session_id();
    let cancel = tx.clone();
    let result = match tokio::time::timeout(COPILOT_TOTAL_TIMEOUT, async {
        tokio::select! {
            biased;
            _ = cancel.closed() => return Err(cancellation_error()),
            result = client.verify_protocol_version() => result?,
        }
        run_turn_with_client(client, request, guard, tx, &session_id).await
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Err(Error::with_message(ErrorKind::Io, "Copilot turn timed out")),
    };
    // The ID is assigned before session.create, so every exit path can issue
    // the persistent-state deletion even when creation, streaming, timeout,
    // or receiver cancellation drops the in-flight SDK future.
    let cleanup = delete_turn_session(client, &session_id).await;
    merge_turn_and_cleanup(result, cleanup)
}

async fn run_turn_with_client(
    client: &Client,
    request: ChatRequest,
    guard: Option<crate::chat_attachment::TempGuard>,
    tx: mpsc::Sender<ChatDelta>,
    session_id: &SessionId,
) -> Result<(), github_copilot_sdk::Error> {
    let session = tokio::select! {
        biased;
        _ = tx.closed() => return Err(cancellation_error()),
        result = create_compatible_session(client, &request, session_id) => result?,
    };
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
    // Subscribe before sending so the first streamed delta cannot race past
    // the observer. Bias toward ready events: once `send_and_wait` sees idle,
    // drain every already-buffered delta before finishing the turn.
    let turn_result = {
        let mut events = session.subscribe();
        let mut events_open = true;
        let send = session.send_and_wait(opts);
        tokio::pin!(send);
        loop {
            tokio::select! {
                biased;
                _ = tx.closed() => break Err(cancellation_error()),
                event = events.recv(), if events_open => match event {
                    Ok(event) => forward_session_event(&event, &tx).await,
                    Err(error) => match error.kind() {
                        RecvErrorKind::Lagged(lagged) => {
                            break Err(Error::with_message(
                                ErrorKind::InvalidConfig,
                                format!(
                                    "Copilot event stream lagged by {} events; incomplete response aborted",
                                    lagged.skipped()
                                ),
                            ));
                        }
                        _ => events_open = false,
                    },
                },
                result = &mut send => break result.map(|_| ()),
            }
        }
    };
    // Temp files are no longer needed once the turn is done.
    drop(guard);
    // These are request/response RPCs too. Bound their combined grace period
    // so a server that stops replying cannot turn Stop/New Chat into the
    // 195-second turn timeout. The caller still deletes the known session ID
    // and stops/reaps the server after this future returns.
    let _ = tokio::time::timeout(COPILOT_SESSION_TEARDOWN_TIMEOUT, async {
        if turn_result.is_err() {
            session.abort().await.ok();
        }
        session.disconnect().await.ok();
    })
    .await;
    turn_result
}

/// Translate one `SessionEvent` into a `ChatDelta`. Unhandled
/// event types are dropped — only streamed text + errors surface
/// to the chat widget today.
async fn forward_session_event(event: &SessionEvent, tx: &mpsc::Sender<ChatDelta>) {
    if event.event_type == "assistant.message_delta" {
        if let Some(text) = event.data.get("deltaContent").and_then(|c| c.as_str()) {
            let _ = tx.send(ChatDelta::TextDelta(text.to_string())).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    async fn read_frame<R>(reader: &mut R) -> serde_json::Value
    where
        R: AsyncBufRead + Unpin,
    {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            let read = reader.read_line(&mut line).await.expect("frame header");
            assert_ne!(read, 0, "unexpected EOF before frame body");
            if line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(value) = line.trim().strip_prefix("Content-Length:").map(str::trim) {
                content_length = Some(value.parse::<usize>().expect("content length"));
            }
        }
        let mut body = vec![0; content_length.expect("Content-Length header")];
        reader.read_exact(&mut body).await.expect("frame body");
        serde_json::from_slice(&body).expect("JSON-RPC body")
    }

    async fn write_frame<W>(writer: &mut W, value: serde_json::Value)
    where
        W: AsyncWrite + Unpin,
    {
        let body = serde_json::to_vec(&value).expect("serialize JSON-RPC response");
        writer
            .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
            .await
            .expect("frame header write");
        writer.write_all(&body).await.expect("frame body write");
        writer.flush().await.expect("frame flush");
    }

    #[test]
    fn provider_label_is_human_readable() {
        assert_eq!(CopilotProvider::new().provider_label(), "GitHub Copilot");
    }

    #[test]
    fn provider_constructs_as_chat_provider_trait_object() {
        let provider: Arc<dyn ChatProvider> = Arc::new(CopilotProvider::new());
        assert!(provider.supports_cancellable_send());
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
    fn turn_session_ids_are_unique_and_openpencil_owned() {
        let first = new_turn_session_id();
        let second = new_turn_session_id();
        assert_ne!(first, second);
        assert!(first.as_str().starts_with("openpencil-"));
    }

    #[test]
    fn cleanup_failure_preserves_the_turn_error() {
        let turn = Error::with_message(ErrorKind::Io, "turn failed");
        let cleanup = Error::with_message(ErrorKind::Io, "delete failed");
        let error = merge_turn_and_cleanup(Err(turn), Err(cleanup)).expect_err("combined error");
        let message = error.to_string();
        assert!(message.contains("turn failed"));
        assert!(message.contains("delete failed"));
    }

    #[tokio::test]
    async fn cancellation_during_create_still_deletes_the_known_session() {
        let (client_stream, server_stream) = tokio::io::duplex(32 * 1024);
        let (client_reader, client_writer) = tokio::io::split(client_stream);
        let (server_reader, mut server_writer) = tokio::io::split(server_stream);
        let mut server_reader = tokio::io::BufReader::new(server_reader);
        let client = Client::from_streams(
            client_reader,
            client_writer,
            std::env::current_dir().expect("current directory"),
        )
        .expect("in-memory SDK client");
        let (tx, rx) = mpsc::channel(1);
        let turn_client = client.clone();
        let turn = tokio::spawn(async move {
            run_turn_on_client(&turn_client, ChatRequest::default(), None, tx).await
        });

        let connect = read_frame(&mut server_reader).await;
        assert_eq!(connect["method"], "connect");
        write_frame(
            &mut server_writer,
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": connect["id"],
                "result": { "ok": true, "protocolVersion": 3, "version": "test" }
            }),
        )
        .await;

        let create = read_frame(&mut server_reader).await;
        assert_eq!(create["method"], "session.create");
        assert_eq!(create["params"]["enableSessionStore"], false);
        let session_id = create["params"]["sessionId"]
            .as_str()
            .expect("OpenPencil-assigned session ID")
            .to_string();

        // Cancel while session.create is still unanswered. The turn must not
        // rely on receiving a Session object before it can delete state.
        drop(rx);
        let delete = tokio::time::timeout(Duration::from_secs(2), read_frame(&mut server_reader))
            .await
            .expect("session.delete after cancellation");
        assert_eq!(delete["method"], "session.delete");
        assert_eq!(delete["params"]["sessionId"], session_id);
        write_frame(
            &mut server_writer,
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": delete["id"],
                "result": {}
            }),
        )
        .await;

        let error = tokio::time::timeout(Duration::from_secs(2), turn)
            .await
            .expect("bounded cancelled turn")
            .expect("turn task")
            .expect_err("cancelled turn error");
        assert!(error.to_string().contains("cancelled"));
        client.force_stop();
    }

    #[test]
    fn session_config_sets_selected_model() {
        let req = ChatRequest {
            model: Some("claude-sonnet-4".into()),
            ..Default::default()
        };
        let session_id = SessionId::new("openpencil-test-session");
        let config = session_config(&req, &session_id);
        assert_eq!(config.model.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(config.streaming, Some(true));
        assert_eq!(config.enable_session_store, Some(false));
        assert_eq!(config.session_id.as_ref(), Some(&session_id));
        assert_eq!(config.reasoning_effort.as_deref(), Some("low"));
    }

    #[test]
    fn session_config_without_model_keeps_cli_default() {
        // No selection → leave `model` unset so the CLI picks its own
        // default; its reasoning capability is also unknown.
        let session_id = SessionId::new("openpencil-test-session");
        let config = session_config(&ChatRequest::default(), &session_id);
        assert!(config.model.is_none());
        assert!(config.reasoning_effort.is_none());
        let config = session_config(
            &ChatRequest {
                model: Some("  ".into()),
                ..Default::default()
            },
            &session_id,
        );
        assert!(config.model.is_none());
        assert!(config.reasoning_effort.is_none());
    }

    #[test]
    fn session_config_auto_leaves_reasoning_selection_to_the_cli() {
        let session_id = SessionId::new("openpencil-test-session");
        let config = session_config(
            &ChatRequest {
                model: Some("auto".into()),
                effort: EffortLevel::Max,
                ..Default::default()
            },
            &session_id,
        );
        assert_eq!(config.model.as_deref(), Some("auto"));
        assert!(config.reasoning_effort.is_none());
    }

    #[test]
    fn reset_hook_is_idempotent_without_global_session_state() {
        reset_copilot_chat_session();
        reset_copilot_chat_session();
    }

    #[test]
    fn for_chat_uses_the_same_tab_isolated_provider() {
        assert_eq!(
            CopilotProvider::for_chat().provider_label(),
            "GitHub Copilot"
        );
    }
}
