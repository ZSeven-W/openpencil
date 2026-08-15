use crate::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
use crate::{Point2D, Rect};
use op_editor_core::agent_settings::{BuiltinAgentField, SettingsFocus};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::EditorState;

fn rect_center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.x
        && b.origin.x < a.origin.x + a.size.x
        && a.origin.y < b.origin.y + b.size.y
        && b.origin.y < a.origin.y + a.size.y
}

fn touch_draft_state(class: EditorSizeClass) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = class;
    state.editor_ui.agent_settings.begin_builtin_agent_draft();
    state
        .editor_ui
        .agent_settings
        .set_builtin_agent_draft_preset(op_editor_core::BuiltinAgentPresetKey::Custom);
    state
        .editor_ui
        .agent_settings
        .builtin_agent_draft
        .as_mut()
        .expect("draft should exist")
        .api_key = "sk-test".into();
    state
}

fn touch_draft_card(state: &EditorState, content: Rect) -> Rect {
    let settings = &state.editor_ui.agent_settings;
    let y = crate::widgets::agent_settings_panel_geometry::agents_body_top(content)
        + crate::widgets::agent_settings_builtin_layout::HEADER_HEIGHT
        + crate::widgets::agent_settings_builtin_layout::SUBTITLE_HEIGHT
        + crate::widgets::agent_settings_builtin_layout::sync_error_height(settings);
    Rect::xywh(
        content.origin.x,
        y,
        content.size.x,
        crate::widgets::agent_settings_builtin_layout::draft_card_height_for_ui(settings, true),
    )
}

#[test]
fn touch_builtin_draft_controls_are_44_points_and_do_not_overlap() {
    for (width, height, class) in [
        (320.0, 568.0, EditorSizeClass::Compact),
        (390.0, 844.0, EditorSizeClass::Compact),
        (834.0, 1_112.0, EditorSizeClass::Medium),
    ] {
        let mut state = touch_draft_state(class);
        let panel = AgentSettingsPanel::for_editor(&state);
        let panel_rect = panel.rect(width, height);
        let content = panel.resolved_content_viewport(panel_rect);
        let card = touch_draft_card(&state, content);
        let settings = &state.editor_ui.agent_settings;
        let form_h = crate::widgets::agent_settings_builtin_layout::expanded_card_height_for_ui(
            settings, None, true,
        );
        let provider =
            crate::widgets::agent_settings_builtin_parts::provider_select_rect(card, true);
        let kind = crate::widgets::agent_settings_builtin_parts::kind_rect(card, true);
        let fields: Vec<_> = [
            BuiltinAgentField::DisplayName,
            BuiltinAgentField::ApiKey,
            BuiltinAgentField::Model,
            BuiltinAgentField::BaseUrl,
        ]
        .into_iter()
        .enumerate()
        .map(|(row, field)| {
            (
                field,
                crate::widgets::agent_settings_builtin_layout::field_input_rect_for_ui(
                    settings, card, None, row, true,
                ),
            )
        })
        .collect();
        let save = crate::widgets::agent_settings_form_actions::save_button_rect_for_ui(
            card, form_h, true,
        );
        let cancel = crate::widgets::agent_settings_form_actions::cancel_button_rect_for_ui(
            card, form_h, true,
        );

        let mut controls = vec![("Provider", provider), ("Kind", kind)];
        controls.extend(fields.iter().map(|(field, rect)| {
            (
                match field {
                    BuiltinAgentField::DisplayName => "Name",
                    BuiltinAgentField::ApiKey => "API Key",
                    BuiltinAgentField::Model => "Model",
                    BuiltinAgentField::BaseUrl => "Base URL",
                },
                *rect,
            )
        }));
        controls.extend([("Save", save), ("Cancel", cancel)]);

        for (label, rect) in &controls {
            assert!(
                rect.size.x >= 44.0 && rect.size.y >= 44.0,
                "{width}x{height} {label} target must be at least 44pt, got {rect:?}"
            );
            assert!(
                card.contains(rect.origin)
                    && card.contains(Point2D::new(
                        rect.origin.x + rect.size.x,
                        rect.origin.y + rect.size.y,
                    )),
                "{width}x{height} {label} must stay inside the draft card"
            );
        }
        assert!(
            kind.size.x / 2.0 >= 44.0,
            "{width}x{height} each Kind segment must be at least 44pt wide"
        );
        for (index, (label, rect)) in controls.iter().enumerate() {
            for (other_label, other) in controls.iter().skip(index + 1) {
                assert!(
                    !rects_overlap(*rect, *other),
                    "{width}x{height} {label} overlaps {other_label}: {rect:?} vs {other:?}"
                );
            }
        }

        let kind_change_point =
            Point2D::new(kind.origin.x + kind.size.x / 4.0, rect_center(kind).y);
        let low_level_hits = [
            (
                rect_center(provider),
                crate::widgets::agent_settings_builtin::BuiltinHit::TogglePresetMenu(None),
            ),
            (
                kind_change_point,
                crate::widgets::agent_settings_builtin::BuiltinHit::ToggleDraftKind,
            ),
            (
                rect_center(save),
                crate::widgets::agent_settings_builtin::BuiltinHit::SaveDraft,
            ),
            (
                rect_center(cancel),
                crate::widgets::agent_settings_builtin::BuiltinHit::CancelDraft,
            ),
        ];
        for (point, expected) in low_level_hits {
            assert_eq!(
                crate::widgets::agent_settings_builtin::hit_test_for_ui(
                    content,
                    settings,
                    &state.editor_ui,
                    point,
                ),
                expected,
                "{width}x{height} touch target should route through built-in hit testing"
            );
        }
        for (field, rect) in &fields {
            assert_eq!(
                crate::widgets::agent_settings_builtin::hit_test_for_ui(
                    content,
                    settings,
                    &state.editor_ui,
                    rect_center(*rect),
                ),
                crate::widgets::agent_settings_builtin::BuiltinHit::FocusDraft(*field),
                "{width}x{height} {field:?} field should route through built-in hit testing"
            );
        }

        // Verify the panel's clipping + scroll transform can expose every
        // target, especially Save/Cancel on the 320-point phone.
        let panel_hits = [
            (
                rect_center(provider),
                AgentSettingsHit::ToggleBuiltinAgentPresetMenu(None),
            ),
            (
                kind_change_point,
                AgentSettingsHit::ToggleBuiltinAgentDraftKind,
            ),
            (rect_center(save), AgentSettingsHit::SaveBuiltinAgentDraft),
            (
                rect_center(cancel),
                AgentSettingsHit::CancelBuiltinAgentDraft,
            ),
        ];
        for (content_point, expected) in panel_hits {
            let max_scroll = AgentSettingsPanel::for_editor(&state).max_scroll(panel_rect);
            state.editor_ui.agent_settings.scroll_y.offset = (content_point.y
                - (content.origin.y + content.size.y / 2.0))
                .clamp(0.0, max_scroll);
            let panel = AgentSettingsPanel::for_editor(&state);
            let screen_point = Point2D::new(
                content_point.x,
                content_point.y - panel.effective_scroll(panel_rect),
            );
            assert!(
                content.contains(screen_point),
                "{width}x{height} target must be reachable by scrolling"
            );
            assert_eq!(panel.hit_test(panel_rect, screen_point), expected);
        }
    }
}

#[test]
fn desktop_builtin_draft_geometry_remains_compact() {
    let state = touch_draft_state(EditorSizeClass::Compact);
    let settings = &state.editor_ui.agent_settings;
    let card = Rect::xywh(20.0, 40.0, 480.0, 232.0);

    let form_h = crate::widgets::agent_settings_builtin_layout::expanded_card_height_for_ui(
        settings, None, false,
    );
    assert_eq!(form_h, 196.0);
    assert_eq!(
        crate::widgets::agent_settings_builtin_layout::draft_card_height_for_ui(settings, false),
        232.0
    );
    assert_eq!(
        crate::widgets::agent_settings_builtin_parts::provider_select_rect(card, false)
            .size
            .y,
        24.0
    );
    assert_eq!(
        crate::widgets::agent_settings_builtin_parts::kind_rect(card, false)
            .size
            .y,
        24.0
    );
    let fields: Vec<_> = (0..4)
        .map(|row| {
            crate::widgets::agent_settings_builtin_layout::field_input_rect_for_ui(
                settings, card, None, row, false,
            )
        })
        .collect();
    assert!(fields.iter().all(|rect| rect.size.y == 24.0));
    assert!(fields
        .windows(2)
        .all(|pair| pair[1].origin.y - pair[0].origin.y == 28.0));
    assert_eq!(
        crate::widgets::agent_settings_form_actions::save_button_rect_for_ui(card, form_h, false,)
            .size,
        Point2D::new(68.0, 26.0)
    );
    assert_eq!(
        crate::widgets::agent_settings_form_actions::cancel_button_rect_for_ui(
            card, form_h, false,
        )
        .size,
        Point2D::new(68.0, 26.0)
    );
}

#[test]
fn focused_model_list_expands_without_overlapping_following_fields_on_touch_and_desktop() {
    for touch in [false, true] {
        let mut state = touch_draft_state(EditorSizeClass::Compact);
        let settings = &mut state.editor_ui.agent_settings;
        let collapsed = crate::widgets::agent_settings_builtin_layout::expanded_card_height_for_ui(
            settings, None, touch,
        );
        settings.focus = Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::Model));
        let expanded = crate::widgets::agent_settings_builtin_layout::expanded_card_height_for_ui(
            settings, None, touch,
        );
        let extra = if touch { 60.0 } else { 44.0 };
        assert_eq!(expanded, collapsed + extra);

        let card = Rect::xywh(20.0, 40.0, 520.0, expanded + 80.0);
        let model = crate::widgets::agent_settings_builtin_layout::field_input_rect_for_ui(
            settings, card, None, 2, touch,
        );
        let base_url = crate::widgets::agent_settings_builtin_layout::field_input_rect_for_ui(
            settings, card, None, 3, touch,
        );
        assert_eq!(model.size.y, if touch { 104.0 } else { 68.0 });
        assert!(
            base_url.origin.y >= model.origin.y + model.size.y + if touch { 8.0 } else { 4.0 },
            "the Base URL row must remain below the multiline editor: {model:?} vs {base_url:?}"
        );
        if touch {
            assert!(model.size.y >= 44.0 && base_url.size.y >= 44.0);
        }
    }
}

#[test]
fn pure_builtin_provider_base_url_is_read_only_hit_target() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_builtin_agent();
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::ApiKey,
    });

    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content_x = crate::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .x;
    let content_y = crate::widgets::agent_settings_panel::content_viewport(rect)
        .origin
        .y;
    let first_card_y =
        content_y + crate::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT + 28.0 + 28.0;
    let point = Point2D::new(content_x + 92.0, first_card_y + 170.0);

    assert_eq!(panel.hit_test(rect, point), AgentSettingsHit::Inside);
}

#[test]
fn credential_sync_error_reserves_a_status_row_in_the_layout() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.add_builtin_agent();
    let without =
        crate::widgets::agent_settings_builtin::content_height(&state.editor_ui.agent_settings);

    state.editor_ui.agent_settings.web_credential_sync_error =
        Some("server rejected the credential snapshot (400)".into());
    let with =
        crate::widgets::agent_settings_builtin::content_height(&state.editor_ui.agent_settings);

    assert!(
        with > without,
        "sync error must reserve extra height (with={with}, without={without})"
    );
}

#[test]
fn model_field_chevron_toggles_and_catalog_rows_select() {
    let mut state = EditorState::default();
    let id = state.editor_ui.agent_settings.add_builtin_agent_config(
        "MiniMax",
        "sk-test",
        "MiniMax-M3",
        op_editor_core::BuiltinAgentKind::Anthropic,
        "https://api.minimaxi.com/anthropic",
    );
    let request = state
        .editor_ui
        .agent_settings
        .begin_builtin_model_catalog_refresh(
            op_editor_core::BuiltinModelCatalogTarget::Agent(id),
            0,
        )
        .expect("discovery request");
    let expected = state
        .editor_ui
        .agent_settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("resolvable");
    state
        .editor_ui
        .agent_settings
        .apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            op_editor_core::BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![
                    op_editor_core::BuiltinModelOption::new("m1", "Model One"),
                    op_editor_core::BuiltinModelOption::new("m2", "Model Two"),
                ],
            },
        );
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::Model,
    });

    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let content = panel.resolved_content_viewport(rect);
    let y = crate::widgets::agent_settings_panel_geometry::agents_body_top(content)
        + crate::widgets::agent_settings_builtin_layout::HEADER_HEIGHT
        + crate::widgets::agent_settings_builtin_layout::SUBTITLE_HEIGHT
        + crate::widgets::agent_settings_builtin_layout::sync_error_height(
            &state.editor_ui.agent_settings,
        );
    let card = Rect::xywh(
        content.origin.x,
        y,
        content.size.x,
        crate::widgets::agent_settings_builtin_layout::card_height_for_ui(
            &state.editor_ui.agent_settings,
            0,
            false,
        ),
    );
    let settings = &state.editor_ui.agent_settings;
    let model_input = crate::widgets::agent_settings_builtin_layout::field_input_rect_for_ui(
        settings,
        card,
        Some(0),
        2,
        false,
    );

    // The right edge of the Model input is the dropdown trigger.
    let chevron = Point2D::new(
        model_input.origin.x + model_input.size.x - 8.0,
        rect_center(model_input).y,
    );
    assert_eq!(
        crate::widgets::agent_settings_builtin::hit_test_for_ui(
            content,
            settings,
            &state.editor_ui,
            chevron,
        ),
        crate::widgets::agent_settings_builtin::BuiltinHit::ToggleModelMenu(Some(0))
    );

    // The rest of the field is a plain focus target.
    assert_eq!(
        crate::widgets::agent_settings_builtin::hit_test_for_ui(
            content,
            settings,
            &state.editor_ui,
            Point2D::new(model_input.origin.x + 30.0, rect_center(model_input).y),
        ),
        crate::widgets::agent_settings_builtin::BuiltinHit::Focus {
            index: 0,
            field: BuiltinAgentField::Model,
        }
    );

    // With the menu open, a click on the second catalog row selects it.
    state.editor_ui.agent_settings.builtin_model_menu_open =
        Some(op_editor_core::agent_settings::BuiltinModelMenuTarget::Agent(0));
    let settings = &state.editor_ui.agent_settings;
    let menu = crate::widgets::agent_settings_builtin_model_menu::model_menu_rect(
        settings,
        card,
        Some(0),
        false,
    );
    let row_1 = Point2D::new(menu.origin.x + 40.0, menu.origin.y + 4.0 + 24.0 + 12.0);
    assert_eq!(
        crate::widgets::agent_settings_builtin::hit_test_for_ui(
            content,
            settings,
            &state.editor_ui,
            row_1,
        ),
        crate::widgets::agent_settings_builtin::BuiltinHit::SelectModel {
            index: Some(0),
            row: 1,
        }
    );
}
