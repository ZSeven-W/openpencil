use super::WidgetHost;
use op_editor_core::agent_settings::{
    AgentSettingsTab, ImageGenField, ImageGenProvider, SettingsFocus,
};
use op_editor_core::chat::{AgentProvider, ModelEntry};
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
use op_editor_ui::widgets::{
    ai_chat_model_picker, AIChatPlaceholder, LayerPanel, LayerPanelHit, PropertyPanel,
    PropertyPanelAction, TOP_BAR_HEIGHT,
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
        .push(ModelEntry::new(
            AgentProvider::Antigravity,
            "default",
            "Antigravity Default",
        ));
}

fn seed_layer_for_context_menu(host: &mut WidgetHost) {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"rectangle","id":"n1","name":"Layer","x":0,"y":0,"width":100,"height":50}
        ]}"#,
    )
    .expect("layer fixture parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn first_layer_row_point(host: &WidgetHost, viewport_h: f32) -> Point2D {
    let rect = host.layer_panel_rect(viewport_h);
    let panel = LayerPanel::from_editor(&host.editor_state);
    let x = 48.0;
    let mut y = rect.origin.y + 1.0;
    while y < rect.origin.y + rect.size.y {
        let point = Point2D::new(x, y);
        if matches!(panel.hit_test(rect, point), Some(LayerPanelHit::Layer(_))) {
            return point;
        }
        y += 1.0;
    }
    panic!("layer row not found");
}

#[test]
fn chat_model_row_press_selects_and_closes_immediately() {
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
        + ai_chat_model_picker::MODEL_GROUP_H
        + ai_chat_model_picker::MODEL_ROW_H / 2.0;

    assert!(host.apply_press(picker.origin.x + 24.0, row_y, 1200.0, 800.0));

    assert_eq!(host.editor_state.chat.selected_model, 1);
    assert_eq!(host.editor_state.editor_ui.chat_selected_agent, 4);
    assert!(!host.editor_state.editor_ui.chat_model_picker.open);
    assert!(!host.editor_state_dirty);
    assert_eq!(host.editor_state.editor_ui.chat_model_picker.pressed, None);

    assert!(!host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state.chat.selected_model, 1);
}

#[test]
fn chat_model_row_wins_over_variables_panel_and_preset_menu() {
    let mut host = WidgetHost::new();
    seed_two_chat_models(&mut host);
    host.editor_state.editor_ui.variables_panel_open = true;
    host.editor_state.editor_ui.variables_preset_menu_open = true;
    host.editor_state.editor_ui.chat_model_picker.open = true;
    let viewport = (1440.0, 600.0);
    let chat = host
        .ai_chat_rect(viewport.0, viewport.1)
        .expect("chat rect");
    let panel = AIChatPlaceholder::from_editor(&host.editor_state);
    let picker = panel.model_picker_bounds(chat).expect("picker rect");
    let point = Point2D::new(
        picker.origin.x + 48.0,
        picker.origin.y
            + ai_chat_model_picker::MODEL_SEARCH_H
            + ai_chat_model_picker::MODEL_PICKER_PAD_Y
            + ai_chat_model_picker::MODEL_GROUP_H
            + ai_chat_model_picker::MODEL_ROW_H
            + ai_chat_model_picker::MODEL_GROUP_H
            + ai_chat_model_picker::MODEL_ROW_H / 2.0,
    );
    assert!(host
        .variables_panel_rect(viewport.0, viewport.1)
        .expect("variables rect")
        .contains(point));
    assert_eq!(
        panel.hit_test(chat, point),
        Some(op_editor_ui::widgets::AIChatHit::SelectModel(1))
    );

    assert!(host.apply_press(point.x, point.y, viewport.0, viewport.1));

    assert_eq!(host.editor_state.chat.selected_model, 1);
    assert!(!host.editor_state.editor_ui.chat_model_picker.open);
    assert!(host.editor_state.editor_ui.variables_panel_open);
    assert!(host.editor_state.editor_ui.variables_preset_menu_open);
}

#[test]
fn right_press_on_model_picker_does_not_open_covered_layer_context_menu() {
    let mut host = WidgetHost::new();
    seed_layer_for_context_menu(&mut host);
    seed_two_chat_models(&mut host);
    let viewport = (1200.0, 800.0);
    // RETIRED PREMISE: the parked floating panel put its picker over
    // the layer rail's rows. The rail (Layers tab) and the picker (the
    // composer card's, right of the rail) are disjoint columns now, so
    // the swallow below is asserted from the picker's own card: a
    // secondary press on it belongs to the floating surface and never
    // reaches the layer context-menu machinery beside it.
    let layer_point = first_layer_row_point(&host, viewport.1);
    host.editor_state.editor_ui.chat_model_picker.open = true;
    let card = host
        .ai_chat_rect(viewport.0, viewport.1)
        .expect("composer card");
    let panel = AIChatPlaceholder::from_editor(&host.editor_state);
    let picker = panel.model_picker_bounds(card).expect("picker rect");
    let point = Point2D::new(
        picker.origin.x + 24.0,
        picker.origin.y
            + ai_chat_model_picker::MODEL_SEARCH_H
            + ai_chat_model_picker::MODEL_PICKER_PAD_Y
            + ai_chat_model_picker::MODEL_GROUP_H
            + ai_chat_model_picker::MODEL_ROW_H / 2.0,
    );
    assert!(picker.contains(point));
    assert!(
        !host.layer_panel_rect(viewport.1).contains(point),
        "the picker no longer reaches the layer rail's column"
    );
    assert!(
        layer_point.x < picker.origin.x,
        "rail rows stay west of the picker"
    );

    assert!(host.apply_right_press(point.x, point.y, viewport.0, viewport.1));

    assert!(host.editor_state.editor_ui.layer_context_menu.is_none());
    assert!(host.editor_state.editor_ui.chat_model_picker.open);
}

/// RETIRED BEHAVIOUR (minimize via chevron): the header chevron that
/// collapsed the chat also closed the model picker and cleared its
/// search. The chevron is gone from desktop; the composer card carries
/// the same cleanup in two reachable steps: the open picker is modal
/// over the card (any press on the card dismisses it AND its search),
/// and once it is gone the header's glyphs — both of them — mean
/// "open the Agent tab".
#[test]
fn the_composer_card_header_opens_the_agent_tab_and_closes_the_model_picker() {
    let mut host = WidgetHost::new();
    seed_two_chat_models(&mut host);
    host.editor_state.editor_ui.chat_model_picker.open = true;
    host.editor_state
        .editor_ui
        .chat_model_picker_input
        .set_text("gpt");
    // The card's slim header only exists while the input is focused.
    host.editor_state.chat.focused = true;
    let card = host.ai_chat_rect(1200.0, 800.0).unwrap();

    // Step 1: the picker is modal over the card — the footer strip
    // below its card is the reachable press that dismisses it.
    assert!(host.apply_click(
        card.origin.x + card.size.x - 28.0,
        card.origin.y + card.size.y - 20.0,
        1200.0,
        800.0
    ));
    assert!(!host.editor_state.editor_ui.chat_model_picker.open);
    assert!(host
        .editor_state
        .editor_ui
        .chat_model_picker_input
        .text()
        .is_empty());
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        op_editor_core::LeftPanelTab::Layers,
        "dismissing the picker stays on this tab"
    );

    // Step 2: with the picker gone the header is reachable — anywhere
    // on the 30 px strip, both glyphs route the same way.
    assert!(host.apply_click(
        card.origin.x + card.size.x / 2.0,
        card.origin.y + 15.0,
        1200.0,
        800.0
    ));
    assert_eq!(
        host.editor_state.editor_ui.slides_panel.tab,
        op_editor_core::LeftPanelTab::Chat,
        "the glyphs' one meaning is: the conversation lives in the Agent tab"
    );
    assert!(!host.editor_state.editor_ui.chat_model_picker.open);
}

#[test]
fn hidden_chat_rect_closes_stale_model_picker_before_lower_press() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.embed = op_editor_core::EmbedHost::VsCode;
    host.editor_state.editor_ui.chat_model_picker.open = true;

    assert!(host.apply_press(300.0, 300.0, 1200.0, 800.0));

    assert!(!host.editor_state.editor_ui.chat_model_picker.open);
}

#[test]
fn chat_model_picker_visible_over_topbar_wins_press() {
    let mut host = WidgetHost::new();
    seed_two_chat_models(&mut host);
    for idx in 2..10 {
        host.editor_state
            .chat
            .available_models
            .push(ModelEntry::new(
                AgentProvider::CodexCli,
                format!("model-{idx}"),
                format!("Model {idx}"),
            ));
    }
    host.set_now_ms(456);
    host.editor_state.chat.maximized = true;
    host.editor_state.editor_ui.chat_model_picker.open = true;
    let (viewport_w, viewport_h) = (1200.0, 330.0);
    let chat = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    let panel = AIChatPlaceholder::from_editor(&host.editor_state);
    let picker = panel.model_picker_bounds(chat).unwrap();
    let search_top = picker.origin.y.max(0.0);
    let search_bottom = (picker.origin.y + ai_chat_model_picker::MODEL_SEARCH_H)
        .min(op_editor_ui::widgets::TOP_BAR_HEIGHT);
    assert!(search_top < search_bottom);
    let point = Point2D::new(picker.origin.x + 24.0, (search_top + search_bottom) / 2.0);
    assert_eq!(
        panel.hit_test(chat, point),
        Some(op_editor_ui::widgets::AIChatHit::FocusModelSearch)
    );

    assert!(host.apply_press(point.x, point.y, viewport_w, viewport_h));

    assert!(host.editor_state.editor_ui.chat_model_picker.open);
    assert_eq!(
        host.editor_state
            .editor_ui
            .chat_model_picker_input
            .next_blink_flip_ms(456),
        956
    );
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

    let panel = AgentSettingsPanel::for_web_editor(&host.editor_state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = op_editor_ui::widgets::agent_settings_panel::secondary_tab_body(rect)
        .origin
        .x;
    let content_y = op_editor_ui::widgets::agent_settings_panel::secondary_tab_body(rect)
        .origin
        .y;
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
