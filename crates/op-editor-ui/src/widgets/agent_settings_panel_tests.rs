use crate::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_settings::{
    AcpAgentField, AgentSettingsTab, BuiltinAgentField, ImageGenField, ImageSearchField,
    SettingsFocus,
};
use op_editor_core::EditorState;

#[derive(Default)]
struct CaptureBackend {
    fills: Vec<(Rect, Color)>,
    icon_strokes: Vec<(Point2D, f32, usize)>,
    ops: Vec<&'static str>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.fills.push((rect, color));
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, at: Point2D, size: f32, _: Color, _: f32) {
        self.icon_strokes.push((at, size, self.ops.len()));
        self.ops.push("icon");
    }
    fn save(&mut self) {
        self.ops.push("save");
    }
    fn restore(&mut self) {
        self.ops.push("restore");
    }
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

fn caret_fills(fills: &[(Rect, Color)], color: Color) -> Vec<Rect> {
    fills
        .iter()
        .filter_map(|(rect, fill)| {
            (color_eq(*fill, color)
                && (rect.size.x - 1.5).abs() < 0.01
                && (rect.size.y - 15.0).abs() < 0.01)
                .then_some(*rect)
        })
        .collect()
}

#[test]
fn close_button_paints_after_scrollable_content() {
    let state = EditorState::default();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let close_origin = Point2D::new(rect.origin.x + rect.size.x - 32.0, rect.origin.y + 16.0);
    let close_idx = backend
        .icon_strokes
        .iter()
        .find_map(|(at, size, idx)| {
            ((at.x - close_origin.x).abs() < 0.01
                && (at.y - close_origin.y).abs() < 0.01
                && (*size - 16.0).abs() < 0.01)
                .then_some(*idx)
        })
        .expect("close icon should paint");
    let restore_idx = backend
        .ops
        .iter()
        .rposition(|op| *op == "restore")
        .expect("content clip should restore");

    assert!(
        close_idx > restore_idx,
        "close button must paint above clipped, scrollable content"
    );
}

#[test]
fn hit_test_resolves_builtin_agent_api_key_field() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_builtin_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let first_card_y = content_y + 12.0 + 28.0 + 28.0;
    let point = crate::Point2D::new(content_x + 92.0, first_card_y + 116.0);

    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::FocusBuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        }
    );
}

#[test]
fn focused_builtin_agent_field_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MINIMAX", "", "MiniMax-M2.7");
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::BaseUrl,
    });
    state.editor_ui.settings_input_draft = "https://api.minimaxi.com/v1".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 100);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert_eq!(caret_fills(&backend.fills, panel.theme.foreground).len(), 1);
}

#[test]
fn focused_builtin_agent_field_hides_caret_at_blink_off_phase() {
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MINIMAX", "", "MiniMax-M2.7");
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::BaseUrl,
    });
    state.editor_ui.settings_input_draft = "https://api.minimaxi.com/v1".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 500);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(caret_fills(&backend.fills, panel.theme.foreground).is_empty());
}

#[test]
fn focused_builtin_agent_field_paints_from_settings_draft() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_builtin_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });
    state.editor_ui.settings_input_draft = "sk-draft".into();

    let panel = AgentSettingsPanel::for_editor(&state);

    assert_eq!(
        panel.settings.focus,
        Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        })
    );
}

#[test]
fn sidebar_nav_uses_ts_compact_rows() {
    let state = EditorState::default();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let x = rect.origin.x + 100.0;

    assert_eq!(
        panel.hit_test(rect, crate::Point2D::new(x, rect.origin.y + 70.0)),
        AgentSettingsHit::SelectTab(AgentSettingsTab::Agents)
    );
    assert_eq!(
        panel.hit_test(rect, crate::Point2D::new(x, rect.origin.y + 100.0)),
        AgentSettingsHit::SelectTab(AgentSettingsTab::Mcp)
    );
}

#[test]
fn builtin_agent_cards_use_ts_compact_height_when_not_editing() {
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("DeepSeek", "sk-test", "deepseek-v4-pro");
    let panel = AgentSettingsPanel::for_editor(&state);

    // Header + subtitle + two TS-style compact cards with one gap
    // after each card. The pre-parity expanded form was >400 px.
    assert_eq!(
        panel.settings.builtin_agents.len(),
        2,
        "fixture should exercise multiple compact provider cards"
    );
    assert!(
        panel.content_total_height() < 850.0,
        "compact provider cards should not force the Agents tab to scroll immediately"
    );
}

#[test]
fn agents_tab_acp_cards_replace_empty_hint_height() {
    let empty = AgentSettingsPanel::for_editor(&EditorState::default()).content_total_height();
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_acp_agent();
    state.editor_ui.agent_settings.add_acp_agent();
    let with_acp = AgentSettingsPanel::for_editor(&state).content_total_height();

    assert!(
        with_acp > empty,
        "configured ACP agents should contribute list-card height instead of a fixed empty hint"
    );
}

#[test]
fn acp_draft_form_reserves_room_for_args_and_env_fields() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.begin_acp_agent_draft();

    let height =
        crate::widgets::agent_settings_acp::content_height(&state.editor_ui.agent_settings);

    assert!(
        height >= 320.0,
        "ACP draft form should include display name, connection type, command, args, env, and actions"
    );
}

#[test]
fn focused_acp_agent_field_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_acp_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::AcpAgent {
        index: 0,
        field: AcpAgentField::Command,
    });
    state.editor_ui.settings_input_draft = "node server.js".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 100);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert_eq!(caret_fills(&backend.fills, panel.theme.foreground).len(), 1);
}

#[test]
fn focused_empty_acp_command_field_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_acp_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::AcpAgent {
        index: 0,
        field: AcpAgentField::Command,
    });
    state.editor_ui.settings_input_draft.clear();

    let panel = AgentSettingsPanel::for_editor_at(&state, 100);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert_eq!(caret_fills(&backend.fills, panel.theme.foreground).len(), 1);
}

#[test]
fn focused_acp_agent_field_hides_caret_at_blink_off_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_acp_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::AcpAgent {
        index: 0,
        field: AcpAgentField::Command,
    });
    state.editor_ui.settings_input_draft = "node server.js".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 500);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(caret_fills(&backend.fills, panel.theme.foreground).is_empty());
}

#[test]
fn hit_test_resolves_builtin_agent_compact_switch() {
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let content_y = rect.origin.y + 24.0;
    let first_card_y = content_y + 12.0 + 28.0 + 28.0;
    let point = crate::Point2D::new(content_x + content_w - 90.0, first_card_y + 30.0);

    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::ToggleBuiltinAgentEnabled(0)
    );
}

#[test]
fn hit_test_resolves_builtin_agent_compact_edit_button() {
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let content_y = rect.origin.y + 24.0;
    let first_card_y = content_y + 12.0 + 28.0 + 28.0;
    let point = crate::Point2D::new(content_x + content_w - 52.0, first_card_y + 30.0);

    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::EditBuiltinAgent(0)
    );
}

#[test]
fn mcp_port_field_is_not_focusable_while_server_is_running() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.mcp_server.running = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let server_card_top = content_y + 36.0;
    let button_x = content_x + content_w - 16.0 - 72.0;
    let port_x = button_x - 8.0 - 64.0;
    let point = crate::Point2D::new(port_x + 32.0, server_card_top + 26.0);

    assert_eq!(panel.hit_test(rect, point), AgentSettingsHit::Inside);
}

#[test]
fn mcp_running_server_exposes_client_config_height() {
    let mut stopped_state = EditorState::default();
    stopped_state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    let stopped = AgentSettingsPanel::for_editor(&stopped_state).content_total_height();

    let mut running_state = EditorState::default();
    running_state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    running_state.editor_ui.agent_settings.mcp_server.running = true;
    let running = AgentSettingsPanel::for_editor(&running_state).content_total_height();

    assert!(
        running > stopped,
        "running MCP server should reserve space for the HTTP client config row"
    );
}

#[test]
fn focused_mcp_port_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    state.editor_ui.settings_input_draft = "3845".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 100);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert_eq!(caret_fills(&backend.fills, panel.theme.foreground).len(), 1);
}

#[test]
fn focused_mcp_port_hides_caret_at_blink_off_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    state.editor_ui.settings_input_draft = "3845".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 500);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(caret_fills(&backend.fills, panel.theme.foreground).is_empty());
}

#[test]
fn system_auto_update_switch_has_click_target() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::System;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let card_y = content_y + 12.0 + 36.0;
    let point = crate::Point2D::new(content_x + content_w - 28.0, card_y + 28.0);

    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::ToggleAutoUpdate
    );
}

#[test]
fn images_tab_profile_rows_expose_active_and_remove_targets() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;

    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0;

    assert_eq!(
        panel.hit_test(rect, crate::Point2D::new(content_x + 15.0, row_y + 16.0)),
        AgentSettingsHit::SetActiveGenConfig(0)
    );
    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + content_w - 12.0, row_y + 16.0)
        ),
        AgentSettingsHit::RemoveGenConfig(0)
    );
}

#[test]
fn images_tab_advanced_search_fields_are_focusable() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.images_advanced_open = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let field_x = content_x + 110.0 + 16.0;
    let first_field_y = content_y + 36.0 + 24.0 + 22.0;

    assert_eq!(
        panel.hit_test(rect, crate::Point2D::new(field_x, first_field_y + 18.0)),
        AgentSettingsHit::FocusSearchField(ImageSearchField::ClientId)
    );
    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(field_x, first_field_y + 36.0 + 10.0 + 18.0)
        ),
        AgentSettingsHit::FocusSearchField(ImageSearchField::ClientSecret)
    );
}

#[test]
fn focused_image_search_field_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.images_advanced_open = true;
    state.editor_ui.agent_settings.focus =
        Some(SettingsFocus::ImageSearch(ImageSearchField::ClientId));
    state.editor_ui.settings_input_draft = "openverse-client".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 100);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert_eq!(caret_fills(&backend.fills, panel.theme.foreground).len(), 1);
}

#[test]
fn focused_image_search_field_hides_caret_at_blink_off_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.images_advanced_open = true;
    state.editor_ui.agent_settings.focus =
        Some(SettingsFocus::ImageSearch(ImageSearchField::ClientId));
    state.editor_ui.settings_input_draft = "openverse-client".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 500);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(caret_fills(&backend.fills, panel.theme.foreground).is_empty());
}

#[test]
fn images_tab_test_search_requires_some_oauth_text() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.images_advanced_open = true;
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let button_x = rect.origin.x + 200.0 + 24.0 + content_w - 28.0;
    let button_y = content_y + 36.0 + 24.0 + 22.0 + 36.0 + 10.0 + 36.0 + 14.0 + 18.0;

    assert_eq!(
        panel.hit_test(rect, crate::Point2D::new(button_x, button_y)),
        AgentSettingsHit::Inside
    );

    state.editor_ui.agent_settings.openverse_client_id = "client".into();
    let panel = AgentSettingsPanel::for_editor(&state);
    assert_eq!(
        panel.hit_test(rect, crate::Point2D::new(button_x, button_y)),
        AgentSettingsHit::TestImageSearch
    );
}

#[test]
fn images_tab_content_height_includes_profile_rows() {
    let mut empty = EditorState::default();
    empty.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    let empty_h = AgentSettingsPanel::for_editor(&empty).content_total_height();

    let mut with_profiles = EditorState::default();
    with_profiles.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    with_profiles
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    with_profiles
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    with_profiles
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    let profiles_h = AgentSettingsPanel::for_editor(&with_profiles).content_total_height();

    assert!(
        profiles_h > empty_h,
        "configured image generation profiles should replace the TS empty state with rows"
    );
}

#[test]
fn focused_image_gen_field_paints_visible_caret_at_blink_on_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::BaseUrl,
    });
    state.editor_ui.settings_input_draft = "https://api.example.com/v1".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 100);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert_eq!(caret_fills(&backend.fills, panel.theme.foreground).len(), 1);
}

#[test]
fn focused_image_gen_field_hides_caret_at_blink_off_phase() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::BaseUrl,
    });
    state.editor_ui.settings_input_draft = "https://api.example.com/v1".into();

    let panel = AgentSettingsPanel::for_editor_at(&state, 500);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(caret_fills(&backend.fills, panel.theme.foreground).is_empty());
}

#[test]
fn images_tab_expanded_profile_fields_are_focusable() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;

    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0;
    let api_field_y = row_y + 32.0 + 8.0 + 36.0 * 2.0;

    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + 110.0 + 20.0, api_field_y + 12.0)
        ),
        AgentSettingsHit::FocusGenConfig {
            index: 0,
            field: ImageGenField::ApiKey,
        }
    );
    assert!(
        panel.content_total_height() > 180.0,
        "focused image profile should expand to show editable fields"
    );
}

#[test]
fn images_tab_expanded_profile_provider_row_is_clickable() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0;
    let provider_y = row_y + 32.0 + 8.0 + 36.0;

    assert_eq!(
        panel.hit_test(
            rect,
            crate::Point2D::new(content_x + 110.0 + 20.0, provider_y + 12.0)
        ),
        AgentSettingsHit::CycleGenProvider(0)
    );
}
