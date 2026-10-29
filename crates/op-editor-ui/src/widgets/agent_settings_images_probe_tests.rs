use crate::widgets::agent_settings_panel::AgentSettingsHit;
use crate::widgets::agent_settings_press_flow::apply_agent_settings_hit;
use op_editor_core::agent_settings::{AgentSettingsTab, ImageGenProvider, ImageTestStatus};
use op_editor_core::host_settings_commit::SettingsCommitScope;
use op_editor_core::EditorState;

fn press_test(provider: ImageGenProvider) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.tab = AgentSettingsTab::Images;
    state.editor_ui.agent_settings.add_image_gen_profile();
    let profile = &mut state.editor_ui.agent_settings.image_gen_profiles[0];
    profile.provider = provider;
    profile.api_key = "test-key".into();
    profile.model = "doubao-seedream-5-0".into();
    profile.base_url = Some("https://ark.cn-beijing.volces.com/api/v3".into());
    apply_agent_settings_hit(
        &mut state,
        AgentSettingsHit::TestGenConfig(0),
        SettingsCommitScope::Operator,
        0,
    );
    state
}

#[test]
fn openai_compatible_test_press_queues_a_real_probe() {
    // A Custom (e.g. Seedream) or OpenAI profile used to flip to Testing
    // with nothing behind it, spinning forever and catching no bad URL.
    for provider in [ImageGenProvider::Custom, ImageGenProvider::OpenAi] {
        let state = press_test(provider);
        let settings = &state.editor_ui.agent_settings;
        assert_eq!(
            settings.image_gen_profiles[0].test_status,
            ImageTestStatus::Testing
        );
        assert_eq!(
            settings.pending_image_gen_test.as_deref(),
            Some(settings.image_gen_profiles[0].id.as_str()),
            "{provider:?} test press must raise the host probe seam"
        );
    }
}

#[test]
fn providers_without_a_probe_do_not_raise_the_seam() {
    let state = press_test(ImageGenProvider::Gemini);
    assert!(state
        .editor_ui
        .agent_settings
        .pending_image_gen_test
        .is_none());
}
