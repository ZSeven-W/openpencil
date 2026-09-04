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
