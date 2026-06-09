// UNVERIFIED: needs EMSDK wasm32 build + browser; run tools/check-wasm-bundle.sh
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
//! Asset bytes are NOT carried into the wasm-clean `editor_state`; the web
//! v1 surfaces the assembled `code` only (Download writes a single file).
//! A `Done` with assets sets `degraded = true` so the panel can flag the
//! omission.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use op_codegen::ai::types::{CodegenInput, PendingRequest, PipelineStep, RequestId};
use op_codegen::ai::CodegenPipeline;
use op_editor_core::codegen::{CodeGenProgress, CodegenPhase};

use crate::web_ai_transport::{post_ai_stream, AiEvent};

/// A delta drained by the rAF pump into `editor_state.codegen`. Mirrors the
/// desktop `CodegenDelta`, minus the asset bytes (web v1 has no zip path).
pub enum WebCodegenDelta {
    Progress(CodeGenProgress),
    Done {
        code: String,
        /// True when the pipeline degraded OR assets were dropped on web.
        degraded: bool,
    },
    Failed(String),
}

/// The in-flight pipeline run. Shared (`Rc<RefCell<…>>`) between the chain of
/// request callbacks and the rAF pump's tick.
struct CodegenRun {
    pipe: CodegenPipeline,
    /// Streamed text for the in-flight request, fed back via `on_delta`.
    buf: String,
    /// The request currently awaiting its SSE stream (None between requests).
    in_flight: Option<RequestId>,
    /// Remaining requests in the current `Dispatch` batch, processed
    /// sequentially: the chain fires the front request, and on its terminal
    /// event pops the next. When empty, the driver re-steps the pipeline.
    batch: VecDeque<PendingRequest>,
    /// Set once a terminal `Done` / `Failed` delta has been queued, so the
    /// rAF pump can stop after draining it.
    terminal: bool,
    /// The model id (wire `value`) to send with each request; "default" lets
    /// the proxy pick the configured provider.
    model: String,
}

/// Shared state threaded through the async driver: the run + the delta queue.
type Shared = Rc<RefCell<(CodegenRun, VecDeque<WebCodegenDelta>)>>;

/// Build pipeline input from the current selection. Host-agnostic — duplicated
/// from the desktop's `codegen_input::build_codegen_input` (the desktop variant
/// also returns the raw nodes JSON for its export bundle; web v1 has no bundle,
/// so this returns only the `CodegenInput`).
fn build_codegen_input(state: &op_editor_core::EditorState) -> Option<CodegenInput> {
    use jian_ops_schema::node::PenNode;
    use op_ai::chat_provider::{EffortLevel, ThinkingMode};
    use op_editor_core::walkers::find_node;

    /// Default per-request token cap — the per-phase prompt builders override
    /// this, so it is only a fallback.
    const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 4096;

    if state.selection.is_empty() {
        return None;
    }

    // Resolve each selected id to its full subtree. The forest is the page
    // children when the document is multi-page, else the root `children`.
    let mut selected: Vec<&PenNode> = Vec::with_capacity(state.selection.set.len());
    if let Some(pages) = state.doc.pages.as_ref() {
        for id in &state.selection.set {
            if let Some(node) = pages.iter().find_map(|page| find_node(&page.children, id)) {
                selected.push(node);
            }
        }
    } else {
        for id in &state.selection.set {
            if let Some(node) = find_node(&state.doc.children, id) {
                selected.push(node);
            }
        }
    }

    // A selection of ids that resolve to no live node yields no input.
    if selected.is_empty() {
        return None;
    }

    let nodes_json = serde_json::to_string(&selected).unwrap_or_else(|_| "[]".to_string());
    let variables_json = state
        .doc
        .variables
        .as_ref()
        .filter(|vars| !vars.is_empty())
        .map(|vars| serde_json::to_string(vars).unwrap_or_else(|_| "{}".to_string()));

    Some(CodegenInput {
        nodes_json,
        framework: state.codegen.framework,
        variables_json,
        max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
        thinking: ThinkingMode::Adaptive,
        effort: EffortLevel::Low,
    })
}

/// Launch a web codegen run: build input, create the pipeline, drive it over
/// async XHR callbacks, and pump deltas into `editor_state.codegen` via rAF.
///
/// `base` is the daemon origin (e.g. `http://127.0.0.1:3100`) — the same
/// origin `live_sync` polls. Returns immediately; the model turns stream in
/// later via the request callbacks.
pub fn start_codegen(inner: Rc<RefCell<crate::Inner>>, base: String) {
    // 1. Build input from the live editor state. A missing / unresolvable
    //    selection surfaces an inline error rather than starting a run.
    let (input, model) = {
        let b = inner.borrow();
        let state = b.host.editor_state();
        let Some(input) = build_codegen_input(state) else {
            drop(b);
            let mut bm = inner.borrow_mut();
            let cg = &mut bm.host.editor_state_mut().codegen;
            cg.error = Some("Select nodes to generate code".into());
            cg.phase = CodegenPhase::Error;
            cg.pending_generate = false;
            cg.pending_regenerate = false;
            bm.host.mark_editor_state_dirty();
            let _ = bm.repaint();
            return;
        };
        // Model id: the selected chat model's wire value, else "default" (the
        // proxy then picks the configured provider). A fresh web shell has an
        // empty model catalog, so "default" is the common case today.
        let model = state
            .chat
            .selected_model_entry()
            .map(|e| e.value.clone())
            .unwrap_or_else(|| "default".to_string());
        (input, model)
    };

    // Reset the panel into the Generating state before the first turn.
    {
        let mut bm = inner.borrow_mut();
        let cg = &mut bm.host.editor_state_mut().codegen;
        cg.progress = Default::default();
        cg.error = None;
        cg.phase = CodegenPhase::Generating;
        bm.host.mark_editor_state_dirty();
        let _ = bm.repaint();
    }

    // 2. Shared run-state + delta queue.
    let shared: Shared = Rc::new(RefCell::new((
        CodegenRun {
            pipe: CodegenPipeline::new(input),
            buf: String::new(),
            in_flight: None,
            batch: VecDeque::new(),
            terminal: false,
            model,
        },
        VecDeque::new(),
    )));

    // 3. Start the rAF pump that drains the queue into editor_state.codegen.
    start_pump(inner.clone(), shared.clone());

    // 4. Kick the driver — it steps the pipeline and fires the first request.
    drive(inner, base, shared);
}

/// Step the pipeline (when no batch is pending) and dispatch the next request,
/// or queue a terminal delta. Re-entered from each request's terminal event.
fn drive(inner: Rc<RefCell<crate::Inner>>, base: String, shared: Shared) {
    // If a batch is still draining, fire its front request instead of stepping.
    let next_req = {
        let mut s = shared.borrow_mut();
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
                    // Assets are dropped on web v1 — flag degraded so the panel
                    // can note the omission.
                    let degraded = degraded || !assets.is_empty();
                    s.0.terminal = true;
                    s.1.push_back(WebCodegenDelta::Done { code, degraded });
                    return;
                }
                PipelineStep::Failed { message } => {
                    s.0.terminal = true;
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
fn fire_request(
    inner: Rc<RefCell<crate::Inner>>,
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

    let body_json = build_body_json(&req, &shared.borrow().0.model);

    // Cloned handles moved into the streaming callback.
    let inner_cb = inner.clone();
    let base_cb = base.clone();
    let shared_cb = shared.clone();

    let on_event: Rc<dyn Fn(AiEvent)> = Rc::new(move |evt: AiEvent| match evt {
        AiEvent::Delta(t) => {
            // Tight borrow: append + drop before returning to the browser.
            shared_cb.borrow_mut().0.buf.push_str(&t);
        }
        AiEvent::Done => {
            // Feed the buffered text into the pipeline, mark complete, queue a
            // progress delta, then re-drive (next batch req or re-step).
            {
                let mut s = shared_cb.borrow_mut();
                let buf = std::mem::take(&mut s.0.buf);
                s.0.pipe.on_delta(id, &buf);
                s.0.pipe.on_complete(id);
                s.0.in_flight = None;
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
            }
            drive(inner_cb.clone(), base_cb.clone(), shared_cb.clone());
        }
    });

    if let Err(_e) = post_ai_stream(&base, body_json, on_event) {
        // Transport refused to even start the request (e.g. XHR open failed).
        // Report the error to the pipeline so the run terminates cleanly.
        {
            let mut s = shared.borrow_mut();
            s.0.pipe
                .on_error(id, "AI stream request failed to start".to_string());
            s.0.in_flight = None;
        }
        drive(inner, base, shared);
    }
}

/// Build the JSON request body for the proxy. Skill NAMES (not expanded
/// prompts) are forwarded; the daemon proxy composes the final system prompt
/// (the same `op_ai_skills::compose_system_prompt` the desktop host runs
/// in-process). Hand-rolled to avoid a serde derive for this tiny payload.
fn build_body_json(req: &PendingRequest, model: &str) -> String {
    let skills_json = req
        .skills
        .iter()
        .map(|s| serde_json::Value::String((*s).to_string()))
        .collect::<Vec<_>>();
    let body = serde_json::json!({
        "model": model,
        "skills": skills_json,
        "user": req.user_message,
        "max_output_tokens": req.max_output_tokens,
        "thinking": req.thinking.as_str(),
        "effort": req.effort.as_str(),
    });
    serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string())
}

/// Start the rAF pump that drains queued deltas into `editor_state.codegen`
/// and repaints. Returns `true` while the run is active; returns `false` once
/// a terminal delta (Done / Failed) has been applied, stopping the loop.
fn start_pump(inner: Rc<RefCell<crate::Inner>>, shared: Shared) {
    let tick: Rc<dyn Fn() -> bool> = Rc::new(move || {
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
            let cg = &mut bm.host.editor_state_mut().codegen;
            match delta {
                WebCodegenDelta::Progress(p) => {
                    cg.progress = p;
                    cg.phase = CodegenPhase::Generating;
                }
                WebCodegenDelta::Done { code, degraded } => {
                    cg.code = code;
                    cg.code_scroll = 0.0;
                    cg.code_selection = None;
                    cg.degraded = degraded;
                    // Asset bytes are not carried on web v1.
                    cg.assets = Vec::new();
                    cg.phase = CodegenPhase::Complete;
                    cg.pending_generate = false;
                    cg.pending_regenerate = false;
                    applied_terminal = true;
                }
                WebCodegenDelta::Failed(e) => {
                    cg.error = Some(e);
                    cg.phase = CodegenPhase::Error;
                    cg.pending_generate = false;
                    cg.pending_regenerate = false;
                    applied_terminal = true;
                }
            }
            bm.host.mark_editor_state_dirty();
            changed = true;
        }

        if changed {
            // Repaint outside the delta loop so a multi-delta frame paints once.
            let _ = inner.borrow_mut().repaint();
        }

        // Keep ticking until a terminal delta lands. `terminal` on the run is
        // set when the terminal delta is QUEUED; we only stop once it has been
        // APPLIED here (so the final frame paints before the pump drops).
        !applied_terminal
    });
    crate::raf_pump::start(tick);
}
