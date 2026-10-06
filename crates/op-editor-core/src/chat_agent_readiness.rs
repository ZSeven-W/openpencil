//! Pre-flight readiness for the chat panel's selected agent.
//!
//! Some CLI agents cannot run a canvas turn at all until a matching MCP
//! integration is switched on in Settings. Antigravity is the measured case:
//! every canvas turn reads `mcpServers.openpencil` out of the user's
//! `~/.gemini/config/mcp_config.json`, and that entry only exists while the
//! Antigravity row on the Settings MCP tab is enabled. Without the toggle the
//! turn dies with "Antigravity requires OpenPencil MCP to be enabled in
//! Settings" — but only AFTER the user has written and sent a whole prompt.
//!
//! This module answers the same question BEFORE the send, so the panel can
//! say so up front instead of spending the user's prompt to find out.

use crate::agent_settings::McpCli;
use crate::chat::models::AgentProvider;
use crate::EditorUiState;

impl EditorUiState {
    /// The MCP integration the currently selected chat agent needs but does
    /// not have, or `None` when the selection can run a canvas turn.
    ///
    /// Only agents whose canvas turn is HARD-GATED on the integration are
    /// reported. An agent that merely benefits from MCP is not a gap: a
    /// warning the user cannot act on is worse than none.
    pub fn chat_agent_mcp_gap(&self) -> Option<McpCli> {
        let provider = AgentProvider::ALL.get(self.chat_selected_agent).copied()?;
        let required = required_mcp_cli(provider)?;
        if self.agent_settings.mcp_cli_enabled[required.index()] {
            return None;
        }
        Some(required)
    }
}

/// The MCP integration a provider's canvas turn cannot start without.
///
/// Antigravity is the only one today: `prepare_antigravity_home` in
/// `op-host-services` refuses the turn outright when the entry is missing.
/// Grok Build also talks to OpenPencil over MCP but degrades to a tool-free
/// turn instead of failing, so it is deliberately not listed.
fn required_mcp_cli(provider: AgentProvider) -> Option<McpCli> {
    match provider {
        AgentProvider::Antigravity => Some(McpCli::Antigravity),
        _ => None,
    }
}

impl crate::EditorState {
    /// True when at least one chat agent can actually answer a turn: a
    /// built-in API-key agent that is enabled with a key, a connected CLI
    /// provider, or an enabled ACP agent. The Home sheet uses this to swap
    /// its send affordance for the 接入卡 instead of letting a first-run
    /// send fail silently.
    pub fn has_usable_chat_agent(&self) -> bool {
        let settings = &self.editor_ui.agent_settings;
        settings
            .builtin_agents
            .iter()
            .any(|agent| agent.enabled && !agent.api_key.trim().is_empty())
            || settings.connected.iter().any(|connected| *connected)
            || settings.acp_agents.iter().any(|agent| agent.enabled)
    }
}

#[cfg(test)]
#[path = "chat_agent_readiness_tests.rs"]
mod tests;
