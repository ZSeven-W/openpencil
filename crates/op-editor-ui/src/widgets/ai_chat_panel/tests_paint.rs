//! Paint-assertion tests for [`super::AIChatPlaceholder`] — drive the
//! widget against a recording [`PanelPaintBackend`] and assert on the
//! emitted draw ops. Sibling of `tests.rs`, split to keep both files
//! under the 800-line cap.

use super::tests::{seed_available_model, toolbar_center_y};
use super::*;

pub(in super::super) fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-4,
        "expected {actual} to be close to {expected}"
    );
}

pub(in super::super) fn rect_close(actual: Rect, expected: Rect) -> bool {
    (actual.origin.x - expected.origin.x).abs() < 0.01
        && (actual.origin.y - expected.origin.y).abs() < 0.01
        && (actual.size.x - expected.size.x).abs() < 0.01
        && (actual.size.y - expected.size.y).abs() < 0.01
}

pub(in super::super) fn color_close(actual: crate::Color, expected: crate::Color) -> bool {
    (actual.r - expected.r).abs() < 0.001
        && (actual.g - expected.g).abs() < 0.001
        && (actual.b - expected.b).abs() < 0.001
        && (actual.a - expected.a).abs() < 0.001
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
    assert_eq!(backend.texts[0].2, (panel.theme.muted_foreground).to_jian());
    assert_close(backend.texts[0].3.x, 12.0 + 13.0 + 6.0);
    assert_eq!(backend.svg_strokes.len(), 2);
    assert_close(backend.svg_strokes[0].0.x, 12.0);
    assert_close(backend.svg_strokes[0].1, 13.0);
    assert_close(backend.svg_strokes[1].0.x, rect.size.x - 12.0 - 12.0);
    assert_close(backend.svg_strokes[1].1, 12.0);
}

#[test]
fn paint_collapsed_bar_hover_adds_visible_feedback_across_pill() {
    let mut s = EditorState::new();
    s.chat.collapsed = true;
    s.editor_ui.chat_header_hover = Some(op_editor_core::ChatHeaderButton::ToggleCollapse);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_COLLAPSED_WIDTH, AI_CHAT_COLLAPSED_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend.round_rects.iter().any(|(r, radius, color)| {
            rect_close(*r, rect)
                && *radius >= 8.0
                && color_close(*color, chat_neutral_hover_color(&panel.theme))
        }),
        "collapsed New Chat hover should paint a visible wash across the full pill"
    );
}

#[test]
fn paint_quick_action_card_hover_adds_visible_feedback() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.editor_ui.chat_example_hover = Some(0);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let cards = crate::widgets::ai_chat_panel_paint::example_card_rects(rect);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend.round_rects.iter().any(|(r, radius, color)| {
            rect_close(*r, cards[0])
                && *radius == 8.0
                && color_close(*color, panel.theme.button_hover)
        }),
        "hovered quick-action card should paint a visible hover wash"
    );
}

#[test]
fn paint_send_button_hover_adds_visible_feedback() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.chat.set_input_text("design a login page");
    s.editor_ui.chat_footer_hover = Some(op_editor_core::ChatFooterButton::Send);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let send_rect = Rect {
        origin: Point2D::new(AI_CHAT_WIDTH - PAD - 24.0, toolbar_center_y() - 24.0 / 2.0),
        size: Point2D::new(24.0, 24.0),
    };
    let fills: Vec<_> = backend
        .round_rects
        .iter()
        .filter(|(r, _, _)| rect_close(*r, send_rect))
        .collect();

    // Ghost button: no resting fill, so hover feedback is the only
    // wash painted over the send rect.
    assert!(
        !fills.is_empty(),
        "hovered send button should paint a hover wash"
    );
}

#[test]
fn from_editor_picks_up_chat_button_press_targets() {
    let mut s = EditorState::new();
    s.editor_ui.pressed_button = Some(op_editor_core::ButtonPressTarget::ChatHeader(
        op_editor_core::ChatHeaderButton::NewChat,
    ));
    let header_panel = AIChatPlaceholder::from_editor(&s);
    assert_eq!(
        header_panel.header_pressed,
        Some(op_editor_core::ChatHeaderButton::NewChat)
    );
    assert_eq!(header_panel.footer_pressed, None);

    s.editor_ui.pressed_button = Some(op_editor_core::ButtonPressTarget::ChatFooter(
        op_editor_core::ChatFooterButton::Send,
    ));
    let footer_panel = AIChatPlaceholder::from_editor(&s);
    assert_eq!(footer_panel.header_pressed, None);
    assert_eq!(
        footer_panel.footer_pressed,
        Some(op_editor_core::ChatFooterButton::Send)
    );
}

#[test]
fn paint_footer_neutral_hovers_use_visible_feedback() {
    let cases = [
        op_editor_core::ChatFooterButton::ModelPicker,
        op_editor_core::ChatFooterButton::AgentTeam,
        op_editor_core::ChatFooterButton::AddAttachment,
        op_editor_core::ChatFooterButton::Send,
    ];

    for hover in cases {
        let mut s = EditorState::new();
        seed_available_model(&mut s);
        s.editor_ui.chat_footer_hover = Some(hover);
        let panel = AIChatPlaceholder::from_editor(&s);
        let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
        let input = panel.input_rect(rect);
        let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
        let footer = panel.footer_layout(rect, input, toolbar_top);
        let target = match hover {
            op_editor_core::ChatFooterButton::ModelPicker => footer.model,
            op_editor_core::ChatFooterButton::AgentTeam => footer.agent_team,
            op_editor_core::ChatFooterButton::AddAttachment => footer.attach,
            op_editor_core::ChatFooterButton::Send => footer.send,
            op_editor_core::ChatFooterButton::Stop => unreachable!(),
        };
        let mut backend = PanelPaintBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend.round_rects.iter().any(|(r, _, color)| {
                rect_close(*r, target)
                    && !color_close(*color, panel.theme.muted)
                    && color.a > panel.theme.button_hover.a + 0.01
            }),
            "{hover:?} hover should paint a visible neutral wash"
        );
    }
}

#[test]
fn paint_model_picker_hover_stays_inside_model_chip() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.editor_ui.chat_footer_hover = Some(op_editor_core::ChatFooterButton::ModelPicker);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let hover = backend
        .round_rects
        .iter()
        .find(|(r, _, color)| {
            rect_close(*r, footer.model)
                && color_close(*color, chat_neutral_hover_color(&panel.theme))
        })
        .expect("model picker hover should paint a visible wash");

    assert!(
        hover.0.origin.x + hover.0.size.x <= footer.agent_team.origin.x - 6.0,
        "model hover should leave visible spacing before the Agent Team chip"
    );
}

#[test]
fn footer_selection_count_sits_close_to_agent_team_chip() {
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.selection.set = vec![op_editor_core::NodeId::new("n1")];
    s.selection.anchor = op_editor_core::NodeId::new("n1");
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let selected_text =
        op_i18n::translate(panel.locale, "common.selected").replace("{{count}}", "1");
    let (_, _, _, origin) = backend
        .texts
        .iter()
        .find(|(text, _, _, _)| text == &selected_text)
        .expect("footer should paint selected-count label");
    let gap = origin.x - (footer.agent_team.origin.x + footer.agent_team.size.x);
    assert_close(gap, 4.0);
}

#[test]
fn paint_expanded_header_title_hover_adds_visible_feedback_across_label() {
    let mut s = EditorState::new();
    s.editor_ui.chat_header_hover = Some(op_editor_core::ChatHeaderButton::ToggleCollapse);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend.round_rects.iter().any(|(r, _, color)| {
            r.origin.x <= PAD
                && r.origin.y <= 6.0
                && r.size.x >= 108.0
                && r.size.y >= 28.0
                && r.size.y <= 34.0
                && color_close(*color, chat_neutral_hover_color(&panel.theme))
        }),
        "expanded New Chat title hover should cover the label, not only the chevron"
    );
}

#[derive(Default)]
struct PanelPaintBackend {
    fills: Vec<(Rect, crate::Color)>,
    round_rects: Vec<(Rect, f32, crate::Color)>,
    texts: Vec<(String, f32, jian_core::scene::Color, Point2D)>,
    svg_strokes: Vec<(Point2D, f32, crate::Color, f32)>,
    svg_paths: Vec<String>,
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

    let key_paths = crate::widgets::icons::Icon::Key.paths();
    assert!(
        key_paths
            .iter()
            .all(|kp| backend.svg_paths.iter().any(|p| p == kp)),
        "built-in selected model chip should paint the lucide Key glyph"
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
