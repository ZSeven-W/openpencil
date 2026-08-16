//! Web code-generation session — drives the pull-based `CodegenPipeline`
//! over async XHR callbacks instead of a worker thread.
//!
//! The desktop host (`op-host-desktop::codegen_session`) runs the same
//! `CodegenPipeline` on a worker thread, draining each request's blocking
//! `ChatProvider::send` iterator off the UI thread and streaming progress
//! back over an `mpsc` channel. The browser can't block or spawn threads,
//! so this module drives the SAME pipeline differently:
//!
//! * Each model request is fired via [`crate::web_ai_transport::post_ai_stream`]
//!   (an SSE-over-XHR POST to the daemon's `/api/ai/stream` proxy). The
//!   streamed `Delta` / `Done` / `Error` events fire LATER (not re-entrantly)
//!   on `onprogress` / `onloadend`, so the driver is a chain of callbacks
//!   rather than a loop.
//! * A `VecDeque<WebCodegenDelta>` queue stands in for the desktop `mpsc`
//!   channel; a `requestAnimationFrame` pump (`crate::raf_pump`) drains it
//!   into `editor_state.codegen` ~once per frame and repaints.
//!
//! Sequential driving: a `Dispatch(reqs)` batch is processed one request at
//! a time (`reqs[0]` fired, on its `Done` the next is fired, …). Once the
//! whole batch settles, the driver re-steps the pipeline, which yields the
//! next `Dispatch` (or a terminal `Done` / `Failed`). At most one HTTP
//! request is in flight at any moment.
//!
//! Desktop-parity surfaces (TS `code-panel.tsx` semantics via the desktop
//! runner, which is canonical):
//! * **Cancel really aborts** — the active run is parked (`cancelled`), its
//!   in-flight XHR is aborted, `CodegenPipeline::cancel` ends the machine,
//!   and the pump drops every queued/late delta so a stale run can never
//!   resurrect the panel (TS `abortRef.current?.abort()`).
//! * **Whole-page fallback** — an empty selection generates from the active
//!   page's children (TS `getTargetNodes`, code-panel.tsx:137-142).
//! * **Assets are kept** — the terminal `Done`'s asset bytes are parked in a
//!   host-side per-framework cache (the wasm-clean `editor_state` carries only
//!   `AssetMeta`), and Download produces `component.zip` (code + `assets/*`)
//!   when assets exist — the same zip layout as the desktop `codegen_export`.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use op_codegen::ai::types::{AssetFile, PendingRequest, PipelineStep, RequestId};
use op_codegen::ai::CodegenPipeline;
use op_editor_core::codegen::{CodeGenProgress, CodegenPhase, CodegenState, Framework};
use op_editor_core::AgentProvider;
use op_editor_host_core::codegen::{
    build_codegen_input_value as build_codegen_input, framework_ext,
};

use crate::repaint_ctx::RepaintContext;
use crate::web_ai_transport::{post_ai_stream, AiEvent, AiStreamHandle};

#[path = "codegen_web_request.rs"]
mod request_body;
use request_body::build_body_json;

type CodegenDocumentIdentity = (u64, u64);

fn document_identity(
    document_epoch: u64,
    state: &op_editor_core::EditorState,
) -> CodegenDocumentIdentity {
    (document_epoch, state.document_generation())
}

/// A delta drained by the rAF pump into `editor_state.codegen`. Mirrors the
/// desktop `CodegenDelta`. A terminal `Done` keeps its raw asset bytes and
/// launch-time framework in the queue until the rAF pump actually applies it;
/// canceled queued results must never get ahead of the painted UI.
pub enum WebCodegenDelta {
    Progress(CodeGenProgress),
    Done {
        code: String,
        degraded: bool,
        framework: Framework,
        assets: Vec<AssetFile>,
    },
    Failed(String),
}

/// The in-flight pipeline run. Shared (`Rc<RefCell<…>>`) between the chain of
/// request callbacks, the rAF pump's tick, and the [`ACTIVE_RUN`] slot.
struct CodegenRun {
    pipe: CodegenPipeline,
    /// Streamed text for the in-flight request, fed back via `on_delta`.
    buf: String,
    /// The request currently awaiting its SSE stream (None between requests).
    in_flight: Option<RequestId>,
    /// Abort handle for the in-flight request's XHR (None between requests).
    handle: Option<AiStreamHandle>,
    /// Remaining requests in the current `Dispatch` batch, processed
    /// sequentially: the chain fires the front request, and on its terminal
    /// event pops the next. When empty, the driver re-steps the pipeline.
    batch: VecDeque<PendingRequest>,
    /// Set once a terminal `Done` / `Failed` delta has been queued, so the
    /// rAF pump can stop after draining it.
    terminal: bool,
    /// Raised by [`cancel_active_run`]: every later event/delta from this run
    /// is dropped (desktop parity — the canceled UI state survives).
    cancelled: bool,
    /// The model id (wire `value`) to send with each request; "default" lets
    /// the proxy pick the configured provider.
    model: String,
    /// Exact provider selected alongside `model`, when one is available.
    provider: Option<AgentProvider>,
    /// Exact daemon built-in selected alongside `model`, when this is an
    /// operator-owned catalog row rather than a browser-local credential.
    builtin_provider_id: Option<String>,
    /// Target framework captured at launch — the Download file extension must
    /// match what was GENERATED even if the user switches tabs afterwards
    /// (desktop `CodegenSession.framework` parity).
    framework: op_editor_core::codegen::Framework,
    /// Generation targets captured at launch and committed to the painted
    /// state only if this exact run completes successfully.
    selection_snapshot: Vec<String>,
    /// Browser-local credential for the selected built-in provider. Captured
    /// once at launch and sent only with this run's model requests.
    credential: Option<serde_json::Value>,
    /// `(host epoch, EditorState generation)` captured at launch. The pair
    /// covers both whole-state Open/New/import and in-place live-sync replace.
    document_identity: CodegenDocumentIdentity,
}

/// Shared state threaded through the async driver: the run + the delta queue.
type Shared = Rc<RefCell<(CodegenRun, VecDeque<WebCodegenDelta>)>>;

/// The completed result kept HOST-SIDE for Download — asset bytes are not
/// carried in the wasm-clean `editor_state`. Mirror of the desktop
/// `CodegenResult` (wasm is single-threaded, so a thread-local cache is the
/// natural owner).
pub(crate) struct WebCodegenResult {
    pub(crate) code: String,
    pub(crate) framework_ext: &'static str,
    pub(crate) assets: Vec<AssetFile>,
}

#[derive(Default)]
struct WebCodegenResults {
    document_identity: Option<CodegenDocumentIdentity>,
    entries: Vec<(Framework, WebCodegenResult)>,
}

impl WebCodegenResults {
    fn insert(
        &mut self,
        document_identity: CodegenDocumentIdentity,
        framework: Framework,
        result: WebCodegenResult,
    ) {
        if self.document_identity != Some(document_identity) {
            self.document_identity = Some(document_identity);
            self.entries.clear();
        }
        if let Some((_, cached)) = self.entries.iter_mut().find(|(fw, _)| *fw == framework) {
            *cached = result;
        } else {
            self.entries.push((framework, result));
        }
    }

    fn get(
        &self,
        document_identity: CodegenDocumentIdentity,
        framework: Framework,
    ) -> Option<&WebCodegenResult> {
        if self.document_identity != Some(document_identity) {
            return None;
        }
        self.entries
            .iter()
            .find(|(fw, _)| *fw == framework)
            .map(|(_, result)| result)
    }
}

thread_local! {
    /// The live (or just-canceled) run. A LIVE run blocks a new launch; a
    /// canceled one does not (desktop `launch_codegen_if_pending` parity:
    /// cancel + regenerate is immediate).
    static ACTIVE_RUN: RefCell<Option<Shared>> = const { RefCell::new(None) };
    /// Completed results for Download, including raw asset bytes.
    static RESULTS: RefCell<WebCodegenResults> =
        const {
            RefCell::new(WebCodegenResults {
                document_identity: None,
                entries: Vec::new(),
            })
        };
}

fn cache_completed_result(
    document_identity: CodegenDocumentIdentity,
    framework: Framework,
    code: String,
    assets: Vec<AssetFile>,
) {
    RESULTS.with(|results| {
        results.borrow_mut().insert(
            document_identity,
            framework,
            WebCodegenResult {
                code,
                framework_ext: framework_ext(framework),
                assets,
            },
        );
    });
}

fn enqueue_completed_result(
    queue: &mut VecDeque<WebCodegenDelta>,
    framework: Framework,
    code: String,
    degraded: bool,
    assets: Vec<AssetFile>,
) {
    queue.push_back(WebCodegenDelta::Done {
        code,
        degraded,
        framework,
        assets,
    });
}

fn discard_queued_deltas(queue: &mut VecDeque<WebCodegenDelta>) {
    queue.clear();
}

fn discard_if_stale_document(
    run_identity: CodegenDocumentIdentity,
    live_identity: CodegenDocumentIdentity,
    queue: &mut VecDeque<WebCodegenDelta>,
) -> bool {
    if run_identity == live_identity {
        return false;
    }
    discard_queued_deltas(queue);
    true
}

fn apply_codegen_delta(
    cg: &mut CodegenState,
    delta: WebCodegenDelta,
    run_identity: CodegenDocumentIdentity,
    selection_snapshot: &mut Vec<String>,
) -> bool {
    match delta {
        WebCodegenDelta::Progress(p) => {
            cg.progress = p;
            cg.phase = CodegenPhase::Generating;
            false
        }
        WebCodegenDelta::Done {
            code,
            degraded,
            framework,
            assets,
        } => {
            cg.code = code.clone();
            cg.code_scroll.offset = 0.0;
            cg.code_selection = None;
            cg.degraded = degraded;
            cg.assets = assets
                .iter()
                .map(|asset| op_editor_core::codegen::AssetMeta {
                    relative_path: asset.relative_path.clone(),
                    byte_len: asset.bytes.len(),
                })
                .collect();
            cg.selection_snapshot = std::mem::take(selection_snapshot);
            cg.phase = CodegenPhase::Complete;
            cg.pending_generate = false;
            cg.pending_regenerate = false;
            cache_completed_result(run_identity, framework, code, assets);
            true
        }
        WebCodegenDelta::Failed(e) => {
            cg.error = Some(e);
            cg.phase = CodegenPhase::Error;
            cg.pending_generate = false;
            cg.pending_regenerate = false;
            true
        }
    }
}

/// True while a non-canceled, non-terminal run for the current document is in
/// flight. A replacement can be followed by Generate before the next rAF pump;
/// retire that stale run here so it cannot swallow the new document's press.
fn has_live_run(live_identity: CodegenDocumentIdentity) -> bool {
    ACTIVE_RUN.with(|slot| {
        let shared = slot.borrow().as_ref().cloned();
        let Some(shared) = shared else {
            return false;
        };
        let run_identity = shared.borrow().0.document_identity;
        if run_identity != live_identity {
            let mut s = shared.borrow_mut();
            s.0.cancelled = true;
            s.0.pipe.cancel();
            s.0.credential = None;
            if let Some(handle) = s.0.handle.take() {
                handle.abort();
            }
            discard_queued_deltas(&mut s.1);
            drop(s);
            let mut active = slot.borrow_mut();
            if active.as_ref().is_some_and(|run| Rc::ptr_eq(run, &shared)) {
                *active = None;
            }
            return false;
        }
        let s = shared.borrow();
        !s.0.cancelled && !s.0.terminal
    })
}

/// Park the active run: raise `cancelled`, abort its in-flight XHR, end the
/// pipeline machine, and drop everything it already queued. The rAF pump
/// observes `cancelled` on its next frame and stops without applying anything
/// (TS `abortRef.current?.abort()`; desktop `CodegenSession::cancel` +
/// canceled-pump parity).
pub(crate) fn cancel_active_run() {
    let run = ACTIVE_RUN.with(|slot| slot.borrow_mut().take());
    if let Some(shared) = run {
        let mut s = shared.borrow_mut();
        s.0.cancelled = true;
        s.0.pipe.cancel();
        s.0.credential = None;
        if let Some(handle) = s.0.handle.take() {
            // Aborting fires the XHR's onloadend, which synthesizes an Error
            // event — the event callback drops it via the `cancelled` flag.
            handle.abort();
        }
        discard_queued_deltas(&mut s.1);
    }
}

/// Drain the Code-panel flags raised during `apply_press` (Cancel first, then
/// Generate / Regenerate — the desktop event-loop drain order). Called from
/// the DOM mousedown listener AFTER its `inner` borrow is released, because
/// `start_codegen` re-borrows `inner` (and the rAF pump it starts borrows it
/// again on later frames).
pub(crate) fn drain_codegen_flags<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let (cancel, generate) = {
        let mut bm = inner.borrow_mut();
        let cg = &mut bm.host_mut().editor_state_mut().codegen;
        let cancel = std::mem::take(&mut cg.pending_cancel);
        let generate = cg.pending_generate || cg.pending_regenerate;
        // Clear the launch flags before launching so a failed launch (e.g.
        // the empty-document error path, which surfaces inline) isn't
        // re-triggered on the next press.
        cg.pending_generate = false;
        cg.pending_regenerate = false;
        (cancel, generate)
    };
    if cancel {
        cancel_active_run();
    }
    if generate {
        // A LIVE run swallows the press (desktop parity: the pending launch
        // never proceeds while a run is active; the panel only shows
        // Generate / Regenerate outside the Generating state anyway).
        let live_identity = {
            let b = inner.borrow();
            document_identity(b.host().document_epoch(), b.host().editor_state())
        };
        if has_live_run(live_identity) {
            return;
        }
        start_codegen(inner.clone(), crate::daemon_base::daemon_base());
    }
}

/// Launch a web codegen run: build input, create the pipeline, drive it over
/// async XHR callbacks, and pump deltas into `editor_state.codegen` via rAF.
///
/// `base` is the daemon origin (e.g. `http://127.0.0.1:3100`) — the same
/// origin `live_sync` polls. Returns immediately; the model turns stream in
/// later via the request callbacks.
pub fn start_codegen<C: RepaintContext + 'static>(inner: Rc<RefCell<C>>, base: String) {
    // 1. Build input from the live editor state. Nothing to generate from
    //    (empty page + no selection) surfaces an inline error (desktop
    //    `launch_codegen_if_pending` parity).
    let (input, provider, builtin_provider_id, model, credential, framework, document_identity) = {
        let b = inner.borrow();
        let state = b.host().editor_state();
        let Some(input) = build_codegen_input(state) else {
            drop(b);
            let mut bm = inner.borrow_mut();
            let cg = &mut bm.host_mut().editor_state_mut().codegen;
            cg.error = Some("Select nodes to generate code".into());
            cg.phase = CodegenPhase::Error;
            cg.pending_generate = false;
            cg.pending_regenerate = false;
            bm.host_mut().mark_editor_state_dirty();
            let _ = bm.repaint();
            return;
        };
        // Model id: the selected chat model's wire value, else "default" (the
        // proxy then picks the configured provider).
        let selected = state.chat.selected_model_entry();
        if selected.is_some_and(|entry| entry.builtin_provider_id.is_none()) {
            drop(b);
            let mut bm = inner.borrow_mut();
            let cg = &mut bm.host_mut().editor_state_mut().codegen;
            cg.error = Some("Select a browser AI provider to generate code".into());
            cg.phase = CodegenPhase::Error;
            cg.pending_generate = false;
            cg.pending_regenerate = false;
            bm.host_mut().mark_editor_state_dirty();
            let _ = bm.repaint();
            return;
        }
        let (model, credential, builtin_provider_id) =
            crate::web_ai_credentials::selected_target(state);
        // The browser surface is built-in-only. Structured built-in catalog
        // entries still carry provider identity, but a transient CLI entry
        // must never opt this request into daemon-side CLI routing.
        let provider = selected
            .filter(|entry| entry.builtin_provider_id.is_some())
            .map(|entry| entry.provider);
        (
            input,
            provider,
            builtin_provider_id,
            model,
            credential,
            state.codegen.framework,
            document_identity(b.host().document_epoch(), state),
        )
    };

    // Reset the panel into the Generating state before the first turn. Keep
    // this run's targets separate until Done: a failed regeneration keeps the
    // previous successful code and must keep its matching target snapshot.
    let selection_snapshot = {
        let mut bm = inner.borrow_mut();
        let selection_snapshot: Vec<String> = bm
            .host()
            .editor_state()
            .selection
            .set
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        let cg = &mut bm.host_mut().editor_state_mut().codegen;
        cg.progress = Default::default();
        cg.error = None;
        cg.phase = CodegenPhase::Generating;
        bm.host_mut().mark_editor_state_dirty();
        let _ = bm.repaint();
        selection_snapshot
    };

    // 2. Shared run-state + delta queue; register as the active run.
    let shared: Shared = Rc::new(RefCell::new((
        CodegenRun {
            pipe: CodegenPipeline::new(input),
            buf: String::new(),
            in_flight: None,
            handle: None,
            batch: VecDeque::new(),
            terminal: false,
            cancelled: false,
            model,
            provider,
            builtin_provider_id,
            framework,
            selection_snapshot,
            credential,
            document_identity,
        },
        VecDeque::new(),
    )));
    ACTIVE_RUN.with(|slot| {
        *slot.borrow_mut() = Some(shared.clone());
    });

    // 3. Start the rAF pump that drains the queue into editor_state.codegen.
    start_pump(inner.clone(), shared.clone());

    // 4. Kick the driver — it steps the pipeline and fires the first request.
    drive(inner, base, shared);
}

/// Step the pipeline (when no batch is pending) and dispatch the next request,
/// or queue a terminal delta. Re-entered from each request's terminal event.
fn drive<C: RepaintContext + 'static>(inner: Rc<RefCell<C>>, base: String, shared: Shared) {
    // If a batch is still draining, fire its front request instead of stepping.
    let next_req = {
        let mut s = shared.borrow_mut();
        if s.0.cancelled {
            return; // parked — nothing further is dispatched
        }
        if s.0.batch.is_empty() {
            // No batch pending — step the pipeline for the next instruction.
            match s.0.pipe.step() {
                PipelineStep::Dispatch(reqs) => {
                    s.0.batch = reqs.into();
                }
                PipelineStep::Waiting => {
                    // Sequential driving never parks on Waiting (at most one
                    // request is ever in flight, and we only re-step after it
                    // settles). Treat as terminal-safe: nothing to do.
                    return;
                }
                PipelineStep::Done {
                    code,
                    degraded,
                    assets,
                } => {
                    // Keep the raw result attached to this run until the rAF
                    // pump applies it. A cancel before that frame drops the
                    // queue without replacing the last painted/downloadable
                    // result.
                    let framework = s.0.framework;
                    s.0.terminal = true;
                    s.0.credential = None;
                    enqueue_completed_result(&mut s.1, framework, code, degraded, assets);
                    return;
                }
                PipelineStep::Failed { message } => {
                    s.0.terminal = true;
                    s.0.credential = None;
                    s.1.push_back(WebCodegenDelta::Failed(message));
                    return;
                }
            }
        }
        // Pop the front request of the (possibly just-filled) batch.
        s.0.batch.pop_front()
    };

    let Some(req) = next_req else {
        // Empty dispatch — re-step to advance (the pipeline may emit Waiting
        // or a terminal next).
        drive(inner, base, shared);
        return;
    };

    fire_request(inner, base, shared, req);
}

/// Fire one model request over the SSE proxy. The `on_event` callback
/// accumulates deltas into `run.buf` and, on the terminal event, feeds the
/// pipeline + re-drives. Borrows are taken tightly inside each callback and
/// dropped before the next async hop is scheduled.
fn fire_request<C: RepaintContext + 'static>(
    inner: Rc<RefCell<C>>,
    base: String,
    shared: Shared,
    req: PendingRequest,
) {
    let id = req.id;
    {
        let mut s = shared.borrow_mut();
        s.0.in_flight = Some(id);
        s.0.buf.clear();
    }

    let body_json = {
        let run = shared.borrow();
        build_body_json(
            &req,
            run.0.provider,
            run.0.builtin_provider_id.as_deref(),
            &run.0.model,
            run.0.credential.as_ref(),
        )
    };

    // Cloned handles moved into the streaming callback.
    let inner_cb = inner.clone();
    let base_cb = base.clone();
    let shared_cb = shared.clone();

    let on_event: Rc<dyn Fn(AiEvent)> = Rc::new(move |evt: AiEvent| {
        // A canceled run ignores everything its stale stream still emits
        // (including the Error the aborted XHR's onloadend synthesizes).
        if shared_cb.borrow().0.cancelled {
            return;
        }
        match evt {
            AiEvent::AgentIdentity { .. } => {}
            AiEvent::Delta(t) => {
                // Tight borrow: append + drop before returning to the browser.
                shared_cb.borrow_mut().0.buf.push_str(&t);
            }
            // Reasoning fragments are a chat-transcript affordance; the codegen
            // pipeline consumes only the answer text.
            AiEvent::Thinking(_) => {}
            AiEvent::Done => {
                // Feed the buffered text into the pipeline, mark complete, queue a
                // progress delta, then re-drive (next batch req or re-step).
                {
                    let mut s = shared_cb.borrow_mut();
                    let buf = std::mem::take(&mut s.0.buf);
                    s.0.pipe.on_delta(id, &buf);
                    s.0.pipe.on_complete(id);
                    s.0.in_flight = None;
                    s.0.handle = None;
                    let progress = s.0.pipe.progress();
                    s.1.push_back(WebCodegenDelta::Progress(progress));
                }
                drive(inner_cb.clone(), base_cb.clone(), shared_cb.clone());
            }
            AiEvent::Error(e) => {
                {
                    let mut s = shared_cb.borrow_mut();
                    s.0.pipe.on_error(id, e);
                    s.0.in_flight = None;
                    s.0.handle = None;
                }
                drive(inner_cb.clone(), base_cb.clone(), shared_cb.clone());
            }
        }
    });

    match post_ai_stream(&base, body_json, on_event) {
        Ok(handle) => {
            // Keep the abort handle so Cancel can kill the in-flight stream.
            shared.borrow_mut().0.handle = Some(handle);
        }
        Err(_e) => {
            // Transport refused to even start the request (e.g. XHR open
            // failed). Report the error to the pipeline so the run
            // terminates cleanly.
            {
                let mut s = shared.borrow_mut();
                s.0.pipe
                    .on_error(id, "AI stream request failed to start".to_string());
                s.0.in_flight = None;
            }
            drive(inner, base, shared);
        }
    }
}

/// Start the rAF pump that drains queued deltas into `editor_state.codegen`
/// and repaints. Stops once a terminal delta (Done / Failed) has been applied,
/// or immediately when the run was canceled (every queued/late delta is
/// dropped so the canceled UI state survives — desktop pump parity).
fn start_pump<C: RepaintContext + 'static>(inner: Rc<RefCell<C>>, shared: Shared) {
    let tick: Rc<dyn Fn() -> bool> = Rc::new(move || {
        // Canceled: drop everything and stop. The Cancel action already
        // flipped the painted phase; nothing here may overwrite it.
        if shared.borrow().0.cancelled {
            discard_queued_deltas(&mut shared.borrow_mut().1);
            return false;
        }
        let live_identity = {
            let b = inner.borrow();
            document_identity(b.host().document_epoch(), b.host().editor_state())
        };
        let stale = {
            let mut s = shared.borrow_mut();
            let run_identity = s.0.document_identity;
            discard_if_stale_document(run_identity, live_identity, &mut s.1)
        };
        if stale {
            let mut s = shared.borrow_mut();
            s.0.cancelled = true;
            s.0.pipe.cancel();
            s.0.credential = None;
            if let Some(handle) = s.0.handle.take() {
                handle.abort();
            }
            drop(s);
            ACTIVE_RUN.with(|slot| {
                let mut active = slot.borrow_mut();
                if active.as_ref().is_some_and(|run| Rc::ptr_eq(run, &shared)) {
                    *active = None;
                }
            });
            return false;
        }
        let mut applied_terminal = false;
        let mut changed = false;

        // Drain every queued delta this frame.
        loop {
            // Pop one delta under a tight borrow so the apply below doesn't
            // hold the shared borrow across a repaint.
            let delta = {
                let mut s = shared.borrow_mut();
                s.1.pop_front()
            };
            let Some(delta) = delta else { break };

            let mut bm = inner.borrow_mut();
            let cg = &mut bm.host_mut().editor_state_mut().codegen;
            let terminal = {
                let mut s = shared.borrow_mut();
                let run_identity = s.0.document_identity;
                apply_codegen_delta(cg, delta, run_identity, &mut s.0.selection_snapshot)
            };
            applied_terminal |= terminal;
            bm.host_mut().mark_editor_state_dirty();
            changed = true;
        }

        if changed {
            // Repaint outside the delta loop so a multi-delta frame paints once.
            let _ = inner.borrow_mut().repaint();
        }

        if applied_terminal {
            // Retire this run from the active slot (unless a newer run
            // already replaced it).
            ACTIVE_RUN.with(|slot| {
                let mut s = slot.borrow_mut();
                if s.as_ref().is_some_and(|r| Rc::ptr_eq(r, &shared)) {
                    *s = None;
                }
            });
            return false;
        }
        true
    });
    crate::raf_pump::start(tick);
}

// ---------------------------------------------------------------------
// Download (Code panel) — code file, or component.zip when assets exist
// ---------------------------------------------------------------------

/// Download the completed generation: `component.<ext>` (single file), or a
/// `component.zip` carrying the code + `assets/*` when image assets came back
/// (desktop `codegen_export::handle_download` layout; TS gates the zip the
/// same way on `assets.length > 0`). Falls back to the painted code buffer
/// when no parked result exists (defensive — Download is only reachable from
/// the Complete state).
pub(crate) fn download_generated(state: &op_editor_core::EditorState, document_epoch: u64) {
    let live_identity = document_identity(document_epoch, state);
    RESULTS.with(|results| {
        let results = results.borrow();
        match results.get(live_identity, state.codegen.framework) {
            Some(result) if result.code.is_empty() => {}
            Some(result) if !result.assets.is_empty() => {
                let bytes = build_code_zip(&result.code, result.framework_ext, &result.assets);
                let _ = crate::web_clipboard::download_bytes(
                    "component.zip",
                    "application/zip",
                    &bytes,
                );
            }
            Some(result) => {
                let _ = crate::web_clipboard::download_bytes(
                    &format!("component.{}", result.framework_ext),
                    "text/plain",
                    result.code.as_bytes(),
                );
            }
            None if !state.codegen.code.is_empty() => {
                let ext = framework_ext(state.codegen.framework);
                let _ = crate::web_clipboard::download_bytes(
                    &format!("component.{ext}"),
                    "text/plain",
                    state.codegen.code.as_bytes(),
                );
            }
            None => {}
        }
    });
}

/// Build the Download zip: `component.<ext>` + one `assets/<...>` entry per
/// asset (each asset's `zip_path` already lives under `assets/`). STORED
/// encoding via the dependency-free encoder — same entry layout as the
/// desktop `codegen_export::build_code_zip`.
fn build_code_zip(code: &str, ext: &str, assets: &[AssetFile]) -> Vec<u8> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::with_capacity(1 + assets.len());
    files.push((format!("component.{ext}"), code.as_bytes().to_vec()));
    for asset in assets {
        files.push((asset.zip_path.clone(), asset.bytes.clone()));
    }
    op_codegen::ai::stored_zip::build_stored_zip(&files)
}

#[cfg(test)]
#[path = "codegen_web_tests.rs"]
mod tests;
