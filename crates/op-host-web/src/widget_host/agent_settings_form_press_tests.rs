use super::WidgetHost;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};
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

fn builtin_draft_card_y(content_y: f32) -> f32 {
    content_y + 12.0 + 28.0 + 28.0
}

fn acp_draft_card_y(content_y: f32) -> f32 {
    content_y + 12.0 + 28.0 + 28.0 + 64.0 + 28.0 + 28.0 + 28.0
}

#[test]
fn builtin_draft_save_press_sets_and_release_clears_agent_settings_button() {
    let mut host = WidgetHost::new();
    host.editor_state
        .editor_ui
        .agent_settings
        .begin_builtin_agent_draft();
    host.editor_state
        .editor_ui
        .agent_settings
        .builtin_agent_draft
        .as_mut()
        .expect("draft should exist")
        .api_key = "sk-test".into();
    let (content_x, content_y, content_w) = content_metrics(&host);

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 12.0 - 34.0,
        builtin_draft_card_y(content_y) + 196.0 + 18.0,
        1200.0,
        800.0,
    ));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::BuiltinSaveDraft
        ))
    );

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn builtin_draft_cancel_press_sets_and_release_clears_agent_settings_button() {
    let mut host = WidgetHost::new();
    host.editor_state
        .editor_ui
        .agent_settings
        .begin_builtin_agent_draft();
    let (content_x, content_y, content_w) = content_metrics(&host);

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 12.0 - 68.0 - 8.0 - 34.0,
        builtin_draft_card_y(content_y) + 196.0 + 18.0,
        1200.0,
        800.0,
    ));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::BuiltinCancelDraft
        ))
    );

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn web_acp_draft_save_control_is_hidden() {
    let mut host = WidgetHost::new();
    host.editor_state
        .editor_ui
        .agent_settings
        .begin_acp_agent_draft();
    host.editor_state
        .editor_ui
        .agent_settings
        .acp_agent_draft
        .as_mut()
        .expect("draft should exist")
        .command = "op-agent".into();
    let (content_x, content_y, content_w) = content_metrics(&host);

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 12.0 - 34.0,
        acp_draft_card_y(content_y) + 332.0 + 18.0,
        1200.0,
        800.0,
    ));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
    assert!(host
        .editor_state
        .editor_ui
        .agent_settings
        .acp_agent_draft
        .is_some());
    assert!(host
        .editor_state
        .editor_ui
        .agent_settings
        .acp_agents
        .is_empty());
}

#[test]
fn web_acp_draft_cancel_control_is_hidden() {
    let mut host = WidgetHost::new();
    host.editor_state
        .editor_ui
        .agent_settings
        .begin_acp_agent_draft();
    let (content_x, content_y, content_w) = content_metrics(&host);

    assert!(host.dispatch_agent_settings_press(
        content_x + content_w - 12.0 - 68.0 - 8.0 - 34.0,
        acp_draft_card_y(content_y) + 332.0 + 18.0,
        1200.0,
        800.0,
    ));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
    assert!(host
        .editor_state
        .editor_ui
        .agent_settings
        .acp_agent_draft
        .is_some());
}
