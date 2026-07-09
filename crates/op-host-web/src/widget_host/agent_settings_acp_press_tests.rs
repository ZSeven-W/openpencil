use super::WidgetHost;
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;

fn content_metrics(host: &WidgetHost) -> (f32, f32, f32) {
    let panel = AgentSettingsPanel::for_web_editor(&host.editor_state);
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
fn web_acp_agent_connect_control_is_hidden() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.agent_settings.add_acp_agent();
    host.editor_state.editor_ui.agent_settings.acp_agents[0].command = "op-agent".into();
    let (content_x, content_y, content_w) = content_metrics(&host);
    let button_x = content_x + content_w - 60.0;

    assert!(host.dispatch_agent_settings_press(
        button_x,
        acp_card_y(content_y) + 30.0,
        1200.0,
        800.0,
    ));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
    assert!(!host.editor_state.editor_ui.agent_settings.acp_agents[0].connected);
    assert_eq!(
        host.editor_state
            .editor_ui
            .agent_settings
            .pending_acp_agent_connect
            .as_deref(),
        None
    );
}
