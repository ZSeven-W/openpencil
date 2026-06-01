use super::WidgetHost;

#[test]
fn ai_chat_maximize_click_expands_panel_geometry_like_ts() {
    let mut host = WidgetHost::new();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let before = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat panel visible");
    let x = before.origin.x + before.size.x - 16.0 - 50.0 + 9.0;
    let y = before.origin.y + 17.0;

    assert!(host.apply_click(x, y, viewport_w, viewport_h));

    let after = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("maximized chat panel visible");
    let (cx0, cy0, cw, ch) = host.canvas_region(viewport_w, viewport_h);
    assert!(host.editor_state.chat.maximized);
    assert_eq!(after.origin.x, cx0 + 12.0);
    assert_eq!(after.origin.y, cy0 + 12.0);
    assert_eq!(after.size.x, cw - 24.0);
    assert_eq!(after.size.y, ch - 24.0);
    assert!(after.size.x > before.size.x);
    assert!(after.size.y > before.size.y);
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
        !host.editor_state.chat.collapsed,
        "TS immediately reopens a minimized chat while a response is streaming"
    );
}
