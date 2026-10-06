//! Floating AI-chat panel: header actions, transcript text selection,
//! resize edges, and the streaming-disabled input guards.
//!
//! Split out of `input_tests.rs` to keep every file under the repo's
//! 800-line cap.

use super::*;

/// Scan the panel's bottom band for the first point whose hit-test
/// resolves to `want`. The footer's controls move with the live
/// layout, so tests must resolve them from geometry, not constants.
fn scan_footer_for_hit(
    host: &WidgetHostNative,
    rect: op_editor_ui::Rect,
    want: op_editor_ui::widgets::AIChatHit,
) -> Option<op_editor_ui::Point2D> {
    // Stamp the host's real owner, mirroring production: a host with
    // messages resolves the transcript cache, which debug-asserts
    // against the UNOWNED sentinel.
    let panel = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state())
        .owned_by(host.chat_panel_owner);
    let bottom = rect.origin.y + rect.size.y;
    let mut y = bottom - 140.0;
    while y < bottom {
        let mut x = rect.origin.x;
        while x < rect.origin.x + rect.size.x {
            let p = op_editor_ui::Point2D::new(x, y);
            if panel.hit_test(rect, p) == Some(want.clone()) {
                return Some(p);
            }
            x += 2.0;
        }
        y += 2.0;
    }
    None
}

/// RETIRED BEHAVIOUR (maximize): the floating panel's maximize glyph
/// grew the panel to fill the canvas. The pinned panel has no window
/// to maximize — the glyph is neither painted nor clickable, and even
/// a stale `chat.maximized` flag cannot move the column.
#[test]
fn the_pinned_panel_has_no_maximize_button_left_to_click() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let before = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    // Where the glyph used to sit: just left of the far-right
    // new-chat circle, in the header's icon band.
    let x = before.origin.x + before.size.x - 16.0 - 50.0 + 9.0;
    let y = before.origin.y + 17.0;
    let panel = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state());
    assert_ne!(
        panel.hit_test(before, op_editor_ui::Point2D::new(x, y)),
        Some(op_editor_ui::widgets::AIChatHit::ToggleMaximize)
    );

    let _ = host.apply_click(x, y, 1200.0, 800.0);
    assert!(!host.editor_state().chat.maximized);
    assert_eq!(host.ai_chat_rect(1200.0, 800.0), Some(before));

    // The flag itself is inert geometry on the pinned panel: even set
    // by a path bypass it cannot resize the rail's column.
    host.editor_state_mut().chat.maximized = true;
    assert_eq!(host.ai_chat_rect(1200.0, 800.0), Some(before));
}

#[test]
fn ai_chat_new_chat_click_clears_transcript_and_queues_abort() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant("old"));
    host.editor_state_mut().chat.set_input_text("draft");
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + rect.size.x - 16.0 - 22.0 + 9.0;
    let y = rect.origin.y + 17.0;

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    assert!(host.editor_state().chat.messages.is_empty());
    assert!(host.editor_state().chat.input.text().is_empty());
    assert!(host.editor_state().chat.pending_new_chat);
}

#[test]
fn dragging_user_transcript_text_selects_and_copy_queues_text() {
    let prompt = "生成一个设计精良的美食应用移动端首页";
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::user(prompt));
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat panel visible");
    // The bubble's text positions come from the live transcript build,
    // so scan the body for the smallest and largest caret offsets the
    // hit-test resolves — the drag then runs from before the first
    // glyph to after the last one. At the rail's 320px width this
    // 18-char CJK prompt wraps to two bubble lines, so the drag has to
    // cross a line break to cover the whole prompt.
    let panel = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state())
        .owned_by(host.chat_panel_owner);
    let body = panel.body_rect(rect);
    let mut start: Option<(usize, op_editor_ui::Point2D)> = None;
    let mut end: Option<(usize, op_editor_ui::Point2D)> = None;
    let mut y = body.origin.y;
    while y < body.origin.y + body.size.y.min(220.0) {
        let mut x = body.origin.x;
        while x < body.origin.x + body.size.x {
            let p = op_editor_ui::Point2D::new(x, y);
            if let Some(op_editor_ui::widgets::AIChatHit::SelectTranscriptText(0, off)) =
                panel.hit_test(rect, p)
            {
                if start.is_none() || off < start.expect("set").0 {
                    start = Some((off, p));
                }
                if end.is_none() || off > end.expect("set").0 {
                    end = Some((off, p));
                }
            }
            x += 4.0;
        }
        y += 2.0;
    }
    let (start_off, start_point) = start.expect("a point before the first glyph");
    let (end_off, end_point) = end.expect("a point after the last glyph");
    assert_eq!(start_off, 0);
    assert_eq!(end_off, prompt.len());
    assert_ne!(
        start_point.y, end_point.y,
        "the whole-prompt drag must cross the wrapped line break"
    );

    assert!(host.apply_press(start_point.x, start_point.y, viewport_w, viewport_h));
    assert!(host.apply_cursor_move(end_point.x, end_point.y));
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));

    assert_eq!(
        host.editor_state().chat.selected_transcript_text(),
        Some(prompt)
    );
    assert!(host.apply_copy());
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some(prompt)
    );
}

#[test]
fn user_transcript_text_uses_text_cursor() {
    let prompt = "生成一个设计精良的美食应用移动端首页";
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::user(prompt));
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat panel visible");
    let x = rect.origin.x + 96.0;
    let y = rect.origin.y + 74.0;

    // `cursor_hint` now reads the LAST BUILT transcript layout (zero hashes on
    // that pass) rather than re-fingerprinting the live transcript. Prime that
    // build the way a paint / cursor-move would — a press over the transcript
    // resolves + stores the canonical — then the hint reads the stored build and
    // flips to the text cursor. This exercises the native cursor_hint end-to-end
    // (event ordering: build stored, then hint reads it).
    assert!(host.apply_press(x, y, viewport_w, viewport_h));
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));

    assert_eq!(
        host.cursor_hint(x, y, viewport_w, viewport_h),
        CursorHint::Text
    );
}

#[test]
fn chat_input_text_uses_text_cursor() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut().chat.set_input_text("abcdef");
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat panel visible");
    // The input block is bottom-anchored in the rail's tall column, so
    // the probe comes from the live text rect, not a fixed offset from
    // the old floating panel's top.
    let text = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state())
        .input_text_rect(rect);
    let x = text.origin.x + 16.0;
    let y = text.origin.y + text.size.y / 2.0;

    assert_eq!(
        host.cursor_hint(x, y, viewport_w, viewport_h),
        CursorHint::Text
    );
}

#[test]
fn ai_chat_east_edge_shows_resize_cursor() {
    let host = WidgetHostNative::new();
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + rect.size.x - 2.0;
    let y = rect.origin.y + rect.size.y / 2.0;

    assert_eq!(host.cursor_hint(x, y, 1200.0, 800.0), CursorHint::ResizeEw);
}

#[test]
fn ai_chat_east_edge_drag_resizes_panel_width() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let before = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = before.origin.x + before.size.x - 2.0;
    let y = before.origin.y + before.size.y / 2.0;

    assert!(host.apply_press(x, y, 1200.0, 800.0));
    assert!(host.apply_cursor_move(x + 72.0, y));

    let after = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible after resize");
    assert!(
        after.size.x > before.size.x + 60.0,
        "dragging the east edge should grow chat width; before={before:?}, after={after:?}"
    );
    assert_eq!(after.origin.x, before.origin.x);
}

#[test]
fn ai_chat_stop_click_keeps_transcript_and_queues_abort() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::user("make a dashboard"));
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    // The stop circle is only a live target while streaming; resolve it
    // from the live footer layout instead of the old fixed toolbar row.
    let stop = scan_footer_for_hit(&host, rect, op_editor_ui::widgets::AIChatHit::Stop)
        .expect("stop circle should be hittable while streaming");

    assert!(host.apply_click(stop.x, stop.y, 1200.0, 800.0));

    assert_eq!(host.editor_state().chat.messages.len(), 2);
    assert!(!host.editor_state().chat.messages[1].streaming);
    assert!(host.editor_state().chat.pending_stop_chat);
}

#[test]
fn ai_chat_agent_team_click_sets_team_size_via_parallel_agents_picker() {
    // #32: the ⚡ footer chip no longer cycles the team size on click — it
    // opens the Parallel Agents picker, and clicking a row (1x–6x) sets
    // `agent_team_size`. Drive that two-step flow through the real hit-test
    // routing (probe with `hit_test`, then click the resolved points).
    use op_editor_ui::widgets::AIChatHit;
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut()
        .chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();

    // Locate the ⚡ speed chip in the live footer band and click it to
    // open the picker (the footer row is bottom-anchored in the rail's
    // tall column, so it is scanned, not assumed at a fixed y).
    let speed_point = scan_footer_for_hit(&host, rect, AIChatHit::ToggleParallelAgentsPicker)
        .expect("footer speed chip should be hittable");
    assert!(host.apply_click(speed_point.x, speed_point.y, viewport_w, viewport_h));
    assert!(host.editor_state().editor_ui.parallel_agents_picker_open);

    // With the picker open, find and click the "2x" row.
    let panel = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state());
    let mut row_point = None;
    let mut ry = rect.origin.y;
    'scan: while ry < rect.origin.y + rect.size.y {
        let mut rx = rect.origin.x;
        while rx < rect.origin.x + rect.size.x {
            let p = op_editor_ui::Point2D::new(rx, ry);
            if panel.hit_test(rect, p) == Some(AIChatHit::SetParallelAgents(2)) {
                row_point = Some(p);
                break 'scan;
            }
            rx += 2.0;
        }
        ry += 2.0;
    }
    let row_point = row_point.expect("parallel agents picker row 2 should be hittable");
    assert!(host.apply_click(row_point.x, row_point.y, viewport_w, viewport_h));
    assert_eq!(host.editor_state().chat.agent_team_size, 2);
    assert!(!host.editor_state().editor_ui.parallel_agents_picker_open);
}

#[test]
fn ai_chat_streaming_textarea_click_does_not_focus_disabled_input() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    // Live text-rect centre (see `chat_input_text_uses_text_cursor`).
    let text = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state())
        .input_text_rect(rect);
    let x = text.origin.x + 40.0;
    let y = text.origin.y + text.size.y / 2.0;

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    assert!(!host.editor_state().chat.focused);
}

#[test]
fn ai_chat_streaming_attachment_click_does_not_open_picker() {
    // The attach glyph is inert while streaming (its hit reads as blank
    // panel), so its rect is resolved on an identical NOT-streaming
    // twin and the very same point is pressed on the streaming host.
    let mut twin = WidgetHostNative::new();
    twin.editor_state_mut().editor_ui.enter_chat_tab();
    let twin_rect = twin
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let attach = scan_footer_for_hit(
        &twin,
        twin_rect,
        op_editor_ui::widgets::AIChatHit::AddAttachment,
    )
    .expect("attach glyph should be hittable when idle");

    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    assert_eq!(rect, twin_rect, "streaming must not move the footer");

    assert!(host.apply_click(attach.x, attach.y, 1200.0, 800.0));

    assert!(!host.editor_state().chat.pending_attachment_pick);
}

/// Sending from the composer card starts the conversation where the
/// conversation lives: the rail switches to its Agent tab and the turn
/// launches there — the reply must never stream into a card that
/// cannot show it.
#[test]
fn sending_from_the_composer_card_lands_the_rail_on_the_agent_tab() {
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
        .set_input_text("design a dashboard");
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("composer card visible");
    let send = scan_footer_for_hit(&host, rect, op_editor_ui::widgets::AIChatHit::Send)
        .expect("send circle should be hittable with text and a model");

    assert!(host.apply_click(send.x, send.y, 1200.0, 800.0));

    assert_eq!(
        host.editor_state().editor_ui.slides_panel.tab,
        op_editor_core::LeftPanelTab::Chat,
        "a send from the card opens the conversation's one home"
    );
    assert!(host.editor_state().chat.pending_send.is_some());
    // The conversation continues in the rail, not on a second surface:
    // the card is gone once the tab owns the rail.
    let pinned = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("pinned panel visible");
    assert!(pinned.size.y > rect.size.y);
}
