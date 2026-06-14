use super::*;
use op_editor_ui::widgets::AIChatPlaceholder;
use op_editor_ui::Point2D;

#[test]
fn cursor_move_tracks_hovered_design_json_card_for_copy_reveal() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant(
            r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
        ));
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    let panel = AIChatPlaceholder::from_editor(host.editor_state());
    let point = Point2D::new(chat_rect.origin.x + 24.0, chat_rect.origin.y + 52.0);
    assert_eq!(panel.design_block_hover_at(chat_rect, point), Some((0, 0)));

    assert!(host.apply_cursor_move(point.x, point.y));

    assert_eq!(
        host.editor_state().editor_ui.chat_design_block_hover,
        Some((0, 0))
    );
}

#[test]
fn cursor_move_tracks_chat_footer_buttons() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
    host.editor_state_mut()
        .chat
        .set_input_text("design a login page");
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    let attach = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 68.0,
        chat_rect.origin.y + chat_rect.size.y - 19.0,
    );
    let send = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 28.0,
        chat_rect.origin.y + chat_rect.size.y - 19.0,
    );

    assert!(host.apply_cursor_move(attach.x, attach.y));
    assert_eq!(
        host.editor_state().editor_ui.chat_footer_hover,
        Some(op_editor_core::ChatFooterButton::AddAttachment)
    );

    assert!(host.apply_cursor_move(send.x, send.y));
    assert_eq!(
        host.editor_state().editor_ui.chat_footer_hover,
        Some(op_editor_core::ChatFooterButton::Send)
    );
}

#[test]
fn cursor_move_tracks_quick_action_card_hover() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    let over_first = Point2D::new(chat_rect.origin.x + 100.0, chat_rect.origin.y + 104.0);
    let off_cards = Point2D::new(chat_rect.origin.x + 24.0, chat_rect.origin.y + 260.0);

    assert!(host.apply_cursor_move(over_first.x, over_first.y));
    assert_eq!(host.editor_state().editor_ui.chat_example_hover, Some(0));

    assert!(host.apply_cursor_move(off_cards.x, off_cards.y));
    assert_eq!(host.editor_state().editor_ui.chat_example_hover, None);
}
