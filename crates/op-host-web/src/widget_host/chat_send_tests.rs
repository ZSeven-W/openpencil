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
    let attach = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 68.0,
        chat_rect.origin.y + chat_rect.size.y - 19.0,
    );

    assert!(host.apply_click(attach.x, attach.y, viewport_w, viewport_h));
    assert!(host.editor_state.chat.pending_attachment_pick);
}
