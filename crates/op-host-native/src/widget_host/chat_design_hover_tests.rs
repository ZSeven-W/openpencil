use super::*;
use op_editor_ui::widgets::AIChatPlaceholder;
use op_editor_ui::Point2D;

#[test]
fn cursor_move_sets_chat_tab_hover_when_over_tab() {
    // Seed two tabs so the tab row renders (tab row only paints when
    // `tabs_snapshot.len() >= 1`). The default ChatSessions starts with
    // one implicit tab; `new_tab()` adds a second.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().chat.new_tab(); // now 2 tabs
    let viewport_w = 1440.0_f32;
    let viewport_h = 900.0_f32;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;

    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();

    // Verify that `tab_hover_at` agrees with our probe point before wiring.
    // Tab 0 body starts at tab_row_left = rect.origin.x + PAD + CHEVRON_W + PILL_GAP
    // (8 + 18 + 6 = 32 px from the panel left edge).  Center of the first tab
    // body: x = tab_row_left + TAB_MAX_W / 2, y = rect.origin.y + HEADER_HEIGHT / 2.
    let over_tab0 = Point2D::new(
        chat_rect.origin.x + 32.0 + 60.0, // tab row left + half TAB_MAX_W
        chat_rect.origin.y + 18.0,        // header mid
    );
    let panel = AIChatPlaceholder::from_editor(host.editor_state());
    assert_eq!(
        panel.tab_hover_at(chat_rect, over_tab0),
        Some(0),
        "tab_hover_at must return tab index 0 for a point inside the first tab body"
    );

    // Now drive the host cursor move and confirm the state field is updated.
    assert!(host.apply_cursor_move(over_tab0.x, over_tab0.y));
    assert_eq!(
        host.editor_state().editor_ui.chat_tab_hover,
        Some(0),
        "apply_cursor_move must write chat_tab_hover = Some(0) when cursor is over tab 0"
    );

    // Moving off the panel clears the hover.
    assert!(host.apply_cursor_move(0.0, 0.0));
    assert_eq!(
        host.editor_state().editor_ui.chat_tab_hover,
        None,
        "apply_cursor_move must clear chat_tab_hover when cursor leaves the panel"
    );
}

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
