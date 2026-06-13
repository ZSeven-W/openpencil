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
fn from_editor_uses_ts_start_designing_hint() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);

    assert_eq!(
        panel.label_start_with_ai,
        op_i18n::translate(s.editor_ui.locale, "ai.startDesigning")
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
    let send_x = AI_CHAT_WIDTH - PAD - 20.0;
    let p = Point2D::new(send_x, toolbar_center_y());

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::FocusInput));
}

#[test]
fn no_model_disables_quick_action_cards() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let card_w = (AI_CHAT_WIDTH - PAD * 2.0 - 8.0) / 2.0;
    let p = Point2D::new(PAD + card_w / 2.0, HEADER_HEIGHT + 32.0 + 35.0);

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::DragHandle));
    assert_eq!(panel.example_hover_at(rect, p), None);
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
    let send_x = AI_CHAT_WIDTH - PAD - 20.0;
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
    let send_x = AI_CHAT_WIDTH - PAD - 20.0;
    let p = Point2D::new(send_x, toolbar_center_y());

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
    let mut s = EditorState::new();
    s.chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let p = Point2D::new(AI_CHAT_WIDTH - PAD - 52.0, toolbar_center_y());

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::Inside));
}

#[test]
fn hit_test_resolves_bottom_toolbar_actions() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.chat.set_input_text("design a login page");
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let y = toolbar_center_y();
    assert_eq!(
        panel.hit_test(rect, Point2D::new(PAD + 8.0, y)),
        Some(AIChatHit::ToggleModelPicker)
    );
    assert_eq!(
        panel.hit_test(rect, Point2D::new(AI_CHAT_WIDTH - PAD - 52.0, y)),
        Some(AIChatHit::AddAttachment)
    );
    assert_eq!(
        panel.hit_test(rect, Point2D::new(AI_CHAT_WIDTH - PAD - 16.0, y)),
        Some(AIChatHit::Send)
    );
}

#[test]
fn footer_hover_maps_bottom_toolbar_actions() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.chat.set_input_text("design a login page");
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let y = toolbar_center_y();

    assert_eq!(
        panel.footer_hover_at(rect, Point2D::new(PAD + 8.0, y)),
        Some(op_editor_core::ChatFooterButton::ModelPicker)
    );
    assert_eq!(
        panel.footer_hover_at(rect, Point2D::new(AI_CHAT_WIDTH - PAD - 52.0, y)),
        Some(op_editor_core::ChatFooterButton::AddAttachment)
    );
    assert_eq!(
        panel.footer_hover_at(rect, Point2D::new(AI_CHAT_WIDTH - PAD - 16.0, y)),
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
fn footer_agent_team_chip_is_clickable_and_hoverable() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    let point = Point2D::new(
        footer.agent_team.origin.x + footer.agent_team.size.x / 2.0,
        footer.agent_team.origin.y + footer.agent_team.size.y / 2.0,
    );

    assert_eq!(panel.hit_test(rect, point), Some(AIChatHit::CycleAgentTeam));
    assert_eq!(
        panel.footer_hover_at(rect, point),
        Some(op_editor_core::ChatFooterButton::AgentTeam)
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
    let mut s = EditorState::new(); // chat empty by default
    seed_available_model(&mut s);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // First example card: top-left of grid.
    let card_w = (AI_CHAT_WIDTH - PAD * 2.0 - 8.0) / 2.0;
    let p = Point2D::new(PAD + card_w / 2.0, HEADER_HEIGHT + 32.0 + 35.0);
    match panel.hit_test(rect, p) {
        Some(AIChatHit::Example(prompt)) => {
            // The click payload is the card's full prompt — what the
            // host inserts into the chat input.
            assert_eq!(prompt, panel.examples[0].prompt);
        }
        other => panic!("expected first example hit, got {:?}", other),
    }
}

#[test]
fn hit_test_uses_taller_ts_quick_action_card_height() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let card_w = (AI_CHAT_WIDTH - PAD * 2.0 - 8.0) / 2.0;
    let p = Point2D::new(PAD + card_w / 2.0, HEADER_HEIGHT + 32.0 + 64.0);

    match panel.hit_test(rect, p) {
        Some(AIChatHit::Example(prompt)) => assert_eq!(prompt, panel.examples[0].prompt),
        other => panic!("expected first example hit in taller TS-style card, got {other:?}"),
    }
}

#[test]
fn hit_test_header_returns_drag_handle() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // Click in the empty header band (between title and icons).
    let p = Point2D::new(AI_CHAT_WIDTH / 2.0, 16.0);
    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::DragHandle));
}

#[test]
fn hit_test_header_title_text_toggles_collapse_for_hover_feedback() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let p = Point2D::new(PAD + 64.0, 18.0);

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::ToggleCollapse));
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
        _: &str,
        top_left: Point2D,
        size: f32,
        color: crate::Color,
        width: f32,
    ) {
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
