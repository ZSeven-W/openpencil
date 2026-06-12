//! Desktop codegen session — drives the pull-based `CodegenPipeline` on a
//! worker thread and streams progress into `editor_state.codegen`. Mirrors
//! `chat_session.rs` (worker thread + mpsc channel + per-frame pump +
//! `launch_if_pending`); like `design_session.rs` it carries a single
//! progress channel and never mutates the document.
//!
//! The pipeline is pull-based: `step()` returns `Dispatch(reqs)` until the
//! host has run each model request and fed the streamed text back via
//! `on_delta` / `on_complete` / `on_error`. The worker drains each request's
//! `ChatProvider::send` iterator (blocking) off the UI thread, then emits a
//! `Progress` delta so the panel can advance. Terminal `Done` / `Failed`
//! carry the assembled code / assets back to the UI pump.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};
use op_codegen::ai::types::{AssetFile, CodegenInput, PipelineStep};
use op_codegen::ai::CodegenPipeline;
use op_editor_core::codegen::CodeGenProgress;
use op_host_native::WidgetHostNative;

use crate::chat_session::provider_for_selected_model;

/// File extension for the active framework's generated component file.
/// Mirrors the TS download naming (`component.<ext>`).
fn framework_ext(fw: op_editor_core::codegen::Framework) -> &'static str {
    use op_editor_core::codegen::Framework;
    match fw {
        Framework::React | Framework::ReactNative => "tsx",
        Framework::Vue => "vue",
        Framework::Svelte => "svelte",
        Framework::Html => "html",
        Framework::Flutter => "dart",
        Framework::SwiftUi => "swift",
        Framework::Compose => "kt",
    }
}

/// Streamed from the worker to the UI pump.
pub enum CodegenDelta {
    Progress(CodeGenProgress),
    Done {
        code: String,
        degraded: bool,
        assets: Vec<AssetFile>,
    },
    Failed(String),
}

/// Monotonic run-epoch allocator — every generation run gets a fresh id
/// (agent-indicator run-epoch pattern: a stale run can be told apart from
/// the run that replaced it). The per-run mpsc channel already isolates
/// deltas; the epoch makes run identity explicit for the cancel path +
/// tests.
static NEXT_RUN_EPOCH: AtomicU64 = AtomicU64::new(1);

/// An in-flight generation. The UI pump drains `rx` each frame.
pub struct CodegenSession {
    pub(crate) rx: Receiver<CodegenDelta>,
    pub(crate) finished: bool,
    /// Target framework captured at launch, for the file extension.
    pub(crate) framework: op_editor_core::codegen::Framework,
    /// Shared abort flag (TS AbortController parity). Raised by
    /// [`CodegenSession::cancel`]; the worker observes it between stream
    /// deltas and pipeline steps and drives `CodegenPipeline::cancel`,
    /// while the pump drops every further delta from this run.
    pub(crate) cancel: Arc<AtomicBool>,
    /// This run's epoch (see [`NEXT_RUN_EPOCH`]). Production code tells
    /// runs apart by channel ownership (one rx per run) + the cancel flag;
    /// the stamp makes run identity explicit for tests / debugging.
    #[allow(dead_code)]
    pub(crate) run_epoch: u64,
}

impl CodegenSession {
    /// Abort this run: the worker stops at its next hook point and the
    /// pump drops every delta the run still emits, so a stale worker can
    /// never resurrect the panel after Cancel.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_canceled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

/// The completed result kept HOST-SIDE for Download — asset bytes are not
/// carried in the wasm-clean `editor_state`. Export AI Bundle no longer
/// reads it: the bundle is built fresh from the LIVE selection at click
/// time (TS parity), so no generation-time node JSON is stashed here.
#[derive(Default, Clone)]
pub struct CodegenResult {
    pub code: String,
    /// File extension for the active framework (e.g. "tsx", "vue", "html").
    pub framework_ext: String,
    pub assets: Vec<AssetFile>,
}

/// Drive the pipeline to completion against `provider`, emitting deltas on
/// `tx`. Runs on the worker thread (or synchronously in tests). Each model
/// request is run sequentially; the streamed text deltas are fed back into
/// the pipeline so the next `step()` can advance.
///
/// `cancel` is the session's shared abort flag: it is observed between
/// pipeline steps, between requests, and between stream deltas. Once
/// raised, `CodegenPipeline::cancel()` parks the machine in its terminal
/// Failed("Aborted") state, which ends the loop after one final (dropped
/// by the pump) terminal delta.
pub(crate) fn run_pipeline(
    provider: &dyn ChatProvider,
    input: CodegenInput,
    tx: &Sender<CodegenDelta>,
    cancel: &AtomicBool,
) {
    let mut pipe = CodegenPipeline::new(input);
    loop {
        if cancel.load(Ordering::Relaxed) {
            // Park the machine terminally — the next `step()` returns
            // Failed("Aborted") and the loop exits below.
            pipe.cancel();
        }
        match pipe.step() {
            PipelineStep::Dispatch(reqs) => {
                for req in reqs {
                    // Observe a cancel raised between requests: stop
                    // dispatching; the loop top cancels the pipeline.
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    // Expand the pipeline's skill NAMES into the final system
                    // prompt (budget 0 = no truncation cap).
                    let system = op_ai_skills::compose_system_prompt(&req.skills, 0);
                    let chat_req = ChatRequest {
                        system_prompt: system,
                        user_message: req.user_message.clone(),
                        // Codegen requests are self-contained — no
                        // chat-transcript history.
                        history: Vec::new(),
                        max_output_tokens: req.max_output_tokens,
                        thinking: req.thinking,
                        effort: req.effort,
                        attachments: Vec::new(),
                        // Codegen rides the provider's default model;
                        // the chat model picker doesn't govern it.
                        model: None,
                    };
                    // Drain the blocking provider iterator; the first `Error`
                    // ends this request and is reported back to the pipeline.
                    // A cancel observed between stream deltas abandons the
                    // request mid-stream (the loop top cancels the pipeline,
                    // so the partial buffer is never parsed).
                    let mut errored: Option<String> = None;
                    for delta in provider.send(chat_req) {
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        match delta {
                            ChatDelta::TextDelta(t) => pipe.on_delta(req.id, &t),
                            ChatDelta::Error(e) => {
                                errored = Some(e);
                                break;
                            }
                            ChatDelta::Done { .. } => break,
                            _ => {}
                        }
                    }
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    match errored {
                        Some(e) => pipe.on_error(req.id, e),
                        None => pipe.on_complete(req.id),
                    }
                    // Stream progress after each request so the panel advances.
                    if tx.send(CodegenDelta::Progress(pipe.progress())).is_err() {
                        return; // panel went away — stop early
                    }
                }
            }
            PipelineStep::Waiting => {}
            PipelineStep::Done {
                code,
                degraded,
                assets,
            } => {
                let _ = tx.send(CodegenDelta::Done {
                    code,
                    degraded,
                    assets,
                });
                return;
            }
            PipelineStep::Failed { message } => {
                let _ = tx.send(CodegenDelta::Failed(message));
                return;
            }
        }
    }
}

impl CodegenSession {
    /// Spawn a worker that drives the pipeline against `provider`. Returns
    /// immediately — the model turns run off the UI thread. `framework` is
    /// stashed on the session so the terminal `Done` can assemble the
    /// `CodegenResult` with the right file extension.
    pub fn start(
        provider: Box<dyn ChatProvider>,
        input: CodegenInput,
        framework: op_editor_core::codegen::Framework,
    ) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        std::thread::Builder::new()
            .name("op-codegen-turn".into())
            .spawn(move || {
                // Guard against a panic in the pipeline / provider so the UI
                // receives a terminal error instead of hanging in `Generating`
                // (the receiver would otherwise only observe a channel
                // disconnect — see pump's Disconnected branch).
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_pipeline(provider.as_ref(), input, &tx, &worker_cancel);
                }));
                if outcome.is_err() {
                    let _ = tx.send(CodegenDelta::Failed(
                        "Code generation failed unexpectedly".into(),
                    ));
                }
            })
            .expect("spawn op-codegen-turn worker");
        CodegenSession {
            rx,
            finished: false,
            framework,
            cancel,
            run_epoch: NEXT_RUN_EPOCH.fetch_add(1, Ordering::Relaxed),
        }
    }
}

/// Pump the in-flight generation's deltas into `editor_state.codegen`.
/// Clears `current` once the turn finishes and parks the completed result
/// (asset bytes) in `last_result`. Returns true when state changed so the
/// caller can dirty the redraw.
pub fn pump(
    host: &mut WidgetHostNative,
    current: &mut Option<CodegenSession>,
    last_result: &mut Option<CodegenResult>,
) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    if session.is_canceled() {
        // Canceled run: drop EVERY delta (Progress would otherwise flip
        // the phase back to Generating and a late Done / Failed would
        // overwrite the canceled UI state). Terminal events / disconnect
        // only retire the session.
        loop {
            match session.rx.try_recv() {
                Ok(CodegenDelta::Progress(_)) => {}
                Ok(CodegenDelta::Done { .. }) | Ok(CodegenDelta::Failed(_)) => {
                    session.finished = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    session.finished = true;
                    break;
                }
            }
        }
        if session.finished {
            *current = None;
        }
        return false;
    }
    let mut changed = false;
    loop {
        match session.rx.try_recv() {
            Ok(CodegenDelta::Progress(p)) => {
                let cg = &mut host.editor_state_mut().codegen;
                cg.progress = p;
                cg.phase = op_editor_core::codegen::CodegenPhase::Generating;
                changed = true;
            }
            Ok(CodegenDelta::Done {
                code,
                degraded,
                assets,
            }) => {
                let metas = assets
                    .iter()
                    .map(|a| op_editor_core::codegen::AssetMeta {
                        relative_path: a.relative_path.clone(),
                        byte_len: a.bytes.len(),
                    })
                    .collect();
                {
                    let cg = &mut host.editor_state_mut().codegen;
                    cg.code = code.clone();
                    cg.code_scroll = 0.0;
                    cg.code_selection = None;
                    cg.degraded = degraded;
                    cg.assets = metas;
                    cg.phase = op_editor_core::codegen::CodegenPhase::Complete;
                    cg.pending_generate = false;
                    cg.pending_regenerate = false;
                }
                *last_result = Some(CodegenResult {
                    code,
                    framework_ext: framework_ext(session.framework).into(),
                    assets,
                });
                session.finished = true;
                changed = true;
            }
            Ok(CodegenDelta::Failed(e)) => {
                let cg = &mut host.editor_state_mut().codegen;
                cg.error = Some(e);
                cg.phase = op_editor_core::codegen::CodegenPhase::Error;
                cg.pending_generate = false;
                cg.pending_regenerate = false;
                session.finished = true;
                changed = true;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => break,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // The worker dropped its sender without a terminal Done/Failed
                // (e.g. an unexpected early exit). Surface an error rather than
                // leaving the UI stuck in `Generating` with no live session.
                if !session.finished {
                    let cg = &mut host.editor_state_mut().codegen;
                    cg.error = Some("Code generation ended unexpectedly".into());
                    cg.phase = op_editor_core::codegen::CodegenPhase::Error;
                    cg.pending_generate = false;
                    cg.pending_regenerate = false;
                    changed = true;
                }
                session.finished = true;
                break;
            }
        }
    }
    if changed {
        host.mark_editor_state_dirty();
    }
    if session.finished {
        *current = None;
    }
    changed
}

/// Drain a Generate / Regenerate request raised by the Code panel and launch
/// a worker turn. Clears the pending flags first, then resolves the input
/// (selection, else the whole active page) + provider; nothing to generate
/// from (empty page / dead selection) or an unconfigured model surfaces an
/// inline error instead of starting a turn. Returns true when state changed
/// (a turn launched OR an error was written).
pub fn launch_codegen_if_pending(
    host: &mut WidgetHostNative,
    current: &mut Option<CodegenSession>,
) -> bool {
    // A LIVE run blocks a new launch; a canceled run still draining its
    // dropped deltas does not — the fresh run replaces it (and gets a
    // strictly larger run epoch), TS parity: cancel + regenerate is
    // immediate.
    if current.as_ref().is_some_and(|s| !s.is_canceled()) {
        return false;
    }
    let cg = &host.editor_state().codegen;
    if !cg.pending_generate && !cg.pending_regenerate {
        return false;
    }
    // Clear the flags first so a failed launch doesn't re-fire every frame.
    {
        let cg = &mut host.editor_state_mut().codegen;
        cg.pending_generate = false;
        cg.pending_regenerate = false;
    }
    let Some((input, _raw)) = crate::codegen_input::build_codegen_input(host.editor_state()) else {
        let cg = &mut host.editor_state_mut().codegen;
        cg.error = Some("Select nodes to generate code".into());
        cg.phase = op_editor_core::codegen::CodegenPhase::Error;
        return true;
    };
    // Capture the target framework BEFORE `input` is moved into the worker.
    let framework = host.editor_state().codegen.framework;
    let Some(provider) = provider_for_selected_model(host) else {
        let cg = &mut host.editor_state_mut().codegen;
        cg.error = Some("No model configured".into());
        cg.phase = op_editor_core::codegen::CodegenPhase::Error;
        return true;
    };
    {
        // Record the targets this run generates against (TS
        // `lastSelectionRef`) before the mutable codegen borrow.
        let selection_snapshot: Vec<String> = host
            .editor_state()
            .selection
            .set
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        let cg = &mut host.editor_state_mut().codegen;
        cg.progress = Default::default();
        cg.error = None;
        cg.phase = op_editor_core::codegen::CodegenPhase::Generating;
        cg.selection_snapshot = selection_snapshot;
    }
    *current = Some(CodegenSession::start(provider, input, framework));
    true
}

/// Drain a Cancel request raised by the Code panel (TS parity:
/// `abortRef.current?.abort()`). Raises the in-flight run's shared abort
/// flag — the worker stops at its next hook point — and leaves the
/// session parked so `pump` drops every delta the stale run still emits.
/// The UI phase was already flipped by the Cancel action itself
/// (idle, or complete when previous code exists).
pub fn drain_codegen_cancel_request(
    host: &mut WidgetHostNative,
    current: &mut Option<CodegenSession>,
) -> bool {
    if !std::mem::take(&mut host.editor_state_mut().codegen.pending_cancel) {
        return false;
    }
    if let Some(session) = current.as_ref() {
        session.cancel();
    }
    host.mark_editor_state_dirty();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::chat_provider::StopReason;
    use op_editor_core::codegen::Framework;

    /// Test-only provider that replays a DIFFERENT scripted turn each time
    /// `send` is called — `EchoProvider` replays the same script, which can't
    /// satisfy the pipeline's three distinct phases (planning → chunk →
    /// assembly). Interior mutability lets it pop one script per request.
    struct ScriptedProvider {
        scripts: std::sync::Mutex<std::collections::VecDeque<Vec<ChatDelta>>>,
    }

    impl ChatProvider for ScriptedProvider {
        fn provider_label(&self) -> &str {
            "scripted"
        }
        fn send(&self, _r: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            let next = self.scripts.lock().unwrap().pop_front().unwrap_or_default();
            Box::new(next.into_iter())
        }
    }

    #[test]
    fn framework_ext_maps_every_framework() {
        assert_eq!(framework_ext(Framework::React), "tsx");
        assert_eq!(framework_ext(Framework::ReactNative), "tsx");
        assert_eq!(framework_ext(Framework::Vue), "vue");
        assert_eq!(framework_ext(Framework::Svelte), "svelte");
        assert_eq!(framework_ext(Framework::Html), "html");
        assert_eq!(framework_ext(Framework::Flutter), "dart");
        assert_eq!(framework_ext(Framework::SwiftUi), "swift");
        assert_eq!(framework_ext(Framework::Compose), "kt");
    }

    fn turn(text: &str) -> Vec<ChatDelta> {
        vec![
            ChatDelta::TextDelta(text.into()),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            },
        ]
    }

    /// A parked (already canceled) run must emit nothing but the terminal
    /// Aborted failure — no model request is ever dispatched.
    #[test]
    fn run_pipeline_pre_canceled_emits_only_aborted_failure() {
        let provider = ScriptedProvider {
            scripts: std::sync::Mutex::new(std::collections::VecDeque::from(vec![turn("{}")])),
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = AtomicBool::new(true);
        run_pipeline(&provider, test_input(), &tx, &cancel);
        drop(tx);

        let deltas: Vec<CodegenDelta> = rx.into_iter().collect();
        assert_eq!(deltas.len(), 1, "exactly one terminal delta");
        match &deltas[0] {
            CodegenDelta::Failed(message) => assert!(message.contains("Aborted")),
            _ => panic!("expected the Aborted terminal failure"),
        }
        // The provider was never consulted.
        assert_eq!(provider.scripts.lock().unwrap().len(), 1);
    }

    /// Test-only provider that raises the shared cancel flag as a side
    /// effect of its first `send` — models the user pressing Cancel while
    /// the first (planning) request streams.
    struct CancelRaisingProvider {
        inner: ScriptedProvider,
        cancel: std::sync::Arc<AtomicBool>,
    }

    impl ChatProvider for CancelRaisingProvider {
        fn provider_label(&self) -> &str {
            "cancel-raising"
        }
        fn send(&self, r: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            self.cancel.store(true, Ordering::Relaxed);
            self.inner.send(r)
        }
    }

    /// A cancel raised mid-run stops the pipeline at the next hook point:
    /// the in-flight request's stream is abandoned, no later phase is
    /// dispatched, and the only delta is the terminal Aborted failure.
    #[test]
    fn run_pipeline_cancel_mid_run_stops_dispatch_and_aborts() {
        let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"r","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let provider = CancelRaisingProvider {
            inner: ScriptedProvider {
                scripts: std::sync::Mutex::new(std::collections::VecDeque::from(vec![
                    turn(plan),
                    turn("chunk code"),
                    turn("assembly code"),
                ])),
            },
            cancel: std::sync::Arc::clone(&cancel),
        };
        let (tx, rx) = std::sync::mpsc::channel();
        run_pipeline(&provider, test_input(), &tx, &cancel);
        drop(tx);

        let deltas: Vec<CodegenDelta> = rx.into_iter().collect();
        assert_eq!(deltas.len(), 1, "no Progress after a canceled stream");
        match &deltas[0] {
            CodegenDelta::Failed(message) => assert!(message.contains("Aborted")),
            _ => panic!("expected the Aborted terminal failure"),
        }
        // Only the planning request ran — chunk + assembly never dispatched.
        assert_eq!(provider.inner.scripts.lock().unwrap().len(), 2);
    }

    /// A canceled session's pump drops EVERYTHING the stale worker still
    /// emits: Progress must not flip the phase back to Generating, and a
    /// terminal Done must not overwrite the canceled UI state or park a
    /// last_result — it only retires the session.
    #[test]
    fn pump_after_cancel_drops_progress_and_terminal_deltas() {
        use op_editor_core::codegen::CodegenPhase;

        let (tx, rx) = std::sync::mpsc::channel();
        let mut current = Some(CodegenSession {
            rx,
            finished: false,
            framework: Framework::React,
            cancel: Arc::new(AtomicBool::new(false)),
            run_epoch: 1,
        });
        let mut last_result: Option<CodegenResult> = None;
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().codegen.phase = CodegenPhase::Generating;

        // Cancel action (property dispatch): flips phase + raises intent.
        host.editor_state_mut().codegen.phase = CodegenPhase::Idle;
        host.editor_state_mut().codegen.pending_cancel = true;
        assert!(drain_codegen_cancel_request(&mut host, &mut current));
        assert!(!host.editor_state().codegen.pending_cancel);
        assert!(current.as_ref().is_some_and(|s| s.is_canceled()));

        // The stale worker keeps streaming: progress, then terminal Done.
        tx.send(CodegenDelta::Progress(CodeGenProgress::default()))
            .unwrap();
        tx.send(CodegenDelta::Done {
            code: "stale code".into(),
            degraded: false,
            assets: Vec::new(),
        })
        .unwrap();

        let changed = pump(&mut host, &mut current, &mut last_result);
        assert!(!changed, "dropped deltas must not dirty the redraw");
        let cg = &host.editor_state().codegen;
        assert_eq!(cg.phase, CodegenPhase::Idle, "canceled state survives");
        assert!(cg.code.is_empty(), "stale Done must not land its code");
        assert!(last_result.is_none(), "no result parked for a canceled run");
        assert!(current.is_none(), "terminal delta retires the session");
    }

    /// A LIVE in-flight run blocks a new launch; a canceled one does not —
    /// the pending Generate proceeds (TS: cancel + regenerate immediately).
    #[test]
    fn launch_blocked_by_live_run_but_not_by_canceled_run() {
        use op_editor_core::codegen::CodegenPhase;

        let mut host = WidgetHostNative::new();
        // Empty the active page so the (canceled-gate-passing) launch
        // below has nothing to generate from — the whole-page fallback
        // would otherwise resolve the starter document's nodes and spin
        // up a real provider worker.
        host.editor_state_mut().active_children_mut().clear();
        host.editor_state_mut().clear_selection();

        // Live session → launch refuses.
        let (_tx_live, rx_live) = std::sync::mpsc::channel();
        let mut current = Some(CodegenSession {
            rx: rx_live,
            finished: false,
            framework: Framework::React,
            cancel: Arc::new(AtomicBool::new(false)),
            run_epoch: 1,
        });
        host.editor_state_mut().codegen.pending_generate = true;
        assert!(!launch_codegen_if_pending(&mut host, &mut current));
        assert!(host.editor_state().codegen.pending_generate);

        // Canceled session → launch proceeds (here onto an empty document,
        // so it surfaces the inline error instead of a fresh worker — the
        // point is that the canceled run no longer blocks the drain).
        current.as_ref().unwrap().cancel();
        assert!(launch_codegen_if_pending(&mut host, &mut current));
        assert!(!host.editor_state().codegen.pending_generate);
        assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Error);
    }

    /// Every `start` stamps a strictly larger run epoch, so the run that
    /// replaces a canceled one is distinguishable from it.
    #[test]
    fn start_allocates_monotonic_run_epochs() {
        let provider = || {
            Box::new(ScriptedProvider {
                scripts: std::sync::Mutex::new(std::collections::VecDeque::new()),
            })
        };
        let s1 = CodegenSession::start(provider(), test_input(), Framework::React);
        let s2 = CodegenSession::start(provider(), test_input(), Framework::React);
        assert!(s2.run_epoch > s1.run_epoch);
        assert!(!s1.is_canceled());
        s1.cancel();
        assert!(s1.is_canceled());
        assert!(!s2.is_canceled(), "cancel flags are per-run");
    }

    fn test_input() -> CodegenInput {
        CodegenInput {
            nodes_json: "[{\"type\":\"frame\",\"id\":\"n1\",\"children\":[]}]".to_string(),
            framework: Framework::React,
            variables_json: None,
            max_output_tokens: 4096,
            thinking: op_ai::chat_provider::ThinkingMode::Adaptive,
            effort: op_ai::chat_provider::EffortLevel::Low,
        }
    }

    #[test]
    fn run_pipeline_drives_three_phases_to_done() {
        // Phase 1: planning JSON (one chunk targeting node `n1`).
        let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"r","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
        // Phase 2: chunk code + contract.
        let chunk =
            "export default function Root(){}\n---CONTRACT---\n{\"componentName\":\"Root\"}";
        // Phase 3: assembly references the chunk component.
        let assembly = "export default function App(){ return <Root/> }";

        let provider = ScriptedProvider {
            scripts: std::sync::Mutex::new(std::collections::VecDeque::from(vec![
                turn(plan),
                turn(chunk),
                turn(assembly),
            ])),
        };

        let input = CodegenInput {
            nodes_json: "[{\"type\":\"frame\",\"id\":\"n1\",\"children\":[]}]".to_string(),
            framework: Framework::React,
            variables_json: None,
            max_output_tokens: 4096,
            thinking: op_ai::chat_provider::ThinkingMode::Adaptive,
            effort: op_ai::chat_provider::EffortLevel::Low,
        };

        let (tx, rx) = std::sync::mpsc::channel();
        run_pipeline(&provider, input, &tx, &AtomicBool::new(false));
        drop(tx);

        let deltas: Vec<CodegenDelta> = rx.into_iter().collect();
        assert!(!deltas.is_empty(), "pipeline must emit at least one delta");
        match deltas.last().expect("a terminal delta") {
            CodegenDelta::Done { code, .. } => {
                // The assembled output wires in the assembly turn's App shell.
                assert!(
                    code.contains("App"),
                    "final code should contain the assembled App component, got: {code}"
                );
            }
            CodegenDelta::Failed(message) => {
                panic!("pipeline failed instead of completing: {message}");
            }
            CodegenDelta::Progress(_) => {
                panic!("last delta should be terminal Done, not Progress");
            }
        }
    }
}
