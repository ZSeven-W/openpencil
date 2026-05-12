//! State types for the multi-section settings modal opened by
//! Cmd+,. Lives outside `document.rs` to keep that file under
//! the 800-line cap.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentProvider {
    ClaudeCode,
    CodexCli,
    OpenCode,
    GithubCopilot,
    GeminiCli,
}

impl AgentProvider {
    pub const ALL: [AgentProvider; 5] = [
        AgentProvider::ClaudeCode,
        AgentProvider::CodexCli,
        AgentProvider::OpenCode,
        AgentProvider::GithubCopilot,
        AgentProvider::GeminiCli,
    ];
    pub fn name(self) -> &'static str {
        match self {
            AgentProvider::ClaudeCode => "Claude Code",
            AgentProvider::CodexCli => "Codex CLI",
            AgentProvider::OpenCode => "OpenCode",
            AgentProvider::GithubCopilot => "GitHub Copilot",
            AgentProvider::GeminiCli => "Gemini CLI",
        }
    }
    /// i18n key for the provider's subtitle. Paint code resolves
    /// the key via `agent_settings_i18n::t` so each locale picks
    /// up the right translation; the previous hard-coded Chinese
    /// strings + the fabricated `fini.yang@gmail.com` line have
    /// both been removed.
    pub fn subtitle_key(self) -> &'static str {
        match self {
            AgentProvider::ClaudeCode => "settings.provider.claudeCode",
            AgentProvider::CodexCli => "settings.provider.codexCli",
            AgentProvider::OpenCode => "settings.provider.openCode",
            AgentProvider::GithubCopilot => "settings.provider.githubCopilot",
            AgentProvider::GeminiCli => "settings.provider.geminiCli",
        }
    }
}

/// Terminal-side MCP integrations the user can flip on/off. Order
/// matches the TS app's MCP settings grid (Claude / Codex / Gemini
/// / OpenCode / Kiro / Copilot, 2 rows × 3 columns) so the index is
/// reusable for both layout and `mcp_cli_enabled[i]`.
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

/// MCP server status — surfaced verbatim on the MCP tab's top
/// card. Default port mirrors the TS app (`pen-mcp` default 3100).
#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub struct AgentSettings {
    pub tab: AgentSettingsTab,
    pub connected: [bool; 5],
    /// Vertical scroll offset of the right content pane in px.
    /// Clamped to `[0, content_height - viewport_height]` by the
    /// host on wheel input.
    pub scroll_y: f32,
    pub mcp_server: McpServer,
    pub mcp_cli_enabled: [bool; 6],
    pub images_advanced_open: bool,
    pub images_search_ready: bool,
    /// Index into `AgentProvider::ALL` of the card the cursor is
    /// currently hovering, used to flip the connect button into a
    /// red 断开连接 affordance on already-connected cards. `usize::MAX`
    /// means no card is hovered.
    pub hover_provider: usize,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            tab: AgentSettingsTab::Agents,
            // Connection state is wired manually for now — no auth
            // backend exists, so every provider starts disconnected.
            connected: [false; 5],
            scroll_y: 0.0,
            mcp_server: McpServer::default(),
            mcp_cli_enabled: [false; 6],
            images_advanced_open: true,
            images_search_ready: true,
            hover_provider: usize::MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsDrag {
    Reserved,
}
