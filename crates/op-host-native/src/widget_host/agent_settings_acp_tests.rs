use super::WidgetHostNative;
use op_editor_core::agent_settings::{AcpAgentField, SettingsFocus};
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;

fn agent_settings_content_metrics(host: &WidgetHostNative) -> (f32, f32, f32) {
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    (
        rect.origin.x + 200.0 + 24.0,
        rect.origin.y + 24.0,
        rect.size.x - 200.0 - 48.0,
    )
}

fn acp_card_y(content_y: f32) -> f32 {
    content_y + 12.0 + 120.0 + 28.0 + 28.0 + 28.0
}

#[test]
fn acp_agent_command_field_accepts_text_and_commits() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_acp_agent();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .hover_acp_agent = 0;

    let (content_x, content_y, content_w) = agent_settings_content_metrics(&host);
    let card_y = acp_card_y(content_y);
    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 156.0,
        card_y + 30.0,
        1200.0,
        800.0
    ));

    assert!(host.dispatch_agent_settings_press(
        content_x + 92.0,
        card_y + 154.0 + 14.0,
        1200.0,
        800.0
    ));
    for c in "op-agent".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(settings.acp_agents[0].command, "op-agent");
    assert!(settings.focus.is_none());
    assert!(host
        .editor_state()
        .editor_ui
        .settings_input_draft
        .is_empty());
}

#[test]
fn acp_agent_args_field_commits_comma_separated_args() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_acp_agent();
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::AcpAgent {
        index: 0,
        field: AcpAgentField::Args,
    });
    host.editor_state_mut().editor_ui.settings_input_draft =
        " --stdio, --workspace /tmp, , --verbose ".into();

    assert!(host.apply_send());

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(
        settings.acp_agents[0].args,
        vec!["--stdio", "--workspace /tmp", "--verbose"]
    );
    assert!(settings.focus.is_none());
    assert!(host
        .editor_state()
        .editor_ui
        .settings_input_draft
        .is_empty());
}

#[test]
fn acp_agent_remove_press_deletes_agent_and_clears_focus() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_acp_agent();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .hover_acp_agent = 0;

    let (content_x, content_y, content_w) = agent_settings_content_metrics(&host);
    let card_y = acp_card_y(content_y);
    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 128.0,
        card_y + 30.0,
        1200.0,
        800.0
    ));

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert!(settings.acp_agents.is_empty());
    assert!(settings.focus.is_none());
    assert!(host
        .editor_state()
        .editor_ui
        .settings_input_draft
        .is_empty());
}

#[test]
fn acp_agent_connect_press_toggles_connected_state() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_acp_agent();
    host.editor_state_mut().editor_ui.agent_settings.acp_agents[0].command = "op-agent".into();

    let (content_x, content_y, content_w) = agent_settings_content_metrics(&host);
    let card_y = acp_card_y(content_y);
    let button_x = content_x + content_w - 60.0;
    assert!(host.dispatch_agent_settings_press(button_x, card_y + 30.0, 1200.0, 800.0));
    assert!(
        host.editor_state().editor_ui.agent_settings.acp_agents[0].connected,
        "configured local ACP agent should become connected after pressing Connect"
    );

    assert!(host.dispatch_agent_settings_press(button_x, card_y + 30.0, 1200.0, 800.0));
    assert!(
        !host.editor_state().editor_ui.agent_settings.acp_agents[0].connected,
        "connected ACP agent should become disconnected after pressing Disconnect"
    );
}
