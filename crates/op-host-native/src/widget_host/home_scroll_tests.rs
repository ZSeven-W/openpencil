//! Home and its hidden canvas must stay still while a modal owns scrolling.

use super::WidgetHostNative;
use op_editor_core::size_class::EditorSizeClass;
use op_editor_ui::Point2D;

fn settings_over_home(touch: bool, width: f32, height: f32) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.last_viewport_w = width;
    host.last_viewport_h = height;
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.touch = touch;
    if touch {
        ui.size_class = EditorSizeClass::Compact;
    }
    ui.home.visible = true;
    ui.home.scroll_y = 100.0;
    ui.agent_settings_open = true;
    // Settings may have been opened with Cmd+, above this existing picker.
    ui.chat_model_picker.open = true;
    ui.chat_model_picker.scroll.offset = 18.0;
    for index in 0..20 {
        ui.agent_settings.add_builtin_agent_with_defaults(
            format!("Provider {index}"),
            format!("sk-test-{index}"),
            "glm-5.3-flash",
        );
    }
    assert_eq!(ui.agent_settings.builtin_agents.len(), 20);
    host
}

fn scroll(host: &mut WidgetHostNative, pan: bool, point: Point2D, width: f32, height: f32) -> bool {
    if pan {
        host.apply_pan_gesture(point.x, point.y, 45.0, -120.0, width, height)
    } else {
        host.apply_wheel(point.x, point.y, -120.0, width, height)
    }
}

#[test]
fn home_settings_scroll_owns_wheel_and_pan_on_desktop_and_touch() {
    for (touch, width, height) in [(false, 1440.0, 900.0), (true, 390.0, 844.0)] {
        for pan in [false, true] {
            let mut host = settings_over_home(touch, width, height);
            let (panel, rect) = host.agent_settings_geometry(width, height);
            assert!(panel.max_scroll(rect) > 0.0, "fixture must overflow");
            let content = panel.resolved_content_viewport(rect);
            let point = Point2D::new(
                content.origin.x + content.size.x / 2.0,
                content.origin.y + content.size.y / 2.0,
            );
            let viewport = host.editor_state().viewport;

            assert!(scroll(&mut host, pan, point, width, height));
            let ui = &host.editor_state().editor_ui;
            assert!(ui.agent_settings.scroll_y.offset > 0.0);
            assert_eq!(ui.home.scroll_y, 100.0, "Home stays under the modal");
            assert_eq!(ui.chat_model_picker.scroll.offset, 18.0);
            assert_eq!(host.editor_state().viewport, viewport);

            // Outside the desktop modal, or in the touch modal's pinned
            // header, no underlying page or canvas may receive the delta.
            let settings_scroll = ui.agent_settings.scroll_y.offset;
            scroll(&mut host, pan, Point2D::new(4.0, 4.0), width, height);
            let ui = &host.editor_state().editor_ui;
            assert_eq!(ui.agent_settings.scroll_y.offset, settings_scroll);
            assert_eq!(ui.home.scroll_y, 100.0);
            assert_eq!(ui.chat_model_picker.scroll.offset, 18.0);
            assert_eq!(host.editor_state().viewport, viewport);
        }
    }
}

#[test]
fn home_settings_scroll_scrim_never_moves_the_home_page() {
    for pan in [false, true] {
        let (width, height) = (1440.0, 900.0);
        let mut host = settings_over_home(false, width, height);
        let viewport = host.editor_state().viewport;
        // Within Home's scrollable y range, but left of the settings card.
        scroll(
            &mut host,
            pan,
            Point2D::new(4.0, height / 2.0),
            width,
            height,
        );
        let ui = &host.editor_state().editor_ui;
        assert_eq!(ui.agent_settings.scroll_y.offset, 0.0);
        assert_eq!(ui.home.scroll_y, 100.0);
        assert_eq!(ui.chat_model_picker.scroll.offset, 18.0);
        assert_eq!(host.editor_state().viewport, viewport);
    }
}

#[test]
fn home_settings_scroll_one_finger_drag_uses_the_modal_gesture() {
    let (width, height) = (390.0, 844.0);
    let mut host = settings_over_home(true, width, height);
    let (panel, rect) = host.agent_settings_geometry(width, height);
    let content = panel.resolved_content_viewport(rect);
    let point = Point2D::new(
        content.origin.x + content.size.x / 2.0,
        content.origin.y + content.size.y / 2.0,
    );
    let viewport = host.editor_state().viewport;
    let enabled = host
        .editor_state()
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .map(|agent| agent.enabled)
        .collect::<Vec<_>>();

    assert!(host.apply_press(point.x, point.y, width, height));
    assert!(host.agent_settings_touch_gesture.is_some());
    assert!(host.apply_cursor_move(point.x, point.y - 100.0));
    assert!(host.apply_release_with_viewport(width, height));

    let ui = &host.editor_state().editor_ui;
    assert!(ui.agent_settings.scroll_y.offset > 0.0);
    assert_eq!(ui.home.scroll_y, 100.0);
    assert_eq!(ui.chat_model_picker.scroll.offset, 18.0);
    assert_eq!(host.editor_state().viewport, viewport);
    assert_eq!(
        ui.agent_settings
            .builtin_agents
            .iter()
            .map(|agent| agent.enabled)
            .collect::<Vec<_>>(),
        enabled,
        "dragging must not activate the provider under the initial press"
    );
    assert_eq!(ui.agent_settings.focus, None);
}
