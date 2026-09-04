//! Design-launch attachment plumbing (C1 M2). Sibling test file — the main
//! `design_session_tests.rs` sits at the 800-line cap.
//!
//! Starting an orchestrator design turn must take the chat panel's staged
//! attachments so a reference screenshot reaches
//! `op_host_services::design_session::start`, and must clear
//! `chat.pending_attachments` so they can't leak into the next send.

use super::*;
use op_editor_core::{
    AgentProvider, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, ModelEntry,
};

#[test]
fn design_turn_launch_consumes_the_staged_attachments() {
    // The design branch's `start()` calls `agent_indicators::begin()` on
    // THIS thread before spawning its worker — a write to the same
    // process-global registry every other design-turn test in this binary
    // guards with this lock (see
    // `chat_session_launch_tests::launch_if_pending_stashes_the_design_request_on_the_cli_standard_route`).
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();

    let mut host = WidgetHostNative::new();
    // A ready builtin entry routes `launch_if_pending` down the builtin
    // design branch — the one that hands the attachments to
    // `design_session::start` (the CLI standard route moves them into the
    // chat request instead).
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .builtin_agents
        .push(BuiltinAgentConfig {
            id: "builtin-1".into(),
            preset: BuiltinAgentPresetKey::Custom,
            display_name: "MiniMax".into(),
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: "sk-test".into(),
            models: vec!["MiniMax-M3".into()],
            // Loopback-nothing port: the spawned worker fails fast in the
            // background; this test only asserts the synchronous launch.
            base_url: "http://localhost:9".into(),
            enabled: true,
        });
    host.editor_state_mut().chat.available_models = vec![ModelEntry::builtin(
        AgentProvider::ClaudeCode,
        "builtin-1",
        "builtin:builtin-1:MiniMax-M3",
        "MiniMax M3",
    )];
    host.editor_state_mut().chat.selected_model = 0;
    assert!(host
        .editor_state_mut()
        .chat
        .add_attachment(op_ai::chat_provider::ChatAttachment {
            name: "reference.png".into(),
            media_type: "image/png".into(),
            data: vec![1, 2, 3],
        }));
    // A dashboard prompt keeps the turn on the orchestrator path even when
    // the design-agent-loop heuristic is enabled (mobile/landing prompts
    // route to the loop instead — which also consumes the attachments).
    host.editor_state_mut()
        .chat
        .set_input_text("design an admin analytics dashboard web app");
    assert!(host.editor_state_mut().chat.begin_send());

    let mut current_chat = None;
    let mut current_design = None;
    let launched =
        crate::chat_session::launch_if_pending(&mut host, &mut current_chat, &mut current_design);

    assert!(launched, "a design turn must launch");
    assert!(
        host.editor_state().chat.pending_attachments.is_empty(),
        "launching the design turn must take the staged attachments"
    );

    op_editor_core::agent_indicators::clear();
}
