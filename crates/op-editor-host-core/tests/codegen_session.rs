use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason, ThinkingMode,
};
use op_codegen::ai::types::CodegenInput;
use op_editor_core::codegen::Framework;
use op_editor_host_core::codegen_session::{run_pipeline, CodegenDelta, CodegenSession};

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
        max_output_tokens: 4096,
        thinking: ThinkingMode::Adaptive,
        effort: EffortLevel::Low,
    }
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
