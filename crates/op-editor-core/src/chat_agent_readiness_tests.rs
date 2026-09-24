use super::*;
use crate::EditorState;

fn state_with_agent(agent: AgentProvider) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.chat_selected_agent = AgentProvider::ALL
        .iter()
        .position(|candidate| *candidate == agent)
        .expect("every provider is registered");
    state
}

#[test]
fn antigravity_without_the_mcp_toggle_reports_the_gap() {
    let state = state_with_agent(AgentProvider::Antigravity);
    assert!(!state.editor_ui.agent_settings.mcp_cli_enabled[McpCli::Antigravity.index()]);
    assert_eq!(
        state.editor_ui.chat_agent_mcp_gap(),
        Some(McpCli::Antigravity),
        "a canvas turn cannot start without the integration, so the panel must say so"
    );
}

#[test]
fn enabling_the_toggle_clears_the_gap() {
    let mut state = state_with_agent(AgentProvider::Antigravity);
    state.editor_ui.agent_settings.mcp_cli_enabled[McpCli::Antigravity.index()] = true;
    assert_eq!(state.editor_ui.chat_agent_mcp_gap(), None);
}

#[test]
fn the_gap_is_read_through_the_positional_index_not_a_literal() {
    // Guards the append-only `mcp_cli_enabled` contract: flipping the flag at
    // Antigravity's registered index — whatever that index becomes — is what
    // clears the notice. A hard-coded 5 here would silently pass while the
    // product read someone else's toggle.
    let mut state = state_with_agent(AgentProvider::Antigravity);
    for (index, flag) in state
        .editor_ui
        .agent_settings
        .mcp_cli_enabled
        .iter_mut()
        .enumerate()
    {
        *flag = index != McpCli::Antigravity.index();
    }
    assert_eq!(
        state.editor_ui.chat_agent_mcp_gap(),
        Some(McpCli::Antigravity),
        "every OTHER integration being on must not clear Antigravity's gap"
    );
}

#[test]
fn agents_that_do_not_hard_require_mcp_report_nothing() {
    // Grok Build talks to OpenPencil over MCP but degrades to a tool-free
    // turn, and Claude Code needs no integration at all. Neither may raise a
    // notice the user cannot act on.
    for agent in [
        AgentProvider::ClaudeCode,
        AgentProvider::CodexCli,
        AgentProvider::GrokBuild,
        AgentProvider::DeepSeekHarness,
    ] {
        let state = state_with_agent(agent);
        assert_eq!(
            state.editor_ui.chat_agent_mcp_gap(),
            None,
            "{agent:?} must not raise an MCP notice"
        );
    }
}

fn builtin_agent(api_key: &str, enabled: bool) -> crate::BuiltinAgentConfig {
    crate::BuiltinAgentConfig {
        id: "builtin-1".into(),
        preset: crate::BuiltinAgentPresetKey::Custom,
        display_name: "Custom".into(),
        kind: crate::BuiltinAgentKind::OpenAiCompat,
        api_key: api_key.into(),
        models: vec!["model-a".into()],
        base_url: "http://localhost:9".into(),
        enabled,
    }
}

#[test]
fn has_usable_chat_agent_is_false_on_a_fresh_state() {
    let state = EditorState::default();
    assert!(
        !state.has_usable_chat_agent(),
        "a first-run machine has no answering agent — Home must offer the connect card"
    );
}

#[test]
fn has_usable_chat_agent_matrix_over_every_agent_source() {
    // Built-in: enabled with a key is usable; disabled, or enabled with a
    // blank key, is not.
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .builtin_agents
        .push(builtin_agent("sk-live", true));
    assert!(state.has_usable_chat_agent());
    state.editor_ui.agent_settings.builtin_agents[0].enabled = false;
    assert!(
        !state.has_usable_chat_agent(),
        "a disabled agent cannot answer"
    );
    state.editor_ui.agent_settings.builtin_agents[0].enabled = true;
    state.editor_ui.agent_settings.builtin_agents[0].api_key = "   ".into();
    assert!(
        !state.has_usable_chat_agent(),
        "an enabled agent without a key cannot answer"
    );

    // CLI: any connected provider flag is usable.
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.connected[0] = true;
    assert!(state.has_usable_chat_agent());

    // ACP: any enabled entry is usable.
    let mut state = EditorState::default();
    state
        .editor_ui
        .agent_settings
        .acp_agents
        .push(crate::AcpAgentConfig {
            id: "acp-1".into(),
            display_name: "Zed AI".into(),
            connection_type: crate::AcpConnectionType::Local,
            command: "zed-ai".into(),
            args: Vec::new(),
            env: Default::default(),
            url: None,
            enabled: true,
            connected: false,
        });
    assert!(state.has_usable_chat_agent());
}

#[test]
fn launch_route_defaults_to_auto_and_only_orchestrator_bypasses_the_loop() {
    assert_eq!(
        crate::ChatState::default().launch_route,
        crate::LaunchRoute::Auto
    );
    assert!(!crate::LaunchRoute::Auto.bypasses_design_agent_loop());
    assert!(crate::LaunchRoute::Orchestrator.bypasses_design_agent_loop());
    // The pending-family reset also re-arms the route.
    let mut chat = crate::ChatState {
        launch_route: crate::LaunchRoute::Orchestrator,
        ..crate::ChatState::default()
    };
    chat.new_chat();
    assert_eq!(chat.launch_route, crate::LaunchRoute::Auto);
}

#[test]
fn only_the_refine_route_forces_an_in_place_edit() {
    assert!(crate::LaunchRoute::Refine.forces_in_place_refine());
    assert!(!crate::LaunchRoute::Auto.forces_in_place_refine());
    assert!(!crate::LaunchRoute::Orchestrator.forces_in_place_refine());
    // A refine is never a new design: it must not claim design intent.
    assert!(!crate::LaunchRoute::Refine.implies_design_intent());
}

#[test]
fn a_daemon_that_serves_models_makes_the_web_host_usable() {
    // The browser keeps no provider key of its own, but the serving daemon
    // answers with its server-held one: Studio Home must offer Start, not
    // the connect card.
    let mut state = EditorState::new();
    assert!(!state.has_usable_chat_agent());
    state.editor_ui.agent_settings.web_served_models = true;
    assert!(state.has_usable_chat_agent());
}
