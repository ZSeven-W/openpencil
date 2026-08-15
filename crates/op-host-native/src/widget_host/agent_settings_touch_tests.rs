use super::WidgetHostNative;
use op_editor_core::agent_settings::{
    AgentSettingsTab, BuiltinAgentField, BuiltinAgentKind, BuiltinModelMenuTarget, SettingsFocus,
};
use op_editor_core::missing_fonts::{MissingFontEntry, MissingFontsPrompt};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{
    BuiltinModelCatalogRefreshOutcome, BuiltinModelCatalogTarget, BuiltinModelOption,
};
use op_editor_ui::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 390.0;
const VIEWPORT_H: f32 = 844.0;

fn touch_settings_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.touch = true;
    ui.size_class = EditorSizeClass::Compact;
    ui.agent_settings_open = true;
    host
}

fn add_builtin_agents(host: &mut WidgetHostNative, count: usize) {
    for index in 0..count {
        host.editor_state_mut()
            .editor_ui
            .agent_settings
            .add_builtin_agent_with_defaults(
                format!("Provider {index}"),
                format!("sk-{index}"),
                format!("model-{index}"),
            );
    }
}

fn add_open_model_catalog(host: &mut WidgetHostNative, count: usize) {
    let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
    let id = settings.add_builtin_agent_config(
        "Provider",
        "sk-test",
        "model-0",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.example.com/v1",
    );
    let request = settings
        .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent(id), 1)
        .expect("configured provider starts discovery");
    let expected = settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("current discovery config");
    settings.take_pending_builtin_model_catalog_refresh();
    assert!(
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            BuiltinModelCatalogRefreshOutcome::Success {
                models: (0..count)
                    .map(|index| {
                        BuiltinModelOption::new(format!("model-{index}"), format!("Model {index}"))
                    })
                    .collect(),
            },
        )
    );
    settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::Model,
    });
    settings.builtin_model_menu_open = Some(BuiltinModelMenuTarget::Agent(0));
    host.editor_state_mut()
        .editor_ui
        .settings_input
        .set_text("model-0");
}

fn visible_model_menu_point(host: &WidgetHostNative) -> Point2D {
    let (panel, panel_rect) = host.agent_settings_geometry(VIEWPORT_W, VIEWPORT_H);
    let content = panel.resolved_content_viewport(panel_rect);
    let menu = panel
        .focused_model_menu_rect(panel_rect)
        .expect("focused model menu");
    let menu_top = menu.origin.y - panel.effective_scroll(panel_rect);
    let top = menu_top.max(content.origin.y);
    let bottom = (menu_top + menu.size.y).min(content.origin.y + content.size.y);
    assert!(
        bottom > top,
        "keyboard-safe body must expose part of the menu"
    );
    Point2D::new(menu.origin.x + menu.size.x / 2.0, (top + bottom) / 2.0)
}

fn model_row_center(host: &WidgetHostNative, row: usize) -> Point2D {
    const MENU_PAD: f32 = 6.0;
    const ROW_H: f32 = 44.0;
    let (panel, panel_rect) = host.agent_settings_geometry(VIEWPORT_W, VIEWPORT_H);
    let menu = panel
        .focused_model_menu_rect(panel_rect)
        .expect("focused model menu");
    Point2D::new(
        menu.origin.x + menu.size.x / 2.0,
        menu.origin.y - panel.effective_scroll(panel_rect) + MENU_PAD + ROW_H * row as f32
            - host
                .editor_state()
                .editor_ui
                .agent_settings
                .builtin_model_menu_scroll
                .offset
            + ROW_H / 2.0,
    )
}

fn center(rect: op_editor_ui::Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn find_toggle(host: &WidgetHostNative, index: usize) -> Point2D {
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let body = panel.resolved_content_viewport(rect);
    let mut y = body.origin.y + 0.5;
    while y < body.origin.y + body.size.y {
        let mut x = body.origin.x + body.size.x - 0.5;
        while x >= body.origin.x {
            if panel.hit_test(rect, Point2D::new(x, y))
                == AgentSettingsHit::ToggleBuiltinAgentEnabled(index)
            {
                return Point2D::new(x, y);
            }
            x -= 2.0;
        }
        y += 2.0;
    }
    panic!("touch toggle hit for provider {index}");
}

#[test]
fn touch_body_tap_commits_once_on_release() {
    let mut host = touch_settings_host();
    add_builtin_agents(&mut host, 1);
    let point = find_toggle(&host, 0);
    let before = host.editor_state().editor_ui.agent_settings.builtin_agents[0].enabled;

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.agent_settings_touch_gesture.is_some());
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.builtin_agents[0].enabled,
        before,
        "touch-down must not run a body action"
    );
    assert!(!host.apply_cursor_move(point.x + 3.0, point.y + 2.0));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.builtin_agents[0].enabled,
        !before
    );

    let once = host.editor_state().editor_ui.agent_settings.builtin_agents[0].enabled;
    assert!(!host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.builtin_agents[0].enabled,
        once,
        "a second release must not replay the tap"
    );
}

#[test]
fn touch_body_drag_scrolls_and_cancels_the_pending_action() {
    let mut host = touch_settings_host();
    add_builtin_agents(&mut host, 18);
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    assert!(panel.max_scroll(rect) > 80.0);
    let body = panel.resolved_content_viewport(rect);
    let start = center(body);
    let enabled_before: Vec<bool> = host
        .editor_state()
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .map(|agent| agent.enabled)
        .collect();
    let viewport_before = host.editor_state().viewport;

    assert!(host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_cursor_move(start.x, start.y - 80.0));
    assert!(
        host.editor_state().editor_ui.agent_settings.scroll_y.offset > 0.0,
        "an upward finger drag must advance the settings body"
    );
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .map(|agent| agent.enabled)
            .collect::<Vec<_>>(),
        enabled_before,
        "promoting to scroll must cancel the down hit"
    );
    assert_eq!(host.editor_state().viewport, viewport_before);
}

#[test]
fn promoted_touch_scroll_keeps_capture_outside_the_body() {
    let mut host = touch_settings_host();
    add_builtin_agents(&mut host, 18);
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let body = panel.resolved_content_viewport(rect);
    let start = Point2D::new(body.origin.x + body.size.x / 2.0, body.origin.y + 36.0);

    assert!(host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_cursor_move(start.x, start.y - 24.0));
    let after_promotion = host.editor_state().editor_ui.agent_settings.scroll_y.offset;
    assert!(after_promotion > 0.0);

    assert!(host.apply_cursor_move(-20.0, 20.0));
    assert!(
        host.editor_state().editor_ui.agent_settings.scroll_y.offset > after_promotion,
        "the body keeps pointer capture after the finger leaves its bounds"
    );
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
}

#[test]
fn touch_model_menu_hands_off_to_keyboard_safe_body_at_scroll_edges() {
    for count in [5, 8] {
        let mut host = touch_settings_host();
        add_open_model_catalog(&mut host, count);
        host.last_viewport_w = VIEWPORT_W;
        host.last_viewport_h = VIEWPORT_H;
        assert!(host.set_keyboard_occlusion(500.0));

        let start = visible_model_menu_point(&host);
        let outer_before = host.editor_state().editor_ui.agent_settings.scroll_y.offset;
        assert!(host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H));

        if count > 5 {
            assert!(host.apply_cursor_move(start.x, start.y - 160.0));
            assert_eq!(
                host.editor_state()
                    .editor_ui
                    .agent_settings
                    .builtin_model_menu_scroll
                    .offset,
                132.0,
                "eight touch rows must reach the three-row internal scroll edge"
            );
            assert_eq!(
                host.editor_state().editor_ui.agent_settings.scroll_y.offset,
                outer_before,
                "internal movement owns the gesture until it reaches its edge"
            );
            assert!(host.apply_cursor_move(start.x, start.y - 260.0));
        } else {
            assert!(host.apply_cursor_move(start.x, start.y - 100.0));
            assert_eq!(
                host.editor_state()
                    .editor_ui
                    .agent_settings
                    .builtin_model_menu_scroll
                    .offset,
                0.0,
                "a five-row menu has no internal overflow"
            );
        }

        let outer_after = host.editor_state().editor_ui.agent_settings.scroll_y.offset;
        assert!(
            outer_after > outer_before,
            "{count} rows must hand an edge drag to the keyboard-safe outer body"
        );
        let last_row = model_row_center(&host, count - 1);
        let (panel, panel_rect) = host.agent_settings_geometry(VIEWPORT_W, VIEWPORT_H);
        let content = panel.resolved_content_viewport(panel_rect);
        assert!(
            content.contains(last_row),
            "the last of {count} rows must be visible above the keyboard"
        );
        assert_eq!(
            panel.hit_test(panel_rect, last_row),
            AgentSettingsHit::SelectBuiltinModel {
                index: Some(0),
                row: count - 1,
            },
            "the last of {count} rows must retain its 44pt hit target"
        );
        assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

        assert!(host.apply_press(last_row.x, last_row.y, VIEWPORT_W, VIEWPORT_H));
        assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
        let settings = &host.editor_state().editor_ui.agent_settings;
        assert!(settings.builtin_agents[0].has_model(&format!("model-{}", count - 1)));
        assert_eq!(
            settings.builtin_model_menu_open,
            Some(BuiltinModelMenuTarget::Agent(0))
        );
        assert_eq!(
            settings.scroll_y.offset, outer_after,
            "multi-select must not snap the keyboard-safe outer slice"
        );
    }
}

#[test]
fn touch_settings_chrome_stays_immediate() {
    let mut host = touch_settings_host();
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let layout = panel.resolved_layout(rect);
    let tabs = panel.navigation_tabs();
    let images_index = tabs
        .iter()
        .position(|tab| *tab == AgentSettingsTab::Images)
        .expect("mobile Images tab");
    let tab = center(layout.nav_item_rect(images_index, tabs.len()));

    assert!(host.apply_press(tab.x, tab.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.tab,
        AgentSettingsTab::Images
    );
    assert!(host.agent_settings_touch_gesture.is_none());

    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let close = center(panel.resolved_layout(rect).close_target);
    assert!(host.apply_press(close.x, close.y, VIEWPORT_W, VIEWPORT_H));
    assert!(!host.editor_state().editor_ui.agent_settings_open);
    assert!(host.agent_settings_touch_gesture.is_none());
}

#[test]
fn touch_model_editor_tap_places_caret_on_the_earlier_visible_line() {
    let mut host = touch_settings_host();
    add_builtin_agents(&mut host, 1);
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::Model,
        });
        ui.settings_input
            .set_text("model-a\nmodel-b\nmodel-c\nmodel-d\nmodel-e");
    }
    let point = {
        let (panel, panel_rect) = host.agent_settings_geometry(VIEWPORT_W, VIEWPORT_H);
        let mut input = panel.focused_input_rect(panel_rect).expect("model input");
        input.origin.y -= panel.effective_scroll(panel_rect);
        let point = Point2D::new(input.origin.x + 13.0, input.origin.y + 6.0);
        assert_eq!(
            panel.hit_test(panel_rect, point),
            AgentSettingsHit::FocusBuiltinAgent {
                index: 0,
                field: BuiltinAgentField::Model,
            }
        );
        point
    };

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.agent_settings_touch_gesture.is_some());
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.settings_input.caret(),
        "model-a\n".len(),
        "the first of four visible touch rows is model-b, not the buffer end"
    );
}

#[test]
fn pan_right_press_and_modal_close_cancel_pending_touch_taps() {
    for cancellation in 0..3 {
        let mut host = touch_settings_host();
        add_builtin_agents(&mut host, 1);
        let point = find_toggle(&host, 0);
        let before = host.editor_state().editor_ui.agent_settings.builtin_agents[0].enabled;
        assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));

        match cancellation {
            0 => {
                assert!(
                    host.apply_pan_gesture(point.x, point.y, 0.0, -40.0, VIEWPORT_W, VIEWPORT_H,)
                );
            }
            1 => assert!(host.apply_right_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H,)),
            _ => {
                assert!(host.apply_toggle_agent_settings());
                assert!(!host.editor_state().editor_ui.agent_settings_open);
            }
        }
        assert!(host.agent_settings_touch_gesture.is_none());
        let _ = host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H);
        assert_eq!(
            host.editor_state().editor_ui.agent_settings.builtin_agents[0].enabled,
            before
        );
    }
}

#[test]
fn open_font_picker_owns_single_finger_scrolling() {
    let mut host = touch_settings_host();
    host.editor_state_mut().editor_ui.agent_settings.tab = AgentSettingsTab::Fonts;
    host.editor_state_mut().editor_ui.missing_fonts_prompt = Some(MissingFontsPrompt {
        entries: vec![MissingFontEntry {
            family: "Missing Sans".into(),
            run_count: 1,
            mismatch_note: None,
            resolved: false,
        }],
    });
    host.editor_state_mut()
        .editor_ui
        .open_missing_font_picker(0, op_editor_core::MissingFontSurface::Settings);
    host.editor_state_mut().editor_ui.system_font_families =
        std::sync::Arc::new((0..80).map(|index| format!("Font {index:02}")).collect());
    let panel = AgentSettingsPanel::for_editor(host.editor_state());
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let point = center(
        panel
            .font_picker_layout(rect)
            .expect("open settings font picker layout")
            .popup,
    );

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.agent_settings_touch_gesture.is_some());
    assert!(host.apply_cursor_move(point.x, point.y - 60.0));
    assert!(
        host.editor_state().editor_ui.font_picker.scroll.offset > 0.0,
        "the popup must scroll instead of the settings body"
    );
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.scroll_y.offset,
        0.0
    );
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
}
