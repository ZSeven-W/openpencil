//! Desktop settings persistence for the selected chat agent.
//!
//! The desktop boots through `op_host_services::settings_io::load`
//! (`app_state.rs`) and auto-saves through `save_if_changed`'s
//! fingerprint — these tests pin the chat-agent field through both
//! ends of that loop.

use op_host_native::WidgetHostNative;

#[test]
fn selected_chat_agent_survives_a_settings_save_and_load_cycle() {
    crate::test_config_root::guard_user_config();

    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.chat_selected_agent = 3; // OpenCode
    let before = op_host_services::settings_io::fingerprint(host.editor_state());
    op_host_services::settings_io::save(host.editor_state());

    let mut reloaded = op_editor_core::EditorState::new();
    op_host_services::settings_io::load(&mut reloaded);
    assert_eq!(
        reloaded.editor_ui.chat_selected_agent, 3,
        "the desktop boot path must restore the last-selected agent"
    );

    // The fingerprint must notice a change too — that is what the
    // desktop's `save_if_changed` loop keys its writes on.
    host.editor_state_mut().editor_ui.chat_selected_agent = 5;
    assert_ne!(
        before,
        op_host_services::settings_io::fingerprint(host.editor_state())
    );
}
