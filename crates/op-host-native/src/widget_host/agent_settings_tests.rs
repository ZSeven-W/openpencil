use super::WidgetHostNative;
use op_editor_core::agent_settings::{
    AgentSettingsTab, BuiltinAgentField, ImageGenField, ImageGenProvider, ImageSearchField,
    SettingsFocus,
};
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;

#[test]
fn builtin_agent_api_key_focus_accepts_text_and_rebuilds_models() {
    let mut host = WidgetHostNative::new();
    let id = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent();
    host.editor_state_mut().rebuild_chat_models();
    assert!(
        !host
            .editor_state()
            .chat
            .available_models
            .iter()
            .any(|m| m.builtin_provider_id.as_deref() == Some(id.as_str())),
        "empty-key built-in agent must not be selectable yet"
    );

    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });
    host.editor_state_mut()
        .editor_ui
        .settings_input_draft
        .clear();
    for c in "sk-test".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());

    let state = host.editor_state();
    assert_eq!(
        state.editor_ui.agent_settings.builtin_agents[0].api_key,
        "sk-test"
    );
    assert!(state.editor_ui.agent_settings.focus.is_none());
    assert!(
        state
            .chat
            .available_models
            .iter()
            .any(|m| m.builtin_provider_id.as_deref() == Some(id.as_str())),
        "committing a ready built-in agent should add it to the chat model list"
    );
}

#[test]
fn builtin_agent_compact_switch_toggles_enabled_and_models() {
    let mut host = WidgetHostNative::new();
    let id = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    host.editor_state_mut().rebuild_chat_models();
    assert!(host
        .editor_state()
        .chat
        .available_models
        .iter()
        .any(|m| m.builtin_provider_id.as_deref() == Some(id.as_str())));

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_w = rect.size.x - 200.0 - 48.0;
    let x = rect.origin.x + 200.0 + 24.0 + content_w - 90.0;
    let y = rect.origin.y + 24.0 + 12.0 + 28.0 + 28.0 + 30.0;
    assert!(host.dispatch_agent_settings_press(x, y, 1200.0, 800.0));

    let state = host.editor_state();
    assert!(!state.editor_ui.agent_settings.builtin_agents[0].enabled);
    assert!(!state
        .chat
        .available_models
        .iter()
        .any(|m| m.builtin_provider_id.as_deref() == Some(id.as_str())));
}

#[test]
fn builtin_agent_compact_edit_focuses_display_name_form() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_w = rect.size.x - 200.0 - 48.0;
    let x = rect.origin.x + 200.0 + 24.0 + content_w - 52.0;
    let y = rect.origin.y + 24.0 + 12.0 + 28.0 + 28.0 + 30.0;
    assert!(host.dispatch_agent_settings_press(x, y, 1200.0, 800.0));

    assert_eq!(
        host.editor_state().editor_ui.agent_settings.focus,
        Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::DisplayName,
        })
    );
    assert_eq!(
        host.editor_state().editor_ui.settings_input_draft,
        "MiniMax"
    );
}

#[test]
fn starting_mcp_server_commits_port_draft_and_clears_focus() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Mcp;
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    host.editor_state_mut().editor_ui.settings_input_draft = "3101".into();

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let server_card_top = content_y + 36.0;
    let button_x = content_x + content_w - 16.0 - 72.0;
    assert!(host.dispatch_agent_settings_press(
        button_x + 36.0,
        server_card_top + 26.0,
        1200.0,
        800.0
    ));

    let state = host.editor_state();
    assert!(state.editor_ui.agent_settings.mcp_server.running);
    assert_eq!(state.editor_ui.agent_settings.mcp_server.port, 3101);
    assert!(state.editor_ui.agent_settings.focus.is_none());
    assert!(state.editor_ui.settings_input_draft.is_empty());
}

#[test]
fn system_auto_update_switch_toggles_preference() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::System;
    assert!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .auto_update_enabled
    );

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let card_y = content_y + 12.0 + 36.0;
    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 28.0,
        card_y + 28.0,
        1200.0,
        800.0
    ));

    assert!(
        !host
            .editor_state()
            .editor_ui
            .agent_settings
            .auto_update_enabled
    );
}

#[test]
fn image_generation_profile_buttons_add_activate_and_remove() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Images;

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let gen_top = content_y + 36.0 + 24.0 + 28.0;

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 36.0,
        gen_top + 14.0,
        1200.0,
        800.0
    ));
    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 36.0,
        gen_top + 14.0,
        1200.0,
        800.0
    ));

    let first = host
        .editor_state()
        .editor_ui
        .agent_settings
        .image_gen_profiles[0]
        .id
        .clone();
    let second = host
        .editor_state()
        .editor_ui
        .agent_settings
        .image_gen_profiles[1]
        .id
        .clone();
    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .active_image_gen_profile_id
            .as_deref(),
        Some(first.as_str())
    );

    let second_row_y = gen_top + 36.0 + 32.0 + 6.0;
    assert!(host.dispatch_agent_settings_press(
        content_x + 15.0,
        second_row_y + 16.0,
        1200.0,
        800.0
    ));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .active_image_gen_profile_id
            .as_deref(),
        Some(second.as_str())
    );

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 12.0,
        second_row_y + 16.0,
        1200.0,
        800.0
    ));
    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(settings.image_gen_profiles.len(), 1);
    assert_eq!(
        settings.active_image_gen_profile_id.as_deref(),
        Some(first.as_str())
    );
}

#[test]
fn image_generation_profile_focus_accepts_text_and_commits() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });
    host.editor_state_mut()
        .editor_ui
        .settings_input_draft
        .clear();

    for c in "Hero Images".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(settings.image_gen_profiles[0].name, "Hero Images");
    assert!(settings.focus.is_none());
    assert!(host
        .editor_state()
        .editor_ui
        .settings_input_draft
        .is_empty());
}

#[test]
fn image_generation_provider_click_cycles_provider_and_clears_model() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .image_gen_profiles[0]
        .model = "dall-e-3".into();
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
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

    let profile = &host
        .editor_state()
        .editor_ui
        .agent_settings
        .image_gen_profiles[0];
    assert_eq!(profile.provider, ImageGenProvider::Gemini);
    assert!(profile.model.is_empty());
    assert!(host.editor_state().editor_ui.agent_settings.focus.is_none());
}

#[test]
fn image_search_oauth_focus_accepts_text_and_commits() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    host.editor_state_mut().editor_ui.agent_settings.focus =
        Some(SettingsFocus::ImageSearch(ImageSearchField::ClientId));
    host.editor_state_mut()
        .editor_ui
        .settings_input_draft
        .clear();

    for c in "openverse-client".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());

    host.editor_state_mut().editor_ui.agent_settings.focus =
        Some(SettingsFocus::ImageSearch(ImageSearchField::ClientSecret));
    host.editor_state_mut()
        .editor_ui
        .settings_input_draft
        .clear();
    for c in "openverse-secret".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(settings.openverse_client_id, "openverse-client");
    assert_eq!(settings.openverse_client_secret, "openverse-secret");
    assert!(settings.focus.is_none());
    assert!(host
        .editor_state()
        .editor_ui
        .settings_input_draft
        .is_empty());
}

#[test]
fn image_search_test_updates_ready_status_from_oauth_completeness() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .images_advanced_open = true;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .openverse_client_id = "client".into();

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let x = rect.origin.x + 200.0 + 24.0 + content_w - 28.0;
    let y = content_y + 36.0 + 24.0 + 22.0 + 36.0 + 10.0 + 36.0 + 14.0 + 18.0;

    assert!(host.dispatch_agent_settings_press(x, y, 1200.0, 800.0));
    assert!(
        !host
            .editor_state()
            .editor_ui
            .agent_settings
            .images_search_ready
    );

    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .openverse_client_secret = "secret".into();
    assert!(host.dispatch_agent_settings_press(x, y, 1200.0, 800.0));
    assert!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .images_search_ready
    );
}
