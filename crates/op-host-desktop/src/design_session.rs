//! Background design-turn runner — orchestrator counterpart of [`ChatSession`](crate::chat_session).
//!
//! The orchestrator (`op_orchestrator::Orchestrator::run`) is `async`,
//! takes `&mut sink` (the `DocSink` trait — synchronous read + write
//! against `EditorState`), and runs to completion across multiple
//! `apply()` calls during scaffold → subtasks → cleanup. Two competing
//! constraints shape the threading model:
//!
//! - **UI owns the canonical `EditorState`.** `EditorCommand::apply`
//!   does ID remapping + history bookkeeping that must run on the UI
//!   thread (see `command_apply.rs`).
//! - **Don't freeze the UI for the whole turn.** A design turn can take
//!   10+ seconds; `block_on(run(...))` on the UI thread would lock the
//!   window during that span.
//!
//! Resolution: the worker thread owns a **`RemoteDocSink`** that
//! forwards each `apply(cmd)` over an mpsc channel to the UI thread,
//! which `apply()`s on the real state and replies with an ack carrying
//! a fresh `EditorState` snapshot. `RemoteDocSink::state()` reads from
//! a locally cached mirror updated by each ack. The orchestrator never
//! sees the channel — it just calls `sink.apply()` synchronously, and
//! the worker's `apply` blocks until UI acks.
//!
//! Progress events emitted by the orchestrator (`Planning`,
//! `SubtaskStarted`, etc.) ride a separate channel into the chat
//! transcript, mirroring `ChatSession`'s delta channel.
//!
//! ## Lifecycle
//!
//! 1. Caller (`chat_session::launch_if_pending`) classifies intent.
//!    For `Intent::Design` + a configured `agent::Provider`, builds a
//!    `DesignSession` via [`DesignSession::start`].
//! 2. `start` clones the current `EditorState` for the worker's
//!    initial mirror and spawns the worker thread. The worker calls
//!    `block_on(Orchestrator::new().run(...))` against its
//!    `RemoteDocSink`.
//! 3. UI event loop drains pending `DesignCmdReq` each frame via
//!    [`pump_commands`] — applies on the real state, replies ack.
//! 4. UI event loop also drains `DesignDelta` via [`pump_progress`]
//!    and renders progress into the trailing chat bubble.
//! 5. On `Done`, the session is dropped and the channels close.
//!
//! Aborting a turn drops `DesignSession`; the worker's next `apply`
//! sees the channel closed and returns `false`, ending the turn.

use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::thread;

use op_editor_core::Viewport;
use op_editor_core::{DocRect, EditorCommand, EditorState};
use op_editor_ui::widgets::TOP_BAR_HEIGHT;
use op_host_native::WidgetHostNative;
use op_orchestrator::{
    AbortFlag, DesignRequest, DocSink, LlmClient, Orchestrator, OrchestratorError, Progress,
    RunSummary, SkippedScreenshotProvider, SkippedVisionLlmClient, ValidationProviders,
};

use crate::chat_runtime::shared_runtime;
use crate::pre_validator::LintPreValidator;

/// One in-flight design turn.
pub struct DesignSession {
    delta_rx: Receiver<DesignDelta>,
    cmd_rx: Receiver<DesignCmdReq>,
    finished: bool,
}

/// Progress / completion events emitted by the worker.
pub enum DesignDelta {
    /// One `op_orchestrator::Progress` event.
    Progress(Progress),
    /// Terminal event — the orchestrator returned. The session is
    /// finished once this arrives.
    Done(Result<RunSummary, OrchestratorError>),
}

/// Request from worker to UI to apply one editor mutation (or undo-batch
/// boundary). The worker blocks until the matching ack arrives.
pub struct DesignCmdReq {
    pub op: DesignCmdOp,
    pub ack: SyncSender<DesignCmdAck>,
}

/// What the worker is asking the UI to do.
pub enum DesignCmdOp {
    Apply(EditorCommand),
    BeginUndoBatch,
    EndUndoBatch,
}

/// UI's reply to one [`DesignCmdReq`]. Carries an `EditorState` clone
/// so the worker's mirror reflects ID-remapped state.
pub struct DesignCmdAck {
    pub applied: bool,
    pub new_state: EditorState,
}

/// Result of one non-blocking progress drain.
pub struct DesignPoll {
    pub progress: Vec<Progress>,
    /// Terminal summary when the turn ended; `None` while running.
    pub summary: Option<Result<RunSummary, OrchestratorError>>,
    pub finished: bool,
}

impl DesignSession {
    /// Spawn a worker that runs `Orchestrator::run` against a
    /// `RemoteDocSink`. Returns immediately; the LLM turn streams off
    /// the UI thread.
    ///
    /// `llm` is any `LlmClient` implementation — production code passes
    /// a `ChatProviderLlmClient` wrapping the user's currently-selected
    /// chat agent (Claude Code / Copilot / Gemini), so the orchestrator
    /// rides whatever CLI auth the chat panel already has.
    pub fn start<L: LlmClient + Send + 'static>(
        llm: L,
        request: DesignRequest,
        initial_state: EditorState,
    ) -> Self {
        let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();

        thread::Builder::new()
            .name("op-design-turn".into())
            .spawn(move || {
                let mut sink = RemoteDocSink::new(cmd_tx, initial_state);
                let abort = AbortFlag::new();
                let pre_validator = LintPreValidator;
                let screenshot = SkippedScreenshotProvider;
                let vision = SkippedVisionLlmClient;
                let providers = ValidationProviders {
                    pre_validator: &pre_validator,
                    screenshot: &screenshot,
                    vision: &vision,
                    system_prompt: String::new(),
                };
                let delta_tx_for_progress = delta_tx.clone();
                let mut on_progress = move |p: Progress| {
                    let _ = delta_tx_for_progress.send(DesignDelta::Progress(p));
                };
                let summary = shared_runtime().block_on(Orchestrator::new().run(
                    request,
                    &mut sink,
                    &llm,
                    &mut on_progress,
                    &abort,
                    &providers,
                ));
                let _ = delta_tx.send(DesignDelta::Done(summary));
            })
            .expect("spawn op-design-turn thread");

        Self {
            delta_rx,
            cmd_rx,
            finished: false,
        }
    }

    /// Drain every progress delta ready right now. Non-blocking.
    pub fn poll_progress(&mut self) -> DesignPoll {
        let mut progress = Vec::new();
        let mut summary = None;
        loop {
            match self.delta_rx.try_recv() {
                Ok(DesignDelta::Progress(p)) => progress.push(p),
                Ok(DesignDelta::Done(r)) => {
                    self.finished = true;
                    summary = Some(r);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
            }
        }
        DesignPoll {
            progress,
            summary,
            finished: self.finished,
        }
    }

    /// Drain every pending apply request. Returns the requests; the
    /// caller must ack each one or the worker will hang on `recv`.
    pub fn drain_cmd_requests(&mut self) -> Vec<DesignCmdReq> {
        let mut out = Vec::new();
        loop {
            match self.cmd_rx.try_recv() {
                Ok(req) => out.push(req),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }

    /// Test-only ctor — wraps externally-supplied channels so a fake
    /// worker thread can drive the UI-side pumps end-to-end without
    /// spinning up a real LLM. Production code goes through
    /// [`DesignSession::start`].
    #[cfg(test)]
    pub fn from_channels(delta_rx: Receiver<DesignDelta>, cmd_rx: Receiver<DesignCmdReq>) -> Self {
        Self {
            delta_rx,
            cmd_rx,
            finished: false,
        }
    }
}

/// Worker-side `DocSink` impl — forwards every mutation to the UI
/// thread over an mpsc channel and blocks on the ack. State reads
/// come from a locally cached mirror updated by each ack.
pub struct RemoteDocSink {
    cmd_tx: Sender<DesignCmdReq>,
    mirror: EditorState,
}

impl RemoteDocSink {
    pub fn new(cmd_tx: Sender<DesignCmdReq>, initial_state: EditorState) -> Self {
        Self {
            cmd_tx,
            mirror: initial_state,
        }
    }

    fn send_and_wait(&mut self, op: DesignCmdOp) -> bool {
        let (ack_tx, ack_rx) = mpsc::sync_channel::<DesignCmdAck>(1);
        let req = DesignCmdReq { op, ack: ack_tx };
        if self.cmd_tx.send(req).is_err() {
            return false; // UI dropped the receiver — turn aborted
        }
        match ack_rx.recv() {
            Ok(ack) => {
                self.mirror = ack.new_state;
                ack.applied
            }
            Err(_) => false,
        }
    }
}

impl DocSink for RemoteDocSink {
    fn state(&self) -> &EditorState {
        &self.mirror
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.send_and_wait(DesignCmdOp::Apply(cmd))
    }

    fn begin_undo_batch(&mut self) {
        let _ = self.send_and_wait(DesignCmdOp::BeginUndoBatch);
    }

    fn end_undo_batch(&mut self) {
        let _ = self.send_and_wait(DesignCmdOp::EndUndoBatch);
    }
}

/// Drain every pending apply request from the in-flight design
/// session and execute it against the real `EditorState`. Each
/// request gets an ack containing a fresh state snapshot so the
/// worker's mirror reflects ID-remapping. Returns true when at least
/// one command applied (caller should mark redraw dirty).
pub fn pump_commands(
    host: &mut WidgetHostNative,
    current: &mut Option<DesignSession>,
    viewport_width: f32,
    viewport_height: f32,
) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    let reqs = session.drain_cmd_requests();
    if reqs.is_empty() {
        return false;
    }
    let state = host.editor_state_mut();
    let mut any_applied = false;
    for req in reqs {
        let applied = match req.op {
            DesignCmdOp::Apply(cmd) => {
                let applied = state.apply(cmd);
                if applied {
                    fit_design_viewport_to_content(state, viewport_width, viewport_height);
                }
                applied
            }
            // TODO(host): wire into op-editor-core history batch mode
            // once available. Today undo-batch boundaries are no-ops so
            // each `EditorCommand::InsertSubtree` is its own undo step —
            // functionally correct, just finer-grained than ideal.
            DesignCmdOp::BeginUndoBatch | DesignCmdOp::EndUndoBatch => true,
        };
        let snapshot = state.clone();
        let ack = DesignCmdAck {
            applied,
            new_state: snapshot,
        };
        // If the ack fails to send, the worker already dropped its
        // receiver (e.g. turn aborted) — nothing to do here.
        let _ = req.ack.send(ack);
        if applied {
            any_applied = true;
        }
    }
    if any_applied {
        host.mark_editor_state_dirty();
    }
    any_applied
}

const DESIGN_FIT_PADDING: f32 = 48.0;

/// Keep the generated design centered and fully visible while the
/// orchestrator progressively applies scaffold/subtask nodes.
fn fit_design_viewport_to_content(
    state: &mut EditorState,
    viewport_width: f32,
    viewport_height: f32,
) -> bool {
    let Some(bounds) = active_content_bounds(state) else {
        return false;
    };
    let (canvas_w, canvas_h) = design_canvas_size(state, viewport_width, viewport_height);
    if canvas_w <= 1.0 || canvas_h <= 1.0 {
        return false;
    }

    let pad_x = DESIGN_FIT_PADDING.min(canvas_w / 4.0);
    let pad_y = DESIGN_FIT_PADDING.min(canvas_h / 4.0);
    let fit_w = (canvas_w - pad_x * 2.0).max(1.0);
    let fit_h = (canvas_h - pad_y * 2.0).max(1.0);
    let content_w = (bounds.w as f32).max(1.0);
    let content_h = (bounds.h as f32).max(1.0);
    let zoom = (fit_w / content_w)
        .min(fit_h / content_h)
        .clamp(Viewport::MIN_ZOOM, Viewport::MAX_ZOOM);
    let center_x = (bounds.x + bounds.w / 2.0) as f32;
    let center_y = (bounds.y + bounds.h / 2.0) as f32;
    let next_pan_x = canvas_w / 2.0 - center_x * zoom;
    let next_pan_y = canvas_h / 2.0 - center_y * zoom;

    let changed = (state.viewport.zoom - zoom).abs() > 0.001
        || (state.viewport.pan_x - next_pan_x).abs() > 0.5
        || (state.viewport.pan_y - next_pan_y).abs() > 0.5;
    state.viewport.zoom = zoom;
    state.viewport.pan_x = next_pan_x;
    state.viewport.pan_y = next_pan_y;
    changed
}

fn active_content_bounds(state: &EditorState) -> Option<DocRect> {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let page = scene.active_page()?;
    let mut iter = page
        .children
        .iter()
        .map(|node| node.aggregate_bounds())
        .filter(|rect| rect.size.x > 0.0 || rect.size.y > 0.0);
    let first = iter.next()?;
    let (mut min_x, mut min_y) = (first.origin.x, first.origin.y);
    let (mut max_x, mut max_y) = (first.origin.x + first.size.x, first.origin.y + first.size.y);
    for rect in iter {
        min_x = min_x.min(rect.origin.x);
        min_y = min_y.min(rect.origin.y);
        max_x = max_x.max(rect.origin.x + rect.size.x);
        max_y = max_y.max(rect.origin.y + rect.size.y);
    }
    Some(DocRect {
        x: min_x as f64,
        y: min_y as f64,
        w: (max_x - min_x) as f64,
        h: (max_y - min_y) as f64,
    })
}

fn design_canvas_size(
    state: &EditorState,
    viewport_width: f32,
    viewport_height: f32,
) -> (f32, f32) {
    let canvas_left = if state.editor_ui.sidebar_open {
        state.editor_ui.layer_panel_width
    } else {
        0.0
    };
    let canvas_right = if state.right_rail_visible() {
        viewport_width - state.editor_ui.property_panel_width
    } else {
        viewport_width
    };
    (
        (canvas_right - canvas_left).max(0.0),
        (viewport_height - TOP_BAR_HEIGHT).max(0.0),
    )
}

/// Drain every pending progress delta and fold it into the trailing
/// assistant message. Clears `current` once the terminal `Done`
/// arrives. Returns true when the transcript changed.
pub fn pump_progress(host: &mut WidgetHostNative, current: &mut Option<DesignSession>) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    let poll = session.poll_progress();
    let mut changed = false;
    if !poll.progress.is_empty() {
        let appended = render_progress(&poll.progress);
        let chat = &mut host.editor_state_mut().chat;
        if let Some(msg) = chat.messages.last_mut() {
            msg.content.push_str(&appended);
            changed = true;
        }
    }
    if let Some(summary) = &poll.summary {
        let chat = &mut host.editor_state_mut().chat;
        if let Some(msg) = chat.messages.last_mut() {
            match summary {
                Ok(s) => {
                    let ok = s.subtasks.iter().filter(|o| o.error.is_none()).count();
                    let failed = s.subtasks.len() - ok;
                    msg.content.push_str(&format!(
                        "\n\nDone — {} subtask(s) succeeded, {} failed, {} node(s) total.",
                        ok, failed, s.total_nodes,
                    ));
                }
                Err(e) => {
                    msg.content = format!("error: {e}");
                }
            }
            msg.streaming = false;
            changed = true;
        }
    }
    if changed {
        host.mark_editor_state_dirty();
    }
    if poll.finished {
        *current = None;
    }
    changed
}

/// Render a list of `Progress` events into a human-readable line block
/// the chat transcript can append. Matches the spirit of TS
/// `apps/web/src/services/ai/visual-ref-orchestrator.ts` step labels.
fn render_progress(progress: &[Progress]) -> String {
    let mut out = String::new();
    for p in progress {
        out.push('\n');
        out.push_str(&progress_label(p));
    }
    out
}

fn progress_label(p: &Progress) -> String {
    match p {
        Progress::Planning => "• Planning…".into(),
        Progress::ScaffoldDone => "• Scaffold ready".into(),
        Progress::SubtaskStarted { id, label } => format!("• Subtask `{id}` — {label}"),
        Progress::SubtaskDone { id, node_count } => {
            format!("• Subtask `{id}` done ({node_count} nodes)")
        }
        Progress::SubtaskFailed { id, error } => format!("• Subtask `{id}` failed: {error}"),
        Progress::CleanupDone => "• Cleanup done".into(),
        Progress::ValidationStarted => "• Validation started".into(),
        Progress::ValidationPreCheckDone { applied, .. } => {
            format!("• Pre-validation applied {applied} fix(es)")
        }
        Progress::ValidationRoundStarted { round } => {
            format!("• Vision round {round} started")
        }
        Progress::ValidationRoundDone {
            round,
            applied,
            quality_score,
        } => {
            format!("• Vision round {round} done — {applied} fix(es), quality {quality_score}/100")
        }
        Progress::ValidationDone { total_applied } => {
            format!("• Validation done — {total_applied} fix(es) total")
        }
        Progress::VisualRefStarted => "• Visual-ref pipeline started".into(),
        Progress::VisualRefDesignSystem { var_count } => {
            format!("• Design system ready — {var_count} variable(s) seeded")
        }
        Progress::VisualRefHtmlGenerated { byte_len } => {
            format!("• Visual-ref HTML generated ({byte_len} bytes)")
        }
        Progress::VisualRefScreenshotReady { skipped } => {
            if *skipped {
                "• Visual-ref screenshot skipped".into()
            } else {
                "• Visual-ref screenshot captured".into()
            }
        }
        Progress::VisualRefFallback { reason } => format!("• Visual-ref fallback: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::EditorCommand;
    use op_orchestrator::{RunSummary, SubtaskOutcome};
    use std::time::{Duration, Instant};

    /// `RemoteDocSink::apply` blocks until UI acks. When the UI side
    /// drops the receiver, `apply` returns false instead of hanging.
    #[test]
    fn remote_doc_sink_returns_false_when_ui_channel_closed() {
        let (tx, rx) = mpsc::channel::<DesignCmdReq>();
        let mut sink = RemoteDocSink::new(tx, EditorState::new());
        drop(rx); // simulate UI session dropped before the worker called apply
        let applied = sink.apply(EditorCommand::ClearSelection);
        assert!(!applied, "apply on closed channel must return false");
    }

    /// Happy-path round-trip: worker sends an apply request; UI thread
    /// acks with an updated state snapshot; worker's mirror reflects it.
    #[test]
    fn remote_doc_sink_updates_mirror_on_ack() {
        let (tx, rx) = mpsc::channel::<DesignCmdReq>();
        let initial = EditorState::new();
        let mut sink = RemoteDocSink::new(tx, initial.clone());

        // Spawn UI-side faker that acks one request with a modified state.
        let ui_thread = thread::spawn(move || {
            let req = rx.recv().expect("worker should send one request");
            let mut new_state = initial.clone();
            // Mutate something the test can observe — viewport zoom.
            new_state.viewport.zoom = 2.0;
            let ack = DesignCmdAck {
                applied: true,
                new_state,
            };
            req.ack.send(ack).expect("ack must reach worker");
        });

        let applied = sink.apply(EditorCommand::ClearSelection);
        ui_thread.join().expect("ui thread must finish");
        assert!(applied, "ack reported applied=true");
        assert_eq!(
            sink.state().viewport.zoom,
            2.0,
            "mirror should reflect ack snapshot"
        );
    }

    /// `BeginUndoBatch` and `EndUndoBatch` are forwarded as their own
    /// `DesignCmdOp` variants so the UI can route them through the
    /// real `History::begin_batch` / `end_batch` once wired.
    #[test]
    fn undo_batch_signals_are_distinguishable_on_the_wire() {
        let (tx, rx) = mpsc::channel::<DesignCmdReq>();
        let mut sink = RemoteDocSink::new(tx, EditorState::new());
        let ui = thread::spawn(move || {
            let mut kinds = Vec::new();
            while let Ok(req) = rx.recv() {
                let label = match req.op {
                    DesignCmdOp::Apply(_) => "apply",
                    DesignCmdOp::BeginUndoBatch => "begin",
                    DesignCmdOp::EndUndoBatch => "end",
                };
                kinds.push(label.to_string());
                let _ = req.ack.send(DesignCmdAck {
                    applied: true,
                    new_state: EditorState::new(),
                });
            }
            kinds
        });
        sink.begin_undo_batch();
        sink.apply(EditorCommand::ClearSelection);
        sink.end_undo_batch();
        drop(sink); // close the channel so the ui-side recv loop exits
        let kinds = ui.join().expect("ui thread finishes");
        assert_eq!(kinds, vec!["begin", "apply", "end"]);
    }

    /// End-to-end smoke through `pump_commands` + `pump_progress`:
    /// a fake worker thread drives a `RemoteDocSink` against
    /// real-looking channels, the UI loop drains both pumps, and we
    /// assert that the chat bubble carries the rendered progress +
    /// terminal summary line, and that the session clears itself
    /// after `Done`.
    ///
    /// This is the host-side complement to the orchestrator's own
    /// end-to-end tests — it exercises the actor seam without
    /// requiring an `agent::Provider` / `ANTHROPIC_API_KEY`. Task #28
    /// covers the live LLM smoke separately.
    #[test]
    fn end_to_end_pump_round_trips_apply_and_progress_via_actor_channels() {
        let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
        let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
        let mut host = WidgetHostNative::new();
        // Seed a streaming assistant bubble — `chat.begin_send`
        // creates one in production; the pumps fold the worker's
        // progress + summary into it.
        host.editor_state_mut()
            .chat
            .messages
            .push(op_editor_core::ChatMessage::assistant_streaming());

        // Fake worker — emits one progress event, asks UI to apply
        // ClearSelection, then a successful `Done`.
        let fake_worker = thread::spawn(move || {
            // Progress first so the bubble starts streaming text
            // before the doc mutation.
            let _ = delta_tx.send(DesignDelta::Progress(Progress::Planning));
            let mut sink = RemoteDocSink::new(cmd_tx, EditorState::new());
            sink.apply(EditorCommand::ClearSelection);
            let _ = delta_tx.send(DesignDelta::Done(Ok(RunSummary {
                root_frame_id: "root".into(),
                subtasks: vec![SubtaskOutcome {
                    id: "s1".into(),
                    node_count: 3,
                    error: None,
                }],
                total_nodes: 3,
            })));
            // Hold the sink so its channel survives until the UI has
            // had a chance to drain (the test polls until `Done`).
            sink
        });

        // UI drives the pumps until the session clears (mirrors the
        // event-loop `RedrawRequested` block). Bound the loop with a
        // timeout so a hung worker fails the test instead of hanging.
        let deadline = Instant::now() + Duration::from_secs(5);
        while current.is_some() && Instant::now() < deadline {
            let _ = pump_commands(&mut host, &mut current, 1440.0, 900.0);
            let _ = pump_progress(&mut host, &mut current);
            if current.is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        // Worker can join now — the sink it returned (and thus its
        // cmd_tx) drops at this scope end.
        let _ = fake_worker.join().expect("fake worker exits cleanly");

        assert!(
            current.is_none(),
            "session must clear after Done — leaving it set would keep the\
             event loop ticking and pump_progress retrying"
        );
        let bubble = host
            .editor_state()
            .chat
            .messages
            .last()
            .expect("seeded bubble survives");
        assert!(
            bubble.content.contains("Planning"),
            "progress line should render Planning, got: {:?}",
            bubble.content
        );
        assert!(
            bubble.content.contains("1 subtask"),
            "summary should report 1 subtask succeeded, got: {:?}",
            bubble.content
        );
        assert!(
            !bubble.streaming,
            "summary path must clear streaming so the chat panel stops the animation"
        );
    }

    #[test]
    fn fit_design_viewport_centers_and_fits_mobile_root() {
        let mut state = EditorState::new();
        state.doc.children = vec![mobile_root()];

        assert!(fit_design_viewport_to_content(&mut state, 1440.0, 900.0));

        let bounds = active_content_bounds(&state).expect("root bounds");
        let (canvas_w, canvas_h) = design_canvas_size(&state, 1440.0, 900.0);
        let left = state.viewport.pan_x + bounds.x as f32 * state.viewport.zoom;
        let top = state.viewport.pan_y + bounds.y as f32 * state.viewport.zoom;
        let right = left + bounds.w as f32 * state.viewport.zoom;
        let bottom = top + bounds.h as f32 * state.viewport.zoom;
        let center_x = (left + right) / 2.0;
        let center_y = (top + bottom) / 2.0;

        assert!(left >= 0.0, "left edge should be visible, got {left}");
        assert!(top >= 0.0, "top edge should be visible, got {top}");
        assert!(
            right <= canvas_w,
            "right edge should be visible: {right} > {canvas_w}"
        );
        assert!(
            bottom <= canvas_h,
            "bottom edge should be visible: {bottom} > {canvas_h}"
        );
        assert!((center_x - canvas_w / 2.0).abs() < 0.5);
        assert!((center_y - canvas_h / 2.0).abs() < 0.5);
    }

    #[test]
    fn pump_commands_refits_viewport_after_design_insert() {
        let (_delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
        let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().doc.children.clear();
        let before = host.editor_state().viewport;

        let (ack_tx, ack_rx) = mpsc::sync_channel::<DesignCmdAck>(1);
        cmd_tx
            .send(DesignCmdReq {
                op: DesignCmdOp::Apply(EditorCommand::InsertSubtree {
                    nodes: vec![mobile_root()],
                    parent_id: op_editor_core::NodeId::NONE,
                }),
                ack: ack_tx,
            })
            .expect("request should queue");

        assert!(pump_commands(&mut host, &mut current, 1440.0, 900.0));
        let ack = ack_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("pump should ack apply request");
        assert!(ack.applied);
        assert!(
            !ack.new_state.doc.children.is_empty(),
            "ack snapshot should include inserted root"
        );
        assert_eq!(
            host.editor_state().doc.children.len(),
            1,
            "host state should receive inserted root"
        );

        let after = host.editor_state().viewport;
        assert_ne!(before, after, "design insert should refit viewport");
        assert!(
            (after.zoom - 0.905).abs() < 0.01,
            "mobile root should fit viewport height, got zoom {}",
            after.zoom
        );
    }

    #[test]
    fn fit_design_viewport_uses_resolved_layout_for_fit_content_root() {
        let mut state = EditorState::new();
        state.doc.children = vec![mobile_fit_content_root()];

        assert!(fit_design_viewport_to_content(&mut state, 1440.0, 900.0));

        let bounds = active_content_bounds(&state).expect("resolved root bounds");
        assert!(
            (bounds.h - 844.0).abs() < 1.0,
            "fit_content root should resolve to full mobile height, got {}",
            bounds.h
        );
        assert!(
            (state.viewport.zoom - 0.905).abs() < 0.01,
            "full mobile root should remain fully visible, got zoom {}",
            state.viewport.zoom
        );
    }

    fn mobile_root() -> jian_ops_schema::node::PenNode {
        serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "x": 80,
            "y": 40,
            "width": 390,
            "height": 844,
            "children": []
        }))
        .expect("mobile root fixture parses")
    }

    fn mobile_fit_content_root() -> jian_ops_schema::node::PenNode {
        serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "x": 80,
            "y": 40,
            "width": 390,
            "height": "fit_content",
            "layout": "vertical",
            "gap": 0,
            "children": [
                {"type": "frame", "id": "status", "name": "Status Bar", "width": "fill_container", "height": 32},
                {"type": "frame", "id": "header", "name": "Header", "width": "fill_container", "height": 92},
                {"type": "frame", "id": "search", "name": "Search", "width": "fill_container", "height": 104},
                {"type": "frame", "id": "promo", "name": "Promo", "width": "fill_container", "height": 132},
                {"type": "frame", "id": "categories", "name": "Categories", "width": "fill_container", "height": 86},
                {"type": "frame", "id": "restaurants", "name": "Restaurants", "width": "fill_container", "height": 314},
                {"type": "frame", "id": "bottom-nav", "name": "Bottom Nav", "width": "fill_container", "height": 84}
            ]
        }))
        .expect("fit_content mobile root fixture parses")
    }
}
