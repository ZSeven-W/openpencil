//! Shared chat turn worker and transcript folding logic.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

use op_ai::chat_provider::{
    BlockerReport, ChatDelta, ChatHistoryRole, ChatProvider, ChatRequest, ChatToolExecutor,
    ChatToolResult, FinalizeReport, QualitySummary, UnfilledScreensReport, CHECK_BLOCKERS_OP,
    LOOP_FINALIZE_OP,
};
use op_editor_core::{ChatMessage, ChatRole, ChatToolCall};

/// One tool call forwarded from an agent-loop worker to a host UI thread.
pub struct ChatToolRequest {
    pub name: String,
    pub args_json: String,
    pub ack: SyncSender<ChatToolResult>,
}

/// Worker-side [`ChatToolExecutor`] that forwards calls over a channel and
/// blocks until the host acks with the real tool result.
pub struct UiChatToolExecutor {
    tx: Mutex<Sender<ChatToolRequest>>,
}

impl UiChatToolExecutor {
    pub fn new(tx: Sender<ChatToolRequest>) -> Self {
        Self { tx: Mutex::new(tx) }
    }
}

impl ChatToolExecutor for UiChatToolExecutor {
    fn execute(&self, name: &str, args_json: &str) -> ChatToolResult {
        self.forward(name, args_json)
    }

    /// Forward the reserved [`LOOP_FINALIZE_OP`] over the same tool channel so
    /// the host (which owns the live `EditorState`) runs
    /// `op_orchestrator::apply_loop_finalize` plus the promise-delivery
    /// invariant's detect-and-mark pass. Blocks on the ack like any tool call;
    /// the ack's `content` carries back the (already canvas-marked)
    /// committed/unfilled report — no new IPC needed, this reuses the same
    /// envelope every other tool result already rides.
    fn finalize(&self) -> FinalizeReport {
        let ack = self.forward(LOOP_FINALIZE_OP, r#"{"checkOnly":false}"#);
        FinalizeReport {
            screens: unfilled_report_from_ack(&ack),
            quality: quality_summary_from_ack(&ack),
        }
    }

    /// Cheap, read-only promise-delivery check — same reserved op, with
    /// `checkOnly: true` so the host runs ONLY the detector
    /// (`unfilled_screens::detect_unfilled_screens` /
    /// `list_screen_candidates`), never `apply_loop_finalize` or the
    /// canvas-marking side effect.
    fn check_unfilled_screens(&self) -> UnfilledScreensReport {
        unfilled_report_from_ack(&self.forward(LOOP_FINALIZE_OP, r#"{"checkOnly":true}"#))
    }

    /// Forward the reserved [`CHECK_BLOCKERS_OP`] over the same tool
    /// channel so the host runs a read-only unresolved-blocker scan
    /// (`op_host_services::loop_blocker_ledger::detect_blockers`) against
    /// the live `EditorState`. Never mutates the document.
    fn check_blockers(&self) -> BlockerReport {
        blocker_report_from_ack(&self.forward(CHECK_BLOCKERS_OP, "{}"))
    }
}

/// Parse the `{"success":true,"committed":[...],"unfilled":[...]}` envelope
/// the host's `LOOP_FINALIZE_OP` interception acks with. Any other shape (a
/// transport error, an old host build that doesn't know the fields yet)
/// degrades to an empty report — the promise-delivery invariant is a
/// best-effort nicety on top of the existing finalize/check calls, never a
/// reason to fail the turn.
fn unfilled_report_from_ack(result: &ChatToolResult) -> UnfilledScreensReport {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&result.content) else {
        return UnfilledScreensReport::default();
    };
    let names = |key: &str| {
        v.get(key)
            .cloned()
            .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
            .unwrap_or_default()
    };
    UnfilledScreensReport {
        committed: names("committed"),
        unfilled: names("unfilled"),
    }
}

/// Parse the `"quality": {"checks":[...],"repairs":[{"check":…,"count":…}]}`
/// half of the same `LOOP_FINALIZE_OP` ack. Degrades to an empty summary on
/// any other shape (transport error, an older host build that predates the
/// field) — and an empty summary renders NO credential, so a host that can't
/// report what it checked never gets credited for checking.
fn quality_summary_from_ack(result: &ChatToolResult) -> QualitySummary {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&result.content) else {
        return QualitySummary::default();
    };
    let Some(quality) = v.get("quality") else {
        return QualitySummary::default();
    };
    let checks = quality
        .get("checks")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let repairs = quality
        .get("repairs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    let check = entry.get("check")?.as_str()?.to_string();
                    let count = entry.get("count")?.as_u64()? as usize;
                    Some((check, count))
                })
                .collect()
        })
        .unwrap_or_default();
    // The itemized `"records"` array is optional: a host build that predates
    // it simply reports counts, which is what the credential always showed.
    let records = quality
        .get("records")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|line| line.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    // Same optional treatment as `records`: a host build that predates notes
    // simply reports none.
    let notes = quality
        .get("notes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|line| line.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    QualitySummary {
        checks,
        repairs,
        records,
        notes,
    }
}

/// Parse the `{"success":true,"blockers":[{"category":...,"detail":...}]}`
/// envelope the host's `CHECK_BLOCKERS_OP` interception acks with. Any other
/// shape (transport error, an old host build that doesn't know this op)
/// degrades to an empty report — same best-effort discipline as
/// [`unfilled_report_from_ack`].
fn blocker_report_from_ack(result: &ChatToolResult) -> BlockerReport {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&result.content) else {
        return BlockerReport::default();
    };
    let blockers = v
        .get("blockers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    let category = entry.get("category")?.as_str()?.to_string();
                    let detail = entry.get("detail")?.as_str()?.to_string();
                    Some(op_ai::chat_provider::BlockerEntry { category, detail })
                })
                .collect()
        })
        .unwrap_or_default();
    BlockerReport { blockers }
}

impl UiChatToolExecutor {
    /// Send one request over the tool channel and block until the host acks.
    fn forward(&self, name: &str, args_json: &str) -> ChatToolResult {
        let (ack_tx, ack_rx) = std::sync::mpsc::sync_channel::<ChatToolResult>(1);
        let req = ChatToolRequest {
            name: name.to_string(),
            args_json: args_json.to_string(),
            ack: ack_tx,
        };
        let sent = match self.tx.lock() {
            Ok(tx) => tx.send(req).is_ok(),
            Err(_) => false,
        };
        if !sent {
            return aborted_result();
        }
        match ack_rx.recv_timeout(std::time::Duration::from_secs(60)) {
            Ok(result) => result,
            Err(_) => timeout_result(),
        }
    }
}

fn timeout_result() -> ChatToolResult {
    ChatToolResult {
        content: r#"{"success":false,"error":"tool execution timed out waiting for the editor"}"#
            .into(),
        is_error: true,
    }
}

fn aborted_result() -> ChatToolResult {
    ChatToolResult {
        content: r#"{"success":false,"error":"chat turn aborted before the tool ran"}"#.into(),
        is_error: true,
    }
}

/// Create the worker-to-host tool channel for one chat turn.
pub fn chat_tool_channel() -> (UiChatToolExecutor, Receiver<ChatToolRequest>) {
    let (tx, rx) = std::sync::mpsc::channel::<ChatToolRequest>();
    (UiChatToolExecutor::new(tx), rx)
}

/// One in-flight chat turn.
pub struct ChatSession {
    rx: Receiver<ChatDelta>,
    /// Shared with `ChatProvider::send_cancellable`. Dropping a session is
    /// the host's Stop/New Chat/close-tab signal, including while a provider
    /// is silent and the worker is blocked waiting for its next delta.
    cancel: Option<Arc<AtomicBool>>,
    tool_rx: Option<Receiver<ChatToolRequest>>,
    deferred_tool_requests: VecDeque<ChatToolRequest>,
    finished: bool,
    /// A design-agent-loop turn preserves the exact narration/tool ordering so
    /// the transcript can interleave visible prose with activity cards. Plain
    /// chat keeps its classic aggregated tool panel.
    is_design_loop: bool,
    loop_finalized: bool,
    viewport_fitted: bool,
}

/// Result of a single non-blocking [`ChatSession::poll`].
pub struct ChatPoll {
    pub text: String,
    pub thinking: String,
    /// Tool calls in provider order. A call produced by [`ChatSession::poll`]
    /// carries a `content_offset` relative to this poll's aggregated `text`,
    /// preserving its position between text deltas without changing the
    /// aggregate fields consumed by existing callers.
    pub tool_calls: Vec<ChatToolCall>,
    pub error: Option<String>,
    pub finished: bool,
}

impl ChatPoll {
    /// True when this poll carried no new content and the turn has not ended.
    pub fn is_idle(&self) -> bool {
        self.text.is_empty()
            && self.thinking.is_empty()
            && self.tool_calls.is_empty()
            && self.error.is_none()
            && !self.finished
    }
}

/// Fold one [`ChatPoll`] into the trailing assistant `message` (narration
/// stays in the visible bubble).
pub fn apply_poll_to_message(message: &mut ChatMessage, poll: &ChatPoll) {
    apply_poll_to_message_with(message, poll, false);
}

/// Fold one [`ChatPoll`] into the trailing assistant `message`. Design-loop
/// turns keep narration visible and stamp tool offsets for chronological
/// interleaving; plain chat keeps the classic grouped tool panel. Errors always
/// surface in `content`.
pub fn apply_poll_to_message_with(message: &mut ChatMessage, poll: &ChatPoll, design_loop: bool) {
    let content_base = message.content.len() as u32;
    if let Some(err) = &poll.error {
        message.content = format!("error: {err}");
    } else {
        // Design-loop narration used to fold into the collapsed thinking
        // area — the panel then showed nothing but "Thinking / N tool
        // calls" while Pencil streams the designer's running commentary
        // as first-class prose (user report 2026-07-12). Narration stays
        // in `content` for every turn now.
        message.content.push_str(&poll.text);
    }
    message.thinking.push_str(&poll.thinking);
    if poll.tool_calls.iter().any(tool_call_defaults_open) {
        message.tools_collapsed = false;
    }
    // Stamp where each call landed in the narration so the transcript can
    // interleave prose and per-call verb chips chronologically. Only
    // design-loop turns interleave; plain chat keeps the aggregated panel.
    let error_offset = message.content.len() as u32;
    message
        .tool_calls
        .extend(poll.tool_calls.iter().cloned().map(|mut call| {
            if design_loop {
                // `ChatSession::poll` records the position within this batch.
                // Hand-built polls from existing callers carry `None`; retain
                // their historical behavior by placing those calls after the
                // batch's complete text.
                call.content_offset = Some(if poll.error.is_some() {
                    // Errors replace, rather than append to, visible content.
                    error_offset
                } else {
                    let within_poll = call.content_offset.unwrap_or(poll.text.len() as u32);
                    content_base.saturating_add(within_poll)
                });
            } else {
                // Offsets drive design-loop narration interleaving only. Plain
                // chat keeps the classic aggregate tool panel.
                call.content_offset = None;
            }
            call
        }));
    if poll.finished {
        message.streaming = false;
    }
}

/// Map the chat transcript into `(role, text)` history pairs for the in-flight
/// turn, excluding the current user message and trailing streaming assistant.
///
/// Classic-orchestrator worker bubbles are a presentation projection of the
/// primary assistant turn, not additional provider turns. Fold every adjacent
/// worker projection into the preceding assistant history entry so a
/// multi-screen result keeps its screen/activity context without producing a
/// run of consecutive `Assistant` messages on the next follow-up.
pub fn chat_history_from_transcript(messages: &[ChatMessage]) -> Vec<(ChatHistoryRole, String)> {
    let mut end = messages.len();
    if end > 0 && messages[end - 1].role == ChatRole::Assistant && messages[end - 1].streaming {
        end -= 1;
    }
    if end > 0 && messages[end - 1].role == ChatRole::User {
        end -= 1;
    }
    let mut history: Vec<(ChatHistoryRole, String)> = Vec::new();
    // Index of the most recent assistant projection since the last user
    // message. Worker bubbles append there; crossing a user boundary clears
    // the target so projections from separate turns can never be combined.
    let mut assistant_target: Option<usize> = None;

    for message in &messages[..end] {
        match message.role {
            ChatRole::User => {
                assistant_target = None;
                if let Some(content) = history_content(message) {
                    history.push((ChatHistoryRole::User, content));
                }
            }
            ChatRole::Assistant if message.design_worker_group.is_some() => {
                let Some(projection) = design_history_projection(message) else {
                    continue;
                };
                if let Some(index) = assistant_target {
                    append_history_projection(&mut history[index].1, &projection);
                } else {
                    // Defensive orphan handling: a worker projection without
                    // its primary still represents the one assistant response
                    // for this user turn. Keep it rather than dropping useful
                    // screen context, while still emitting only one entry.
                    history.push((ChatHistoryRole::Assistant, projection));
                    assistant_target = Some(history.len() - 1);
                }
            }
            ChatRole::Assistant => {
                let content = if message.design_worker_screen.is_some() {
                    design_history_projection(message)
                } else {
                    history_content(message)
                };
                if let Some(content) = content {
                    history.push((ChatHistoryRole::Assistant, content));
                    assistant_target = Some(history.len() - 1);
                }
            }
        }
    }

    history
}

/// Keep structured design completions and activity titles in provider history
/// even when the UI intentionally carries them outside visible prose.
fn history_content(message: &ChatMessage) -> Option<String> {
    let completion = (message.role == ChatRole::Assistant)
        .then_some(message.completion)
        .flatten();
    let mut text = message.content.clone();
    if let Some(completion) = completion {
        if !text.trim().is_empty() {
            text.push_str("\n\n");
        }
        text.push_str(&format!(
            "Design work completed: {} succeeded, {} failed.",
            completion.succeeded, completion.failed
        ));
    }
    let sections: Vec<&str> = message
        .activities
        .iter()
        .filter(|activity| !activity.id.starts_with("__"))
        .map(|activity| activity.title.trim())
        .filter(|title| !title.is_empty())
        .collect();
    if !sections.is_empty() {
        if !text.trim().is_empty() {
            text.push(' ');
        }
        text.push_str("Sections: ");
        text.push_str(&sections.join("; "));
        text.push('.');
    }
    (!text.trim().is_empty()).then_some(text)
}

/// Add an explicit screen label to a primary/worker projection. Worker prose
/// currently contains a localized "Designing …" line, but the structured
/// label is the stable source of truth and must survive even when prose is
/// empty or changes wording.
fn design_history_projection(message: &ChatMessage) -> Option<String> {
    let content = history_content(message);
    let screen = message
        .design_worker_screen
        .as_deref()
        .map(str::trim)
        .filter(|screen| !screen.is_empty());
    match (screen, content) {
        (Some(screen), Some(content)) => Some(format!("Screen {screen}: {content}")),
        (Some(screen), None) => Some(format!("Screen {screen}.")),
        (None, content) => content,
    }
}

fn append_history_projection(target: &mut String, projection: &str) {
    if projection.trim().is_empty() {
        return;
    }
    if !target.trim().is_empty() {
        target.push_str("\n\n");
    }
    target.push_str(projection);
}

fn tool_call_defaults_open(call: &ChatToolCall) -> bool {
    if let Some(level) = tool_level_from_args(&call.args) {
        return matches!(level.as_str(), "modify" | "delete" | "orchestrate");
    }
    matches!(
        call.name.as_str(),
        "update_node"
            | "replace_node"
            | "move_node"
            | "set_variables"
            | "set_themes"
            | "load_theme_preset"
            | "rename_page"
            | "reorder_page"
            | "batch_design"
            | "set_design_md"
            | "export_design_md"
            | "delete_node"
            | "remove_page"
    )
}

fn tool_level_from_args(args: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(args)
        .ok()?
        .get("level")?
        .as_str()
        .map(str::to_string)
}

impl ChatSession {
    /// Spawn a worker that drains an explicitly cancellable provider turn into
    /// a channel. Providers that do not override `send_cancellable` retain
    /// their existing behavior through the trait's default delegation.
    pub fn start(provider: Box<dyn ChatProvider>, req: ChatRequest) -> Self {
        Self::start_with_tools(provider, req, None)
    }

    /// [`start`](Self::start) plus a tool-request receiver for providers that
    /// execute host tools.
    pub fn start_with_tools(
        provider: Box<dyn ChatProvider>,
        req: ChatRequest,
        tool_rx: Option<Receiver<ChatToolRequest>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        thread::Builder::new()
            .name("op-chat-turn".into())
            .spawn(move || {
                for delta in provider.send_cancellable(req, worker_cancel) {
                    if tx.send(delta).is_err() {
                        return;
                    }
                }
            })
            .expect("spawn op-chat-turn thread");
        Self {
            rx,
            cancel: Some(cancel),
            tool_rx,
            deferred_tool_requests: VecDeque::new(),
            finished: false,
            is_design_loop: false,
            loop_finalized: false,
            viewport_fitted: false,
        }
    }

    /// Mark this turn as a design-agent loop so the pump stamps tool positions
    /// into the visible narration timeline (see [`apply_poll_to_message_with`]).
    /// Chainable.
    pub fn into_design_loop(mut self) -> Self {
        self.is_design_loop = true;
        self
    }

    /// True when this turn is a design-agent loop.
    pub fn is_design_loop(&self) -> bool {
        self.is_design_loop
    }

    /// Record that the loop's structural finalize pass ran for this turn.
    pub fn mark_loop_finalized(&mut self) {
        self.loop_finalized = true;
    }

    /// Record that the host centered the viewport on this design turn's
    /// first sized root.
    pub fn mark_viewport_fitted(&mut self) {
        self.viewport_fitted = true;
    }

    /// True once the host centered the viewport on this turn's design root.
    pub fn viewport_fitted(&self) -> bool {
        self.viewport_fitted
    }

    /// True when the loop's structural finalize pass already ran. A design
    /// loop that dies early (429 / quota / abort) never sends the finalize
    /// op — the host uses this to run the backstop on teardown (measured:
    /// an aborted run shipped an empty 68px TabBar shell + empty MiniPlayer,
    /// test0711-22).
    pub fn loop_finalized(&self) -> bool {
        self.loop_finalized
    }

    /// Wrap externally supplied channels, used when a host owns its own
    /// routing worker but wants the shared poll/finish behavior.
    pub fn from_channels(
        rx: Receiver<ChatDelta>,
        tool_rx: Option<Receiver<ChatToolRequest>>,
    ) -> Self {
        Self::from_channels_with_optional_cancel(rx, tool_rx, None)
    }

    /// Wrap externally supplied channels while retaining the cancellation
    /// flag owned by the worker behind them. Direct-route launchers use this
    /// so dropping the UI session interrupts a silent provider promptly.
    pub fn from_channels_with_cancel(
        rx: Receiver<ChatDelta>,
        tool_rx: Option<Receiver<ChatToolRequest>>,
        cancel: Arc<AtomicBool>,
    ) -> Self {
        Self::from_channels_with_optional_cancel(rx, tool_rx, Some(cancel))
    }

    fn from_channels_with_optional_cancel(
        rx: Receiver<ChatDelta>,
        tool_rx: Option<Receiver<ChatToolRequest>>,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Self {
        Self {
            rx,
            cancel,
            tool_rx,
            deferred_tool_requests: VecDeque::new(),
            finished: false,
            is_design_loop: false,
            loop_finalized: false,
            viewport_fitted: false,
        }
    }

    /// Drain every delta ready right now without blocking.
    pub fn poll(&mut self) -> ChatPoll {
        let mut text = String::new();
        let mut thinking = String::new();
        let mut tool_calls = Vec::new();
        let mut error = None;
        loop {
            match self.rx.try_recv() {
                Ok(ChatDelta::TextDelta(s)) => text.push_str(&s),
                Ok(ChatDelta::Thinking(s)) => thinking.push_str(&s),
                Ok(ChatDelta::ToolUse { name, args }) => {
                    tool_calls.push(ChatToolCall {
                        name,
                        args,
                        // Preserve this call's exact position among text
                        // deltas drained by the same poll. Thinking remains in
                        // its compatible aggregate field and does not affect
                        // visible narration offsets.
                        content_offset: Some(text.len() as u32),
                    });
                }
                Ok(ChatDelta::Error(msg)) => {
                    if error.is_none() {
                        error = Some(msg);
                    }
                }
                Ok(ChatDelta::Done { .. }) => self.finished = true,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
            }
        }
        ChatPoll {
            text,
            thinking,
            tool_calls,
            error,
            finished: self.finished,
        }
    }

    /// Drain pending host tool requests without blocking.
    pub fn drain_tool_requests(&mut self) -> Vec<ChatToolRequest> {
        let mut requests: Vec<ChatToolRequest> = self.deferred_tool_requests.drain(..).collect();
        let Some(tool_rx) = self.tool_rx.as_ref() else {
            return requests;
        };
        while let Ok(req) = tool_rx.try_recv() {
            requests.push(req);
        }
        requests
    }

    pub fn defer_tool_request(&mut self, req: ChatToolRequest) {
        self.deferred_tool_requests.push_back(req);
    }

    pub fn finished(&self) -> bool {
        self.finished
    }
}

impl Drop for ChatSession {
    fn drop(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Release);
        }
    }
}
