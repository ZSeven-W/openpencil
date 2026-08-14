use super::WidgetHostNative;
use op_editor_core::agent_settings::AgentSettingsTab;
use op_editor_core::missing_fonts::{MissingFontEntry, MissingFontsPrompt};
use op_editor_core::size_class::EditorSizeClass;
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
