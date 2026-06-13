use crate::widgets::agent_settings_panel::AgentSettingsPanel;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_settings::AgentSettingsTab;
use op_editor_core::editor_ui_state::ThemeMode;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget, EditorState};

#[derive(Default)]
struct CaptureBackend {
    round_fills: Vec<(Rect, Color)>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, _: f32, color: Color) {
        self.round_fills.push((rect, color));
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn fill_oval(&mut self, _: Rect, _: Color) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
    fn measure_text(&mut self, text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size * 0.5
    }
}

fn color_eq(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 1e-6
        && (a.g - b.g).abs() < 1e-6
        && (a.b - b.b).abs() < 1e-6
        && (a.a - b.a).abs() < 1e-6
}

fn rect_eq(a: Rect, b: Rect) -> bool {
    (a.origin.x - b.origin.x).abs() < 0.01
        && (a.origin.y - b.origin.y).abs() < 0.01
        && (a.size.x - b.size.x).abs() < 0.01
        && (a.size.y - b.size.y).abs() < 0.01
}

fn content_metrics(panel: Rect) -> (f32, f32, f32) {
    (
        panel.origin.x + 200.0 + 24.0,
        panel.origin.y + 24.0,
        panel.size.x - 200.0 - 48.0,
    )
}

fn form_save_rect(card: Rect, form_h: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 12.0 - 68.0,
            card.origin.y + form_h + 5.0,
        ),
        size: Point2D::new(68.0, 26.0),
    }
}

fn form_cancel_rect(card: Rect, form_h: f32) -> Rect {
    let save = form_save_rect(card, form_h);
    Rect {
        origin: Point2D::new(save.origin.x - 8.0 - 68.0, save.origin.y),
        size: Point2D::new(68.0, 26.0),
    }
}

fn builtin_draft_card(panel: Rect) -> Rect {
    let (content_x, content_y, content_w) = content_metrics(panel);
    Rect {
        origin: Point2D::new(content_x, content_y + 12.0 + 28.0 + 28.0),
        size: Point2D::new(content_w, 232.0),
    }
}

fn acp_draft_card_after_empty_builtin(panel: Rect) -> Rect {
    let (content_x, content_y, content_w) = content_metrics(panel);
    let acp_header_y = content_y + 12.0 + 28.0 + 28.0 + 64.0 + 28.0;
    Rect {
        origin: Point2D::new(content_x, acp_header_y + 28.0 + 28.0),
        size: Point2D::new(content_w, 370.0),
    }
}

fn builtin_save_rect(panel: Rect) -> Rect {
    form_save_rect(builtin_draft_card(panel), 196.0)
}

fn builtin_cancel_rect(panel: Rect) -> Rect {
    form_cancel_rect(builtin_draft_card(panel), 196.0)
}

fn acp_save_rect(panel: Rect) -> Rect {
    form_save_rect(acp_draft_card_after_empty_builtin(panel), 332.0)
}

fn acp_cancel_rect(panel: Rect) -> Rect {
    form_cancel_rect(acp_draft_card_after_empty_builtin(panel), 332.0)
}

#[test]
fn pressed_builtin_draft_form_buttons_use_shared_feedback() {
    for (button, expected_rect) in [
        (
            AgentSettingsButton::BuiltinCancelDraft,
            builtin_cancel_rect as fn(Rect) -> Rect,
        ),
        (
            AgentSettingsButton::BuiltinSaveDraft,
            builtin_save_rect as fn(Rect) -> Rect,
        ),
    ] {
        let mut state = EditorState::default();
        state.editor_ui.theme_mode = ThemeMode::Light;
        state.editor_ui.agent_settings.tab = AgentSettingsTab::Agents;
        state.editor_ui.agent_settings.begin_builtin_agent_draft();
        state
            .editor_ui
            .agent_settings
            .builtin_agent_draft
            .as_mut()
            .expect("draft should exist")
            .api_key = "sk-test".into();
        state.editor_ui.pressed_button = Some(ButtonPressTarget::AgentSettings(button));
        let panel = AgentSettingsPanel::for_editor(&state);
        let rect = panel.rect(1200.0, 800.0);
        let target = expected_rect(rect);
        let expected = panel
            .theme
            .button_hover
            .with_alpha(panel.theme.button_hover.a * 1.8);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .round_fills
                .iter()
                .any(|(r, color)| rect_eq(*r, target) && color_eq(*color, expected)),
            "pressed {button:?} should paint the shared pressed feedback token"
        );
    }
}

#[test]
fn pressed_acp_draft_form_buttons_use_shared_feedback() {
    for (button, expected_rect) in [
        (
            AgentSettingsButton::AcpCancelDraft,
            acp_cancel_rect as fn(Rect) -> Rect,
        ),
        (
            AgentSettingsButton::AcpSaveDraft,
            acp_save_rect as fn(Rect) -> Rect,
        ),
    ] {
        let mut state = EditorState::default();
        state.editor_ui.theme_mode = ThemeMode::Light;
        state.editor_ui.agent_settings.tab = AgentSettingsTab::Agents;
        state.editor_ui.agent_settings.begin_acp_agent_draft();
        state
            .editor_ui
            .agent_settings
            .acp_agent_draft
            .as_mut()
            .expect("draft should exist")
            .command = "op-agent".into();
        state.editor_ui.pressed_button = Some(ButtonPressTarget::AgentSettings(button));
        let panel = AgentSettingsPanel::for_editor(&state);
        let rect = panel.rect(1200.0, 800.0);
        let target = expected_rect(rect);
        let expected = panel
            .theme
            .button_hover
            .with_alpha(panel.theme.button_hover.a * 1.8);
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        panel.paint(&mut cx, rect);

        assert!(
            backend
                .round_fills
                .iter()
                .any(|(r, color)| rect_eq(*r, target) && color_eq(*color, expected)),
            "pressed {button:?} should paint the shared pressed feedback token"
        );
    }
}
