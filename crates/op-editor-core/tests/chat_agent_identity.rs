use op_editor_core::{AgentProvider, ChatMessage, ChatState, ModelEntry};

#[test]
fn chat_message_constructors_default_to_no_agent_identity() {
    for message in [
        ChatMessage::user("prompt"),
        ChatMessage::assistant("answer"),
        ChatMessage::assistant_streaming(),
    ] {
        assert_eq!(message.agent_name, None);
        assert_eq!(message.agent_color, None);
    }
}

#[test]
fn begin_send_stamps_selected_agent_provider_name() {
    let mut chat = ChatState {
        available_models: vec![ModelEntry::new(
            AgentProvider::ClaudeCode,
            "sonnet",
            "Sonnet",
        )],
        ..Default::default()
    };
    chat.set_input_text("hello");

    assert!(chat.begin_send());

    let assistant = &chat.messages[1];
    assert_eq!(assistant.agent_name.as_deref(), Some("Claude Code"));
    assert_eq!(assistant.agent_color, None);
}
