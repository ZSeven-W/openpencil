//! Tests for `codegen_session.rs` — the generation pump, the launch
//! path, the per-framework result cache, and cancellation.
//!
//! Split out of `codegen_session.rs` (pure code motion) to keep that file
//! under the repo's 800-line-per-file cap.

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
/// dispatched. The planning-running snapshot precedes the cancellation;
/// no progress is emitted after the terminal Aborted failure.
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
    assert_eq!(deltas.len(), 2, "running snapshot then terminal failure");
    assert!(matches!(deltas[0], CodegenDelta::Progress(_)));
    match &deltas[1] {
        CodegenDelta::Failed(message) => assert!(message.contains("Aborted")),
        _ => panic!("expected the Aborted terminal failure"),
    }
    // Only the planning request ran — chunk + assembly never dispatched.
    assert_eq!(provider.inner.scripts.lock().unwrap().len(), 2);
}

/// A canceled session's pump drops EVERYTHING the stale worker still
/// emits: Progress must not flip the phase back to Generating, and a
/// terminal Done must not overwrite the canceled UI state or park a
/// result — it only retires the session.
#[test]
fn pump_after_cancel_drops_progress_and_terminal_deltas() {
    use op_editor_core::codegen::CodegenPhase;

    let (tx, rx) = std::sync::mpsc::channel();
    let mut current = Some(CodegenSession {
        rx,
        finished: false,
        framework: Framework::React,
        document_identity: (0, 0, 0),
        selection_snapshot: Vec::new(),
        model: None,
        cancel: Arc::new(AtomicBool::new(false)),
        run_epoch: 1,
    });
    let mut results = CodegenResults::default();
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

    let changed = pump(&mut host, &mut current, &mut results);
    assert!(!changed, "dropped deltas must not dirty the redraw");
    let cg = &host.editor_state().codegen;
    assert_eq!(cg.phase, CodegenPhase::Idle, "canceled state survives");
    assert!(cg.code.is_empty(), "stale Done must not land its code");
    assert!(results.is_empty(), "no result parked for a canceled run");
    assert!(current.is_none(), "terminal delta retires the session");
}

#[test]
fn failed_regeneration_keeps_previous_code_and_target_snapshot_together() {
    use op_editor_core::codegen::CodegenPhase;

    let mut host = WidgetHostNative::new();
    {
        let cg = &mut host.editor_state_mut().codegen;
        cg.framework = Framework::Html;
        cg.phase = CodegenPhase::Generating;
        cg.code = "<main>previous result</main>".into();
        cg.selection_snapshot = vec!["old-node".into()];
    }
    let launched_identity = document_identity(&host);
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(CodegenDelta::Failed("regeneration failed".into()))
        .expect("queue failure");
    drop(tx);
    let mut current = Some(CodegenSession {
        rx,
        finished: false,
        framework: Framework::Html,
        document_identity: launched_identity,
        selection_snapshot: vec!["new-node".into()],
        model: None,
        cancel: Arc::new(AtomicBool::new(false)),
        run_epoch: 1,
    });
    let mut results = CodegenResults::default();

    assert!(pump(&mut host, &mut current, &mut results));
    let cg = &host.editor_state().codegen;
    assert_eq!(cg.phase, CodegenPhase::Error);
    assert_eq!(cg.code, "<main>previous result</main>");
    assert_eq!(cg.selection_snapshot, ["old-node"]);

    // Switching away caches the Error-with-old-code result. Its code and
    // successful target snapshot must still restore as one unit.
    assert!(host
        .editor_state_mut()
        .codegen
        .select_framework(Framework::Vue));
    assert!(host
        .editor_state_mut()
        .codegen
        .select_framework(Framework::Html));
    assert_eq!(host.editor_state().codegen.selection_snapshot, ["old-node"]);
}

#[test]
fn successful_regeneration_commits_its_session_target_snapshot() {
    use op_editor_core::codegen::CodegenPhase;

    let mut host = WidgetHostNative::new();
    {
        let cg = &mut host.editor_state_mut().codegen;
        cg.phase = CodegenPhase::Generating;
        cg.code = "previous result".into();
        cg.selection_snapshot = vec!["old-node".into()];
    }
    let launched_identity = document_identity(&host);
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(CodegenDelta::Done {
        code: "new result".into(),
        degraded: false,
        assets: Vec::new(),
    })
    .expect("queue completion");
    drop(tx);
    let mut current = Some(CodegenSession {
        rx,
        finished: false,
        framework: Framework::React,
        document_identity: launched_identity,
        selection_snapshot: vec!["new-node".into()],
        model: None,
        cancel: Arc::new(AtomicBool::new(false)),
        run_epoch: 1,
    });
    let mut results = CodegenResults::default();

    assert!(pump(&mut host, &mut current, &mut results));
    let cg = &host.editor_state().codegen;
    assert_eq!(cg.phase, CodegenPhase::Complete);
    assert_eq!(cg.code, "new result");
    assert_eq!(cg.selection_snapshot, ["new-node"]);
}

#[test]
fn pump_drops_terminal_delta_after_live_document_replacement() {
    let mut host = WidgetHostNative::new();
    let launched_identity = document_identity(&host);

    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(CodegenDelta::Done {
        code: "old document code".into(),
        degraded: false,
        assets: Vec::new(),
    })
    .expect("queue old completion");
    drop(tx);
    let mut current = Some(CodegenSession {
        rx,
        finished: false,
        framework: Framework::React,
        document_identity: launched_identity,
        selection_snapshot: Vec::new(),
        model: None,
        cancel: Arc::new(AtomicBool::new(false)),
        run_epoch: 1,
    });
    let mut results = CodegenResults::default();

    // MCP/live-sync replaces the document in-place: the host epoch stays
    // the same, but EditorState's document generation advances.
    host.editor_state_mut()
        .replace_document(op_editor_core::EditorState::new().doc);
    assert_ne!(document_identity(&host), launched_identity);

    assert!(!pump(&mut host, &mut current, &mut results));
    assert!(current.is_none(), "the superseded run is retired");
    assert!(
        host.editor_state().codegen.code.is_empty(),
        "old completion must not paint into the replacement document"
    );
    assert!(
        results.is_empty(),
        "old completion must not become downloadable"
    );
}

#[test]
fn pump_drops_late_done_after_remote_commit_preserves_document_generation() {
    let mut host = WidgetHostNative::new();
    let launched_identity = document_identity(&host);
    let host_epoch = host.document_epoch();
    let document_generation = host.editor_state().document_generation();

    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(CodegenDelta::Done {
        code: "pre-commit code".into(),
        degraded: false,
        assets: Vec::new(),
    })
    .expect("queue old completion");
    drop(tx);
    let mut current = Some(CodegenSession {
        rx,
        finished: false,
        framework: Framework::React,
        document_identity: launched_identity,
        selection_snapshot: Vec::new(),
        model: None,
        cancel: Arc::new(AtomicBool::new(false)),
        run_epoch: 1,
    });
    let mut results = CodegenResults::default();

    let remote = host.editor_state().doc.clone();
    host.editor_state_mut()
        .install_verified_document(remote, op_editor_core::EditOrigin::RemoteCommit)
        .expect("remote commit installs");
    assert_eq!(host.document_epoch(), host_epoch);
    assert_eq!(
        host.editor_state().document_generation(),
        document_generation
    );
    assert_ne!(document_identity(&host), launched_identity);

    assert!(!pump(&mut host, &mut current, &mut results));
    assert!(current.is_none());
    assert!(host.editor_state().codegen.code.is_empty());
    assert!(results.is_empty());
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
        document_identity: (0, 0, 0),
        selection_snapshot: Vec::new(),
        model: None,
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

#[test]
fn stale_document_run_does_not_block_new_document_launch() {
    use op_editor_core::codegen::CodegenPhase;

    let mut host = WidgetHostNative::new();
    let old_identity = document_identity(&host);
    let (_tx, rx) = std::sync::mpsc::channel();
    let mut current = Some(CodegenSession {
        rx,
        finished: false,
        framework: Framework::React,
        document_identity: old_identity,
        selection_snapshot: Vec::new(),
        model: None,
        cancel: Arc::new(AtomicBool::new(false)),
        run_epoch: 1,
    });

    host.editor_state_mut()
        .replace_document(op_editor_core::EditorState::new().doc);
    assert_ne!(document_identity(&host), old_identity);
    host.editor_state_mut().codegen.pending_generate = true;

    // The replacement document is empty, so this reaches the normal
    // inline launch error. Critically, the old live session does not
    // consume/block the press.
    assert!(launch_codegen_if_pending(&mut host, &mut current));
    assert!(current.is_none());
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Error);
    assert!(!host.editor_state().codegen.pending_generate);
}

#[test]
fn disconnected_fixed_provider_fails_before_spawning_a_worker() {
    use op_editor_core::codegen::CodegenPhase;

    let mut host = WidgetHostNative::new();
    host.editor_state_mut().codegen.pending_generate = true;
    let mut current = None;

    assert!(launch_codegen_if_pending(&mut host, &mut current));
    assert!(current.is_none(), "an unconfigured CLI must not be spawned");
    let cg = &host.editor_state().codegen;
    assert_eq!(cg.phase, CodegenPhase::Error);
    assert!(
        cg.error
            .as_deref()
            .is_some_and(|message| message.contains("Agent Settings")),
        "error should direct the user to provider setup: {:?}",
        cg.error
    );
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

#[test]
fn dropping_a_session_raises_its_worker_cancel_token() {
    let (_tx, rx) = std::sync::mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let _session = CodegenSession {
            rx,
            finished: false,
            framework: Framework::React,
            document_identity: (0, 0, 0),
            selection_snapshot: Vec::new(),
            model: None,
            cancel: Arc::clone(&cancel),
            run_epoch: 1,
        };
        assert!(!cancel.load(Ordering::Relaxed));
    }
    assert!(cancel.load(Ordering::Relaxed));
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
    let chunk = "export default function Root(){}\n---CONTRACT---\n{\"componentName\":\"Root\"}";
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
