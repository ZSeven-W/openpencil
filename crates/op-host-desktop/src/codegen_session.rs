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

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};
#[cfg(test)]
use op_codegen::ai::types::CodegenInput;
use op_editor_host_core::codegen::framework_ext;
#[cfg(test)]
use op_editor_host_core::codegen_session::run_pipeline;
pub use op_editor_host_core::codegen_session::{CodegenDelta, CodegenResult, CodegenSession};
use op_host_native::WidgetHostNative;

use crate::chat_session::provider_for_selected_model;

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
                    cg.code_scroll.offset = 0.0;
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
    use op_editor_core::codegen::{CodeGenProgress, Framework};

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
