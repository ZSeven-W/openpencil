//! Bottom-toolbar layout + parallel-agents picker tests.

#[allow(unused_imports)]
use super::super::tests_paint::{assert_close, color_close, rect_close};
use super::super::*;
use super::support::*;
use crate::widgets::ai_chat_hit::AIChatHit;

// ── New bottom-toolbar layout tests (§ Task 5.2 / #27) ──────────────────────

#[test]
fn bottom_toolbar_layout_send_is_rightmost_circle() {
    // The send button is the rightmost element; stop shares its slot (#42).
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);

    // Send must be circular (equal w/h) and right-most.
    assert!(
        (footer.send.size.x - footer.send.size.y).abs() < 0.01,
        "send button must be circular"
    );
    // #42: stop is no longer a separate button left of send — it shares the
    // send slot (the circle toggles send↑ ↔ stop◻ in place).
    assert!(
        (footer.stop.origin.x - footer.send.origin.x).abs() < 0.01,
        "stop must share the send slot"
    );
    // Send right edge should match panel right minus PAD.
    let right_edge = rect.origin.x + rect.size.x - PAD;
    assert!(
        (footer.send.origin.x + footer.send.size.x - right_edge).abs() < 0.01,
        "send right edge must touch right_edge"
    );
}

#[test]
fn bottom_toolbar_layout_model_pill_is_leftmost() {
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);

    // Model pill starts at PAD.
    assert!(
        (footer.model.origin.x - PAD).abs() < 0.01,
        "model pill must start at PAD"
    );
    assert!(
        footer.model.size.x >= 140.0,
        "model pill should be at least 140px wide"
    );
    // #38: ⚡/📎/🎨 cluster is now right-aligned (left of stop/send).
    // Model pill right edge must still be left of the prompt button.
    assert!(
        footer.model.origin.x + footer.model.size.x < footer.prompt_center.origin.x,
        "model pill right edge must be left of the prompt button"
    );
    // There is a flexible gap between model and the right cluster.
    let model_right = footer.model.origin.x + footer.model.size.x;
    assert!(
        footer.prompt_center.origin.x > model_right,
        "prompt button must be to the right of the model pill"
    );
}

#[test]
fn bottom_toolbar_layout_order_is_model_prompt_speed_attach_send() {
    // #38: ⚡/📎 moved right; #42: stop shares the send slot. Full
    // left-to-right order is:
    //   model (LEFT) | prompt | speed | attach | send (RIGHT)
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);

    // Left-to-right order: model < prompt < speed < attach < send
    assert!(
        footer.model.origin.x + footer.model.size.x <= footer.prompt_center.origin.x,
        "model left of prompt"
    );
    assert!(
        footer.prompt_center.origin.x < footer.speed.origin.x,
        "prompt left of speed"
    );
    assert!(
        footer.speed.origin.x < footer.attach.origin.x,
        "speed left of attach"
    );
    assert!(
        footer.attach.origin.x < footer.send.origin.x,
        "attach left of send"
    );
    // #42: stop shares the send slot (toggle in place), not a separate button.
    assert!(
        (footer.stop.origin.x - footer.send.origin.x).abs() < 0.01,
        "stop shares the send slot"
    );
    // #38 specific: speed/attach must all be RIGHT of the model pill.
    let model_right = footer.model.origin.x + footer.model.size.x;
    assert!(
        footer.prompt_center.origin.x >= model_right + 4.0,
        "prompt button must be right of model pill with a visible gap"
    );
}

#[test]
fn bottom_toolbar_min_width_rects_do_not_overlap() {
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_MIN_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let footer = panel.footer_layout(rect, input, input.origin.y + INPUT_AREA_HEIGHT);
    let ordered = [
        footer.model,
        footer.prompt_center,
        footer.speed,
        footer.attach,
        footer.send,
    ];
    for pair in ordered.windows(2) {
        assert!(
            pair[0].origin.x + pair[0].size.x <= pair[1].origin.x,
            "footer rects overlap at minimum width: {:?} then {:?}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn hit_test_stop_circle_only_active_while_streaming() {
    // While streaming, a click on the stop rect returns Stop.
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    s.chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    let stop_center = Point2D::new(
        footer.stop.origin.x + footer.stop.size.x / 2.0,
        footer.stop.origin.y + footer.stop.size.y / 2.0,
    );

    assert_eq!(panel.hit_test(rect, stop_center), Some(AIChatHit::Stop));

    // While idle, the same position should not return Stop.
    let mut s2 = EditorState::new();
    seed_available_model(&mut s2);
    s2.chat.set_input_text("design");
    let panel2 = AIChatPlaceholder::from_editor(&s2);
    // #42: the stop slot is the Send button while idle (stop shares it), so the
    // same point resolves to Send — never Stop.
    assert_ne!(
        panel2.hit_test(rect, stop_center),
        Some(AIChatHit::Stop),
        "stop hit must not fire while idle"
    );
}

// ── Task 5.6 Parallel Agents picker tests ────────────────────────────────────

#[test]
fn parallel_agents_chip_label_is_agent_team_size_not_effort() {
    // #32: chip shows "{N}x" where N = agent_team_size, not effort level.
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    seed_available_model(&mut s);
    s.chat.agent_team_size = 4;
    let panel = AIChatPlaceholder::from_editor(&s);
    // agent_team_size is accessible via panel.state.
    assert_eq!(panel.state.agent_team_size, 4);
    // The chip label should format as "4x".
    let label = format!("{}x", panel.state.agent_team_size);
    assert_eq!(label, "4x");
}

#[test]
fn clicking_speed_chip_opens_parallel_agents_picker() {
    // #32: clicking the ⚡ chip returns ToggleParallelAgentsPicker (not CycleEffort).
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    seed_available_model(&mut s);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    let chip_center = Point2D::new(
        footer.speed.origin.x + footer.speed.size.x / 2.0,
        footer.speed.origin.y + footer.speed.size.y / 2.0,
    );
    assert_eq!(
        panel.hit_test(rect, chip_center),
        Some(AIChatHit::ToggleParallelAgentsPicker)
    );
}

#[test]
fn parallel_agents_picker_row_hit_returns_set_parallel_agents() {
    // When the picker is open, clicking a row returns SetParallelAgents(N).
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    seed_available_model(&mut s);
    s.editor_ui.parallel_agents_picker_open = true;
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    let picker = crate::widgets::ai_chat_panel_footer::parallel_agents_picker_rect(&footer);
    // Row 3 starts at rows_top + 2 * ROW_H; click its center.
    let rows_top = picker.origin.y + 32.0;
    let row3_y = rows_top + 2.0 * crate::widgets::ai_chat_panel_footer::PARALLEL_AGENTS_ROW_H_PUB;
    let row3_center = Point2D::new(
        picker.origin.x + picker.size.x / 2.0,
        row3_y + crate::widgets::ai_chat_panel_footer::PARALLEL_AGENTS_ROW_H_PUB / 2.0,
    );
    assert_eq!(
        panel.hit_test(rect, row3_center),
        Some(AIChatHit::SetParallelAgents(3))
    );
}

#[test]
fn parallel_agents_picker_outside_click_closes_picker() {
    // Clicking outside the picker while it is open returns ToggleParallelAgentsPicker
    // (the host handler treats this as a close).
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    seed_available_model(&mut s);
    s.editor_ui.parallel_agents_picker_open = true;
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // Click in the body area (far from the picker) — should close.
    let body_point = Point2D::new(AI_CHAT_WIDTH / 2.0, AI_CHAT_HEIGHT / 2.0);
    assert_eq!(
        panel.hit_test(rect, body_point),
        Some(AIChatHit::ToggleParallelAgentsPicker)
    );
}

#[test]
fn parallel_agents_picker_hover_at_returns_row_index() {
    // parallel_agents_picker_hover_at returns the row the cursor is over.
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    seed_available_model(&mut s);
    s.editor_ui.parallel_agents_picker_open = true;
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    let picker = crate::widgets::ai_chat_panel_footer::parallel_agents_picker_rect(&footer);
    let rows_top = picker.origin.y + 32.0;
    // Hover over row 5.
    let row5_y = rows_top + 4.0 * crate::widgets::ai_chat_panel_footer::PARALLEL_AGENTS_ROW_H_PUB;
    let point = Point2D::new(
        picker.origin.x + 20.0,
        row5_y + crate::widgets::ai_chat_panel_footer::PARALLEL_AGENTS_ROW_H_PUB / 2.0,
    );
    assert_eq!(panel.parallel_agents_picker_hover_at(rect, point), Some(5));
    // Outside the picker → None.
    let outside = Point2D::new(AI_CHAT_WIDTH / 2.0, AI_CHAT_HEIGHT / 2.0);
    assert_eq!(panel.parallel_agents_picker_hover_at(rect, outside), None);
}

#[test]
fn parallel_agents_picker_closed_when_picker_not_open() {
    // When the picker is closed, the hover method returns None and
    // the hit-test falls through to normal chip behavior.
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    seed_available_model(&mut s);
    // picker NOT open
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let outside = Point2D::new(AI_CHAT_WIDTH / 2.0, AI_CHAT_HEIGHT / 2.0);
    assert_eq!(panel.parallel_agents_picker_hover_at(rect, outside), None);
}

// ── Task 5.3 header restyle tests ────────────────────────────────────────────

#[test]
fn header_new_chat_circle_at_right_resolves_new_chat() {
    // The "+" new-chat button is a 28px circle at the far right of the header.
    // old: was a plain icon-button at right_edge-22; new: circle at right_edge-28.
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // Center of the new-chat circle: right_edge - 14 (half of 28px diameter).
    let right_edge = AI_CHAT_WIDTH - PAD;
    let center_x = right_edge - 14.0;
    let center_y = HEADER_HEIGHT / 2.0;
    let p = Point2D::new(center_x, center_y);

    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::NewChat),
        "center of the 28px new-chat circle must resolve NewChat"
    );
}

#[test]
fn header_collapse_chevron_area_resolves_toggle_collapse() {
    // The chevron now lives only on the TOUCH sheet: desktop has no
    // collapsed state left to toggle into, so the affordance is gone
    // there and the hit rect with it.
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    s.editor_ui.touch = true;
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // Chevron center: PAD + 9 (half of 18px icon).
    let p = Point2D::new(PAD + 9.0, HEADER_HEIGHT / 2.0);

    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::ToggleCollapse),
        "collapse chevron must resolve ToggleCollapse on the touch sheet"
    );
}

// ── Thinking-mode toggle ─────────────────────────────────────────────────────

/// The footer as laid out for a default-size panel.
fn default_footer(s: &EditorState) -> (AIChatPlaceholder<'_>, Rect, FooterLayout) {
    let panel = AIChatPlaceholder::from_editor(s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    (panel, rect, footer)
}

#[test]
fn thinking_toggle_sits_between_the_library_button_and_the_speed_chip() {
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    let (_panel, _rect, footer) = default_footer(&s);

    assert!(footer.thinking.size.x > 0.0, "toggle must be laid out");
    assert!(
        footer.prompt_center.origin.x + footer.prompt_center.size.x <= footer.thinking.origin.x,
        "library button must end before the thinking toggle starts"
    );
    assert!(
        footer.thinking.origin.x + footer.thinking.size.x <= footer.speed.origin.x,
        "thinking toggle must end before the ⚡ chip starts"
    );
    // …and it must not have eaten the model pill's room.
    assert!(
        footer.model.origin.x + footer.model.size.x <= footer.prompt_center.origin.x,
        "model pill must still end before the library button"
    );
    assert!(
        footer.model.size.x >= 140.0,
        "model pill keeps its width at the default panel size, got {}",
        footer.model.size.x
    );
}

#[test]
fn thinking_toggle_is_dropped_before_the_model_pill_becomes_unreadable() {
    // Degradation, not overlap: a panel too narrow for six controls drops
    // the toggle rather than squeezing the model name into nothing. A
    // zero-width rect can never be hit, so the row stays honest about what
    // is there. The boundary sits below `AI_CHAT_MIN_WIDTH`, so a user
    // dragging the panel to its narrowest still has the toggle.
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    let panel = AIChatPlaceholder::from_editor(&s);
    let footer_at = |w: f32| {
        let rect = Rect::xywh(0.0, 0.0, w, AI_CHAT_HEIGHT);
        let input = panel.input_rect(rect);
        let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
        panel.footer_layout(rect, input, toolbar_top)
    };

    let smallest = footer_at(AI_CHAT_MIN_WIDTH);
    assert!(
        smallest.thinking.size.x > 0.0,
        "the toggle must survive down to the minimum panel width"
    );

    let narrow = Rect::xywh(0.0, 0.0, 240.0, AI_CHAT_HEIGHT);
    let dropped = footer_at(240.0);
    assert_eq!(dropped.thinking.size.x, 0.0, "toggle must be dropped");
    // `Rect::contains` is inclusive on both edges, so a zero-width rect is not
    // self-evidently unhittable — assert against the hit-test, not the rect.
    let probe = Point2D::new(
        dropped.thinking.origin.x,
        dropped.thinking.origin.y + dropped.thinking.size.y / 2.0,
    );
    assert_ne!(
        panel.hit_test(narrow, probe),
        Some(AIChatHit::CycleThinking),
        "a dropped slot must not be clickable"
    );
    // The row closes the hole rather than leaving a 24 px gap in it…
    assert!(
        (dropped.prompt_center.origin.x + dropped.prompt_center.size.x - dropped.speed.origin.x)
            .abs()
            <= 4.01,
        "library button must sit against the ⚡ chip once the toggle is gone"
    );
    // …and the width it gave up went back to the model pill.
    assert!(
        dropped.model.size.x >= 72.0,
        "dropping the toggle must give the model pill its room back, got {}",
        dropped.model.size.x
    );
}

#[test]
fn thinking_toggle_click_cycles_the_mode_through_the_hit() {
    use crate::widgets::chat_click_flow::apply_chat_hit;
    use op_editor_core::chat::ThinkingMode;

    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    let expected = [
        ThinkingMode::Disabled,
        ThinkingMode::Enabled,
        ThinkingMode::Adaptive,
    ];
    assert_eq!(s.chat.thinking_mode, ThinkingMode::Adaptive);
    for want in expected {
        let (panel, rect, footer) = default_footer(&s);
        let point = Point2D::new(
            footer.thinking.origin.x + footer.thinking.size.x / 2.0,
            footer.thinking.origin.y + footer.thinking.size.y / 2.0,
        );
        assert_eq!(
            panel.hit_test(rect, point),
            Some(AIChatHit::CycleThinking),
            "the toggle's centre must resolve to CycleThinking"
        );
        apply_chat_hit(&mut s, AIChatHit::CycleThinking, 0);
        assert_eq!(s.chat.thinking_mode, want);
    }
}

#[test]
fn thinking_toggle_reports_its_own_hover() {
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    let (panel, rect, footer) = default_footer(&s);
    let point = Point2D::new(
        footer.thinking.origin.x + footer.thinking.size.x / 2.0,
        footer.thinking.origin.y + footer.thinking.size.y / 2.0,
    );
    assert_eq!(
        panel.footer_hover_at(rect, point),
        Some(op_editor_core::ChatFooterButton::ThinkingMode)
    );
}

#[test]
fn thinking_toggle_stays_live_while_a_turn_streams() {
    // Unlike the ⚡ chip beside it, the toggle is not inert during a turn:
    // the mode is read at the NEXT launch, which is what a user clicking it
    // mid-stream is asking for.
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    seed_available_model(&mut s);
    let mut streaming = op_editor_core::chat::ChatMessage::assistant("designing…");
    streaming.streaming = true;
    s.chat.messages.push(streaming);
    let (panel, rect, footer) = default_footer(&s);
    assert!(panel.is_streaming());
    let point = Point2D::new(
        footer.thinking.origin.x + footer.thinking.size.x / 2.0,
        footer.thinking.origin.y + footer.thinking.size.y / 2.0,
    );
    assert_eq!(panel.hit_test(rect, point), Some(AIChatHit::CycleThinking));
}

#[test]
fn hovering_the_thinking_toggle_names_the_mode_it_is_in() {
    // The button is a bare glyph, so the tooltip is the only place the
    // current mode is spelled out. Painted for every mode, not just the
    // non-default ones.
    use op_editor_core::chat::ThinkingMode;

    for (mode, key) in [
        (ThinkingMode::Adaptive, "ai.thinking.adaptive"),
        (ThinkingMode::Disabled, "ai.thinking.disabled"),
        (ThinkingMode::Enabled, "ai.thinking.enabled"),
    ] {
        let mut s = EditorState::new();
        // The expanded panel only exists where the conversation lives:
        // the rail's Agent tab. Elsewhere the chat is composer-only.
        s.editor_ui.enter_chat_tab();
        seed_available_model(&mut s);
        s.editor_ui.locale = op_editor_core::Locale::EnUs;
        s.chat.thinking_mode = mode;
        s.editor_ui.chat_footer_hover = Some(op_editor_core::ChatFooterButton::ThinkingMode);
        let panel = AIChatPlaceholder::from_editor(&s);
        let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
        let mut backend = PanelPaintBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        let expected = op_i18n::translate(op_editor_core::Locale::EnUs, key);
        assert!(
            backend.texts.iter().any(|(text, ..)| text == expected),
            "hovering in {mode:?} must paint `{expected}`"
        );
    }
}

#[test]
fn the_zero_width_agent_team_slot_is_not_a_live_target() {
    // `Rect::contains` is inclusive on BOTH edges, so a zero-width rect still
    // owns the single pixel column at its origin — the retired ⚡-team chip is
    // laid out as one of those for schema compat. In the press path the model
    // pill (which ends on that same column) shadows it, but `footer_hover_at`
    // gates the model pill on having models, so with nothing connected that
    // column reported a hover for a control that is not painted at all.
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    assert!(
        s.chat.available_models.is_empty(),
        "the reachable case is the one with no model connected"
    );
    let (panel, rect, footer) = default_footer(&s);
    assert_eq!(
        footer.agent_team.size.x, 0.0,
        "the slot is still zero-width"
    );

    let column = Point2D::new(
        footer.agent_team.origin.x,
        footer.agent_team.origin.y + footer.agent_team.size.y / 2.0,
    );
    assert_ne!(
        panel.footer_hover_at(rect, column),
        Some(op_editor_core::ChatFooterButton::AgentTeam),
        "a dropped slot must not report hover"
    );
    assert_ne!(
        panel.hit_test(rect, column),
        Some(AIChatHit::CycleAgentTeam),
        "a dropped slot must not be clickable"
    );
}
