use super::WidgetHost;
use op_editor_core::agent_settings::{
    AgentSettingsTab, BuiltinAgentField, ImageGenField, ImageGenProvider, ImageTestStatus,
    SettingsFocus,
};
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;

#[test]
fn toggling_builtin_kind_commits_focused_api_key_draft() {
    let mut host = WidgetHost::new();
    host.editor_state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MINIMAX", "", "MiniMax-M2.7");
    host.editor_state
        .editor_ui
        .agent_settings
        .set_builtin_agent_preset(0, op_editor_core::BuiltinAgentPresetKey::MiniMax);
    host.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });
    host.editor_state.editor_ui.settings_input_draft = "sk-web".into();

    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let first_card_y = content_y + 12.0 + 28.0 + 28.0;
    let kind_x = content_x + content_w - 172.0 + 120.0;
    let kind_y = first_card_y + 22.0;

    assert!(host.dispatch_agent_settings_press(kind_x, kind_y, 1200.0, 800.0));

    let agent = &host.editor_state.editor_ui.agent_settings.builtin_agents[0];
    assert_eq!(agent.api_key, "sk-web");
    assert_eq!(
        agent.kind,
        op_editor_core::agent_settings::BuiltinAgentKind::OpenAiCompat
    );
    assert!(host.editor_state.editor_ui.agent_settings.focus.is_none());
}

#[test]
fn add_provider_opens_unsaved_builtin_agent_draft() {
    let mut host = WidgetHost::new();
    host.set_now_ms(1234);

    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let add_x = content_x + content_w - 48.0;
    let add_y = content_y + 24.0;

    assert!(host.dispatch_agent_settings_press(add_x, add_y, 1200.0, 800.0));

    let settings = &host.editor_state.editor_ui.agent_settings;
    assert!(settings.builtin_agents.is_empty());
    assert!(settings.builtin_agent_draft.is_some());
    assert_eq!(
        settings.focus,
        Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey))
    );
    assert_eq!(host.editor_state.editor_ui.settings_input_draft, "");
    assert_eq!(
        host.editor_state.editor_ui.settings_input_caret_anchor_ms,
        1234
    );
}

#[test]
fn builtin_provider_menu_selects_ts_preset_for_draft() {
    let mut host = WidgetHost::new();
    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let add_x = content_x + content_w - 48.0;
    let add_y = content_y + 24.0;

    assert!(host.dispatch_agent_settings_press(add_x, add_y, 1200.0, 800.0));
    let card_y = content_y + 12.0 + 28.0 + 28.0;
    let provider_x = content_x + 68.0 + 24.0;
    let provider_y = card_y + 60.0;
    assert!(host.dispatch_agent_settings_press(provider_x, provider_y, 1200.0, 800.0));
    let minimax_y = card_y + 76.0 + 4.0 + 5.0 * 24.0 + 12.0;
    assert!(host.dispatch_agent_settings_press(provider_x, minimax_y, 1200.0, 800.0));

    let draft = host
        .editor_state
        .editor_ui
        .agent_settings
        .builtin_agent_draft
        .as_ref()
        .expect("draft remains open");
    assert_eq!(draft.preset, op_editor_core::BuiltinAgentPresetKey::MiniMax);
    assert_eq!(draft.display_name, "MiniMax");
    assert_eq!(draft.model, "MiniMax-M2.7");
    assert_eq!(draft.base_url, "https://api.minimaxi.com/anthropic");
}

#[test]
fn save_builtin_agent_draft_persists_provider() {
    let mut host = WidgetHost::new();
    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let add_x = content_x + content_w - 48.0;
    let add_y = content_y + 24.0;

    assert!(host.dispatch_agent_settings_press(add_x, add_y, 1200.0, 800.0));
    for c in "sk-web".chars() {
        assert!(host.apply_text(c));
    }
    let card_y = content_y + 12.0 + 28.0 + 28.0;
    let save_x = content_x + content_w - 12.0 - 34.0;
    let save_y = card_y + 196.0 + 18.0;
    assert!(host.dispatch_agent_settings_press(save_x, save_y, 1200.0, 800.0));

    let settings = &host.editor_state.editor_ui.agent_settings;
    assert_eq!(settings.builtin_agents.len(), 1);
    assert!(settings.builtin_agent_draft.is_none());
    assert_eq!(settings.builtin_agents[0].api_key, "sk-web");
    assert!(settings.focus.is_none());
}

#[test]
fn image_generation_profile_test_tracks_testing_status_like_ts() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    host.editor_state
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    host.editor_state
        .editor_ui
        .agent_settings
        .image_gen_profiles[0]
        .api_key = "sk-test".into();
    host.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });

    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0;
    let api_field_y = row_y + 32.0 + 8.0 + 36.0 * 2.0;

    assert!(host.dispatch_agent_settings_press(
        content_x + 430.0,
        api_field_y + 12.0,
        1200.0,
        800.0
    ));

    let profile = &host
        .editor_state
        .editor_ui
        .agent_settings
        .image_gen_profiles[0];
    assert_eq!(profile.test_status, ImageTestStatus::Testing);
}

#[test]
fn image_generation_provider_select_commits_and_closes_menu() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    host.editor_state
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    host.editor_state
        .editor_ui
        .agent_settings
        .image_gen_profiles[0]
        .model = "dall-e-3".into();
    host.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });

    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0;
    let provider_y = row_y + 32.0 + 8.0 + 36.0;

    assert!(host.dispatch_agent_settings_press(
        content_x + 110.0 + 20.0,
        provider_y + 12.0,
        1200.0,
        800.0
    ));
    assert_eq!(
        host.editor_state
            .editor_ui
            .agent_settings
            .image_gen_provider_menu_open,
        Some(0)
    );

    assert!(host.dispatch_agent_settings_press(
        content_x + 110.0 + 20.0,
        provider_y + 60.0,
        1200.0,
        800.0
    ));

    let settings = &host.editor_state.editor_ui.agent_settings;
    let profile = &settings.image_gen_profiles[0];
    assert_eq!(profile.provider, ImageGenProvider::Gemini);
    assert!(profile.model.is_empty());
    assert!(settings.image_gen_provider_menu_open.is_none());
}

#[test]
fn image_generation_profile_header_click_toggles_editor_closed() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    host.editor_state
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    host.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });
    host.editor_state.editor_ui.settings_input_draft = "Config 1".into();

    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0;

    assert!(host.dispatch_agent_settings_press(content_x + 72.0, row_y + 16.0, 1200.0, 800.0));

    assert_eq!(host.editor_state.editor_ui.agent_settings.focus, None);
    assert!(host.editor_state.editor_ui.settings_input_draft.is_empty());
}

#[test]
fn copying_mcp_client_config_records_feedback_time() {
    let mut host = WidgetHost::new();
    host.set_now_ms(4_321);
    host.editor_state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    host.editor_state
        .editor_ui
        .agent_settings
        .mcp_server
        .running = true;

    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let client_config_y = content_y + 36.0 + 52.0 + 8.0;

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 22.0,
        client_config_y + 18.0,
        1200.0,
        800.0
    ));

    assert_eq!(
        host.editor_state
            .editor_ui
            .agent_settings
            .mcp_client_config_copied_at_ms,
        Some(4_321)
    );
    assert_eq!(
        host.editor_state.chat.pending_copy_text.as_deref(),
        Some("{\n  \"type\": \"http\",\n  \"url\": \"http://127.0.0.1:3100/mcp\"\n}")
    );
}

#[test]
fn mcp_server_button_hover_tracks_cursor() {
    let mut host = WidgetHost::new();
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state.editor_ui.agent_settings_open = true;
    host.editor_state.editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    host.editor_state
        .editor_ui
        .agent_settings
        .mcp_server
        .running = true;

    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let server_card_y = content_y + 36.0;
    let button_x = content_x + content_w - 16.0 - 72.0;

    assert!(host.update_agent_settings_hover(button_x + 36.0, server_card_y + 26.0,));
    assert!(
        host.editor_state
            .editor_ui
            .agent_settings
            .hover_mcp_server_button
    );
}

#[test]
fn image_settings_button_hover_tracks_cursor() {
    let mut host = WidgetHost::new();
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state.editor_ui.agent_settings_open = true;
    host.editor_state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    host.editor_state
        .editor_ui
        .agent_settings
        .images_advanced_open = true;

    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;

    assert!(host.update_agent_settings_hover(content_x + content_w - 28.0, content_y + 196.0,));
    assert!(
        host.editor_state
            .editor_ui
            .agent_settings
            .hover_image_search_test_button
    );

    assert!(host.update_agent_settings_hover(content_x + content_w - 36.0, content_y + 260.0,));
    assert!(
        host.editor_state
            .editor_ui
            .agent_settings
            .hover_image_gen_add_button
    );
    assert!(
        !host
            .editor_state
            .editor_ui
            .agent_settings
            .hover_image_search_test_button
    );
}
