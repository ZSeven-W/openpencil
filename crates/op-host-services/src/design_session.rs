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

use crate::chat_runtime::shared_runtime;
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
                vision_provider,
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
    vision_provider: Option<Arc<dyn ChatProvider>>,
) {
    let mut sink = RemoteDocSink::new(cmd_tx, initial_state);
    let abort = AbortFlag::new();
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
    let delta_tx_for_progress = delta_tx.clone();
    let mut on_progress = move |p: Progress| {
        let _ = delta_tx_for_progress.send(DesignDelta::Progress(p));
    };
    let summary = shared_runtime().block_on(async {
        let mut request = request;
        maybe_generate_design_md_for_follow_on_screen(&llm, &mut request, &mut sink, &abort).await;
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
    });
    let _ = delta_tx.send(DesignDelta::Done(summary));
}

/// Run N spawned sub-agents CONCURRENTLY against a `RemoteDocSink`, reusing
/// the orchestrator's per-subtask runner + concurrency cap
/// (`op_orchestrator::run_spawned_agents_concurrent`).
///
/// This is the loop-path `spawn_agents` bridge body: it owns the same
/// cross-thread `RemoteDocSink` + `shared_runtime` machinery
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
    shared_runtime().block_on(async {
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

#[cfg(test)]
mod spawn_worker_tests {
    use super::*;
    use futures::stream::BoxStream;
    use op_editor_host_core::design::{DesignCmdAck, DesignCmdOp};
    use op_orchestrator::{CallRequest, LlmChunk, LlmError};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc as std_mpsc, Arc};
    use std::thread;
    use std::time::{Duration, Instant};

    /// A recording `LlmClient` — counts how many times `call` ran and
    /// returns one scripted node-script response per call (round-robin by
    /// call order). Proves the worker invokes the REAL per-subtask runner
    /// N times, not a canned ack.
    struct RecordingLlm {
        calls: Arc<AtomicUsize>,
        responses: std::sync::Mutex<std::collections::VecDeque<String>>,
    }

    impl op_orchestrator::LlmClient for RecordingLlm {
        fn call(&self, _req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let text = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_default();
            Box::pin(futures::stream::iter(vec![Ok(LlmChunk::Text(text))]))
        }
    }

    // Script-gen is the default subagent generation protocol, so the fixture
    // is a JS program calling the bound `I(parent, obj)` recorder (a single
    // insert whose object nests its children inline) rather than raw
    // `_parent` JSONL. The batch_design executor reassigns fresh ids to every
    // inserted node regardless of what's authored here, so `name` (not `id`)
    // is the field callers must key off of if they need to identify which
    // response landed where.
    fn node_json(id: &str) -> String {
        format!(
            r#"I(null, {{"type":"frame","name":"Sec-{id}","x":0,"y":0,"width":400,"height":120,"children":[{{"type":"text","content":"Hi","fontSize":18}}]}});"#
        )
    }

    fn make_req() -> DesignRequest {
        DesignRequest {
            prompt: "p".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 3,
            append_context: None,
            validation_enabled: false,
            visual_ref_enabled: false,
        }
    }

    /// End-to-end: `run_spawned_agents_worker` (off a worker thread) drives
    /// the real concurrent subagent core through a `RemoteDocSink`; the UI
    /// side acks each `DesignCmdReq` against a live `EditorState`. Proves
    /// N real LLM calls happened AND N InsertSubtree commands were forwarded
    /// over the bridge channel — not a placeholder ack.
    #[test]
    fn spawn_worker_drives_n_real_llm_calls_and_forwards_n_insert_subtrees() {
        let calls = Arc::new(AtomicUsize::new(0));
        let llm = RecordingLlm {
            calls: Arc::clone(&calls),
            responses: std::sync::Mutex::new(
                vec![node_json("a"), node_json("b"), node_json("c")].into(),
            ),
        };
        let specs = vec![
            SpawnAgentSpec {
                id: "a".into(),
                label: "A".into(),
                prompt: "design A".into(),
                parent_frame_id: None,
            },
            SpawnAgentSpec {
                id: "b".into(),
                label: "B".into(),
                prompt: "design B".into(),
                parent_frame_id: None,
            },
            SpawnAgentSpec {
                id: "c".into(),
                label: "C".into(),
                prompt: "design C".into(),
                parent_frame_id: None,
            },
        ];

        let (cmd_tx, cmd_rx) = std_mpsc::channel::<DesignCmdReq>();
        let results_slot: Arc<std::sync::Mutex<Option<Vec<SpawnAgentResult>>>> =
            Arc::new(std::sync::Mutex::new(None));
        let results_for_worker = Arc::clone(&results_slot);

        // Worker thread drives the concurrent core; the live state lives on
        // the test (UI) thread, mirrored through the RemoteDocSink channel.
        let worker = thread::spawn(move || {
            let out =
                run_spawned_agents_worker(llm, specs, make_req(), EditorState::new(), cmd_tx, None);
            *results_for_worker.lock().unwrap() = Some(out);
        });

        // UI side: a live EditorState that applies each forwarded command and
        // acks with a fresh snapshot (mirrors `pump_commands`).
        let mut state = EditorState::new();
        let mut insert_subtree_count = 0usize;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(req) => {
                    let applied = match req.op {
                        DesignCmdOp::Apply(cmd) => {
                            if matches!(cmd, EditorCommand::InsertSubtree { .. }) {
                                insert_subtree_count += 1;
                            }
                            state.apply(cmd)
                        }
                        DesignCmdOp::BeginUndoBatch | DesignCmdOp::EndUndoBatch => true,
                    };
                    let _ = req.ack.send(DesignCmdAck {
                        applied,
                        new_state: state.clone(),
                    });
                }
                Err(std_mpsc::RecvTimeoutError::Disconnected) => break,
                Err(std_mpsc::RecvTimeoutError::Timeout) => {
                    if Instant::now() > deadline {
                        panic!("spawn worker did not finish within the deadline");
                    }
                }
            }
        }
        worker.join().expect("spawn worker exits cleanly");

        // N real LLM calls happened (the genuine per-subtask runner ran).
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "three real subagent LLM calls"
        );
        // N InsertSubtree commands were forwarded over the bridge channel.
        assert_eq!(
            insert_subtree_count, 3,
            "three subtrees forwarded into the live document"
        );
        // The live document now carries three inserted roots.
        assert_eq!(state.active_children().len(), 3);
        // The structured results name what each agent created.
        let results = results_slot.lock().unwrap().take().expect("results set");
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.error.is_none(), "agent {} failed: {:?}", r.id, r.error);
            assert_eq!(r.node_count, 1, "each agent produced one real section root");
        }
        // NOTE: `inserted_root_ids` stays empty on the `RemoteDocSink` path —
        // the default `insert_subtree_returning_root_ids` trait impl applies
        // the command over the channel and cannot surface the post-remap ids
        // the UI thread mints. The orchestrator-core test
        // (`spawn_concurrent_tests::run_spawned_agents_invokes_real_runner…`)
        // proves real id capture against an immediate-apply `VecDocSink`.
    }
}
