use crate::widgets::agent_settings_panel::AgentSettingsPanel;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_settings::{AgentSettingsTab, McpCli};
use op_editor_core::editor_ui_state::ThemeMode;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget, EditorState};

#[derive(Default)]
struct CaptureBackend {
    round_fills: Vec<(Rect, Color)>,
    ovals: Vec<(Rect, Color)>,
    round_strokes: Vec<(Rect, Color, f32)>,
    text_points: Vec<Point2D>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, point: Point2D) {
        self.text_points.push(point);
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, _: f32, color: Color) {
        self.round_fills.push((rect, color));
    }
    fn stroke_round_rect(&mut self, rect: Rect, _: f32, color: Color, width: f32) {
        self.round_strokes.push((rect, color, width));
    }
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn fill_oval(&mut self, rect: Rect, color: Color) {
        self.ovals.push((rect, color));
    }
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
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

fn has_visible_hover_fill(
    backend: &CaptureBackend,
    rect: Rect,
    theme: crate::theme::Theme,
) -> bool {
    backend.round_fills.iter().any(|(r, color)| {
        rect_eq(*r, rect) && !color_eq(*color, theme.muted) && color.a > theme.button_hover.a + 0.01
    })
}

fn settings_content_metrics(rect: Rect) -> (f32, f32, f32) {
    (
        rect.origin.x + 200.0 + 24.0,
        rect.origin.y + 24.0,
        rect.size.x - 200.0 - 48.0,
    )
}

fn codex_mcp_cell_rect(rect: Rect) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let cell_w = (content_w - 16.0) / 2.0;
    let grid_top = content_y + 36.0 + 52.0 + 28.0 + 28.0 + 20.0 * 2.0 + 12.0;
    Rect {
        origin: Point2D::new(content_x + cell_w + 16.0, grid_top),
        size: Point2D::new(cell_w, 52.0),
    }
}

fn ts_switch_knob_rect(track: Rect, enabled: bool) -> Rect {
    let inset = 2.0;
    let knob = 16.0;
    let x = if enabled {
        track.origin.x + track.size.x - knob - inset
    } else {
        track.origin.x + inset
    };
    Rect {
        origin: Point2D::new(x, track.origin.y + inset),
        size: Point2D::new(knob, knob),
    }
}

fn builtin_agent_switch_rect(rect: Rect) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let card_y = content_y + 12.0 + 28.0 + 28.0;
    let card = Rect {
        origin: Point2D::new(content_x, card_y),
        size: Point2D::new(content_w, 60.0),
    };
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 12.0 - 36.0 - 8.0 - 24.0 * 2.0 - 4.0,
            card.origin.y + (card.size.y - 20.0) / 2.0,
        ),
        size: Point2D::new(36.0, 20.0),
    }
}

fn mcp_client_config_copy_rect(rect: Rect) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let client_config_y = content_y + 36.0 + 52.0 + 8.0;
    Rect {
        origin: Point2D::new(content_x + content_w - 12.0 - 20.0, client_config_y + 8.0),
        size: Point2D::new(20.0, 20.0),
    }
}

fn mcp_server_button_rect(rect: Rect) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let server_card_y = content_y + 36.0;
    Rect {
        origin: Point2D::new(content_x + content_w - 16.0 - 72.0, server_card_y + 12.0),
        size: Point2D::new(72.0, 28.0),
    }
}

fn add_provider_button_rect(rect: Rect, text_w: f32) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    super::agent_settings_header_action::header_action_rect(
        Rect {
            origin: Point2D::new(content_x, content_y),
            size: Point2D::new(content_w, 0.0),
        },
        content_y + 12.0,
        text_w,
    )
}

fn add_acp_agent_button_rect(rect: Rect, text_w: f32) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let builtin_h = 28.0 + 28.0 + 64.0;
    let acp_y = content_y + 12.0 + builtin_h + 28.0;
    super::agent_settings_header_action::header_action_rect(
        Rect {
            origin: Point2D::new(content_x, content_y),
            size: Point2D::new(content_w, 0.0),
        },
        acp_y,
        text_w,
    )
}

fn image_search_test_button_rect(rect: Rect) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let register_y = content_y + 36.0 + 24.0 + 22.0 + 36.0 + 10.0 + 36.0 + 14.0;
    Rect {
        origin: Point2D::new(content_x + content_w - 56.0, register_y + 4.0),
        size: Point2D::new(56.0, 28.0),
    }
}

fn image_gen_add_button_rect(rect: Rect) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let advanced_body_h = 22.0 + 36.0 + 10.0 + 36.0 + 14.0 + 36.0;
    let gen_top = content_y + 36.0 + 24.0 + advanced_body_h + 28.0;
    Rect {
        origin: Point2D::new(content_x + content_w - 72.0, gen_top + 4.0),
        size: Point2D::new(72.0, 28.0),
    }
}

#[test]
fn hovered_mcp_server_button_paints_hover_wash() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    state.editor_ui.agent_settings.hover_mcp_server_button = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let button = mcp_server_button_rect(rect);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(r, color)| rect_eq(*r, button) && color_eq(*color, panel.theme.button_hover)),
        "hovering the MCP server start/stop button should paint the same subtle wash as other buttons"
    );
}

#[test]
fn pressed_mcp_server_button_uses_shared_button_feedback() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    state.editor_ui.pressed_button = Some(ButtonPressTarget::AgentSettings(
        AgentSettingsButton::McpServer,
    ));
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let button = mcp_server_button_rect(rect);
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
            .any(|(r, color)| rect_eq(*r, button) && color_eq(*color, expected)),
        "pressed MCP server button should paint the shared pressed feedback token"
    );
}

#[test]
fn hovered_image_settings_buttons_paint_hover_wash() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.images_advanced_open = true;
    state
        .editor_ui
        .agent_settings
        .hover_image_search_test_button = true;
    state.editor_ui.agent_settings.hover_image_gen_add_button = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let search_test = image_search_test_button_rect(rect);
    let add = image_gen_add_button_rect(rect);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        has_visible_hover_fill(&backend, search_test, panel.theme),
        "hovering the image search test button should paint a visible wash over its base fill"
    );
    assert!(
        has_visible_hover_fill(&backend, add, panel.theme),
        "hovering the image generation add button should paint a visible wash over its base fill"
    );
}

#[test]
fn enabled_mcp_cli_cell_uses_subtle_border_not_primary_outline() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    let codex_idx = McpCli::ALL
        .iter()
        .position(|cli| *cli == McpCli::Codex)
        .unwrap();
    state.editor_ui.agent_settings.mcp_cli_enabled[codex_idx] = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let cell = codex_mcp_cell_rect(rect);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .round_strokes
            .iter()
            .any(|(r, color, width)| rect_eq(*r, cell)
                && color_eq(*color, panel.theme.border)
                && (*width - 1.0).abs() < 0.01),
        "enabled MCP CLI cell should keep the same subtle border as inactive cards"
    );
    assert!(
        !backend
            .round_strokes
            .iter()
            .any(|(r, color, _)| rect_eq(*r, cell) && color_eq(*color, panel.theme.primary)),
        "enabled MCP CLI cell should not look like a focused field with a primary outline"
    );
}

#[test]
fn enabled_mcp_cli_switch_uses_light_thumb() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    let codex_idx = McpCli::ALL
        .iter()
        .position(|cli| *cli == McpCli::Codex)
        .unwrap();
    state.editor_ui.agent_settings.mcp_cli_enabled[codex_idx] = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let cell = codex_mcp_cell_rect(rect);
    let track = Rect {
        origin: Point2D::new(
            cell.origin.x + cell.size.x - 16.0 - 36.0,
            cell.origin.y + (52.0 - 20.0) / 2.0,
        ),
        size: Point2D::new(36.0, 20.0),
    };
    let knob = ts_switch_knob_rect(track, true);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .ovals
            .iter()
            .any(|(r, color)| rect_eq(*r, knob) && color_eq(*color, panel.theme.primary_foreground)),
        "enabled MCP CLI switch should use a light switch thumb"
    );
    assert!(
        !backend
            .ovals
            .iter()
            .any(|(r, color)| rect_eq(*r, knob) && color_eq(*color, panel.theme.foreground)),
        "enabled MCP CLI switch thumb should not be black in light theme"
    );
}

#[test]
fn system_auto_update_switch_uses_light_thumb() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
    state.editor_ui.agent_settings.auto_update_enabled = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let card_y = content_y + 12.0 + 36.0;
    let track = Rect {
        origin: Point2D::new(
            content_x + content_w - 16.0 - 36.0,
            card_y + (58.0 - 20.0) / 2.0,
        ),
        size: Point2D::new(36.0, 20.0),
    };
    let knob = ts_switch_knob_rect(track, true);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .ovals
            .iter()
            .any(|(r, color)| rect_eq(*r, knob) && color_eq(*color, panel.theme.primary_foreground)),
        "auto-update switch should use a light switch thumb"
    );
    assert!(
        !backend
            .ovals
            .iter()
            .any(|(r, color)| rect_eq(*r, knob) && color_eq(*color, panel.theme.foreground)),
        "auto-update switch thumb should not be black in light theme"
    );
}

#[test]
fn system_auto_update_switch_matches_ts_unchecked_geometry_and_track() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
    state.editor_ui.agent_settings.auto_update_enabled = false;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let card_y = content_y + 12.0 + 36.0;
    let track = Rect {
        origin: Point2D::new(
            content_x + content_w - 16.0 - 36.0,
            card_y + (58.0 - 20.0) / 2.0,
        ),
        size: Point2D::new(36.0, 20.0),
    };
    let knob = ts_switch_knob_rect(track, false);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(r, color)| rect_eq(*r, track) && color_eq(*color, panel.theme.border)),
        "unchecked settings switch should use the TS bg-input color, mapped to the Rust border token"
    );
    assert!(
        backend
            .ovals
            .iter()
            .any(|(r, color)| rect_eq(*r, knob) && color_eq(*color, panel.theme.primary_foreground)),
        "unchecked settings switch thumb should be 16px with 2px inset like the TS Switch"
    );
}

#[test]
fn builtin_agent_switch_uses_same_geometry_as_mcp_and_system_switches() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Agents;
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    state.editor_ui.agent_settings.builtin_agents[0].enabled = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let track = builtin_agent_switch_rect(rect);
    let knob = ts_switch_knob_rect(track, true);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(r, color)| rect_eq(*r, track) && color_eq(*color, panel.theme.primary)),
        "builtin Agent switch should use the same 36x20 track as MCP and System switches"
    );
    assert!(
        backend
            .ovals
            .iter()
            .any(|(r, color)| rect_eq(*r, knob) && color_eq(*color, panel.theme.primary_foreground)),
        "builtin Agent switch should use the same 16px thumb geometry as MCP and System switches"
    );
}

#[test]
fn hovered_mcp_client_config_copy_button_paints_hover_wash() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    state.editor_ui.agent_settings.hover_mcp_client_config_copy = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let copy = mcp_client_config_copy_rect(rect);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(r, color)| rect_eq(*r, copy) && color_eq(*color, panel.theme.button_hover)),
        "hovering the MCP client config copy button should paint the same subtle wash as other icon buttons"
    );
}

#[test]
fn pressed_mcp_client_config_copy_button_uses_shared_button_feedback() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    state.editor_ui.pressed_button = Some(ButtonPressTarget::AgentSettings(
        AgentSettingsButton::McpClientConfigCopy,
    ));
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let copy = mcp_client_config_copy_rect(rect);
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
            .any(|(r, color)| rect_eq(*r, copy) && color_eq(*color, expected)),
        "pressed MCP client config copy button should paint the shared pressed feedback token"
    );
}

#[test]
fn hovered_agent_add_buttons_paint_hover_wash() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Agents;
    state.editor_ui.agent_settings.hover_add_provider = true;
    state.editor_ui.agent_settings.hover_add_acp_agent = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let add_provider_label =
        op_i18n::translate(state.editor_ui.locale, "settings.agents.addProvider");
    let add_acp_label = op_i18n::translate(state.editor_ui.locale, "settings.agents.addAcp");
    let add_provider =
        add_provider_button_rect(rect, backend.measure_text(add_provider_label, 12.0));
    let add_acp = add_acp_agent_button_rect(rect, backend.measure_text(add_acp_label, 12.0));
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(r, color)| rect_eq(*r, add_provider)
                && color_eq(*color, panel.theme.button_hover)),
        "hovering the add provider button should paint the same subtle wash as other text buttons"
    );
    assert!(
        backend
            .round_fills
            .iter()
            .any(|(r, color)| rect_eq(*r, add_acp) && color_eq(*color, panel.theme.button_hover)),
        "hovering the add ACP agent button should paint the same subtle wash as other text buttons"
    );
}

#[test]
fn builtin_add_provider_text_is_centered_in_hover_wash() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.hover_add_provider = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let content = Rect {
        origin: Point2D::new(content_x, content_y),
        size: Point2D::new(content_w, 0.0),
    };
    let y = content_y + 12.0;
    let label = op_i18n::translate(state.editor_ui.locale, "settings.agents.addProvider");
    let mut backend = CaptureBackend::default();
    let label_w = backend.measure_text(label, 12.0);
    let hover_rect = super::agent_settings_header_action::header_action_rect(content, y, label_w);
    let expected_x = super::agent_settings_header_action::header_action_text_x(hover_rect, label_w);
    let expected_y = hover_rect.origin.y + hover_rect.size.y / 2.0 + 4.0;

    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        super::agent_settings_builtin::paint_builtin_section(
            &mut cx,
            &panel.theme,
            &state.editor_ui.agent_settings,
            &state.editor_ui,
            content,
            y,
            0,
        );
    }

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(r, color)| rect_eq(*r, hover_rect)
                && color_eq(*color, panel.theme.button_hover)),
        "hovering add provider should paint the padded header action rect"
    );
    assert!(
        backend.text_points.len() >= 2,
        "built-in section should paint title then add-provider action"
    );
    let action_point = backend.text_points[1];
    assert!(
        (action_point.x - expected_x).abs() < 0.01,
        "add-provider label should be centered in its hover rect"
    );
    assert!(
        (action_point.y - expected_y).abs() < 0.01,
        "add-provider label should use balanced vertical padding in its hover rect"
    );
}

#[test]
fn acp_add_agent_text_uses_balanced_vertical_padding_in_hover_wash() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.hover_add_acp_agent = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let label = op_i18n::translate(state.editor_ui.locale, "settings.agents.addAcp");
    let mut backend = CaptureBackend::default();
    let hover_rect = add_acp_agent_button_rect(rect, backend.measure_text(label, 12.0));
    let expected_baseline_y = hover_rect.origin.y + hover_rect.size.y / 2.0 + 4.0;

    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);
    }

    let acp_action_point = backend
        .text_points
        .iter()
        .copied()
        .find(|p| {
            (p.x - hover_rect.origin.x).abs() <= hover_rect.size.x
                && p.y >= hover_rect.origin.y
                && p.y <= hover_rect.origin.y + hover_rect.size.y
        })
        .expect("ACP add-agent action should be painted inside the hover rect");
    assert!(
        (acp_action_point.y - expected_baseline_y).abs() < 0.01,
        "add-agent label should use the shared centered baseline"
    );
}
