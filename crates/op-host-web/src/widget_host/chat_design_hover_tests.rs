use super::WidgetHost;
use op_editor_ui::widgets::AIChatPlaceholder;
use op_editor_ui::Point2D;

#[test]
fn cursor_move_tracks_hovered_design_json_card_for_copy_reveal() {
    let mut host = WidgetHost::new();
    host.editor_state
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
    let panel = AIChatPlaceholder::from_editor(&host.editor_state);
    let point = Point2D::new(chat_rect.origin.x + 24.0, chat_rect.origin.y + 52.0);
    assert_eq!(panel.design_block_hover_at(chat_rect, point), Some((0, 0)));

    assert!(host.apply_cursor_move(point.x, point.y));

    assert_eq!(
        host.editor_state.editor_ui.chat_design_block_hover,
        Some((0, 0))
    );
}

#[test]
fn cursor_move_tracks_chat_footer_buttons() {
    let mut host = WidgetHost::new();
    host.editor_state
        .chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
    host.editor_state.chat.set_input_text("design a login page");
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    // Footer right cluster (#38/#42): ⚡ speed | 📎 attach | 🎨 palette | ↑ send,
    // laid out right-to-left. The attach icon centres at `size.x - 88` (the old
    // `-68` now lands on the 🎨 palette icon that was added to the cluster).
    let attach = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 88.0,
        chat_rect.origin.y + chat_rect.size.y - 19.0,
    );
    let send = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 28.0,
        chat_rect.origin.y + chat_rect.size.y - 19.0,
    );

    assert!(host.apply_cursor_move(attach.x, attach.y));
    assert_eq!(
        host.editor_state.editor_ui.chat_footer_hover,
        Some(op_editor_core::ChatFooterButton::AddAttachment)
    );

    assert!(host.apply_cursor_move(send.x, send.y));
    assert_eq!(
        host.editor_state.editor_ui.chat_footer_hover,
        Some(op_editor_core::ChatFooterButton::Send)
    );
}

#[test]
fn cursor_move_tracks_chat_model_picker_row_hover() {
    use op_editor_ui::widgets::ai_chat_model_picker::{
        MODEL_GROUP_H, MODEL_PICKER_PAD_Y, MODEL_ROW_H, MODEL_SEARCH_H,
    };

    let mut host = WidgetHost::new();
    host.editor_state
        .chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
    host.editor_state.editor_ui.chat_model_picker.open = true;
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    let picker = AIChatPlaceholder::from_editor(&host.editor_state)
        .model_picker_bounds(chat_rect)
        .unwrap();
    let row = Point2D::new(
        picker.origin.x + 48.0,
        picker.origin.y + MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0,
    );

    assert!(host.apply_cursor_move(row.x, row.y));

    assert_eq!(host.editor_state.editor_ui.chat_model_picker.hover, Some(0));
}
