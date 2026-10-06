use super::WidgetHostNative;
use op_editor_core::agent_settings::{
    AgentSettingsTab, ImageGenField, ImageGenProvider, SettingsFocus,
};
use op_editor_core::chat::{AgentProvider, ModelEntry};
use op_editor_core::{
    AgentSettingsButton, ButtonPressTarget, GitCandidateFile, GitOverflowView, GitPanelState,
};
use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
use op_editor_ui::widgets::{
    ai_chat_model_picker, AIChatPlaceholder, GitPanel, LayerPanel, LayerPanelHit, PropertyPanel,
    PropertyPanelAction, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

fn seed_two_chat_models(host: &mut WidgetHostNative) {
    host.editor_state_mut()
        .chat
        .available_models
        .push(ModelEntry::new(AgentProvider::CodexCli, "gpt-5", "GPT-5"));
    host.editor_state_mut()
        .chat
        .available_models
        .push(ModelEntry::new(
            AgentProvider::Antigravity,
            "default",
            "Antigravity Default",
        ));
}

fn seed_many_chat_models(host: &mut WidgetHostNative, count: usize) {
    host.editor_state_mut().chat.available_models = (0..count)
        .map(|idx| {
            ModelEntry::new(
                AgentProvider::CodexCli,
                format!("model-{idx}"),
                format!("Model {idx}"),
            )
        })
        .collect();
}

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

fn seed_layer_for_context_menu(host: &mut WidgetHostNative) {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"rectangle","id":"n1","name":"Layer","x":0,"y":0,"width":100,"height":50}
        ]}"#,
    )
    .expect("layer fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

fn first_layer_row_point(host: &WidgetHostNative, viewport_h: f32) -> Point2D {
    let rect = Rect::xywh(
        0.0,
        TOP_BAR_HEIGHT,
        host.editor_state().editor_ui.layer_panel_width,
        viewport_h - TOP_BAR_HEIGHT,
    );
    let panel = LayerPanel::from_editor(host.editor_state());
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
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    seed_two_chat_models(&mut host);
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    let chat_rect = host.ai_chat_rect(1200.0, 800.0).unwrap();
    let panel = AIChatPlaceholder::from_editor(host.editor_state());
    let picker = panel.model_picker_bounds(chat_rect).unwrap();
    let row_y = picker.origin.y
        + ai_chat_model_picker::MODEL_SEARCH_H
        + ai_chat_model_picker::MODEL_PICKER_PAD_Y
        + ai_chat_model_picker::MODEL_GROUP_H
        + ai_chat_model_picker::MODEL_ROW_H
        + ai_chat_model_picker::MODEL_GROUP_H
        + ai_chat_model_picker::MODEL_ROW_H / 2.0;

    assert!(host.apply_press(picker.origin.x + 24.0, row_y, 1200.0, 800.0));

    assert_eq!(host.editor_state().chat.selected_model, 1);
    assert_eq!(host.editor_state().editor_ui.chat_selected_agent, 4);
    assert!(!host.editor_state().editor_ui.chat_model_picker.open);
    assert!(!host.editor_state_dirty);
    assert_eq!(
        host.editor_state().editor_ui.chat_model_picker.pressed,
        None
    );

    assert!(!host.apply_release_with_viewport(1200.0, 800.0));
    assert_eq!(host.editor_state().chat.selected_model, 1);
}

#[test]
fn chat_model_row_outside_chat_wins_over_covered_canvas() {
    // RETIRED PREMISE: the floating panel could be parked over the
    // layer rail so its picker covered layer rows. The picker's only
    // out-of-card reach now is over the CANVAS, from the composer
    // card — so the covered surface underneath is a canvas node, and
    // the row press must win over selecting it.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"rectangle","id":"under-picker","name":"Under picker",
           "x":0,"y":0,"width":3000,"height":3000}
        ]}"#,
    );
    seed_many_chat_models(&mut host, 10);
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    let card = host.ai_chat_rect(1200.0, 800.0).unwrap();
    let panel = AIChatPlaceholder::from_editor(host.editor_state());
    let picker = panel.model_picker_bounds(card).unwrap();
    let point = Point2D::new(
        picker.origin.x + 24.0,
        picker.origin.y
            + ai_chat_model_picker::MODEL_SEARCH_H
            + ai_chat_model_picker::MODEL_PICKER_PAD_Y
            + ai_chat_model_picker::MODEL_GROUP_H
            + ai_chat_model_picker::MODEL_ROW_H
            + ai_chat_model_picker::MODEL_ROW_H / 2.0,
    );
    assert!(!card.contains(point), "row must sit outside the card");
    assert!(
        point.x > host.editor_state().editor_ui.layer_panel_width,
        "the picker can no longer reach the layer rail's column"
    );
    assert_eq!(
        panel.hit_test(card, point),
        Some(op_editor_ui::widgets::AIChatHit::SelectModel(1))
    );
    let selection_before = host.editor_state().selection.clone();

    assert!(host.apply_press(point.x, point.y, 1200.0, 800.0));

    assert_eq!(host.editor_state().chat.selected_model, 1);
    assert_eq!(
        host.editor_state().selection,
        selection_before,
        "the covered canvas node must not be selected by the row press"
    );
    assert!(!host.editor_state().editor_ui.chat_model_picker.open);
}

#[test]
fn chat_model_row_over_a_covered_resize_gutter_wins_press() {
    // RETIRED PREMISE: the parked floating panel put its picker over
    // the LAYER rail's resize gutter. The gutter a picker still covers
    // is the VariablesPanel's south edge under the composer card's
    // picker — and the row press must win over starting that resize.
    let mut host = WidgetHostNative::new();
    seed_many_chat_models(&mut host, 10);
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    // Row 1's centre from the live picker layout, then the variables
    // panel is sized so its south gutter runs exactly through it — the
    // user-sized panel is real persisted state, this just aims it.
    let row1_center = {
        let card = host.ai_chat_rect(1200.0, 800.0).expect("composer card");
        let picker = AIChatPlaceholder::from_editor(host.editor_state())
            .model_picker_bounds(card)
            .expect("picker rect");
        picker.origin.y
            + ai_chat_model_picker::MODEL_SEARCH_H
            + ai_chat_model_picker::MODEL_PICKER_PAD_Y
            + ai_chat_model_picker::MODEL_GROUP_H
            + ai_chat_model_picker::MODEL_ROW_H
            + ai_chat_model_picker::MODEL_ROW_H / 2.0
    };
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    let vars_origin_y = host
        .variables_panel_rect(1200.0, 800.0)
        .expect("variables panel rect")
        .origin
        .y;
    host.editor_state_mut().editor_ui.variables_panel_size =
        Some((744.0, row1_center - vars_origin_y));
    let variables_rect = host
        .variables_panel_rect(1200.0, 800.0)
        .expect("variables panel rect");
    let card = host.ai_chat_rect(1200.0, 800.0).expect("composer card");
    let panel = AIChatPlaceholder::from_editor(host.editor_state());
    let picker = panel.model_picker_bounds(card).expect("picker rect");
    let south_edge = variables_rect.origin.y + variables_rect.size.y;
    let overlap_lo = picker.origin.x.max(variables_rect.origin.x);
    let overlap_hi =
        (picker.origin.x + picker.size.x).min(variables_rect.origin.x + variables_rect.size.x);
    let point = Point2D::new((overlap_lo + overlap_hi) / 2.0, row1_center);
    assert!(picker.contains(point), "probe must sit on picker row 1");
    assert!(
        (point.y - south_edge).abs() <= 3.0,
        "probe must sit on the south resize gutter"
    );
    assert_eq!(
        panel.hit_test(card, point),
        Some(op_editor_ui::widgets::AIChatHit::SelectModel(1))
    );

    assert!(host.apply_press(point.x, point.y, 1200.0, 800.0));

    assert!(
        host.variables_resize.is_none(),
        "no resize gesture may start"
    );
    assert!(!host.is_resizing_panel());
    assert_eq!(host.editor_state().chat.selected_model, 1);
    assert!(!host.editor_state().editor_ui.chat_model_picker.open);
}

#[test]
fn right_press_on_model_picker_does_not_open_covered_layer_context_menu() {
    let mut host = WidgetHostNative::new();
    seed_layer_for_context_menu(&mut host);
    seed_many_chat_models(&mut host, 10);
    let viewport = (1200.0, 800.0);
    // RETIRED PREMISE: the parked floating panel put its picker over
    // the layer rail's rows. The rail (Layers tab) and the picker (the
    // composer card's, right of the rail) are disjoint columns now, so
    // the swallow below is asserted from the picker's own card: a
    // secondary press on it belongs to the floating surface and never
    // reaches the layer context-menu machinery beside it.
    let layer_point = first_layer_row_point(&host, viewport.1);
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    let card = host
        .ai_chat_rect(viewport.0, viewport.1)
        .expect("composer card");
    let panel = AIChatPlaceholder::from_editor(host.editor_state());
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
        !host.cursor_over_layer_panel(point.x, point.y, viewport.0, viewport.1),
        "the picker no longer reaches the layer rail's column"
    );
    assert!(
        layer_point.x < picker.origin.x,
        "rail rows stay west of the picker"
    );

    assert!(host.apply_right_press(point.x, point.y, viewport.0, viewport.1));

    assert!(host.editor_state().editor_ui.layer_context_menu.is_none());
    assert!(host.editor_state().editor_ui.chat_model_picker.open);
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
    let mut host = WidgetHostNative::new();
    seed_two_chat_models(&mut host);
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    host.editor_state_mut()
        .editor_ui
        .chat_model_picker_input
        .set_text("gpt");
    // The card's slim header only exists while the input is focused.
    host.editor_state_mut().chat.focused = true;
    let card = host.ai_chat_rect(1200.0, 800.0).unwrap();

    // Step 1: the picker is modal over the card — the footer strip
    // below its card is the reachable press that dismisses it.
    assert!(host.apply_click(
        card.origin.x + card.size.x - 28.0,
        card.origin.y + card.size.y - 20.0,
        1200.0,
        800.0
    ));
    assert!(!host.editor_state().editor_ui.chat_model_picker.open);
    assert!(host
        .editor_state()
        .editor_ui
        .chat_model_picker_input
        .text()
        .is_empty());
    assert_eq!(
        host.editor_state().editor_ui.slides_panel.tab,
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
        host.editor_state().editor_ui.slides_panel.tab,
        op_editor_core::LeftPanelTab::Chat,
        "the glyphs' one meaning is: the conversation lives in the Agent tab"
    );
    assert!(!host.editor_state().editor_ui.chat_model_picker.open);
}

#[test]
fn chat_model_picker_visible_over_topbar_wins_press() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    seed_many_chat_models(&mut host, 10);
    host.set_now_ms(456);
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    // A SHORT viewport is the one desktop shape where the capped picker
    // still climbs past the rail top into the top bar's band: the rail
    // body is only as tall as the window, and the picker is 288 px.
    // The floating panel that used to reach up here on its own is
    // retired — this is the remaining route, and the press precedence
    // it always guarded still applies.
    let viewport = (1200.0, 360.0);
    let chat = host.ai_chat_rect(viewport.0, viewport.1).unwrap();
    let panel = AIChatPlaceholder::from_editor(host.editor_state());
    let picker = panel.model_picker_bounds(chat).unwrap();
    assert!(
        picker.origin.y < op_editor_ui::widgets::TOP_BAR_HEIGHT,
        "fixture must put the picker's search strip over the top bar"
    );
    let search_top = picker.origin.y.max(0.0);
    let search_bottom = (picker.origin.y + ai_chat_model_picker::MODEL_SEARCH_H)
        .min(op_editor_ui::widgets::TOP_BAR_HEIGHT);
    assert!(search_top < search_bottom);
    let point = Point2D::new(picker.origin.x + 24.0, (search_top + search_bottom) / 2.0);
    assert_eq!(
        panel.hit_test(chat, point),
        Some(op_editor_ui::widgets::AIChatHit::FocusModelSearch)
    );

    assert!(host.apply_press(point.x, point.y, viewport.0, viewport.1));

    assert!(host.editor_state().editor_ui.chat_model_picker.open);
    assert_eq!(
        host.editor_state()
            .editor_ui
            .chat_model_picker_input
            .next_blink_flip_ms(456),
        956
    );
}

#[test]
fn image_provider_option_press_defers_selection_until_release() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .add_image_gen_profile();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .image_gen_profiles[0]
        .provider = ImageGenProvider::OpenAi;
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

    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(
        settings.image_gen_profiles[0].provider,
        ImageGenProvider::OpenAi
    );
    assert_eq!(settings.image_gen_profiles[0].model, "dall-e-3");
    assert_eq!(settings.image_gen_provider_menu_open, Some(0));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::ImageProviderOption {
                index: 0,
                provider: ImageGenProvider::Gemini,
            },
        ))
    );

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(
        settings.image_gen_profiles[0].provider,
        ImageGenProvider::Gemini
    );
    assert!(settings.image_gen_profiles[0].model.is_empty());
    assert!(settings.image_gen_provider_menu_open.is_none());
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn font_weight_row_press_defers_selection_until_release() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    *host.editor_state_mut() = op_editor_core::EditorState::sample();
    host.editor_state_mut().editor_ui.font_weight_picker_open = true;
    let property_rect = Rect {
        origin: Point2D::new(
            1200.0 - host.editor_state().editor_ui.property_panel_width,
            TOP_BAR_HEIGHT,
        ),
        size: Point2D::new(
            host.editor_state().editor_ui.property_panel_width,
            800.0 - TOP_BAR_HEIGHT,
        ),
    };
    let panel = PropertyPanel::for_selection(host.editor_state()).unwrap();
    let before_weight = selected_font_weight(host.editor_state());
    let (point, choice) = find_font_weight_action_point(&panel, property_rect, before_weight);

    assert!(host.dismiss_font_weight_picker_on_press(point.x, point.y, 1200.0, 800.0));

    assert_eq!(selected_font_weight(host.editor_state()), before_weight);
    assert!(host.editor_state().editor_ui.font_weight_picker_open);
    assert!(matches!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::FontWeightPicker(_))
    ));

    assert!(host.apply_release_with_viewport(1200.0, 800.0));
    assert!(!host.editor_state().editor_ui.font_weight_picker_open);
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
    assert_eq!(selected_font_weight(host.editor_state()), choice.value());
}

#[test]
fn tracked_picker_row_press_defers_selection_until_release() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        *panel = GitPanelState {
            open: true,
            in_repo: true,
            branch: Some("main".into()),
            overflow_open: true,
            overflow_view: GitOverflowView::TrackedPicker,
            candidate_files: vec![candidate("a.op"), candidate("b.op")],
            ..Default::default()
        };
    }
    let (vw, vh) = (1400.0, 900.0);
    let body = host.git_panel_rect(vw, vh).expect("panel open");
    let panel = GitPanel::for_editor(host.editor_state()).unwrap();
    let point = find_tracked_picker_row_point(&panel, body, 1);

    assert!(host.dispatch_git_panel_press(point.x, point.y, vw, vh));

    let git = &host.editor_state().editor_ui.git_panel;
    assert_eq!(git.tracked_picker_selected, None);
    assert_eq!(git.tracked_picker.pressed, Some(1));

    assert!(host.apply_release_with_viewport(vw, vh));
    let git = &host.editor_state().editor_ui.git_panel;
    assert_eq!(git.tracked_picker_selected, Some(1));
    assert_eq!(git.tracked_picker.pressed, None);
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

fn candidate(name: &str) -> GitCandidateFile {
    GitCandidateFile {
        path: format!("/repo/{name}"),
        relative_path: name.to_string(),
        milestone_count: 0,
        last_commit_time: "now".into(),
        last_commit_message: None,
    }
}

fn find_tracked_picker_row_point(panel: &GitPanel<'_>, body: Rect, row: usize) -> Point2D {
    let mut y = body.origin.y;
    while y <= body.origin.y + body.size.y + 240.0 {
        let mut x = body.origin.x;
        while x <= body.origin.x + body.size.x {
            let point = Point2D::new(x, y);
            if panel.tracked_picker_select_hit(body, point)
                == op_editor_ui::widgets::git_panel::SelectHit::Row(row)
            {
                return point;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    panic!("expected tracked picker row point");
}
