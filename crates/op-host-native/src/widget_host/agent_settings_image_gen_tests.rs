use super::WidgetHostNative;
use op_editor_core::agent_settings::{
    AgentSettingsTab, ImageGenField, ImageGenProvider, ImageTestStatus, SettingsFocus,
};
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;

#[test]
fn image_generation_profile_test_tracks_testing_status_like_ts() {
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
        .api_key = "sk-test".into();
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0 + 8.0;
    let api_field_y = row_y + 32.0 + 8.0 + 36.0 * 2.0;

    assert!(host.dispatch_agent_settings_press(
        content_x + 430.0,
        api_field_y + 12.0,
        1200.0,
        800.0
    ));

    let profile = &host
        .editor_state()
        .editor_ui
        .agent_settings
        .image_gen_profiles[0];
    assert_eq!(profile.test_status, ImageTestStatus::Testing);
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::ImageProfileTest(0)
        ))
    );

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn image_generation_profile_hover_tracks_controls_and_close() {
    let mut host = WidgetHostNative::new();
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    host.editor_state_mut().editor_ui.agent_settings.focus = Some(SettingsFocus::ImageGenProfile {
        index: 0,
        field: ImageGenField::Name,
    });

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(1200.0, 800.0);
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let content_w = rect.size.x - 200.0 - 48.0;
    let row_x = content_x + 8.0;
    let row_w = content_w - 16.0;
    let gen_top = content_y + 36.0 + 24.0 + 28.0;
    let row_y = gen_top + 36.0 + 8.0;

    assert!(
        host.update_agent_settings_hover(rect.origin.x + rect.size.x - 24.0, rect.origin.y + 24.0)
    );
    assert!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .hover_agent_settings_close
    );

    assert!(host.update_agent_settings_hover(row_x + 160.0, row_y + 16.0));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .hover_image_gen_profile_header,
        Some(0)
    );

    assert!(host.update_agent_settings_hover(row_x + row_w - 12.0, row_y + 16.0));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .hover_image_gen_profile_remove,
        Some(0)
    );

    assert!(host.update_agent_settings_hover(row_x + 130.0, row_y + 40.0 + 36.0 + 12.0));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .hover_image_gen_profile_provider,
        Some(0)
    );

    assert!(
        host.update_agent_settings_hover(row_x + row_w - 12.0 - 28.0, row_y + 40.0 + 72.0 + 12.0)
    );
    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .hover_image_gen_profile_test,
        Some(0)
    );

    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .image_gen_provider_menu_open = Some(0);
    assert!(host.update_agent_settings_hover(row_x + 130.0, row_y + 40.0 + 36.0 + 24.0 + 36.0));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .hover_image_gen_provider_option,
        Some((0, ImageGenProvider::Gemini))
    );
}
