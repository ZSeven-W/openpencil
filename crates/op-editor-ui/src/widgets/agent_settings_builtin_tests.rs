use crate::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
use crate::Point2D;
use op_editor_core::agent_settings::{BuiltinAgentField, SettingsFocus};
use op_editor_core::EditorState;

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
    let content_x = rect.origin.x + 200.0 + 24.0;
    let content_y = rect.origin.y + 24.0;
    let first_card_y = content_y + 12.0 + 28.0 + 28.0;
    let point = Point2D::new(content_x + 92.0, first_card_y + 170.0);

    assert_eq!(panel.hit_test(rect, point), AgentSettingsHit::Inside);
}
