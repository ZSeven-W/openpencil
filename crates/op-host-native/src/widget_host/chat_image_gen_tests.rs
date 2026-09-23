//! Footer image-generation toggle pressed through the real dispatch path.

use super::WidgetHostNative;
use op_editor_core::AgentSettingsTab;
use op_editor_ui::widgets::AIChatPlaceholder;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn seed_profile(host: &mut WidgetHostNative) {
    let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
    settings.add_image_gen_profile();
    settings.image_gen_profiles[0].api_key = "sk-image".into();
}

fn toggle_center(host: &WidgetHostNative) -> op_editor_ui::Point2D {
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    let rect = host
        .ai_chat_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("chat panel visible");
    let panel = AIChatPlaceholder::from_editor(host.editor_state());
    let toggle = panel
        .footer_image_gen_rect(rect)
        .expect("image-gen toggle laid out");
    op_editor_ui::Point2D::new(
        toggle.origin.x + toggle.size.x / 2.0,
        toggle.origin.y + toggle.size.y / 2.0,
    )
}

#[test]
fn unconfigured_image_gen_press_opens_settings_on_the_images_tab() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let point = toggle_center(&host);

    assert!(host.apply_click(point.x, point.y, VIEWPORT_W, VIEWPORT_H));

    let state = host.editor_state();
    assert!(
        !state.editor_ui.agent_settings.image_gen_enabled,
        "an unconfigured press must not flip the flag"
    );
    assert!(state.editor_ui.agent_settings_open);
    assert_eq!(state.editor_ui.agent_settings.tab, AgentSettingsTab::Images);
}

#[test]
fn configured_image_gen_press_toggles_the_flag_in_place() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.enter_chat_tab();
    seed_profile(&mut host);
    let point = toggle_center(&host);

    assert!(host.apply_click(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .image_gen_enabled
    );
    assert!(!host.editor_state().editor_ui.agent_settings_open);

    assert!(host.apply_click(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(
        !host
            .editor_state()
            .editor_ui
            .agent_settings
            .image_gen_enabled
    );
}
