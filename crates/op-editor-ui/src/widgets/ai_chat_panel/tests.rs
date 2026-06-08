//! Layout + hit-test unit tests for [`super::AIChatPlaceholder`].
//! Split into a sibling file to keep `ai_chat_panel.rs` under the
//! 800-line cap.

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

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-4,
        "expected {actual} to be close to {expected}"
    );
}

#[test]
fn paint_collapsed_bar_matches_ts_minimized_bar_style() {
    let mut s = EditorState::new();
    s.chat.collapsed = true;
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_COLLAPSED_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    // TS: `h-8 bg-card rounded-lg gap-1.5 px-3`, with 13px
    // MessageSquare, 12px chevron, and muted 12px title text.
    assert_close(AI_CHAT_COLLAPSED_HEIGHT, 32.0);
    assert_eq!(backend.round_rects[0].0, rect);
    assert_close(backend.round_rects[0].1, 8.0);
    assert_eq!(backend.round_rects[0].2, panel.theme.card);
    assert_eq!(backend.texts[0].0, "New Chat");
    assert_close(backend.texts[0].1, 12.0);
    assert_eq!(
        backend.texts[0].2,
        to_jian_color(panel.theme.muted_foreground)
    );
    assert_close(backend.texts[0].3.x, 12.0 + 13.0 + 6.0);
    assert_eq!(backend.svg_strokes.len(), 2);
    assert_close(backend.svg_strokes[0].0.x, 12.0);
    assert_close(backend.svg_strokes[0].1, 13.0);
    assert_close(backend.svg_strokes[1].0.x, rect.size.x - 12.0 - 12.0);
    assert_close(backend.svg_strokes[1].1, 12.0);
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
fn toolbar_center_y() -> f32 {
    AI_CHAT_HEIGHT - INPUT_BASE_HEIGHT + 1.0 + INPUT_AREA_HEIGHT + INPUT_TOOLBAR_HEIGHT / 2.0
}

fn seed_available_model(s: &mut EditorState) {
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
    s.chat.input = "design a login page".into();
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
    s.chat.input = "design a login page".into();
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
    s.chat.input = "design a login page".into();
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
fn hit_test_resolves_model_search_clear_button() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.editor_ui.chat_model_picker_open = true;
    s.editor_ui.chat_model_picker_search = "231".into();
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

#[test]
fn body_rect_reserves_space_for_fixed_step_checklist() {
    let mut s = EditorState::new();
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    message.content =
        r#"<step title="Checking guidelines" status="streaming">Analyzing request...</step>"#
            .into();
    s.chat.messages.push(message);

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let body = panel.body_rect(rect);
    let legacy_bottom = rect.origin.y + rect.size.y - INPUT_BASE_HEIGHT - PAD - 8.0;

    assert!(
        body.origin.y + body.size.y < legacy_bottom - 1.0,
        "fixed step checklist should reserve bottom space outside transcript"
    );
}

#[test]
fn body_rect_reserves_less_space_when_fixed_step_checklist_collapsed() {
    let mut expanded_state = EditorState::new();
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    message.content = r#"<step title="Plan" status="done"></step>
<step title="Draw" status="streaming"></step>"#
        .into();
    expanded_state.chat.messages.push(message.clone());

    let mut collapsed_state = EditorState::new();
    collapsed_state.chat.messages.push(message);
    collapsed_state.chat.checklist_collapsed = true;

    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let expanded = AIChatPlaceholder::from_editor(&expanded_state).body_rect(rect);
    let collapsed = AIChatPlaceholder::from_editor(&collapsed_state).body_rect(rect);

    assert!(collapsed.size.y > expanded.size.y);
}

#[test]
fn hit_test_resolves_fixed_checklist_header_toggle() {
    let mut s = EditorState::new();
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    message.content = r#"<step title="Plan" status="done"></step>
<step title="Draw" status="streaming"></step>"#
        .into();
    s.chat.messages.push(message);

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let checklist_h = fixed_checklist_height(&s.chat.messages, s.chat.checklist_collapsed);
    let checklist = fixed_checklist_rect(rect, INPUT_BASE_HEIGHT, checklist_h);
    let p = Point2D::new(
        checklist.origin.x + checklist.size.x / 2.0,
        checklist.origin.y + 2.0 + 32.0 / 2.0,
    );

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::ToggleChecklist));
}

#[test]
fn hit_test_resolves_individual_tool_card_header_toggle() {
    let mut s = EditorState::new();
    let mut message = op_editor_core::ChatMessage::assistant("answer");
    message.tools_collapsed = false;
    message.tool_calls.push(op_editor_core::ChatToolCall {
        name: "snapshot_layout".into(),
        args: r#"{"args":{"pageId":"page-1"}}"#.into(),
    });
    s.chat.messages.push(message);

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let card_header = crate::widgets::ai_chat_transcript::build_transcript(
        &s.chat.messages,
        panel.body_rect(rect),
        panel.locale,
    )[0]
    .tools
    .as_ref()
    .unwrap()
    .cards[0]
        .header;
    let p = Point2D::new(
        card_header.origin.x + card_header.size.x / 2.0,
        card_header.origin.y + card_header.size.y / 2.0,
    );

    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::SetToolCallCardExpanded(0, 0, true))
    );
}

#[test]
fn hit_test_resolves_design_block_header_toggle() {
    let mut s = EditorState::new();
    s.chat.messages.push(op_editor_core::ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    ));

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let header = crate::widgets::ai_chat_transcript::build_transcript(
        &s.chat.messages,
        panel.body_rect(rect),
        panel.locale,
    )[0]
    .design_blocks[0]
        .header;
    let p = Point2D::new(
        header.origin.x + header.size.x / 2.0,
        header.origin.y + header.size.y / 2.0,
    );

    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::SetDesignBlockExpanded(0, 0, true))
    );
}

#[test]
fn hit_test_resolves_design_block_copy_button() {
    let code = r#"[{"id":"frame-1","type":"Frame"}]"#;
    let mut s = EditorState::new();
    s.chat
        .messages
        .push(op_editor_core::ChatMessage::assistant(format!(
            r#"```json
{code}
```"#
        )));

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let block = &crate::widgets::ai_chat_transcript::build_transcript(
        &s.chat.messages,
        panel.body_rect(rect),
        panel.locale,
    )[0]
    .design_blocks[0];
    let p = Point2D::new(
        block.header.origin.x + block.header.size.x - 38.0,
        block.header.origin.y + block.header.size.y / 2.0,
    );

    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::CopyDesignBlock(code.to_string()))
    );
}

#[derive(Default)]
struct PanelPaintBackend {
    fills: Vec<(Rect, crate::Color)>,
    round_rects: Vec<(Rect, f32, crate::Color)>,
    texts: Vec<(String, f32, jian_core::scene::Color, Point2D)>,
    svg_strokes: Vec<(Point2D, f32, crate::Color, f32)>,
    stroke_lines: usize,
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

#[test]
fn paint_model_chip_uses_key_glyph_for_builtin_model() {
    let mut s = EditorState::new();
    s.chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::builtin_with_display_name(
            op_editor_core::chat::AgentProvider::CodexCli,
            "builtin-minimax",
            "MiniMax",
            "builtin:builtin-minimax:MiniMax-M2.7",
            "MiniMax-M2.7",
        ));
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend.stroke_lines >= 2,
        "built-in selected model chip should paint the TS-style Key glyph"
    );
}

fn has_fill_rect(fills: &[(Rect, crate::Color)], expected: Rect) -> bool {
    fills.iter().any(|(rect, _)| {
        (rect.origin.x - expected.origin.x).abs() < 1e-4
            && (rect.origin.y - expected.origin.y).abs() < 1e-4
            && (rect.size.x - expected.size.x).abs() < 1e-4
            && (rect.size.y - expected.size.y).abs() < 1e-4
    })
}

#[test]
fn paint_draws_header_divider_and_message_body_background() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(10.0, 20.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input_h = INPUT_BASE_HEIGHT;
    let sep_y = rect.origin.y + rect.size.y - input_h;
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(has_fill_rect(
        &backend.fills,
        Rect::xywh(
            rect.origin.x + 1.0,
            rect.origin.y + HEADER_HEIGHT,
            rect.size.x - 2.0,
            1.0
        )
    ));
    assert!(has_fill_rect(
        &backend.fills,
        Rect::xywh(
            rect.origin.x + 1.0,
            rect.origin.y + HEADER_HEIGHT + 1.0,
            rect.size.x - 2.0,
            sep_y - (rect.origin.y + HEADER_HEIGHT + 1.0),
        )
    ));
}
