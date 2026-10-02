//! Settings over Home must expose mouse actions and own its whole scrim.

use super::WidgetHost;
use op_editor_core::agent_settings::{BuiltinAgentField, SettingsFocus};
use op_editor_ui::widgets::agent_settings_panel::{content_viewport, AgentSettingsPanel};
use op_editor_ui::Point2D;

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn settings_over_home() -> WidgetHost {
    let mut host = WidgetHost::new();
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    let ui = &mut host.editor_state.editor_ui;
    ui.home.visible = true;
    ui.home.composer_focused = false;
    ui.home.set_draft("keep this brief");
    ui.agent_settings_open = true;
    ui.agent_settings
        .add_builtin_agent_with_defaults("GLM", "sk-test", "glm-5.3-flash");
    host
}

#[test]
fn home_settings_hover_exposes_provider_edit_and_owns_the_following_press() {
    let mut host = settings_over_home();
    // An open settings modal remains above earlier Home overlays.
    host.editor_state.editor_ui.chat_model_picker.open = true;
    host.editor_state.editor_ui.account_ui_available = true;
    host.editor_state.editor_ui.login_modal_open = true;
    let panel = AgentSettingsPanel::for_web_editor(&host.editor_state);
    let content = content_viewport(panel.rect(W, H));
    let y = content.origin.y
        + op_editor_ui::widgets::agent_settings_panel::AGENTS_HERO_HEIGHT
        + 28.0
        + 28.0
        + 30.0;
    let point = Point2D::new(content.origin.x + content.size.x - 52.0, y);

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state
            .editor_ui
            .agent_settings
            .hover_builtin_agent,
        0
    );
    assert_eq!(host.editor_state.editor_ui.home.hover, None);
    assert!(host.apply_press(point.x, point.y, W, H));
    assert_eq!(
        host.editor_state.editor_ui.agent_settings.focus,
        Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::DisplayName,
        })
    );
    assert!(!host.editor_state.editor_ui.home.composer_focused);
    assert_eq!(host.editor_state.editor_ui.home.draft, "keep this brief");
    assert_eq!(host.editor_state.editor_ui.home.pressed, None);
}

#[test]
fn home_settings_scrim_clears_provider_hover_without_touching_home() {
    let mut host = settings_over_home();
    host.editor_state
        .editor_ui
        .agent_settings
        .hover_builtin_agent = 0;
    let blank = Point2D::new(8.0, H - 8.0);
    host.apply_cursor_move(blank.x, blank.y);
    assert_eq!(
        host.editor_state
            .editor_ui
            .agent_settings
            .hover_builtin_agent,
        usize::MAX
    );
    assert_eq!(host.editor_state.editor_ui.home.hover, None);
    assert!(host.apply_press(blank.x, blank.y, W, H));
    assert!(host.home_visible());
    assert!(!host.editor_state.editor_ui.home.composer_focused);
    assert_eq!(host.editor_state.editor_ui.home.pressed, None);
}

#[test]
fn home_settings_scroll_owns_wheel_and_pan_without_moving_home_or_canvas() {
    for pan in [false, true] {
        let mut host = settings_over_home();
        let ui = &mut host.editor_state.editor_ui;
        ui.home.scroll_y = 100.0;
        ui.chat_model_picker.open = true;
        ui.chat_model_picker.scroll.offset = 18.0;
        for index in 0..20 {
            ui.agent_settings.add_builtin_agent_with_defaults(
                format!("Provider {index}"),
                format!("sk-test-{index}"),
                "glm-5.3-flash",
            );
        }
        let panel = AgentSettingsPanel::for_web_editor(&host.editor_state);
        let rect = panel.rect(W, H);
        assert!(panel.max_scroll(rect) > 0.0, "fixture must overflow");
        let content = content_viewport(rect);
        let point = Point2D::new(
            content.origin.x + content.size.x / 2.0,
            content.origin.y + content.size.y / 2.0,
        );
        let viewport = host.editor_state.viewport;
        let scroll = |host: &mut WidgetHost, point: Point2D| {
            if pan {
                host.apply_pan_gesture(point.x, point.y, 45.0, -120.0, W, H)
            } else {
                host.apply_wheel(point.x, point.y, -120.0, W, H)
            }
        };
        assert!(scroll(&mut host, point));
        let ui = &host.editor_state.editor_ui;
        let settings_scroll = ui.agent_settings.scroll_y.offset;
        assert!(settings_scroll > 0.0);
        assert_eq!(ui.home.scroll_y, 100.0);
        assert_eq!(ui.chat_model_picker.scroll.offset, 18.0);
        assert_eq!(host.editor_state.viewport, viewport);

        scroll(&mut host, Point2D::new(4.0, H / 2.0));
        let ui = &host.editor_state.editor_ui;
        assert_eq!(ui.agent_settings.scroll_y.offset, settings_scroll);
        assert_eq!(ui.home.scroll_y, 100.0);
        assert_eq!(ui.chat_model_picker.scroll.offset, 18.0);
        assert_eq!(host.editor_state.viewport, viewport);
    }
}

#[test]
fn home_modal_keyboard_select_all_paste_and_backspace_stay_in_web_settings() {
    let mut host = settings_over_home();
    let ui = &mut host.editor_state.editor_ui;
    ui.home.composer_focused = true;
    ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::Model,
    });
    ui.settings_input.set_text("glm-5.2");
    assert!(host.apply_select_all());
    assert_eq!(
        host.focused_input_selected_text().as_deref(),
        Some("glm-5.2")
    );
    assert!(host.apply_paste_text("glm-5.3-flash"));
    assert_eq!(
        host.editor_state.editor_ui.settings_input.text(),
        "glm-5.3-flash"
    );
    assert!(host.apply_select_all());
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state.editor_ui.settings_input.text(), "");
    assert_eq!(host.editor_state.editor_ui.home.draft, "keep this brief");
    assert_eq!(
        host.editor_state.editor_ui.home.input.highlight_range(),
        None
    );
}
