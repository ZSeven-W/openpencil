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
