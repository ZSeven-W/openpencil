use crate::widgets::agent_settings_panel::AgentSettingsPanel;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_settings::AgentSettingsTab;
use op_editor_core::editor_ui_state::ThemeMode;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget, EditorState};

const ACTION_W: f32 = 24.0;
const SWITCH_W: f32 = 36.0;
const CONNECT_BTN_W: f32 = 96.0;

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

fn builtin_card_rect(panel: Rect) -> Rect {
    let (content_x, content_y, content_w) = content_metrics(panel);
    Rect {
        origin: Point2D::new(content_x, content_y + 12.0 + 28.0 + 28.0),
        size: Point2D::new(content_w, 60.0),
    }
}

fn builtin_edit_rect(panel: Rect) -> Rect {
    let card = builtin_card_rect(panel);
    let switch_x = card.origin.x + card.size.x - 12.0 - SWITCH_W - 8.0 - ACTION_W * 2.0 - 4.0;
    Rect {
        origin: Point2D::new(
            switch_x + SWITCH_W + 8.0,
            card.origin.y + (card.size.y - ACTION_W) / 2.0,
        ),
        size: Point2D::new(ACTION_W, ACTION_W),
    }
}

fn builtin_remove_rect(panel: Rect) -> Rect {
    let edit = builtin_edit_rect(panel);
    Rect {
        origin: Point2D::new(edit.origin.x + ACTION_W + 4.0, edit.origin.y),
        size: Point2D::new(ACTION_W, ACTION_W),
    }
}

fn acp_card_rect_after_empty_builtin(panel: Rect) -> Rect {
    let (content_x, content_y, content_w) = content_metrics(panel);
    let acp_header_y = content_y + 12.0 + 28.0 + 28.0 + 64.0 + 28.0;
    Rect {
        origin: Point2D::new(content_x, acp_header_y + 28.0 + 28.0),
        size: Point2D::new(content_w, 60.0),
    }
}

fn acp_edit_rect(panel: Rect) -> Rect {
    let card = acp_card_rect_after_empty_builtin(panel);
    let connection_x = card.origin.x + card.size.x - 12.0 - CONNECT_BTN_W;
    Rect {
        origin: Point2D::new(
            connection_x - 8.0 - ACTION_W * 2.0 - 4.0,
            card.origin.y + (card.size.y - ACTION_W) / 2.0,
        ),
        size: Point2D::new(ACTION_W, ACTION_W),
    }
}

fn acp_remove_rect(panel: Rect) -> Rect {
    let edit = acp_edit_rect(panel);
    Rect {
        origin: Point2D::new(edit.origin.x + ACTION_W + 4.0, edit.origin.y),
        size: Point2D::new(ACTION_W, ACTION_W),
    }
}

#[test]
fn pressed_builtin_compact_actions_use_shared_button_feedback() {
    for (button, expected_rect) in [
        (
            AgentSettingsButton::BuiltinEdit(0),
            builtin_edit_rect as fn(Rect) -> Rect,
        ),
        (
            AgentSettingsButton::BuiltinRemove(0),
            builtin_remove_rect as fn(Rect) -> Rect,
        ),
    ] {
        let mut state = EditorState::default();
        state.editor_ui.theme_mode = ThemeMode::Light;
        state.editor_ui.agent_settings.tab = AgentSettingsTab::Agents;
        state
            .editor_ui
            .agent_settings
            .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
        state.editor_ui.agent_settings.hover_builtin_agent = 0;
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
fn pressed_acp_compact_actions_use_shared_button_feedback() {
    for (button, expected_rect) in [
        (
            AgentSettingsButton::AcpEdit(0),
            acp_edit_rect as fn(Rect) -> Rect,
        ),
        (
            AgentSettingsButton::AcpRemove(0),
            acp_remove_rect as fn(Rect) -> Rect,
        ),
    ] {
        let mut state = EditorState::default();
        state.editor_ui.theme_mode = ThemeMode::Light;
        state.editor_ui.agent_settings.tab = AgentSettingsTab::Agents;
        state.editor_ui.agent_settings.add_acp_agent();
        state.editor_ui.agent_settings.hover_acp_agent = 0;
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
