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
    Antigravity,
    GrokBuild,
    /// DeepSeek Harness (`dsh`) — one-shot subprocess CLI, no ACP.
    DeepSeekHarness,
}

impl AgentProvider {
    /// Append-only (mirror of `op_editor_core::chat::models`):
    /// persisted `connected` flags are indexed positionally, so the
    /// DeepSeek Harness slot was appended at the tail and positioned in
    /// the card list via [`AgentProvider::DISPLAY`].
    pub const ALL: [AgentProvider; 7] = [
        AgentProvider::ClaudeCode,
        AgentProvider::CodexCli,
        AgentProvider::OpenCode,
        AgentProvider::GithubCopilot,
        AgentProvider::Antigravity,
        AgentProvider::GrokBuild,
        AgentProvider::DeepSeekHarness,
    ];

    /// Card order on the Agents tab — a permutation of `ALL`, NOT the
    /// persistence order. DeepSeek Harness renders after Codex, before
    /// OpenCode (above the generic ACP block).
    pub const DISPLAY: [AgentProvider; 7] = [
        AgentProvider::ClaudeCode,
        AgentProvider::CodexCli,
        AgentProvider::DeepSeekHarness,
        AgentProvider::OpenCode,
        AgentProvider::GithubCopilot,
        AgentProvider::Antigravity,
        AgentProvider::GrokBuild,
    ];

    /// Position in [`AgentProvider::ALL`] — the index of this
    /// provider's flag in the persisted positional flag arrays.
    pub fn index(self) -> usize {
        AgentProvider::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .expect("every AgentProvider variant is registered in AgentProvider::ALL")
    }

    pub fn name(self) -> &'static str {
        match self {
            AgentProvider::ClaudeCode => "Claude Code",
            AgentProvider::CodexCli => "Codex CLI",
            AgentProvider::OpenCode => "OpenCode",
            AgentProvider::GithubCopilot => "GitHub Copilot",
            AgentProvider::Antigravity => "Antigravity",
            AgentProvider::GrokBuild => "Grok Build",
            AgentProvider::DeepSeekHarness => "DeepSeek Harness",
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
            AgentProvider::Antigravity => "settings.provider.antigravity",
            AgentProvider::GrokBuild => "settings.provider.grokBuild",
            AgentProvider::DeepSeekHarness => "settings.provider.deepSeekHarness",
        }
    }
}

/// Terminal-side MCP integrations the user can flip on/off. Order
/// matches the product's MCP settings grid (Claude / Codex / OpenCode /
/// Kiro / Copilot / Antigravity / Grok Build / Gemini / Qwen / Cursor /
/// Kimi / ZCode / DeepSeek Harness) so the index is reusable for both
/// layout and `mcp_cli_enabled[i]`. Kept in sync with
/// `op_editor_core::agent_settings::McpCli`; append only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpCli {
    ClaudeCode,
    Codex,
    OpenCode,
    Kiro,
    GithubCopilot,
    Antigravity,
    GrokBuild,
    GeminiCli,
    QwenCode,
    Cursor,
    Kimi,
    ZCode,
    Dsh,
}

impl McpCli {
    pub const ALL: [McpCli; 13] = [
        McpCli::ClaudeCode,
        McpCli::Codex,
        McpCli::OpenCode,
        McpCli::Kiro,
        McpCli::GithubCopilot,
        McpCli::Antigravity,
        McpCli::GrokBuild,
        McpCli::GeminiCli,
        McpCli::QwenCode,
        McpCli::Cursor,
        McpCli::Kimi,
        McpCli::ZCode,
        McpCli::Dsh,
    ];
    pub fn label(self) -> &'static str {
        match self {
            McpCli::ClaudeCode => "Claude Code CLI",
            McpCli::Codex => "Codex CLI",
            McpCli::OpenCode => "OpenCode CLI",
            McpCli::Kiro => "Kiro CLI",
            McpCli::GithubCopilot => "GitHub Copilot CLI",
            McpCli::Antigravity => "Antigravity CLI",
            McpCli::GrokBuild => "Grok Build CLI",
            McpCli::GeminiCli => "Gemini CLI",
            McpCli::QwenCode => "Qwen Code CLI",
            McpCli::Cursor => "Cursor",
            McpCli::Kimi => "Kimi CLI",
            McpCli::ZCode => "ZCode",
            McpCli::Dsh => "DeepSeek Harness",
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

/// Editable inputs on the settings modal that aren't tied to a
/// `Node` (so they don't fit the property-panel's `PropertyFocus`).
/// Currently just the MCP server port; OAuth fields will likely
/// reuse this enum when wired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFocus {
    McpPort,
}

#[derive(Debug, Clone, Copy)]
pub struct AgentSettings {
    pub tab: AgentSettingsTab,
    pub connected: [bool; 7],
    /// Vertical scroll offset of the right content pane in px.
    /// Clamped to `[0, content_height - viewport_height]` by the
    /// host on wheel input.
    pub scroll_y: f32,
    pub mcp_server: McpServer,
    pub mcp_cli_enabled: [bool; 13],
    pub images_advanced_open: bool,
    pub images_search_ready: bool,
    /// Currently-focused editable input on the modal. `None` while
    /// nothing is in edit mode; flips to `Some(SettingsFocus::*)`
    /// on click and back to `None` on Enter/Escape/outside-click.
    pub focus: Option<SettingsFocus>,
    /// Index into `AgentProvider::ALL` of the card the cursor is
    /// currently hovering, used to flip the connect button into a
    /// red disconnect affordance on already-connected cards. `usize::MAX`
    /// means no card is hovered.
    pub hover_provider: usize,
    /// Sidebar nav item under the cursor (Agents / MCP / Images /
    /// System). `None` = no hover. Drives a `muted`-tinted background
    /// on the row so the user sees which tab the next click hits.
    pub hover_nav: Option<AgentSettingsTab>,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            tab: AgentSettingsTab::Agents,
            // Connection state is wired manually for now — no auth
            // backend exists, so every provider starts disconnected.
            connected: [false; 7],
            scroll_y: 0.0,
            mcp_server: McpServer::default(),
            mcp_cli_enabled: [false; 13],
            images_advanced_open: true,
            images_search_ready: true,
            focus: None,
            hover_provider: usize::MAX,
            hover_nav: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsDrag {
    Reserved,
}
