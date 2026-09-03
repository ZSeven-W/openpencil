//! Background design-turn worker + viewport-fit math — the host-free
//! half carved out of `op-host-desktop`'s `design_session.rs` (the GUI
//! pumps `pump_commands` / `pump_progress` stay desktop-side).
//!
//! The orchestrator (`op_orchestrator::Orchestrator::run`) is `async`,
//! takes `&mut sink` (the `DocSink` trait — synchronous read + write
//! against `EditorState`), and runs to completion across multiple
//! `apply()` calls during scaffold → subtasks → cleanup. The worker
//! thread owns a `RemoteDocSink` that forwards each `apply(cmd)` over an
//! mpsc channel to the UI thread, which `apply()`s on the real state and
//! replies with an ack carrying a fresh `EditorState` snapshot. The UI
//! drains those requests via the desktop residual's `pump_commands`, and
//! progress deltas via `pump_progress`.

use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;

use op_ai::chat_provider::ChatProvider;
use op_editor_core::{DocRect, EditorCommand, EditorState, Viewport};
pub use op_editor_host_core::design::{DesignCmdReq, DesignDelta, DesignSession, RemoteDocSink};
use op_editor_ui::widgets::TOP_BAR_HEIGHT;
use op_orchestrator::{
    AbortFlag, DesignRequest, DocSink, LlmClient, Orchestrator, Progress,
    SkippedScreenshotProvider, SkippedVisionLlmClient, SpawnAgentResult, SpawnAgentSpec,
    ValidationProviders,
};

use crate::chat_runtime::block_on_anywhere;
use crate::pre_validator::LintPreValidator;
use crate::validation_providers::{
    validation_system_prompt, vision_validation_enabled, ChatVisionLlmClient,
    RealScreenshotProvider,
};

/// Spawn a worker that runs `Orchestrator::run` against a `RemoteDocSink`.
///
/// `vision_provider` (when `Some` AND `OPENPENCIL_VISION_VALIDATION=1`)
/// drives the REAL Class-C vision-validation loop; `None` / flag-off keeps
/// the no-op stubs so the default path is unchanged. Pass the same
/// `Arc<dyn ChatProvider>` that backs the design `llm` so the vision call
/// reuses the user's selected auth/model.
pub fn start<L: LlmClient + Send + 'static>(
    llm: L,
    request: DesignRequest,
    initial_state: EditorState,
    vision_provider: Option<Arc<dyn ChatProvider>>,
) -> DesignSession {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();

    let indicator_epoch = op_editor_core::agent_indicators::begin();
    let abort = AbortFlag::new();
    let worker_abort = abort.clone();

    let failure_tx = delta_tx.clone();
    if let Err(err) = thread::Builder::new()
        .name("op-design-turn".into())
        .spawn(move || {
            run_design_worker(
                llm,
                request,
                initial_state,
                delta_tx,
                cmd_tx,
                indicator_epoch,
                worker_abort,
                vision_provider,
            )
        })
    {
        // Thread creation can fail under FD/memory pressure; surface a failed
        // turn instead of panicking the UI thread.
        eprintln!("[design-session] failed to spawn op-design-turn thread: {err}");
        op_editor_core::agent_indicators::end_if_epoch(indicator_epoch);
        let _ = failure_tx.send(DesignDelta::Done(Err(
            op_orchestrator::OrchestratorError::Internal(format!(
                "failed to spawn design worker thread: {err}"
            )),
        )));
    }

    DesignSession::from_channels_with_epoch_and_abort(delta_rx, cmd_rx, indicator_epoch, abort)
}

/// One full design turn against a `RemoteDocSink` — the body of
/// [`start`]'s worker thread, callable directly by the CLI intent
/// router's worker (which already runs off the UI thread).
#[allow(clippy::too_many_arguments)]
pub fn run_design_worker<L: LlmClient + Send>(
    llm: L,
    request: DesignRequest,
    initial_state: EditorState,
    delta_tx: Sender<DesignDelta>,
    cmd_tx: Sender<DesignCmdReq>,
    indicator_epoch: u64,
    abort: AbortFlag,
    vision_provider: Option<Arc<dyn ChatProvider>>,
) {
    let mut sink = RemoteDocSink::new(cmd_tx, initial_state);
    let pre_validator = LintPreValidator;

    // ── Class-C vision-validation provider selection (Track-1 Step 3) ──────────
    // REAL providers only when a vision `ChatProvider` was supplied AND
    // `OPENPENCIL_VISION_VALIDATION=1` (defaults OFF); otherwise the no-op
    // stubs keep `run_post_generation_validation` a guaranteed short-circuit
    // so the default path is byte-for-byte unchanged.
    let real = vision_provider
        .filter(|_| vision_validation_enabled())
        .map(|p| {
            (
                RealScreenshotProvider,
                ChatVisionLlmClient::new(p).with_model(request.model.clone()),
                validation_system_prompt(),
            )
        });
    let stub_screenshot = SkippedScreenshotProvider;
    let stub_vision = SkippedVisionLlmClient;
    let (screenshot, vision, system_prompt): (
        &dyn op_orchestrator::ScreenshotProvider,
        &dyn op_orchestrator::VisionLlmClient,
        String,
    ) = match &real {
        Some((shot, vis, prompt)) => (shot, vis, prompt.clone()),
        None => (&stub_screenshot, &stub_vision, String::new()),
    };
    let providers = ValidationProviders {
        pre_validator: &pre_validator,
        screenshot,
        vision,
        system_prompt,
    };
    let summary = {
        // Keep progress and completion on one sender handle so the worker's
        // terminal event cannot overtake its final queued progress update.
        let mut on_progress = |p: Progress| {
            let _ = delta_tx.send(DesignDelta::Progress(p));
        };
        block_on_anywhere(async {
            let mut request = request;
            let reference_used = match crate::reference_context::resolve_reference_context(
                &llm,
                &request.prompt,
                request.model.clone(),
                request.provider.clone(),
                &abort,
            )
            .await
            {
                Ok(Some(context)) => {
                    request.reference_skeleton = Some(context.skeleton);
                    request.design_md = Some(context.design_md.clone());
                    let _ = sink.apply(EditorCommand::SetDesignMd {
                        spec: Box::new(context.design_md),
                    });
                    true
                }
                Ok(None) => false,
                Err(error) => {
                    let reason = format!("reference page could not be used: {error}");
                    eprintln!("[design-session] {reason}");
                    on_progress(Progress::ReferenceUnavailable {
                        reason: reason.clone(),
                    });
                    false
                }
            };
            if !reference_used {
                maybe_generate_design_md_for_follow_on_screen(
                    &llm,
                    &mut request,
                    &mut sink,
                    &abort,
                )
                .await;
            }
            Orchestrator::new()
                .with_indicator_epoch(indicator_epoch)
                .run(
                    request,
                    &mut sink,
                    &llm,
                    &mut on_progress,
                    &abort,
                    &providers,
                )
                .await
        })
    };
    let _ = delta_tx.send(DesignDelta::Done(summary));
}

/// Spawn a worker that retries exactly ONE previously-failed subtask against
/// a `RemoteDocSink` — the manual layer of the failed-subtask remediation
/// feature (the progress panel's per-row "Retry" button, see
/// `op_orchestrator::retry_subtask`). Mirrors [`start`]'s shape
/// (channel-based `DesignSession`, `RemoteDocSink`, `block_on_anywhere`) but
/// drives `retry_subtask` instead of the full `Orchestrator::run` pipeline:
/// ONE attempt at full complexity, no 3-attempt ladder, no salvage pass —
/// the user is in the loop here (they clicked) and will decide whether to
/// click again, switch provider, or fall back to the chat modify flow.
///
/// `llm` is built fresh from WHATEVER provider is currently selected — see
/// `chat_provider_llm::ChatProviderLlmClient`, which adapts any
/// `ChatProvider` (CLI subprocess agent or builtin API-key provider) so this
/// works identically for either.
pub fn start_subtask_retry<L: LlmClient + Send + 'static>(
    llm: L,
    request: DesignRequest,
    subtask: op_orchestrator::plan::Subtask,
    initial_state: EditorState,
) -> DesignSession {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();

    let indicator_epoch = op_editor_core::agent_indicators::begin();
    let abort = AbortFlag::new();
    let worker_abort = abort.clone();

    let failure_tx = delta_tx.clone();
    if let Err(err) = thread::Builder::new()
        .name("op-subtask-retry".into())
        .spawn(move || {
            run_subtask_retry_worker(
                llm,
                request,
                subtask,
                initial_state,
                delta_tx,
                cmd_tx,
                indicator_epoch,
                worker_abort,
            )
        })
    {
        // Same degrade-instead-of-panic path as `start` above.
        eprintln!("[design-session] failed to spawn op-subtask-retry thread: {err}");
        op_editor_core::agent_indicators::end_if_epoch(indicator_epoch);
        let _ = failure_tx.send(DesignDelta::Done(Err(
            op_orchestrator::OrchestratorError::Internal(format!(
                "failed to spawn subtask-retry thread: {err}"
            )),
        )));
    }

    DesignSession::from_channels_with_epoch_and_abort(delta_rx, cmd_rx, indicator_epoch, abort)
}

/// Body of [`start_subtask_retry`]'s worker thread.
#[allow(clippy::too_many_arguments)]
fn run_subtask_retry_worker<L: LlmClient + Send>(
    llm: L,
    request: DesignRequest,
    subtask: op_orchestrator::plan::Subtask,
    initial_state: EditorState,
    delta_tx: Sender<DesignDelta>,
    cmd_tx: Sender<DesignCmdReq>,
    indicator_epoch: u64,
    abort: AbortFlag,
) {
    let mut sink = RemoteDocSink::new(cmd_tx, initial_state);
    let _ = delta_tx.send(DesignDelta::Progress(Progress::SubtaskStarted {
        id: subtask.id.clone(),
        label: subtask.label.clone(),
    }));
    let outcome = block_on_anywhere(op_orchestrator::retry_subtask::retry_subtask(
        &subtask,
        &request,
        &llm,
        &mut sink,
        &abort,
        Some(indicator_epoch),
        None,
    ));
    let delta = if outcome.node_count > 0 {
        DesignDelta::Progress(Progress::SubtaskDone {
            id: subtask.id.clone(),
            node_count: outcome.node_count,
        })
    } else {
        DesignDelta::Progress(Progress::SubtaskFailed {
            id: subtask.id.clone(),
            error: outcome
                .error
                .unwrap_or_else(|| "retry produced no content".into()),
        })
    };
    let _ = delta_tx.send(delta);
    // Deliberately NO `DesignDelta::Done` here — dropping `delta_tx` lets
    // `DesignSession::poll_progress`'s `TryRecvError::Disconnected` branch
    // set `finished` on its own. A `Done(Ok(RunSummary{..}))` would run
    // `pump_progress`'s WHOLE-TURN completion handling (marks every
    // Pending/Running activity Done, appends a second "Finished..."
    // narration line) — correct for a full orchestrator run, wrong for a
    // single-row retry that must touch ONLY the one row it retried.
}

/// Run N spawned sub-agents CONCURRENTLY against a `RemoteDocSink`, reusing
/// the orchestrator's per-subtask runner + concurrency cap
/// (`op_orchestrator::run_spawned_agents_concurrent`).
///
/// This is the loop-path `spawn_agents` bridge body: it owns the same
/// cross-thread `RemoteDocSink` + `block_on_anywhere` machinery
/// [`run_design_worker`] uses, so each produced subtree merges into the live
/// `EditorState` on the UI thread via the existing `pump_commands` drain — but
/// the subtasks generate in parallel (bounded by `concurrency`) instead of the
/// orchestrator's full screen-group pipeline.
///
/// Returns one [`SpawnAgentResult`] per spec (spec order) — the structured
/// result the agentic loop hands back to the model.
///
/// Called from a worker thread (never the UI thread — `RemoteDocSink::apply`
/// blocks on the UI ack, so running this on the UI thread would deadlock).
pub fn run_spawned_agents_worker<L: LlmClient + Send>(
    llm: L,
    specs: Vec<SpawnAgentSpec>,
    request: DesignRequest,
    initial_state: EditorState,
    cmd_tx: Sender<DesignCmdReq>,
    indicator_epoch: Option<u64>,
) -> Vec<SpawnAgentResult> {
    let mut sink = RemoteDocSink::new(cmd_tx, initial_state);
    let abort = AbortFlag::new();
    let concurrency = request.concurrency.max(1);
    block_on_anywhere(async {
        op_orchestrator::run_spawned_agents_concurrent(
            &specs,
            &request,
            &llm,
            &mut sink,
            &abort,
            concurrency,
            indicator_epoch,
        )
        .await
    })
}

async fn maybe_generate_design_md_for_follow_on_screen<L: LlmClient + Send>(
    llm: &L,
    request: &mut DesignRequest,
    sink: &mut RemoteDocSink,
    abort: &AbortFlag,
) {
    let state = sink.state().clone();
    if !crate::chat_intent::should_auto_generate_design_md(
        &state,
        &request.prompt,
        request.append_context.as_ref(),
    ) {
        return;
    }

    match crate::design_md_llm::generate_design_md_spec(
        llm,
        &state,
        &request.prompt,
        request.model.clone(),
        request.provider.clone(),
        abort,
    )
    .await
    {
        Ok(spec) => {
            request.design_md = Some(spec.clone());
            let _ = sink.apply(EditorCommand::SetDesignMd {
                spec: Box::new(spec),
            });
        }
        Err(message) => {
            let _ = message;
        }
    }
}

const DESIGN_FIT_PADDING: f32 = 48.0;

/// Keep the generated design centered and fully visible while the
/// orchestrator progressively applies scaffold/subtask nodes. Called
/// by the desktop residual's `pump_commands` after each applied command.
pub fn fit_design_viewport_to_content(
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

/// Bounding box of the active page's content in document space, or
/// `None` for an empty page. Public so the desktop residual's
/// viewport-fit tests can exercise it across the crate boundary.
pub fn active_content_bounds(state: &EditorState) -> Option<DocRect> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
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

/// The visible canvas region (width, height) given the current sidebar /
/// right-rail state. Public for the desktop residual's viewport-fit tests.
/// True when the active page's content is FULLY visible in the canvas
/// region at the current viewport — used by the design-loop host to decide
/// whether a growth batch pushed the design out of view (refit) or the
/// user's current framing still covers it (leave the viewport alone).
pub fn design_content_fits_viewport(
    state: &EditorState,
    viewport_width: f32,
    viewport_height: f32,
) -> bool {
    let Some(bounds) = active_content_bounds(state) else {
        return true;
    };
    let (canvas_w, canvas_h) = design_canvas_size(state, viewport_width, viewport_height);
    if canvas_w <= 1.0 || canvas_h <= 1.0 {
        return true;
    }
    let zoom = state.viewport.zoom;
    let left = bounds.x as f32 * zoom + state.viewport.pan_x;
    let top = bounds.y as f32 * zoom + state.viewport.pan_y;
    let right = left + bounds.w as f32 * zoom;
    let bottom = top + bounds.h as f32 * zoom;
    const MARGIN: f32 = 2.0;
    left >= -MARGIN && top >= -MARGIN && right <= canvas_w + MARGIN && bottom <= canvas_h + MARGIN
}

pub fn design_canvas_size(
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

#[cfg(test)]
#[path = "design_session_tests.rs"]
mod tests;
