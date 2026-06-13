use super::WidgetHost;
use op_editor_core::agent_settings::{AcpAgentField, BuiltinAgentField, SettingsFocus};
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;

fn content_metrics(host: &WidgetHost) -> (f32, f32, f32) {
    let panel = AgentSettingsPanel::for_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    (
        rect.origin.x + 200.0 + 24.0,
        rect.origin.y + 24.0,
        rect.size.x - 200.0 - 48.0,
    )
}

fn builtin_card_y(content_y: f32) -> f32 {
    content_y + 12.0 + 28.0 + 28.0
}

fn acp_card_y(content_y: f32) -> f32 {
    content_y + 12.0 + 28.0 + 28.0 + 64.0 + 28.0 + 28.0 + 28.0
}

#[test]
fn builtin_compact_edit_press_sets_and_release_clears_agent_settings_button() {
    let mut host = WidgetHost::new();
    host.editor_state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    host.editor_state
        .editor_ui
        .agent_settings
        .hover_builtin_agent = 0;
    let (content_x, content_y, content_w) = content_metrics(&host);

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 52.0,
        builtin_card_y(content_y) + 30.0,
        1200.0,
        800.0,
    ));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::BuiltinEdit(0)
        ))
    );
    assert_eq!(
        host.editor_state.editor_ui.agent_settings.focus,
        Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::DisplayName,
        })
    );

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn builtin_compact_remove_press_sets_and_release_clears_agent_settings_button() {
    let mut host = WidgetHost::new();
    host.editor_state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("MiniMax", "sk-test", "MiniMax-M2.7");
    host.editor_state
        .editor_ui
        .agent_settings
        .hover_builtin_agent = 0;
    let (content_x, content_y, content_w) = content_metrics(&host);

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 24.0,
        builtin_card_y(content_y) + 30.0,
        1200.0,
        800.0,
    ));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::BuiltinRemove(0)
        ))
    );
    assert!(host
        .editor_state
        .editor_ui
        .agent_settings
        .builtin_agents
        .is_empty());

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn acp_compact_edit_press_sets_and_release_clears_agent_settings_button() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.agent_settings.add_acp_agent();
    host.editor_state.editor_ui.agent_settings.hover_acp_agent = 0;
    let (content_x, content_y, content_w) = content_metrics(&host);

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 156.0,
        acp_card_y(content_y) + 30.0,
        1200.0,
        800.0,
    ));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::AcpEdit(0)
        ))
    );
    assert_eq!(
        host.editor_state.editor_ui.agent_settings.focus,
        Some(SettingsFocus::AcpAgent {
            index: 0,
            field: AcpAgentField::DisplayName,
        })
    );

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn acp_compact_remove_press_sets_and_release_clears_agent_settings_button() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.agent_settings.add_acp_agent();
    host.editor_state.editor_ui.agent_settings.hover_acp_agent = 0;
    let (content_x, content_y, content_w) = content_metrics(&host);

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 128.0,
        acp_card_y(content_y) + 30.0,
        1200.0,
        800.0,
    ));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::AcpRemove(0)
        ))
    );
    assert!(host
        .editor_state
        .editor_ui
        .agent_settings
        .acp_agents
        .is_empty());

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}
