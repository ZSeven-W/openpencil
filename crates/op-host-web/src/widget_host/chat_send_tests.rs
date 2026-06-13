use super::WidgetHost;
use op_editor_core::chat::{AgentProvider, ChatAttachment, ChatRole, ModelEntry};

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
