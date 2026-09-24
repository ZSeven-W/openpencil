use super::*;

use op_ai::chat_provider::{EffortLevel, ThinkingMode};

struct NeverProvider;

impl ChatProvider for NeverProvider {
    fn provider_label(&self) -> &str {
        "never"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        panic!("a failed spawn must never run the provider")
    }
}

fn input() -> CodegenInput {
    CodegenInput {
        nodes_json: "[]".into(),
        framework: Framework::React,
        variables_json: None,
        themes_json: None,
        components_json: None,
        max_output_tokens: 4096,
        thinking: ThinkingMode::Adaptive,
        effort: EffortLevel::Low,
    }
}

#[test]
fn injected_thread_spawn_failure_is_returned_as_a_typed_error() {
    let result = CodegenSession::try_start_with_model_and_spawner(
        Box::new(NeverProvider),
        input(),
        Framework::React,
        None,
        |_worker| Err(std::io::Error::other("injected spawn failure")),
    );

    let Err(CodegenStartError::ThreadSpawn { source }) = result else {
        panic!("spawn failure must be returned")
    };
    assert_eq!(source.kind(), std::io::ErrorKind::Other);
    assert_eq!(source.to_string(), "injected spawn failure");
}

#[test]
fn deterministic_target_queues_its_result_without_a_provider() {
    let session = CodegenSession::start_deterministic(
        CodegenInput {
            nodes_json: r#"[{"type":"frame","id":"root","name":"Card","width":320}]"#.into(),
            framework: Framework::ReactTailwind,
            ..input()
        },
        Framework::ReactTailwind,
    );
    assert!(matches!(
        session.rx.try_recv(),
        Ok(CodegenDelta::Progress(_))
    ));
    let Ok(CodegenDelta::Done { code, degraded, .. }) = session.rx.try_recv() else {
        panic!("deterministic session must queue Done");
    };
    assert!(!degraded);
    assert!(code.contains("export default function Card()"), "{code}");
    assert!(code.contains("w-80"), "{code}");
}

#[test]
fn model_backed_target_is_refused_by_the_deterministic_entry() {
    let session = CodegenSession::start_deterministic(input(), Framework::React);
    let _progress = session.rx.try_recv();
    let Ok(CodegenDelta::Failed(message)) = session.rx.try_recv() else {
        panic!("a model-backed target must not pretend to be deterministic");
    };
    assert!(
        message.contains("not a deterministic code target"),
        "{message}"
    );
}
