//! Agent providers + the chat model catalog entry.

/// Which CLI agent backs a model / chat turn. Ported verbatim from
/// shell-core's `agent_settings_state::AgentProvider`.
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
    /// Append-only: the persisted `connected` flags in `settings.json`
    /// are indexed positionally by this array, and
    /// `settings_io::migrate_connected_provider_flags` reads saved
    /// flags by the same index. Inserting in the middle silently
    /// reassigns a user's saved connections to the wrong providers —
    /// the DeepSeek Harness slot was therefore APPENDED at the tail
    /// (5 → 6) and positioned in the card list via [`AgentProvider::DISPLAY`]
    /// instead, exactly like `McpCli::ALL` / `McpCli::DISPLAY`.
    pub const ALL: [AgentProvider; 7] = [
        AgentProvider::ClaudeCode,
        AgentProvider::CodexCli,
        AgentProvider::OpenCode,
        AgentProvider::GithubCopilot,
        AgentProvider::Antigravity,
        AgentProvider::GrokBuild,
        AgentProvider::DeepSeekHarness,
    ];

    /// Card order on the Agents tab. Deliberately NOT the persistence
    /// order — paint and hit-test both walk this array (row `i` shows
    /// `DISPLAY[i]`) while the connect state is read through
    /// [`AgentProvider::index`] into the append-only `connected`
    /// layout. Product order: Claude Code, Codex, DeepSeek Harness,
    /// OpenCode, … (DeepSeek Harness sits above the generic ACP block).
    /// Kept a permutation of `ALL`; asserted by tests.
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
    /// provider's flag in `connected` (and in the persisted positional
    /// flag arrays).
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

    /// i18n key for the provider's subtitle.
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

/// One selectable model in the chat model picker. Ported from
/// shell-core's `chat_models::ModelEntry`.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelEntry {
    /// Which CLI agent backs this model — also picks the chat
    /// transport.
    pub provider: AgentProvider,
    /// Wire id passed to the CLI (e.g. `gpt-5.5`, `claude-sonnet-4-6`).
    pub value: String,
    /// Human label shown in the picker (e.g. `GPT-5.5`).
    pub display_name: String,
    /// `Some(id)` when this model belongs to a built-in API-key
    /// provider rather than an external CLI.
    pub builtin_provider_id: Option<String>,
    /// Display label for the built-in provider group (for example
    /// `MiniMax`). Kept separate from `display_name`, which is the
    /// model row label.
    pub builtin_provider_display_name: Option<String>,
}

impl ModelEntry {
    pub fn new(
        provider: AgentProvider,
        value: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            value: value.into(),
            display_name: display_name.into(),
            builtin_provider_id: None,
            builtin_provider_display_name: None,
        }
    }

    pub fn builtin(
        provider: AgentProvider,
        builtin_provider_id: impl Into<String>,
        value: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            value: value.into(),
            display_name: display_name.into(),
            builtin_provider_id: Some(builtin_provider_id.into()),
            builtin_provider_display_name: None,
        }
    }

    pub fn builtin_with_display_name(
        provider: AgentProvider,
        builtin_provider_id: impl Into<String>,
        builtin_provider_display_name: impl Into<String>,
        value: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            value: value.into(),
            display_name: display_name.into(),
            builtin_provider_id: Some(builtin_provider_id.into()),
            builtin_provider_display_name: Some(builtin_provider_display_name.into()),
        }
    }

    pub fn acp(acp_agent_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        let id = acp_agent_id.into();
        Self {
            provider: AgentProvider::CodexCli,
            value: format!("acp:{id}"),
            display_name: display_name.into(),
            builtin_provider_id: None,
            builtin_provider_display_name: None,
        }
    }

    pub fn acp_agent_id(&self) -> Option<&str> {
        self.value.strip_prefix("acp:")
    }

    /// Concrete wire model carried by a built-in catalog row.
    ///
    /// The provider id and model may both contain `:`, so parsing must strip
    /// the exact prefix built from the already-structured provider identity.
    pub fn builtin_model_id(&self) -> Option<&str> {
        let id = self.builtin_provider_id.as_deref()?;
        let prefix = format!("builtin:{id}:");
        self.value
            .strip_prefix(&prefix)
            .filter(|model| !model.trim().is_empty())
    }
}
