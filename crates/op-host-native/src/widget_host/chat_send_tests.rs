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
    host.editor_state_mut()
        .chat
        .set_input_text("design a login page");

    assert!(!host.apply_send());
    assert_eq!(host.editor_state().chat.input.text(), "design a login page");
    assert!(host.editor_state().chat.messages.is_empty());
    assert!(host.editor_state().chat.pending_send.is_none());
}

#[test]
fn apply_send_queues_chat_when_model_is_available() {
    let mut host = WidgetHostNative::new();
    seed_available_model(&mut host);
    host.editor_state_mut()
        .chat
        .set_input_text("design a login page");

    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().chat.pending_send.as_deref(),
        Some("design a login page")
    );
}

#[test]
fn chat_collapse_click_stays_expanded_while_streaming_like_ts() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat panel visible");

    assert!(host.apply_click(
        rect.origin.x + 18.0,
        rect.origin.y + 16.0,
        viewport_w,
        viewport_h
    ));

    assert!(
        !host.editor_state().chat.collapsed,
        "TS immediately reopens a minimized chat while a response is streaming"
    );
}
