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
fn paint_header_uses_auto_generated_chat_title() {
    let mut s = EditorState::new();
    s.chat.title = "现代移动端登录页面".into();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    // old→new (MT.2 tab row): tab title font is now TAB_FONT_SIZE = 12px
    // (was 13px in the single active pill from #27 restyle). The tab row
    // always renders the active tab title — possibly ellipsized when the
    // title is longer than the tab width. We check that a text run whose
    // content starts with the title prefix appears at TAB_FONT_SIZE.
    let tab_font = crate::widgets::ai_chat_panel_header::TAB_FONT_SIZE;
    assert!(
        backend
            .texts
            .iter()
            .any(|(text, size, _, _)| text.starts_with("现代移动端")
                && (*size - tab_font).abs() < 1e-4),
        "expanded header tab row should render the current chat title at TAB_FONT_SIZE={tab_font}"
    );
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
    // old→new (#33): pills use EXAMPLE_PILL_RADIUS (~20px) not 8px.
    // old→new (#37): pills now use EXAMPLE_PILL_RADIUS=14px (compact).
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
                && *radius >= 12.0
                && color_close(*color, panel.theme.button_hover)
        }),
        "hovered quick-action pill should paint a visible hover wash with large radius"
    );
}

#[test]
fn paint_quick_action_card_pressed_uses_shared_feedback() {
    // old→new (#33): pills use EXAMPLE_PILL_RADIUS (~20px) not 8px.
    // old→new (#37): pills now use EXAMPLE_PILL_RADIUS=14px (compact).
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.editor_ui.pressed_button = Some(op_editor_core::ButtonPressTarget::ChatExample(0));
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let cards = crate::widgets::ai_chat_panel_paint::example_card_rects(rect);
    let expected = panel
        .theme
        .button_hover
        .with_alpha(panel.theme.button_hover.a * 1.8);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend.round_rects.iter().any(|(r, radius, color)| {
            rect_close(*r, cards[0]) && *radius >= 12.0 && color_close(*color, expected)
        }),
        "pressed quick-action pill should paint the shared pressed feedback token with large radius"
    );
}

#[test]
fn paint_new_chat_tooltip_after_transcript_bubbles() {
    let mut s = EditorState::new();
    s.editor_ui.chat_header_hover = Some(op_editor_core::ChatHeaderButton::NewChat);
    s.chat
        .messages
        .push(op_editor_core::ChatMessage::user("Design the KPI row"));
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let body = panel.body_rect(rect);
    let user_bubble =
        crate::widgets::ai_chat_transcript::build_transcript(&s.chat.messages, body, panel.locale)
            [0]
        .bubble
        .as_ref()
        .expect("user bubble")
        .rect;
    let tooltip = crate::widgets::ai_chat_panel_header::new_chat_tooltip_rect(rect);
    let tooltip_bg = crate::Color {
        r: 0.12,
        g: 0.12,
        b: 0.14,
        a: 0.97,
    };
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let user_bubble_op = backend
        .ops
        .iter()
        .position(|op| {
            matches!(
                op,
                PanelPaintOp::FillRoundRect { rect, radius, color }
                    if rect_close(*rect, user_bubble)
                        && (*radius - 14.0).abs() < 0.01
                        && color_close(*color, panel.theme.user_bubble)
            )
        })
        .expect("user transcript bubble painted");
    let tooltip_op = backend
        .ops
        .iter()
        .position(|op| {
            matches!(
                op,
                PanelPaintOp::FillRoundRect { rect, radius, color }
                    if rect_close(*rect, tooltip)
                        && (*radius - 6.0).abs() < 0.01
                        && color_close(*color, tooltip_bg)
            )
        })
        .expect("new-chat tooltip painted");

    assert!(
        tooltip_op > user_bubble_op,
        "tooltip fill op index {tooltip_op} must be after transcript bubble index {user_bubble_op}"
    );
}

#[test]
fn paint_send_button_hover_adds_visible_feedback() {
    // The active send circle dims on hover (rest 1.0 → hover 0.9 alpha), so the
    // hovered fill must visibly differ from the resting fill — not just exist.
    let send_fill = |hovered: bool| -> crate::Color {
        let mut s = EditorState::new();
        seed_available_model(&mut s);
        s.chat.set_input_text("design a login page");
        if hovered {
            s.editor_ui.chat_footer_hover = Some(op_editor_core::ChatFooterButton::Send);
        }
        let panel = AIChatPlaceholder::from_editor(&s);
        let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
        let mut backend = PanelPaintBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);
        // Use footer_layout to get the exact rect rather than hardcoding.
        let input = panel.input_rect(rect);
        let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
        let footer = panel.footer_layout(rect, input, toolbar_top);
        backend
            .round_rects
            .iter()
            .filter(|(r, _, _)| rect_close(*r, footer.send))
            .map(|(_, _, c)| *c)
            .next_back()
            .expect("send circle must paint a fill")
    };

    let resting = send_fill(false);
    let hovered = send_fill(true);
    assert!(
        !color_close(resting, hovered),
        "hovered send fill must visibly differ from the resting fill"
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
    // #27 restyle: model pill and speed/attach have hover washes; send
    // is now a filled circle (bg always present); palette is bare icon.
    // Test the subset that emit a neutral wash rect on hover.
    let cases = [
        op_editor_core::ChatFooterButton::ModelPicker,
        op_editor_core::ChatFooterButton::SpeedChip,
        op_editor_core::ChatFooterButton::AddAttachment,
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
        // old→new: AgentTeam removed (zero-width in #27); Send is always
        // a filled circle so its hover is a bg-alpha change, not a separate wash.
        let target = match hover {
            op_editor_core::ChatFooterButton::ModelPicker => footer.model,
            op_editor_core::ChatFooterButton::SpeedChip => footer.speed,
            op_editor_core::ChatFooterButton::AgentTeam => footer.agent_team,
            op_editor_core::ChatFooterButton::AddAttachment => footer.attach,
            op_editor_core::ChatFooterButton::Palette => footer.palette,
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

    // old→new (#38): ⚡ chip is now RIGHT-aligned; model hover rect must stay
    // within the model pill bounds (does not extend to the speed chip).
    assert!(
        hover.0.origin.x + hover.0.size.x <= footer.model.origin.x + footer.model.size.x + 0.01,
        "model hover rect should stay within the model pill bounds"
    );
    // Model right edge stays well before the speed chip (now right-aligned).
    assert!(
        footer.model.origin.x + footer.model.size.x < footer.speed.origin.x,
        "model pill right edge must be left of the speed chip (#38)"
    );
}

#[test]
fn footer_speed_chip_paints_agent_team_size_after_zap_icon() {
    // old→new (#32): the ⚡ chip now shows "{N}x" (agent_team_size) instead of the
    // effort level label. old test was footer_speed_chip_paints_effort_label_after_zap_icon.
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    // Set a non-default value to confirm it's the agent_team_size being painted.
    s.chat.agent_team_size = 3;
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

    // The "{N}x" label should appear right of the ⚡ icon origin (x = speed.origin + 19).
    let agents_lbl = "3x";
    let (_, _, _, label_origin) = backend
        .texts
        .iter()
        .find(|(text, _, _, _)| text == agents_lbl)
        .expect("footer should paint agent_team_size label (e.g. '3x') inside speed chip");
    // Label must be to the right of the speed chip start and within the chip.
    assert!(
        label_origin.x > footer.speed.origin.x,
        "agents label must be right of the chip origin"
    );
    assert!(
        label_origin.x < footer.speed.origin.x + footer.speed.size.x + 4.0,
        "agents label must stay within the chip bounds (got x={}, chip right={})",
        label_origin.x,
        footer.speed.origin.x + footer.speed.size.x
    );
}

#[test]
fn footer_agent_team_chip_is_zero_width_in_27_layout() {
    // old test: footer_agent_team_chip_uses_comfortable_pill_spacing
    // old→new: #27 removes the agent-team chip from the visible toolbar;
    // the speed chip (⚡ + effort label) takes its visual role.
    // The agent_team rect is retained in FooterLayout for round-trip
    // compatibility but has zero width so contains() never fires.
    let mut s = EditorState::new();
    seed_available_model(&mut s);
    s.chat.agent_team_size = 2;
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);

    assert_close(footer.agent_team.size.x, 0.0);
    // Speed chip is now the effort indicator; it follows the model pill.
    assert!(footer.speed.size.x > 0.0);
    assert!(footer.speed.origin.x > footer.model.origin.x + footer.model.size.x);
}

#[test]
fn footer_model_dropdown_keeps_short_model_name_readable() {
    let mut s = EditorState::new();
    s.chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "haiku",
            "Haiku",
        ));
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

    assert!(
        footer.model.size.x >= 96.0,
        "short model names still need a comfortable dropdown width"
    );
    assert!(
        backend.texts.iter().any(|(text, _, _, _)| text == "Haiku"),
        "short model name should not truncate inside the dropdown"
    );
}

#[test]
fn footer_toolbar_labels_align_and_model_label_leaves_chevron_room() {
    let mut s = EditorState::new();
    s.chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "default",
            "Default (recommended)",
        ));
    s.selection.set = vec![op_editor_core::NodeId::new("n1")];
    s.selection.anchor = op_editor_core::NodeId::new("n1");
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let toolbar_center = toolbar_center_y();
    let footer = panel.footer_layout(rect, input, toolbar_top);
    assert!(
        footer.model.size.x >= 128.0,
        "model dropdown should be wide enough to show a recognizable model name"
    );
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let (painted_model, model_size, _, model_origin) = backend
        .texts
        .iter()
        .find(|(text, _, _, _)| text.starts_with("Default") && text.ends_with('…'))
        .expect("overflowing model label should truncate with an ellipsis");
    let model_right = model_origin.x + footer_label_width(painted_model, *model_size);
    let chevron_left = footer.model.origin.x + footer.model.size.x - 16.0;
    assert!(
        model_right <= chevron_left - 4.0,
        "model label should leave room for chevron; label right={model_right}, chevron={chevron_left}"
    );

    // old→new (#32): the effort label was replaced by the agent_team_size "{N}x" label.
    // old→new (#27): "1x" agent_team chip and "已选择 1 个" selected count were removed.
    // Both the model label and the "{N}x" chip label must be vertically centered.
    let agents_lbl = format!("{}x", s.chat.agent_team_size);
    for label in [painted_model.as_str(), agents_lbl.as_str()] {
        let (_, size, _, origin) = backend
            .texts
            .iter()
            .find(|(text, _, _, _)| text == label)
            .expect("footer label should paint");
        let visual_center = origin.y - size * 0.35;
        assert!(
            (visual_center - toolbar_center).abs() <= 1.0,
            "{label} should be vertically centered; text center={visual_center}, toolbar={toolbar_center}"
        );
    }
}

#[test]
fn paint_expanded_header_active_tab_pill_is_painted() {
    // old→new (MT.2 tab row): ToggleCollapse hover no longer paints a wash across
    // the whole title area — hovering the chevron only changes the chevron icon
    // color. Instead the tab row always paints the active tab with a filled pill.
    // This test verifies: with one tab (default), the active tab pill is painted
    // inside the tab zone (x >= tab_row_left, y in header range).
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let tab_left = crate::widgets::ai_chat_panel_header::tab_row_left(rect);
    let tab_h = 26.0; // PILL_H
    let tab_radius = 8.0; // PILL_RADIUS
                          // Active tab pill: a round-rect with PILL_RADIUS inside the tab zone,
                          // filled with theme.secondary. Verify at least one such rect is painted.
    assert!(
        backend.round_rects.iter().any(|(r, radius, color)| {
            r.origin.x >= tab_left - 0.01
                && r.origin.x < tab_left + 200.0     // within the tab zone
                && (r.size.y - tab_h).abs() < 0.01   // exact pill height
                && (*radius - tab_radius).abs() < 0.01
                && color_close(*color, panel.theme.secondary)
        }),
        "active tab pill must be painted inside the tab zone with PILL_H={tab_h} and theme.secondary fill"
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
    ops: Vec<PanelPaintOp>,
}

#[derive(Debug, Clone, Copy)]
enum PanelPaintOp {
    FillRoundRect {
        rect: Rect,
        radius: f32,
        color: crate::Color,
    },
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
        self.ops.push(PanelPaintOp::FillRoundRect {
            rect,
            radius,
            color,
        });
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
