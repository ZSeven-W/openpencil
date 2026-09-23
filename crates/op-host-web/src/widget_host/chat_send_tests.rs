use super::WidgetHost;
use op_editor_core::chat::{AgentProvider, ChatAttachment, ChatRole, ModelEntry};
use op_editor_ui::Point2D;

#[test]
fn send_allows_attachment_only_chat_turn() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    host.editor_state
        .chat
        .available_models
        .push(ModelEntry::new(AgentProvider::CodexCli, "gpt-5", "GPT-5"));
    assert!(host.editor_state.chat.add_attachment(ChatAttachment {
        name: "reference.png".into(),
        media_type: "image/png".into(),
        data: vec![1, 2, 3],
    }));

    assert!(host.apply_send());

    let messages = &host.editor_state.chat.messages;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, ChatRole::User);
    assert_eq!(messages[0].content, "");
    assert_eq!(messages[0].images.len(), 1);
    assert_eq!(messages[1].role, ChatRole::Assistant);
}

#[test]
fn attachment_button_queues_web_file_picker_like_native() {
    let mut host = WidgetHost::new();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let chat_rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat panel visible");
    // Footer right cluster: ⚡ speed | 📎 attach | ↑ send (the inert palette
    // slot is gone). The 📎 attach icon centres at `size.x - 60`.
    let attach = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 60.0,
        chat_rect.origin.y + chat_rect.size.y - 19.0,
    );

    assert!(host.apply_click(attach.x, attach.y, viewport_w, viewport_h));
    assert!(host.editor_state.chat.pending_attachment_pick);
}

#[test]
fn composer_card_send_click_opens_the_agent_tab() {
    let mut host = WidgetHost::new();
    host.editor_state
        .chat
        .available_models
        .push(ModelEntry::new(AgentProvider::CodexCli, "gpt-5", "GPT-5"));
    host.editor_state.chat.set_input_text("design a login page");
    // The default rail shows Layers, so the chat is the composer-only
    // card at the canvas floor.
    assert!(host.editor_state.editor_ui.chat_composer_only());
    let card = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("composer card visible");
    let send = Point2D::new(
        card.origin.x + card.size.x - 28.0,
        card.origin.y + card.size.y - 19.0,
    );

    assert!(host.apply_click(send.x, send.y, 1200.0, 800.0));

    // A send from the card starts a conversation, and a conversation
    // lives in the rail's Agent tab — the click hands the user there so
    // the reply does not stream somewhere they cannot see.
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        op_editor_core::LeftPanelTab::Chat,
        "the send must land the user in the Agent tab"
    );
    assert!(!host.editor_state.editor_ui.chat_composer_only());
    assert_eq!(
        host.editor_state.chat.pending_send.as_deref(),
        Some("design a login page")
    );
}

#[test]
fn composer_card_enter_send_opens_the_agent_tab() {
    let mut host = WidgetHost::new();
    host.editor_state
        .chat
        .available_models
        .push(ModelEntry::new(AgentProvider::CodexCli, "gpt-5", "GPT-5"));
    host.editor_state.chat.set_input_text("design a login page");
    assert!(host.editor_state.editor_ui.chat_composer_only());

    assert!(host.apply_send());

    // Same contract as the card's Send button: the Enter-key send moves
    // the user to the rail's Agent tab, where the reply is.
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        op_editor_core::LeftPanelTab::Chat,
        "the Enter send must land the user in the Agent tab"
    );
    assert!(!host.editor_state.editor_ui.chat_composer_only());
    assert_eq!(
        host.editor_state.chat.pending_send.as_deref(),
        Some("design a login page")
    );
}
