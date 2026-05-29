use super::WidgetHostNative;
use op_editor_core::agent_settings::{BuiltinAgentField, SettingsFocus};
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
