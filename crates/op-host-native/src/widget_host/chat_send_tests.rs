use super::WidgetHostNative;

fn seed_available_model(host: &mut WidgetHostNative) {
    host.editor_state_mut()
        .chat
        .available_models
        .push(op_editor_core::ModelEntry::new(
            op_editor_core::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
}

#[test]
fn apply_send_ignores_chat_when_no_model_is_available() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().chat.input = "design a login page".into();

    assert!(!host.apply_send());
    assert_eq!(host.editor_state().chat.input, "design a login page");
    assert!(host.editor_state().chat.messages.is_empty());
    assert!(host.editor_state().chat.pending_send.is_none());
}

#[test]
fn apply_send_queues_chat_when_model_is_available() {
    let mut host = WidgetHostNative::new();
    seed_available_model(&mut host);
    host.editor_state_mut().chat.input = "design a login page".into();

    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().chat.pending_send.as_deref(),
        Some("design a login page")
    );
}
