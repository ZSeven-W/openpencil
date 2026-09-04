use crate::widgets::agent_settings_panel::AgentSettingsPanel;
use crate::widgets::text_metrics;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_settings::{AgentSettingsTab, McpCli};
use op_editor_core::editor_ui_state::ThemeMode;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget, EditorState};

mod images;

#[derive(Default)]
struct CaptureBackend {
    round_fills: Vec<(Rect, Color)>,
    ovals: Vec<(Rect, Color)>,
    round_strokes: Vec<(Rect, Color, f32)>,
    clips: Vec<Rect>,
    lines: Vec<(Point2D, Point2D, Color, f32)>,
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
    fn clip_rect(&mut self, rect: Rect) {
        self.clips.push(rect);
    }
    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
        self.lines.push((from, to, color, width));
    }
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

fn settings_content_metrics(rect: Rect) -> (f32, f32, f32) {
    (
        crate::widgets::agent_settings_panel::content_viewport(rect)
            .origin
            .x,
        crate::widgets::agent_settings_panel::content_viewport(rect)
            .origin
            .y,
        crate::widgets::agent_settings_panel::content_viewport(rect)
            .size
            .x,
    )
}

fn codex_mcp_row_rect(rect: Rect) -> Rect {
    // Rows are painted in `McpCli::DISPLAY` order, so the row index is the
    // position in DISPLAY, not in ALL.
    let codex_idx = McpCli::DISPLAY
        .iter()
        .position(|cli| *cli == McpCli::Codex)
        .unwrap();
    super::agent_settings_mcp::cli_row_rect(
        crate::widgets::agent_settings_panel::content_viewport(rect),
        codex_idx,
    )
}

/// Right-aligned switch track on a settings row.
fn row_switch_track(row: Rect) -> Rect {
    super::agent_settings_rows::row_control_rect(
        row,
        super::agent_settings_switch::SETTINGS_SWITCH_W,
        super::agent_settings_switch::SETTINGS_SWITCH_H,
    )
}

/// System-tab row `index`, below that tab's compact heading. Its rows
/// differ in height (two of them carry a second line), so this walks the
/// mixed ladder rather than assuming a uniform stride.
fn system_row_rect(rect: Rect, index: usize) -> Rect {
    use super::agent_settings_rows::RowLines;
    const SYSTEM_ROWS: [RowLines; 4] = [RowLines::One, RowLines::Two, RowLines::Two, RowLines::One];
    let content = crate::widgets::agent_settings_panel::content_viewport(rect);
    super::agent_settings_rows::row_rect_in(
        content,
        content.origin.y + super::agent_settings_rows::tab_intro_height(true),
        &SYSTEM_ROWS,
        index,
    )
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

/// Switch on the first saved built-in agent row. Asks the section for
/// the row box and the layout module for the control inside it — a
/// hand-copied ladder here is a second source of truth, and it was
/// already wrong by the row height when the rows got tighter.
fn builtin_agent_switch_rect(rect: Rect) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let card = Rect {
        origin: Point2D::new(content_x, content_y + first_builtin_row_offset()),
        size: Point2D::new(
            content_w,
            crate::widgets::agent_settings_metrics::ROW_H_TWO_LINE,
        ),
    };
    super::agent_settings_builtin_layout::compact_switch_rect(card)
}

/// Offset from the content top to the first saved built-in agent row.
fn first_builtin_row_offset() -> f32 {
    crate::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT
        + super::agent_settings_metrics::SECTION_HEADER_H
        + super::agent_settings_metrics::SECTION_SUBTITLE_H
}

fn mcp_client_config_copy_rect(rect: Rect) -> Rect {
    super::agent_settings_mcp::client_config_copy_button_rect(
        crate::widgets::agent_settings_panel::content_viewport(rect),
        // Every state in this file is `EditorState::default()`, whose
        // `external_cli_available` is the desktop default.
        true,
    )
}

fn mcp_server_button_rect(rect: Rect) -> Rect {
    super::agent_settings_mcp::server_button_rect(
        crate::widgets::agent_settings_panel::content_viewport(rect),
    )
}

fn add_provider_button_rect(rect: Rect, _text_w: f32) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    super::agent_settings_header_action::header_action_rect(
        Rect {
            origin: Point2D::new(content_x, content_y),
            size: Point2D::new(content_w, 0.0),
        },
        content_y + crate::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT,
    )
}

fn add_acp_agent_button_rect(rect: Rect, _text_w: f32) -> Rect {
    let (content_x, content_y, content_w) = settings_content_metrics(rect);
    let builtin_h = super::agent_settings_metrics::SECTION_HEADER_H
        + super::agent_settings_metrics::SECTION_SUBTITLE_H
        + super::agent_settings_metrics::EMPTY_BLOCK_H;
    let acp_y = content_y
        + crate::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT
        + builtin_h
        + super::agent_settings_metrics::SECTION_GAP;
    super::agent_settings_header_action::header_action_rect(
        Rect {
            origin: Point2D::new(content_x, content_y),
            size: Point2D::new(content_w, 0.0),
        },
        acp_y,
    )
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
fn enabled_mcp_cli_row_is_borderless_with_a_hairline_separator() {
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
    let row = codex_mcp_row_rect(rect);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        !backend
            .round_strokes
            .iter()
            .any(|(r, _, _)| rect_eq(*r, row)),
        "CLI integrations list rows carry no card outline — the hairline is the separator"
    );
    assert!(
        !backend.round_fills.iter().any(|(r, _)| rect_eq(*r, row)),
        "an enabled CLI row must not tint its background; the green switch carries the state"
    );
    let hairline_y = row.origin.y + row.size.y;
    assert!(
        backend.lines.iter().any(|(from, to, color, width)| {
            (from.y - hairline_y).abs() < 0.01
                && (to.y - hairline_y).abs() < 0.01
                && (from.x - row.origin.x).abs() < 0.01
                && (to.x - (row.origin.x + row.size.x)).abs() < 0.01
                && color_eq(*color, panel.theme.border)
                && (*width - 1.0).abs() < 0.01
        }),
        "each CLI row should be separated by a full-width hairline"
    );
}

#[test]
fn settings_content_clip_preserves_full_width_row_hairlines() {
    let mut state = EditorState::default();
    state.editor_ui.theme_mode = ThemeMode::Light;
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content = super::agent_settings_panel_geometry::content_rect(rect);
    let content_right = content.origin.x + content.size.x;
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let clip = backend
        .clips
        .first()
        .expect("settings body should be clipped");
    assert!((clip.origin.y - content.origin.y).abs() < 0.01);
    assert!((clip.size.y - content.size.y).abs() < 0.01);

    let edge_hairlines: Vec<_> = backend
        .lines
        .iter()
        .filter(|(from, to, _, width)| {
            (*width - 1.0).abs() < 0.01
                && (from.x - content.origin.x).abs() < 0.01
                && (to.x - content_right).abs() < 0.01
        })
        .collect();
    assert!(
        edge_hairlines.len() >= 4,
        "the server row and the CLI list should separate with content-wide hairlines"
    );
    for (from, to, _, _) in edge_hairlines {
        assert!(clip.origin.x <= from.x);
        assert!(clip.origin.x + clip.size.x >= to.x);
    }
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
    let track = row_switch_track(codex_mcp_row_rect(rect));
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
    // Row order is Appearance, Auto-update, Experimental, Pencil cursor.
    let track = row_switch_track(system_row_rect(rect, 1));
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
    // Row order is Appearance, Auto-update, Experimental, Pencil cursor.
    let track = row_switch_track(system_row_rect(rect, 1));
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
            .any(|(r, color)| rect_eq(*r, track) && color_eq(*color, panel.theme.input)),
        "unchecked settings switch off-track should use the shadcn `input` token \
         (distinct from `border` now that the light theme gives input its own value)"
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
            .any(|(r, color)| rect_eq(*r, track) && color_eq(*color, panel.theme.status_success)),
        "builtin Agent switch should use the same 36x20 green ON track as MCP and System switches"
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
    let add_provider = add_provider_button_rect(
        rect,
        text_metrics::measure_chrome(&mut backend, add_provider_label, 12.0),
    );
    let add_acp = add_acp_agent_button_rect(
        rect,
        text_metrics::measure_chrome(&mut backend, add_acp_label, 12.0),
    );
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
fn pressed_agent_add_buttons_use_shared_button_feedback() {
    for (button, label_key, rect_for_label) in [
        (
            AgentSettingsButton::AddProvider,
            "settings.agents.addProvider",
            add_provider_button_rect as fn(Rect, f32) -> Rect,
        ),
        (
            AgentSettingsButton::AddAcpAgent,
            "settings.agents.addAcp",
            add_acp_agent_button_rect as fn(Rect, f32) -> Rect,
        ),
    ] {
        let mut state = EditorState::default();
        state.editor_ui.theme_mode = ThemeMode::Light;
        state.editor_ui.agent_settings.tab = AgentSettingsTab::Agents;
        state.editor_ui.pressed_button = Some(ButtonPressTarget::AgentSettings(button));
        let panel = AgentSettingsPanel::for_editor(&state);
        let rect = panel.rect(1200.0, 800.0);
        let mut backend = CaptureBackend::default();
        let label = op_i18n::translate(state.editor_ui.locale, label_key);
        let target = rect_for_label(
            rect,
            text_metrics::measure_chrome(&mut backend, label, 12.0),
        );
        let expected = panel
            .theme
            .button_hover
            .with_alpha(panel.theme.button_hover.a * 1.8);
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
    let y = content_y + crate::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT;
    let label = op_i18n::translate(state.editor_ui.locale, "settings.agents.addProvider");
    let mut backend = CaptureBackend::default();
    let label_w = text_metrics::measure_chrome(&mut backend, label, 12.0);
    let hover_rect = super::agent_settings_header_action::header_action_rect(content, y);
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
    let hover_rect = add_acp_agent_button_rect(
        rect,
        text_metrics::measure_chrome(&mut backend, label, 12.0),
    );
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
