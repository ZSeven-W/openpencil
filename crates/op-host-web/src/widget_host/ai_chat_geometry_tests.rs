use super::WidgetHost;

#[test]
fn composer_header_opens_the_pinned_chat_tab() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let before = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("composer card visible");
    let x = before.origin.x + before.size.x / 2.0;
    let y = before.origin.y + 15.0;

    assert!(host.apply_click(x, y, viewport_w, viewport_h));

    let after = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("pinned chat panel visible");
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        op_editor_core::LeftPanelTab::Chat
    );
    assert!(!host.editor_state.chat.maximized);
    assert_eq!(after, host.layers_content_rect(viewport_h));
    assert!(after.size.y > before.size.y);
}

#[test]
fn vscode_embed_has_no_chat_rect() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.embed = op_editor_core::EmbedHost::VsCode;
    assert!(host.ai_chat_rect(1600.0, 1000.0).is_none());
}

#[test]
fn ai_chat_collapse_click_stays_expanded_while_streaming_like_ts() {
    let mut host = WidgetHost::new();
    host.editor_state
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
        !host.editor_state.chat.is_minimized(),
        "TS immediately reopens a minimized chat while a response is streaming"
    );
}
