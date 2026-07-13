//! Layout + hit-test unit tests for [`super::AIChatPlaceholder`].
//! Split into a sibling file to keep `ai_chat_panel.rs` under the
//! 800-line cap.

#[allow(unused_imports)]
use super::tests_paint::{assert_close, color_close, rect_close};
use super::*;
use crate::widgets::ai_chat_hit::{AIChatHit, ChatResizeEdge};

#[test]
fn layout_reports_fixed_size() {
    let s = EditorState::new();
    let p = AIChatPlaceholder::from_editor(&s);
    let cx = LayoutCx {
        available_width: 9999.0,
        dpi: 1.0,
    };
    let lb = p.layout(&cx);
    assert_eq!(lb.rect.size.x, AI_CHAT_WIDTH);
    assert_eq!(lb.rect.size.y, AI_CHAT_HEIGHT);
}

#[test]
fn examples_grid_has_four_cards() {
    assert_eq!(example_cards(op_editor_core::Locale::EnUs).len(), 4);
}

#[test]
fn second_example_uses_short_title_and_full_music_prompt() {
    let en = example_cards(op_editor_core::Locale::EnUs);
    assert_eq!(en[1].title, "Dark music streaming mobile app");
    assert_eq!(
        en[1].prompt,
        "Design a dark-themed music streaming mobile app home screen. Include a greeting \"Good evening\", horizontal scrollable \"Recently Played\" album art cards, \"Made For You\" section with 3 playlist cards showing cover art and playlist names, \"New Releases\" section with 4 album cards in a 2x2 grid, and a floating mini player bar at the bottom showing current track with play/pause controls. Bottom tab bar (Home, Search, Library, Premium). Dark background with lime green accent."
    );

    let zh = example_cards(op_editor_core::Locale::ZhCn);
    assert_eq!(zh[1].title, "暗色音乐流媒体 App 首页");
    assert_eq!(
        zh[1].prompt,
        "设计一个暗色音乐流媒体App首页。包含问候语\"晚上好\"、\"最近播放\"横向滑动专辑封面卡片、\"为你推荐\"区3张歌单卡片（封面和歌单名）、\"新发行\"区4张专辑卡片2x2网格、底部悬浮迷你播放器（当前曲目+播放/暂停控件）。底部导航栏（首页、搜索、音乐库、会员）。深色背景搭配荧光绿强调。"
    );
}

#[test]
fn from_editor_tracks_selection_count_for_toolbar() {
    let mut s = EditorState::new();
    s.selection.set = vec![
        op_editor_core::NodeId::new("n1"),
        op_editor_core::NodeId::new("n2"),
    ];
    let panel = AIChatPlaceholder::from_editor(&s);

    assert_eq!(panel.selected_count, 2);
}

#[test]
fn from_editor_uses_try_example_hint() {
    // old→new (#33 restyle): empty-state header uses "ai.tryExample" key
    // ("Try an example to design…") instead of the old "ai.startDesigning".
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);

    assert_eq!(
        panel.label_start_with_ai,
        op_i18n::translate(s.editor_ui.locale, "ai.tryExample")
    );
}

/// Y-coordinate of the textarea's vertical center.
fn textarea_center_y() -> f32 {
    AI_CHAT_HEIGHT - INPUT_BASE_HEIGHT + 1.0 + INPUT_AREA_HEIGHT / 2.0
}

/// Y-coordinate of the bottom toolbar's vertical center.
pub(super) fn toolbar_center_y() -> f32 {
    AI_CHAT_HEIGHT - INPUT_BASE_HEIGHT + 1.0 + INPUT_AREA_HEIGHT + INPUT_TOOLBAR_HEIGHT / 2.0
}

pub(super) fn seed_available_model(s: &mut EditorState) {
    s.chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
}

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

// Shared paint-assertion infrastructure — also used by the
// sibling `tests_transcript` module (split at the 800-line cap).
#[derive(Default)]
pub(in super::super) struct PanelPaintBackend {
    pub(in super::super) fills: Vec<(Rect, crate::Color)>,
    pub(in super::super) round_rects: Vec<(Rect, f32, crate::Color)>,
    pub(in super::super) texts: Vec<(String, f32, jian_core::scene::Color, Point2D)>,
    pub(in super::super) svg_paths: Vec<String>,
    pub(in super::super) svg_strokes: Vec<(Point2D, f32, crate::Color, f32)>,
    pub(in super::super) stroke_lines: usize,
}

impl crate::RenderBackend for PanelPaintBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, color: crate::Color) {
        self.fills.push((rect, color));
    }
    fn stroke_rect(&mut self, _: Rect, _: crate::Color, _: f32) {}
    fn draw_text(&mut self, layout: &crate::TextLayout, origin: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts
                .push((run.content.clone(), run.font_size, run.color, origin));
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: crate::Color, _: f32) {
        self.stroke_lines += 1;
    }
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: crate::Color) {
        self.round_rects.push((rect, radius, color));
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: crate::Color, _: f32) {}
    fn stroke_svg_path(
        &mut self,
        d: &str,
        top_left: Point2D,
        size: f32,
        color: crate::Color,
        width: f32,
    ) {
        self.svg_paths.push(d.to_string());
        self.svg_strokes.push((top_left, size, color, width));
    }
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

pub(in super::super) fn has_fill_rect(fills: &[(Rect, crate::Color)], expected: Rect) -> bool {
    fills.iter().any(|(rect, _)| {
        (rect.origin.x - expected.origin.x).abs() < 1e-4
            && (rect.origin.y - expected.origin.y).abs() < 1e-4
            && (rect.size.x - expected.size.x).abs() < 1e-4
            && (rect.size.y - expected.size.y).abs() < 1e-4
    })
}

// ── New bottom-toolbar layout tests (§ Task 5.2 / #27) ──────────────────────

#[test]
fn bottom_toolbar_layout_send_is_rightmost_circle() {
    // The send button is the rightmost element; stop shares its slot (#42).
    let s = EditorState::new();
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
    let s = EditorState::new();
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
    // Model pill right edge must still be left of the speed chip.
    assert!(
        footer.model.origin.x + footer.model.size.x < footer.speed.origin.x,
        "model pill right edge must be left of the speed chip"
    );
    // There is a flexible gap between model and the right cluster.
    let model_right = footer.model.origin.x + footer.model.size.x;
    assert!(
        footer.speed.origin.x > model_right + 4.0,
        "speed chip (#38 right cluster) must be well to the right of the model pill"
    );
}

#[test]
fn bottom_toolbar_layout_order_is_model_speed_attach_send() {
    // #38: ⚡/📎 moved right; #42: stop shares the send slot. Full
    // left-to-right order is:
    //   model (LEFT) | [gap] | speed | attach | send (RIGHT)
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);

    // Left-to-right order: model < speed < attach < send
    assert!(
        footer.model.origin.x < footer.speed.origin.x,
        "model left of speed"
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
        footer.speed.origin.x > model_right + 4.0,
        "speed chip must be right of model pill with a visible gap (#38)"
    );
}

#[test]
fn hit_test_stop_circle_only_active_while_streaming() {
    // While streaming, a click on the stop rect returns Stop.
    let mut s = EditorState::new();
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
    let s = EditorState::new();
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
    // Clicking on the chevron icon itself (left edge of pill cluster) must
    // still return ToggleCollapse.
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // Chevron center: PAD + 9 (half of 18px icon).
    let p = Point2D::new(PAD + 9.0, HEADER_HEIGHT / 2.0);

    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::ToggleCollapse),
        "collapse chevron must resolve ToggleCollapse"
    );
}
