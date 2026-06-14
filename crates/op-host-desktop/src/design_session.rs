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

use std::sync::mpsc::{self, Sender};
use std::thread;

use op_editor_core::{DocRect, EditorState, Viewport};
pub use op_editor_host_core::design::{
    DesignCmdAck, DesignCmdOp, DesignCmdReq, DesignDelta, DesignSession, RemoteDocSink,
};
use op_editor_ui::widgets::TOP_BAR_HEIGHT;
use op_host_native::WidgetHostNative;
use op_orchestrator::{
    AbortFlag, DesignRequest, LlmClient, Orchestrator, Progress, SkippedScreenshotProvider,
    SkippedVisionLlmClient, ValidationProviders,
};

use crate::chat_runtime::shared_runtime;
use crate::pre_validator::LintPreValidator;

/// Spawn a worker that runs `Orchestrator::run` against a `RemoteDocSink`.
pub fn start<L: LlmClient + Send + 'static>(
    llm: L,
    request: DesignRequest,
    initial_state: EditorState,
) -> DesignSession {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();

    let indicator_epoch = op_editor_core::agent_indicators::begin();

    thread::Builder::new()
        .name("op-design-turn".into())
        .spawn(move || {
            run_design_worker(
                llm,
                request,
                initial_state,
                delta_tx,
                cmd_tx,
                indicator_epoch,
            )
        })
        .expect("spawn op-design-turn thread");

    DesignSession::from_channels_with_epoch(delta_rx, cmd_rx, indicator_epoch)
}

/// One full design turn against a `RemoteDocSink` — the body of
/// [`DesignSession::start`]'s worker thread, callable directly by the
/// CLI intent router's worker (which already runs off the UI thread).
pub(crate) fn run_design_worker<L: LlmClient + Send>(
    llm: L,
    request: DesignRequest,
    initial_state: EditorState,
    delta_tx: Sender<DesignDelta>,
    cmd_tx: Sender<DesignCmdReq>,
    indicator_epoch: u64,
) {
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
    let summary = shared_runtime().block_on(
        Orchestrator::new()
            .with_indicator_epoch(indicator_epoch)
            .run(
                request,
                &mut sink,
                &llm,
                &mut on_progress,
                &abort,
                &providers,
            ),
    );
    let _ = delta_tx.send(DesignDelta::Done(summary));
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
            msg.thinking.push_str(&appended);
            msg.thinking_collapsed = false;
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
#[path = "design_session_tests.rs"]
mod tests;
