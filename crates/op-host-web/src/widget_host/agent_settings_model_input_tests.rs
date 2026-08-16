use super::WidgetHost;
use op_editor_core::agent_settings::{BuiltinAgentField, SettingsFocus};
use op_editor_ui::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

#[test]
fn web_model_editor_click_places_caret_on_an_earlier_visible_line() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.agent_settings_open = true;
    host.editor_state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Provider", "sk-test", "model-a");
    host.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::Model,
    });
    host.editor_state
        .editor_ui
        .settings_input
        .set_text("model-a\nmodel-b\nmodel-c\nmodel-d\nmodel-e");

    let point = {
        let panel = AgentSettingsPanel::for_web_editor(&host.editor_state);
        let panel_rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
        let mut input = panel.focused_input_rect(panel_rect).expect("model input");
        input.origin.y -= panel.effective_scroll(panel_rect);
        let point = Point2D::new(input.origin.x + 7.0, input.origin.y + 6.0);
        assert_eq!(
            panel.hit_test(panel_rect, point),
            AgentSettingsHit::FocusBuiltinAgent {
                index: 0,
                field: BuiltinAgentField::Model,
            }
        );
        point
    };

    assert!(host.dispatch_agent_settings_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H,));
    assert_eq!(
        host.editor_state.editor_ui.settings_input.caret(),
        "model-a\nmodel-b\n".len(),
        "the first of three visible desktop rows is model-c"
    );
}
