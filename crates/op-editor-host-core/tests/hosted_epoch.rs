use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};
use op_editor_host_core::chat::ChatSession;
use op_editor_host_core::codegen_session::CodegenSession;

struct HostedProvider;

impl ChatProvider for HostedProvider {
    fn provider_label(&self) -> &str {
        "hosted-test"
    }
    fn hosted_session_epoch(&self) -> Option<u64> {
        Some(42)
    }
    fn send(&self, _: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(std::iter::empty())
    }
}

#[test]
fn chat_and_codegen_capture_provider_account_epoch_before_launch() {
    let chat = ChatSession::start(Box::new(HostedProvider), ChatRequest::default());
    assert_eq!(chat.hosted_session_epoch(), Some(42));

    let input = op_codegen::ai::types::CodegenInput {
        nodes_json: "[]".into(),
        framework: op_editor_core::codegen::Framework::React,
        variables_json: None,
        themes_json: None,
        components_json: None,
        max_output_tokens: 128,
        thinking: Default::default(),
        effort: Default::default(),
    };
    let codegen = CodegenSession::try_start(
        Box::new(HostedProvider),
        input,
        op_editor_core::codegen::Framework::React,
    )
    .unwrap();
    assert_eq!(codegen.hosted_session_epoch, Some(42));
}
