use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason, ThinkingMode,
};
use op_codegen::ai::types::CodegenInput;
use op_editor_core::codegen::Framework;
use op_editor_host_core::codegen_session::{
    run_pipeline, run_pipeline_with_model, CodegenDelta, CodegenSession,
};

struct ScriptedProvider {
    scripts: Mutex<VecDeque<Vec<ChatDelta>>>,
}

impl ChatProvider for ScriptedProvider {
    fn provider_label(&self) -> &str {
        "scripted"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let next = self.scripts.lock().unwrap().pop_front().unwrap_or_default();
        Box::new(next.into_iter())
    }
}

fn turn(text: &str) -> Vec<ChatDelta> {
    vec![
        ChatDelta::TextDelta(text.into()),
        ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        },
    ]
}

fn test_input() -> CodegenInput {
    CodegenInput {
        nodes_json: "[{\"type\":\"frame\",\"id\":\"n1\",\"children\":[]}]".to_string(),
        framework: Framework::React,
        variables_json: None,
        themes_json: None,
        components_json: None,
        max_output_tokens: 4096,
        thinking: ThinkingMode::Adaptive,
        effort: EffortLevel::Low,
    }
}

fn invalid_fallback_input() -> CodegenInput {
    let mut input = test_input();
    // The planner can target this id, while the deterministic fallback cannot
    // deserialize a numeric node type as a canonical PenNode. Diagnostic
    // tests therefore remain terminal after every rescue layer.
    input.nodes_json = r#"[{"type":42,"id":"n1","children":[]}]"#.to_string();
    input
}

#[test]
fn run_pipeline_pre_canceled_emits_only_aborted_failure() {
    let provider = ScriptedProvider {
        scripts: Mutex::new(VecDeque::from(vec![turn("{}")])),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let cancel = AtomicBool::new(true);
    run_pipeline(&provider, test_input(), &tx, &cancel);
    drop(tx);

    let deltas: Vec<CodegenDelta> = rx.into_iter().collect();
    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        CodegenDelta::Failed(message) => assert!(message.contains("Aborted")),
        _ => panic!("expected aborted failure"),
    }
    assert_eq!(provider.scripts.lock().unwrap().len(), 1);
}

#[test]
fn run_pipeline_drives_three_phases_to_done() {
    let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"r","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
    let chunk = "export default function Root(){}\n---CONTRACT---\n{\"componentName\":\"Root\"}";
    let assembly = "export default function App(){ return <Root/> }";
    let provider = ScriptedProvider {
        scripts: Mutex::new(VecDeque::from(vec![
            turn(plan),
            turn(chunk),
            turn(assembly),
        ])),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    run_pipeline(&provider, test_input(), &tx, &AtomicBool::new(false));
    drop(tx);

    let deltas: Vec<CodegenDelta> = rx.into_iter().collect();
    assert!(!deltas.is_empty());
    match deltas.last().expect("terminal delta") {
        CodegenDelta::Done { code, .. } => assert!(code.contains("App")),
        CodegenDelta::Failed(message) => panic!("pipeline failed: {message}"),
        CodegenDelta::Progress(_) => panic!("last delta should be terminal"),
    }
}

#[test]
fn start_allocates_monotonic_run_epochs_and_independent_cancel_flags() {
    let provider = || {
        Box::new(ScriptedProvider {
            scripts: Mutex::new(VecDeque::new()),
        }) as Box<dyn ChatProvider>
    };
    let s1 = CodegenSession::start(provider(), test_input(), Framework::React);
    let s2 = CodegenSession::start(provider(), test_input(), Framework::React);
    assert!(s2.run_epoch > s1.run_epoch);
    assert!(!s1.is_canceled());
    s1.cancel();
    assert!(s1.is_canceled());
    assert!(!s2.is_canceled());
}

struct RecordingProvider {
    scripts: Mutex<VecDeque<Vec<ChatDelta>>>,
    models: Mutex<Vec<Option<String>>>,
    system_prompts: Mutex<Vec<String>>,
}

impl ChatProvider for RecordingProvider {
    fn provider_label(&self) -> &str {
        "recording"
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.system_prompts
            .lock()
            .unwrap()
            .push(request.system_prompt.clone());
        self.models.lock().unwrap().push(request.model);
        let next = self.scripts.lock().unwrap().pop_front().unwrap_or_default();
        Box::new(next.into_iter())
    }
}

#[test]
fn selected_model_is_forwarded_to_every_codegen_phase() {
    let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"r","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
    let provider = RecordingProvider {
        scripts: Mutex::new(VecDeque::from(vec![
            turn(plan),
            turn("export default function Root(){}\n---CONTRACT---\n{\"componentName\":\"Root\"}"),
            turn("export default function App(){ return <Root/> }"),
        ])),
        models: Mutex::new(Vec::new()),
        system_prompts: Mutex::new(Vec::new()),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    run_pipeline_with_model(
        &provider,
        test_input(),
        Some("claude-sonnet-4-6"),
        &tx,
        &AtomicBool::new(false),
    );
    drop(tx);

    assert!(matches!(
        rx.into_iter().last(),
        Some(CodegenDelta::Done { .. })
    ));
    assert_eq!(
        *provider.models.lock().unwrap(),
        vec![
            Some("claude-sonnet-4-6".to_string()),
            Some("claude-sonnet-4-6".to_string()),
            Some("claude-sonnet-4-6".to_string()),
        ]
    );
    assert!(provider
        .system_prompts
        .lock()
        .unwrap()
        .iter()
        .all(|prompt| prompt.contains("NEVER use tools")));
}

#[test]
fn terminal_failure_keeps_actionable_provider_diagnostics() {
    let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"r","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
    let failed_turn = || {
        vec![ChatDelta::Error(
            "authentication rejected by provider".into(),
        )]
    };
    let provider = ScriptedProvider {
        scripts: Mutex::new(VecDeque::from(vec![
            turn(plan),
            failed_turn(),
            failed_turn(),
            failed_turn(),
        ])),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    run_pipeline_with_model(
        &provider,
        invalid_fallback_input(),
        Some("test-model"),
        &tx,
        &AtomicBool::new(false),
    );
    drop(tx);

    let deltas: Vec<_> = rx.into_iter().collect();
    let terminal = deltas.last().expect("terminal delta");
    assert!(deltas.iter().any(|delta| matches!(
        delta,
        CodegenDelta::Progress(progress)
            if progress.chunks.iter().any(|chunk| chunk.status == op_editor_core::codegen::ChunkStatus::Failed)
    )), "terminal progress must expose the failed chunk");
    let CodegenDelta::Failed(message) = terminal else {
        panic!("expected terminal failure");
    };
    assert!(message.contains("Code generation failed"), "{message}");
    assert!(
        message.contains("provider=scripted; model=test-model"),
        "{message}"
    );
    assert!(message.contains("chunk c1"), "{message}");
    assert!(
        message.contains("authentication rejected by provider"),
        "{message}"
    );
}

#[test]
fn tool_use_and_token_limit_retry_then_use_deterministic_fallback() {
    let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"r","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
    let provider = ScriptedProvider {
        scripts: Mutex::new(VecDeque::from(vec![
            turn(plan),
            vec![ChatDelta::ToolUse {
                name: "Read".into(),
                args: "{}".into(),
            }],
            vec![ChatDelta::Done {
                stop_reason: StopReason::MaxTokens,
            }],
            vec![ChatDelta::Error("single-file rescue failed".into())],
        ])),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    run_pipeline_with_model(&provider, test_input(), None, &tx, &AtomicBool::new(false));
    drop(tx);

    let deltas: Vec<_> = rx.into_iter().collect();
    assert!(deltas.iter().any(|delta| matches!(
        delta,
        CodegenDelta::Progress(progress)
            if progress.chunks.iter().any(|chunk| chunk.status == op_editor_core::codegen::ChunkStatus::Failed)
    )), "chunk must become failed before rescue");
    let Some(CodegenDelta::Done { code, degraded, .. }) = deltas.last() else {
        panic!("expected deterministic fallback success");
    };
    assert!(*degraded, "local fallback must be marked degraded");
    assert!(!code.trim().is_empty(), "local fallback must produce code");
    assert!(
        provider.scripts.lock().unwrap().is_empty(),
        "tool-use, token-limit retry, and rescue scripts must all run"
    );
}

#[test]
fn empty_text_turn_retries_then_uses_deterministic_fallback() {
    let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"r","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
    let empty_turn = || {
        vec![ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        }]
    };
    let provider = ScriptedProvider {
        scripts: Mutex::new(VecDeque::from(vec![
            turn(plan),
            empty_turn(),
            empty_turn(),
            vec![ChatDelta::Error("single-file rescue failed".into())],
        ])),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    run_pipeline_with_model(&provider, test_input(), None, &tx, &AtomicBool::new(false));
    drop(tx);

    let terminal = rx.into_iter().last().expect("terminal delta");
    let CodegenDelta::Done { code, degraded, .. } = terminal else {
        panic!("expected deterministic fallback success");
    };
    assert!(degraded, "local fallback must be marked degraded");
    assert!(!code.trim().is_empty(), "local fallback must produce code");
    assert!(
        provider.scripts.lock().unwrap().is_empty(),
        "retry must run"
    );
}

#[test]
fn unterminated_partial_stream_is_retried_instead_of_accepted() {
    let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"r","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
    let provider = ScriptedProvider {
        scripts: Mutex::new(VecDeque::from(vec![
            turn(plan),
            vec![ChatDelta::TextDelta(
                "export default function Truncated(){ return <main>".into(),
            )],
            turn(
                "export default function Root(){ return <main/> }\n---CONTRACT---\n{\"componentName\":\"Root\"}",
            ),
            turn("export default function App(){ return <Root/> }"),
        ])),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    run_pipeline_with_model(&provider, test_input(), None, &tx, &AtomicBool::new(false));
    drop(tx);

    let Some(CodegenDelta::Done { code, .. }) = rx.into_iter().last() else {
        panic!("expected the retried pipeline to complete");
    };
    assert!(code.contains("App"), "{code}");
    assert!(provider.scripts.lock().unwrap().is_empty());
}

#[test]
fn oversized_provider_stream_is_bounded_and_falls_back() {
    let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"r","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
    let oversized = format!(
        "export default function Root(){{/*{}*/}}",
        "x".repeat(70 * 1024)
    );
    let provider = ScriptedProvider {
        scripts: Mutex::new(VecDeque::from(vec![
            turn(plan),
            turn(&oversized),
            turn(&oversized),
            vec![ChatDelta::Error("rescue unavailable".into())],
        ])),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    run_pipeline_with_model(&provider, test_input(), None, &tx, &AtomicBool::new(false));
    drop(tx);

    let Some(CodegenDelta::Done { code, degraded, .. }) = rx.into_iter().last() else {
        panic!("expected deterministic fallback success");
    };
    assert!(degraded);
    assert!(code.len() < oversized.len());
    assert!(provider.scripts.lock().unwrap().is_empty());
}
