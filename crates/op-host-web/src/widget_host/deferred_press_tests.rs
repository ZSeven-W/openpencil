use super::WidgetHost;
use op_editor_core::agent_settings::{
    AgentSettingsTab, ImageGenField, ImageGenProvider, SettingsFocus,
};
use op_editor_core::chat::{AgentProvider, ModelEntry};
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
use op_editor_ui::widgets::{
    ai_chat_model_picker, AIChatPlaceholder, PropertyPanel, PropertyPanelAction, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

fn seed_two_chat_models(host: &mut WidgetHost) {
    host.editor_state
        .chat
        .available_models
        .push(ModelEntry::new(AgentProvider::CodexCli, "gpt-5", "GPT-5"));
    host.editor_state
        .chat
        .available_models
        .push(ModelEntry::new(AgentProvider::CodexCli, "gpt-4", "GPT-4"));
}

#[test]
fn chat_model_row_press_defers_selection_until_release() {
    let mut host = WidgetHost::new();
    seed_two_chat_models(&mut host);
    host.editor_state.editor_ui.chat_model_picker.open = true;
    let chat_rect = host.ai_chat_rect(1200.0, 800.0).unwrap();
    let panel = AIChatPlaceholder::from_editor(&host.editor_state);
    let picker = panel.model_picker_bounds(chat_rect).unwrap();
    let row_y = picker.origin.y
        + ai_chat_model_picker::MODEL_SEARCH_H
        + ai_chat_model_picker::MODEL_PICKER_PAD_Y
        + ai_chat_model_picker::MODEL_GROUP_H
        + ai_chat_model_picker::MODEL_ROW_H
        + ai_chat_model_picker::MODEL_ROW_H / 2.0;

    assert!(host.apply_click(picker.origin.x + 24.0, row_y, 1200.0, 800.0));

    assert_eq!(host.editor_state.chat.selected_model, 0);
    assert!(host.editor_state.editor_ui.chat_model_picker.open);
    assert_eq!(
        host.editor_state.editor_ui.chat_model_picker.pressed,
        Some(1)
    );

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state.chat.selected_model, 1);
    assert!(!host.editor_state.editor_ui.chat_model_picker.open);
    assert_eq!(host.editor_state.editor_ui.chat_model_picker.pressed, None);
}

#[test]
fn image_provider_option_press_defers_selection_until_release() {
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
        .provider = ImageGenProvider::OpenAi;
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
    let row_y = gen_top + 36.0 + 8.0;
    let provider_y = row_y + 32.0 + 8.0 + 36.0;

    assert!(host.dispatch_agent_settings_press(
        content_x + 110.0 + 20.0,
        provider_y + 12.0,
        1200.0,
        800.0
    ));
    assert!(host.apply_release_with_viewport(1200.0, 800.0));

    assert!(host.dispatch_agent_settings_press(
        content_x + 110.0 + 20.0,
        provider_y + 60.0,
        1200.0,
        800.0
    ));

    let settings = &host.editor_state.editor_ui.agent_settings;
    assert_eq!(
        settings.image_gen_profiles[0].provider,
        ImageGenProvider::OpenAi
    );
    assert_eq!(settings.image_gen_profiles[0].model, "dall-e-3");
    assert_eq!(settings.image_gen_provider_menu_open, Some(0));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::ImageProviderOption {
                index: 0,
                provider: ImageGenProvider::Gemini,
            },
        ))
    );

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    let settings = &host.editor_state.editor_ui.agent_settings;
    assert_eq!(
        settings.image_gen_profiles[0].provider,
        ImageGenProvider::Gemini
    );
    assert!(settings.image_gen_profiles[0].model.is_empty());
    assert!(settings.image_gen_provider_menu_open.is_none());
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn font_weight_row_press_defers_selection_until_release() {
    let mut host = WidgetHost::new();
    host.editor_state = op_editor_core::EditorState::sample();
    host.editor_state.editor_ui.font_weight_picker_open = true;
    let property_rect = Rect {
        origin: Point2D::new(
            1200.0 - host.editor_state.editor_ui.property_panel_width,
            TOP_BAR_HEIGHT,
        ),
        size: Point2D::new(
            host.editor_state.editor_ui.property_panel_width,
            800.0 - TOP_BAR_HEIGHT,
        ),
    };
    let panel = PropertyPanel::for_selection(&host.editor_state).unwrap();
    let before_weight = selected_font_weight(&host.editor_state);
    let (point, choice) = find_font_weight_action_point(&panel, property_rect, before_weight);

    assert!(host.apply_press(point.x, point.y, 1200.0, 800.0));

    assert_eq!(selected_font_weight(&host.editor_state), before_weight);
    assert!(host.editor_state.editor_ui.font_weight_picker_open);
    assert!(matches!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::FontWeightPicker(_))
    ));

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert!(!host.editor_state.editor_ui.font_weight_picker_open);
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
    assert_eq!(selected_font_weight(&host.editor_state), choice.value());
}

fn find_font_weight_action_point(
    panel: &PropertyPanel,
    rect: Rect,
    current_weight: u16,
) -> (Point2D, op_editor_ui::widgets::FontWeightChoice) {
    let mut y = rect.origin.y;
    while y <= rect.origin.y + rect.size.y {
        let mut x = rect.origin.x;
        while x <= rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            if let Some(PropertyPanelAction::SetFontWeight(choice)) =
                panel.hit_test_action(rect, point)
            {
                if choice.value() != current_weight {
                    return (point, choice);
                }
            }
            x += 4.0;
        }
        y += 4.0;
    }
    panic!("expected font weight action point");
}

fn selected_font_weight(state: &op_editor_core::EditorState) -> u16 {
    PropertyPanel::for_selection(state)
        .and_then(|panel| panel.snapshot.text.map(|text| text.font_weight))
        .expect("selected text node")
}
