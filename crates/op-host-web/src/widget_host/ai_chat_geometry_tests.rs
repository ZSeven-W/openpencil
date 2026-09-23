use super::WidgetHost;

#[test]
fn the_composer_card_header_pins_the_chat_into_the_agent_rail() {
    let mut host = WidgetHost::new();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    // The composer-only card's slim header only exists while the input
    // is focused.
    host.editor_state.chat.focused = true;
    let before = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("composer card visible");
    let x = before.origin.x + before.size.x - 16.0 - 50.0 + 9.0;
    let y = before.origin.y + 17.0;

    assert!(host.apply_click(x, y, viewport_w, viewport_h));

    // RETIRED BEHAVIOUR (maximize): the old floating panel's maximize
    // glyph blew the panel up over the canvas region. On the rail-merge
    // contract the composer card has no maximize — both header glyphs
    // mean "open the conversation", and the conversation's one home is
    // the rail's Agent tab. What "expands" means now: the rail switches
    // to the Agent tab and the chat rect becomes the rail body — the
    // `PinnedChat::At` branch — which is strictly taller than the card.
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        op_editor_core::LeftPanelTab::Chat,
        "the card header's one meaning is: take me to the Agent tab"
    );
    assert!(
        !host.editor_state.editor_ui.chat_composer_only(),
        "the rail now shows the conversation"
    );
    assert!(!host.editor_state.chat.maximized);
    let after = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("pinned chat panel visible");
    let pinned = match op_editor_ui::widgets::host_canvas_geometry::pinned_chat(
        &host.editor_state,
        viewport_w,
        viewport_h,
    ) {
        Some(op_editor_ui::widgets::host_canvas_geometry::PinnedChat::At(rect)) => rect,
        other => panic!("the Agent tab must pin the chat into the rail, got {other:?}"),
    };
    assert_eq!(after, pinned, "the chat rect IS the pinned rail body");
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
