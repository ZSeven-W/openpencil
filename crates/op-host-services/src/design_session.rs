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
use std::thread;

use op_editor_core::{DocRect, EditorState, Viewport};
pub use op_editor_host_core::design::{DesignCmdReq, DesignDelta, DesignSession, RemoteDocSink};
use op_editor_ui::widgets::TOP_BAR_HEIGHT;
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
/// [`start`]'s worker thread, callable directly by the CLI intent
/// router's worker (which already runs off the UI thread).
pub fn run_design_worker<L: LlmClient + Send>(
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

/// The visible canvas region (width, height) given the current sidebar /
/// right-rail state. Public for the desktop residual's viewport-fit tests.
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
