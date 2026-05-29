//! Agent-settings modal state for `EditorState`.
//!
//! Faithful copy of `openpencil-shell-core::agent_settings_state` —
//! the state types for the multi-section settings modal opened by
//! `Cmd+,`. These are plain data types (enums + a `Copy` struct of
//! primitives), no widget or platform coupling, so they move cleanly
//! into the wasm-clean editor-state layer.
//!
//! `AgentProvider` itself lives in `chat.rs` (it is also the chat
//! model's backing-agent discriminator) and is re-exported here so
//! both the chat layer and the settings layer share one definition.

pub use crate::chat::AgentProvider;

/// Which section of the settings modal is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsTab {
    Agents,
    Mcp,
    Images,
    System,
}

impl AgentSettingsTab {
    pub const ALL: [AgentSettingsTab; 4] = [
        AgentSettingsTab::Agents,
        AgentSettingsTab::Mcp,
        AgentSettingsTab::Images,
        AgentSettingsTab::System,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AgentSettingsTab::Agents => "Agents",
            AgentSettingsTab::Mcp => "MCP",
            AgentSettingsTab::Images => "Images",
            AgentSettingsTab::System => "系统",
        }
    }
}

/// Terminal-side MCP integrations the user can flip on/off. Order
/// matches the TS app's MCP settings grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpCli {
    ClaudeCode,
    Codex,
    Gemini,
    OpenCode,
    Kiro,
    GithubCopilot,
}

impl McpCli {
    pub const ALL: [McpCli; 6] = [
        McpCli::ClaudeCode,
        McpCli::Codex,
        McpCli::Gemini,
        McpCli::OpenCode,
        McpCli::Kiro,
        McpCli::GithubCopilot,
    ];

    pub fn label(self) -> &'static str {
        match self {
            McpCli::ClaudeCode => "Claude Code CLI",
            McpCli::Codex => "Codex CLI",
            McpCli::Gemini => "Gemini CLI",
            McpCli::OpenCode => "OpenCode CLI",
            McpCli::Kiro => "Kiro CLI",
            McpCli::GithubCopilot => "GitHub Copilot CLI",
        }
    }
}

/// MCP server status — surfaced on the MCP tab's top card. Default
/// port mirrors the TS app (`pen-mcp` default 3100).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpServer {
    pub running: bool,
    pub port: u16,
}

impl Default for McpServer {
    fn default() -> Self {
        Self {
            running: false,
            port: 3100,
        }
    }
}

/// Editable inputs on the settings modal that aren't tied to a `Node`
/// (so they don't fit the property-panel's `PropertyFocus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinAgentField {
    DisplayName,
    ApiKey,
    Model,
    BaseUrl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFocus {
    McpPort,
    BuiltinAgent {
        index: usize,
        field: BuiltinAgentField,
    },
}

/// Built-in provider backend configured directly in OpenPencil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinAgentKind {
    Anthropic,
    OpenAiCompat,
}

impl BuiltinAgentKind {
    pub fn default_base_url(self) -> &'static str {
        match self {
            BuiltinAgentKind::Anthropic => "https://api.anthropic.com",
            BuiltinAgentKind::OpenAiCompat => "https://api.openai.com/v1",
        }
    }

    pub fn model_provider(self) -> AgentProvider {
        match self {
            BuiltinAgentKind::Anthropic => AgentProvider::ClaudeCode,
            BuiltinAgentKind::OpenAiCompat => AgentProvider::CodexCli,
        }
    }
}

/// One configured built-in Agent/API-key provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinAgentConfig {
    pub id: String,
    pub display_name: String,
    pub kind: BuiltinAgentKind,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub enabled: bool,
}

impl BuiltinAgentConfig {
    pub fn ready(&self) -> bool {
        self.enabled && !self.api_key.trim().is_empty() && !self.model.trim().is_empty()
    }
}

/// State for the floating agent-settings modal.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentSettings {
    pub tab: AgentSettingsTab,
    pub connected: [bool; 5],
    pub builtin_agents: Vec<BuiltinAgentConfig>,
    pub next_builtin_agent_id: u64,
    /// Vertical scroll offset of the right content pane in px.
    pub scroll_y: f32,
    pub mcp_server: McpServer,
    pub mcp_cli_enabled: [bool; 6],
    pub images_advanced_open: bool,
    pub images_search_ready: bool,
    /// Currently-focused editable input on the modal.
    pub focus: Option<SettingsFocus>,
    /// Index into `AgentProvider::ALL` of the hovered card;
    /// `usize::MAX` means no card is hovered.
    pub hover_provider: usize,
    /// Index into `builtin_agents` of the hovered provider card.
    pub hover_builtin_agent: usize,
    /// Sidebar nav item under the cursor; `None` = no hover.
    pub hover_nav: Option<AgentSettingsTab>,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            tab: AgentSettingsTab::Agents,
            connected: [false; 5],
            builtin_agents: Vec::new(),
            next_builtin_agent_id: 1,
            scroll_y: 0.0,
            mcp_server: McpServer::default(),
            mcp_cli_enabled: [false; 6],
            images_advanced_open: true,
            images_search_ready: true,
            focus: None,
            hover_provider: usize::MAX,
            hover_builtin_agent: usize::MAX,
            hover_nav: None,
        }
    }
}

impl AgentSettings {
    pub fn add_builtin_agent(&mut self) -> String {
        let n = self.next_builtin_agent_id.max(1);
        let name = format!("Built-in Agent {n}");
        self.add_builtin_agent_with_defaults(&name, "", "claude-sonnet-4-5")
    }

    pub fn add_builtin_agent_with_defaults(
        &mut self,
        display_name: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> String {
        let id = format!("builtin-{}", self.next_builtin_agent_id.max(1));
        self.next_builtin_agent_id = self.next_builtin_agent_id.max(1).saturating_add(1);
        self.builtin_agents.push(BuiltinAgentConfig {
            id: id.clone(),
            display_name: display_name.into(),
            kind: BuiltinAgentKind::Anthropic,
            api_key: api_key.into(),
            model: model.into(),
            base_url: BuiltinAgentKind::Anthropic.default_base_url().into(),
            enabled: true,
        });
        id
    }
}

/// Drag state for the settings modal. Reserved — the modal does not
/// support dragging yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsDrag {
    Reserved,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_quiescent() {
        let s = AgentSettings::default();
        assert_eq!(s.tab, AgentSettingsTab::Agents);
        assert_eq!(s.connected, [false; 5]);
        assert!(s.builtin_agents.is_empty());
        assert_eq!(s.mcp_server.port, 3100);
        assert!(s.focus.is_none());
        assert_eq!(s.hover_provider, usize::MAX);
        assert_eq!(s.hover_builtin_agent, usize::MAX);
    }

    #[test]
    fn tab_and_cli_arrays_cover_all_variants() {
        assert_eq!(AgentSettingsTab::ALL.len(), 4);
        assert_eq!(McpCli::ALL.len(), 6);
    }
}
