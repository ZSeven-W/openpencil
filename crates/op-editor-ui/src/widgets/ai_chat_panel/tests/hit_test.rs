//! Hit-test coverage for the AI chat panel — input focus, send / stop,
//! bottom-toolbar actions, example pills, resize edges, header buttons.

#[allow(unused_imports)]
use super::super::tests_paint::{assert_close, color_close, rect_close};
use super::super::*;
use super::support::*;
use crate::widgets::ai_chat_hit::{AIChatHit, ChatResizeEdge};

#[test]
fn hit_test_resolves_input_focus() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // Click near the textarea center → FocusInput.
    let p = Point2D::new(120.0, textarea_center_y());
    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::FocusInput));
}

#[test]
fn no_model_disables_send_hit() {
    let mut s = EditorState::new();
    s.chat.set_input_text("design a login page");
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // old→new: send circle is now 28px wide (FOOTER_CIRCLE_D) at right_edge-28; center is right_edge-14.
    let send_x = AI_CHAT_WIDTH - PAD - 15.0;
    let p = Point2D::new(send_x, toolbar_center_y());

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::FocusInput));
}

#[test]
fn no_model_still_allows_quick_action_cards() {
    // #43: example pills are clickable/hoverable regardless of model
    // connection — clicking one fills the input (sending separately requires a
    // model). So with no model the first pill still hits `Example`, not the
    // panel drag handle. (#33: pills are full-width stacked; use first center.)
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let pills = crate::widgets::ai_chat_panel_paint::example_card_rects(rect);
    let p = Point2D::new(
        pills[0].origin.x + pills[0].size.x / 2.0,
        pills[0].origin.y + pills[0].size.y / 2.0,
    );

    assert!(matches!(
        panel.hit_test(rect, p),
        Some(AIChatHit::Example { index: 0, .. })
    ));
    assert_eq!(panel.example_hover_at(rect, p), Some(0));
}

/// Keyboard-shrunk compact sheet: a pill whose rect would run under the
/// bottom-anchored composer is dropped from hover and click hit-testing,
/// matching paint (which drops it too instead of overlapping the composer).
#[test]
fn short_panel_drops_occluded_example_pills_from_hit_testing() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    // Short rect — like the compact AI sheet with the software keyboard up:
    // the raw pill stack runs past the composer block.
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, 300.0);
    let region = panel.empty_state_region(rect);
    let content_bottom = region.origin.y + region.size.y;
    let input = panel.input_rect(rect);
    assert!(
        content_bottom <= input.origin.y,
        "empty-state region must end above the composer"
    );
    let pills = crate::widgets::ai_chat_panel_paint::example_card_rects(rect);
    assert!(
        pills[3].origin.y + pills[3].size.y > content_bottom,
        "fixture: the last pill must overlap the composer for this test to bite"
    );
    let occluded = Point2D::new(
        pills[3].origin.x + pills[3].size.x / 2.0,
        pills[3].origin.y + pills[3].size.y / 2.0,
    );
    assert!(
        !matches!(
            panel.hit_test(rect, occluded),
            Some(AIChatHit::Example { .. })
        ),
        "a dropped pill must not claim taps aimed at the composer"
    );
    assert_eq!(panel.example_hover_at(rect, occluded), None);
    // Pills that still fit stay clickable.
    let first = Point2D::new(
        pills[0].origin.x + pills[0].size.x / 2.0,
        pills[0].origin.y + pills[0].size.y / 2.0,
    );
    assert!(matches!(
        panel.hit_test(rect, first),
        Some(AIChatHit::Example { index: 0, .. })
    ));
    assert_eq!(panel.example_hover_at(rect, first), Some(0));
}

#[test]
fn no_model_disables_model_picker_toggle() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let p = Point2D::new(PAD + 8.0, toolbar_center_y());

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::FocusInput));
}

#[test]
fn hit_test_resolves_send_at_right() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.chat.set_input_text("design a login page");
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // old→new: send circle center is at right_edge - 14 (28px diameter = FOOTER_CIRCLE_D).
    let send_x = AI_CHAT_WIDTH - PAD - 15.0;
    let p = Point2D::new(send_x, toolbar_center_y());
    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::Send));
}

#[test]
fn hit_test_resolves_stop_at_right_while_streaming() {
    let mut s = EditorState::new();
    s.chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // #42: stop shares the send slot — the circle toggles send↑ ↔ stop◻ in
    // place (center at right_edge - 15, same as `hit_test_resolves_send_at_right`).
    // While streaming, a click on the single circle resolves to Stop because the
    // hit-test checks `streaming && stop` before `send`.
    let stop_x = AI_CHAT_WIDTH - PAD - 15.0;
    let p = Point2D::new(stop_x, toolbar_center_y());

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::Stop));
}

#[test]
fn streaming_textarea_click_is_consumed_without_focusing_like_ts_disabled_input() {
    let mut s = EditorState::new();
    s.chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let p = Point2D::new(120.0, textarea_center_y());

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::Inside));
}

#[test]
fn streaming_attachment_button_is_consumed_without_opening_picker_like_ts() {
    // While streaming, clicking the attach button should be consumed (Inside),
    // NOT open the attachment picker — same behaviour as the TS disabled input.
    // #38: attach is now right-aligned (left of stop/send); use footer rect for robustness.
    let mut s = EditorState::new();
    s.chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    let attach_center = Point2D::new(
        footer.attach.origin.x + footer.attach.size.x / 2.0,
        footer.attach.origin.y + footer.attach.size.y / 2.0,
    );

    assert_eq!(panel.hit_test(rect, attach_center), Some(AIChatHit::Inside));
}

#[test]
fn hit_test_resolves_bottom_toolbar_actions() {
    // #38: ⚡/📎/🎨 cluster is now right-aligned (left of stop/send).
    // Use footer_layout rects for robustness instead of hardcoded coords.
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.chat.set_input_text("design a login page");
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let y = toolbar_center_y();
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);

    // model pill center (x=PAD+8 = 24, in range 16..196)
    assert_eq!(
        panel.hit_test(rect, Point2D::new(PAD + 8.0, y)),
        Some(AIChatHit::ToggleModelPicker)
    );
    let prompt_center = Point2D::new(
        footer.prompt_center.origin.x + footer.prompt_center.size.x / 2.0,
        y,
    );
    assert_eq!(
        panel.hit_test(rect, prompt_center),
        Some(AIChatHit::OpenPromptCenter)
    );
    // attach: use the footer rect center (now right-aligned, #38)
    let attach_center = Point2D::new(footer.attach.origin.x + footer.attach.size.x / 2.0, y);
    assert_eq!(
        panel.hit_test(rect, attach_center),
        Some(AIChatHit::AddAttachment)
    );
    // send: right_edge-circle(28) center at right_edge-14.
    assert_eq!(
        panel.hit_test(rect, Point2D::new(AI_CHAT_WIDTH - PAD - 14.0, y)),
        Some(AIChatHit::Send)
    );
}

#[test]
fn footer_hover_maps_bottom_toolbar_actions() {
    // #38: ⚡/📎/🎨 cluster is now right-aligned (left of stop/send).
    // Use footer_layout rects for robustness instead of hardcoded coords.
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.chat.set_input_text("design a login page");
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let y = toolbar_center_y();
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);

    assert_eq!(
        panel.footer_hover_at(rect, Point2D::new(PAD + 8.0, y)),
        Some(op_editor_core::ChatFooterButton::ModelPicker)
    );
    let prompt_center = Point2D::new(
        footer.prompt_center.origin.x + footer.prompt_center.size.x / 2.0,
        y,
    );
    assert_eq!(
        panel.footer_hover_at(rect, prompt_center),
        Some(op_editor_core::ChatFooterButton::PromptCenter)
    );
    // attach: use footer rect center (now right-aligned, #38)
    let attach_center = Point2D::new(footer.attach.origin.x + footer.attach.size.x / 2.0, y);
    assert_eq!(
        panel.footer_hover_at(rect, attach_center),
        Some(op_editor_core::ChatFooterButton::AddAttachment)
    );
    // send center at right_edge - 14 = AI_CHAT_WIDTH - PAD - 14.
    assert_eq!(
        panel.footer_hover_at(rect, Point2D::new(AI_CHAT_WIDTH - PAD - 14.0, y)),
        Some(op_editor_core::ChatFooterButton::Send)
    );
}

#[test]
fn example_hover_maps_quick_action_cards() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let cards = crate::widgets::ai_chat_panel_paint::example_card_rects(rect);
    let p = Point2D::new(
        cards[1].origin.x + cards[1].size.x / 2.0,
        cards[1].origin.y + cards[1].size.y / 2.0,
    );

    assert_eq!(panel.example_hover_at(rect, p), Some(1));
}

#[test]
fn footer_speed_chip_is_clickable_and_opens_parallel_agents_picker() {
    // old→new (#32): the ⚡ chip is now the Parallel Agents chip — it opens the
    // "PARALLEL AGENTS" picker on click (ToggleParallelAgentsPicker), not CycleEffort.
    // Hover state still maps to SpeedChip for the button-wash.
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    let point = Point2D::new(
        footer.speed.origin.x + footer.speed.size.x / 2.0,
        footer.speed.origin.y + footer.speed.size.y / 2.0,
    );

    // old→new: CycleEffort → ToggleParallelAgentsPicker
    assert_eq!(
        panel.hit_test(rect, point),
        Some(AIChatHit::ToggleParallelAgentsPicker)
    );
    // Hover state stays SpeedChip (drives button-wash on the chip).
    assert_eq!(
        panel.footer_hover_at(rect, point),
        Some(op_editor_core::ChatFooterButton::SpeedChip)
    );
}

#[test]
fn multiline_input_expands_above_footer_toolbar() {
    let mut s = EditorState::new();
    s.chat.set_input_text(
        "是的是啊打撒但是 codex 是的撒的 sad 是的撒d大城市多少是多少啊打撒打撒的".repeat(3),
    );
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);

    assert!(
        panel.input_area_height_for_rect(rect) > INPUT_AREA_HEIGHT,
        "multi-line chat input should grow so wrapped text does not overlap the footer toolbar"
    );
    assert!(panel.input_height() > INPUT_BASE_HEIGHT);
}

#[test]
fn hit_test_resolves_model_search_clear_button() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.editor_ui.chat_model_picker.open = true;
    s.editor_ui.chat_model_picker_input.set_text("231");
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input_h = INPUT_BASE_HEIGHT;
    let input_rect = Rect::xywh(
        PAD,
        AI_CHAT_HEIGHT - input_h + 1.0,
        AI_CHAT_WIDTH - PAD * 2.0,
        input_h,
    );
    let picker = panel.model_picker_rect(rect, input_rect);
    let p = Point2D::new(
        picker.origin.x + picker.size.x - 26.0,
        picker.origin.y + 19.0,
    );

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::ClearModelSearch));
}

#[test]
fn hit_test_resolves_attachment_chip_at_painted_position() {
    // With an attachment staged, the input block grows by the
    // attachment row. The click must land where `paint` draws the
    // chip — a regression guard for hit-test / paint y-alignment.
    let mut s = EditorState::new();
    s.chat.add_attachment(op_editor_core::chat::ChatAttachment {
        name: "ref.png".into(),
        media_type: "image/png".into(),
        data: vec![1],
    });
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // paint: input block top = bottom - input_h + 1; the
    // attachment row sits right below the textarea.
    let input_h = INPUT_BASE_HEIGHT + ATTACHMENT_ROW_HEIGHT;
    let input_top = AI_CHAT_HEIGHT - input_h + 1.0;
    let attach_row_center = input_top + INPUT_AREA_HEIGHT + ATTACHMENT_ROW_HEIGHT / 2.0;
    let p = Point2D::new(PAD + 30.0, attach_row_center);
    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::RemoveAttachment(0))
    );
}

#[test]
fn hit_test_resolves_first_example_when_empty() {
    // old→new (#33): first pill is full-width at HEADER_HEIGHT + hint + gap.
    // Use the pill center computed from example_card_rects.
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let pills = crate::widgets::ai_chat_panel_paint::example_card_rects(rect);
    let p = Point2D::new(
        pills[0].origin.x + pills[0].size.x / 2.0,
        pills[0].origin.y + pills[0].size.y / 2.0,
    );
    match panel.hit_test(rect, p) {
        Some(AIChatHit::Example { index, prompt }) => {
            assert_eq!(index, 0);
            assert_eq!(prompt, panel.examples[0].prompt);
        }
        other => panic!("expected first example hit, got {:?}", other),
    }
}

#[test]
fn hit_test_pill_resolves_anywhere_inside_pill_bounds() {
    // old→new (#33): was testing the "taller TS card height" (72px 2×2 grid).
    // old→new (#37): pill is now 40px tall (compact); any click inside resolves the example.
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let pills = crate::widgets::ai_chat_panel_paint::example_card_rects(rect);
    // Click near the bottom of the first pill (verifies full height is live).
    let p = Point2D::new(
        pills[0].origin.x + pills[0].size.x / 2.0,
        pills[0].origin.y + pills[0].size.y - 4.0,
    );

    match panel.hit_test(rect, p) {
        Some(AIChatHit::Example { index, prompt }) => {
            assert_eq!(index, 0);
            assert_eq!(prompt, panel.examples[0].prompt);
        }
        other => panic!("expected first example hit anywhere inside pill, got {other:?}"),
    }
}

#[test]
fn hit_test_header_returns_drag_handle() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // old→new: the #27 header restyle fills most of the header with a pill
    // (chevron + pill covers x=PAD..right_edge-60) and right-side icons.
    // The only drag-handle area is the narrow gap between the pill's right
    // edge and the maximize button: approx x=284..292 for AI_CHAT_WIDTH=360.
    // Pick x=288 (mid-gap between pill right ~284 and maximize left ~292).
    let p = Point2D::new(288.0, 18.0);
    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::DragHandle));
}

#[test]
fn hit_test_header_tab_body_returns_switch_tab() {
    // old→new (MT.2 tab row): clicking inside the tab row now returns
    // SwitchTab(0) for the single default tab, not ToggleCollapse.
    // ToggleCollapse is now scoped to the chevron icon only.
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // Click at x=PAD+64 (well inside the tab zone, past the chevron).
    let p = Point2D::new(PAD + 64.0, 18.0);
    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::SwitchTab(0)));
}

#[test]
fn resize_edge_at_resolves_all_ts_handles_when_not_maximized() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(100.0, 80.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let mid_x = rect.origin.x + rect.size.x / 2.0;
    let mid_y = rect.origin.y + rect.size.y / 2.0;
    let right = rect.origin.x + rect.size.x;
    let bottom = rect.origin.y + rect.size.y;

    let cases = [
        (Point2D::new(mid_x, rect.origin.y + 2.0), ChatResizeEdge::N),
        (Point2D::new(mid_x, bottom - 2.0), ChatResizeEdge::S),
        (Point2D::new(right - 2.0, mid_y), ChatResizeEdge::E),
        (Point2D::new(rect.origin.x + 2.0, mid_y), ChatResizeEdge::W),
        (
            Point2D::new(right - 2.0, rect.origin.y + 2.0),
            ChatResizeEdge::Ne,
        ),
        (
            Point2D::new(rect.origin.x + 2.0, rect.origin.y + 2.0),
            ChatResizeEdge::Nw,
        ),
        (Point2D::new(right - 2.0, bottom - 2.0), ChatResizeEdge::Se),
        (
            Point2D::new(rect.origin.x + 2.0, bottom - 2.0),
            ChatResizeEdge::Sw,
        ),
    ];

    for (point, edge) in cases {
        assert_eq!(panel.resize_edge_at(rect, point), Some(edge));
        assert_eq!(panel.hit_test(rect, point), Some(AIChatHit::Resize(edge)));
    }
}

#[test]
fn hit_test_resolves_header_maximize_button() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let p = Point2D::new(AI_CHAT_WIDTH - PAD - 50.0 + 9.0, 17.0);
    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::ToggleMaximize));
}

#[test]
fn maximized_panel_uses_minimize_icon_for_restore_button() {
    let mut s = EditorState::new();
    s.chat.maximized = true;
    let panel = AIChatPlaceholder::from_editor(&s);

    assert_eq!(panel.maximize_icon(), crate::widgets::icons::Icon::Minimize);
}

#[test]
fn maximized_header_empty_space_is_not_a_drag_handle() {
    let mut s = EditorState::new();
    s.chat.maximized = true;
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let p = Point2D::new(AI_CHAT_WIDTH / 2.0, 16.0);

    assert_ne!(panel.hit_test(rect, p), Some(AIChatHit::DragHandle));
}

#[test]
fn hit_test_resolves_header_new_chat_button() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let p = Point2D::new(AI_CHAT_WIDTH - PAD - 22.0 + 9.0, 17.0);
    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::NewChat));
}
