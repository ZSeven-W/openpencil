use crate::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
use op_editor_core::agent_settings::{AgentSettingsTab, BuiltinAgentField, SettingsFocus};
use op_editor_core::EditorState;

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
    let point = crate::Point2D::new(content_x + 92.0, first_card_y + 88.0);

    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::FocusBuiltinAgent {
            index: 0,
            field: BuiltinAgentField::ApiKey,
        }
    );
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
