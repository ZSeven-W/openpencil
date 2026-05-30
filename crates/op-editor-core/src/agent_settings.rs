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

use std::collections::BTreeMap;

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
pub enum ImageGenField {
    Name,
    ApiKey,
    Model,
    BaseUrl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSearchField {
    ClientId,
    ClientSecret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFocus {
    McpPort,
    ImageSearch(ImageSearchField),
    BuiltinAgent {
        index: usize,
        field: BuiltinAgentField,
    },
    ImageGenProfile {
        index: usize,
        field: ImageGenField,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinAgentPreset {
    pub display_name: &'static str,
    pub kind: BuiltinAgentKind,
    pub model: &'static str,
    pub base_url: &'static str,
}

pub const BUILTIN_AGENT_PRESETS: [BuiltinAgentPreset; 4] = [
    BuiltinAgentPreset {
        display_name: "MINIMAX",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "MiniMax-M2.7",
        base_url: "https://api.minimaxi.com/v1",
    },
    BuiltinAgentPreset {
        display_name: "百炼CP",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "qwen3-coder-plus",
        base_url: "https://coding.dashscope.aliyuncs.com/v1",
    },
    BuiltinAgentPreset {
        display_name: "方舟CP",
        kind: BuiltinAgentKind::Anthropic,
        model: "ark-code-latest",
        base_url: "https://ark.cn-beijing.volces.com/api/coding",
    },
    BuiltinAgentPreset {
        display_name: "DS",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "deepseek-v4-pro",
        base_url: "https://api.deepseek.com/v1",
    },
];

/// ACP-compatible agent connection style mirrored from the TS
/// `AcpAgentConfig.connectionType` union.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpConnectionType {
    Local,
    Remote,
}

/// One configured ACP-compatible external agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpAgentConfig {
    pub id: String,
    pub display_name: String,
    pub connection_type: AcpConnectionType,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub url: Option<String>,
    pub enabled: bool,
}

/// Image-generation service providers mirrored from the TS
/// `ImageGenProvider` union.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageGenProvider {
    OpenAi,
    Gemini,
    Replicate,
    Custom,
}

impl ImageGenProvider {
    pub const ALL: [ImageGenProvider; 4] = [
        ImageGenProvider::OpenAi,
        ImageGenProvider::Gemini,
        ImageGenProvider::Replicate,
        ImageGenProvider::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ImageGenProvider::OpenAi => "OpenAI",
            ImageGenProvider::Gemini => "Google Gemini",
            ImageGenProvider::Replicate => "Replicate",
            ImageGenProvider::Custom => "Custom",
        }
    }

    pub fn default_model_placeholder(self) -> &'static str {
        match self {
            ImageGenProvider::OpenAi => "dall-e-3",
            ImageGenProvider::Gemini => "gemini-2.0-flash-preview-image-generation",
            ImageGenProvider::Replicate => "black-forest-labs/flux-1.1-pro",
            ImageGenProvider::Custom => "model-name",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

/// One image-generation configuration profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageGenProfile {
    pub id: String,
    pub name: String,
    pub provider: ImageGenProvider,
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
}

/// State for the floating agent-settings modal.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentSettings {
    pub tab: AgentSettingsTab,
    pub connected: [bool; 5],
    pub builtin_agents: Vec<BuiltinAgentConfig>,
    pub next_builtin_agent_id: u64,
    pub acp_agents: Vec<AcpAgentConfig>,
    pub next_acp_agent_id: u64,
    /// Vertical scroll offset of the right content pane in px.
    pub scroll_y: f32,
    pub mcp_server: McpServer,
    pub mcp_cli_enabled: [bool; 6],
    pub images_advanced_open: bool,
    pub images_search_ready: bool,
    pub openverse_client_id: String,
    pub openverse_client_secret: String,
    pub image_gen_profiles: Vec<ImageGenProfile>,
    pub active_image_gen_profile_id: Option<String>,
    pub next_image_gen_profile_id: u64,
    /// Whether the desktop host should check GitHub releases on
    /// startup. Manual "Check for Updates" stays available.
    pub auto_update_enabled: bool,
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
            acp_agents: Vec::new(),
            next_acp_agent_id: 1,
            scroll_y: 0.0,
            mcp_server: McpServer::default(),
            mcp_cli_enabled: [false; 6],
            images_advanced_open: false,
            images_search_ready: true,
            openverse_client_id: String::new(),
            openverse_client_secret: String::new(),
            image_gen_profiles: Vec::new(),
            active_image_gen_profile_id: None,
            next_image_gen_profile_id: 1,
            auto_update_enabled: true,
            focus: None,
            hover_provider: usize::MAX,
            hover_builtin_agent: usize::MAX,
            hover_nav: None,
        }
    }
}

impl AgentSettings {
    pub fn add_builtin_agent(&mut self) -> String {
        if let Some(preset) = BUILTIN_AGENT_PRESETS.iter().find(|preset| {
            !self
                .builtin_agents
                .iter()
                .any(|agent| agent.display_name == preset.display_name)
        }) {
            return self.add_builtin_agent_config(
                preset.display_name,
                "",
                preset.model,
                preset.kind,
                preset.base_url,
            );
        }
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
        self.add_builtin_agent_config(
            display_name,
            api_key,
            model,
            BuiltinAgentKind::Anthropic,
            BuiltinAgentKind::Anthropic.default_base_url(),
        )
    }

    pub fn add_builtin_agent_config(
        &mut self,
        display_name: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        kind: BuiltinAgentKind,
        base_url: impl Into<String>,
    ) -> String {
        let id = format!("builtin-{}", self.next_builtin_agent_id.max(1));
        self.next_builtin_agent_id = self.next_builtin_agent_id.max(1).saturating_add(1);
        self.builtin_agents.push(BuiltinAgentConfig {
            id: id.clone(),
            display_name: display_name.into(),
            kind,
            api_key: api_key.into(),
            model: model.into(),
            base_url: base_url.into(),
            enabled: true,
        });
        id
    }

    pub fn add_acp_agent(&mut self) -> String {
        let n = self.next_acp_agent_id.max(1);
        self.add_acp_agent_config(
            format!("ACP Agent {n}"),
            AcpConnectionType::Local,
            "",
            Vec::new(),
            BTreeMap::new(),
            None,
            true,
        )
    }

    pub fn add_acp_agent_config(
        &mut self,
        display_name: impl Into<String>,
        connection_type: AcpConnectionType,
        command: impl Into<String>,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        url: Option<String>,
        enabled: bool,
    ) -> String {
        let id = format!("acp-{}", self.next_acp_agent_id.max(1));
        self.next_acp_agent_id = self.next_acp_agent_id.max(1).saturating_add(1);
        self.acp_agents.push(AcpAgentConfig {
            id: id.clone(),
            display_name: display_name.into(),
            connection_type,
            command: command.into(),
            args,
            env,
            url,
            enabled,
        });
        id
    }

    pub fn add_image_gen_profile(&mut self) -> String {
        let n = self.next_image_gen_profile_id.max(1);
        let id = format!("igp-{n}");
        self.next_image_gen_profile_id = n.saturating_add(1);
        self.image_gen_profiles.push(ImageGenProfile {
            id: id.clone(),
            name: format!("Config {n}"),
            provider: ImageGenProvider::OpenAi,
            api_key: String::new(),
            model: String::new(),
            base_url: None,
        });
        if self.active_image_gen_profile_id.is_none() {
            self.active_image_gen_profile_id = Some(id.clone());
        }
        id
    }

    pub fn set_active_image_gen_profile(&mut self, id: &str) -> bool {
        if self.image_gen_profiles.iter().any(|p| p.id == id) {
            self.active_image_gen_profile_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    pub fn remove_image_gen_profile(&mut self, id: &str) -> bool {
        let before = self.image_gen_profiles.len();
        self.image_gen_profiles.retain(|p| p.id != id);
        if self.image_gen_profiles.len() == before {
            return false;
        }
        if self.active_image_gen_profile_id.as_deref() == Some(id) {
            self.active_image_gen_profile_id =
                self.image_gen_profiles.first().map(|p| p.id.clone());
        }
        true
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
        assert!(s.image_gen_profiles.is_empty());
        assert!(s.active_image_gen_profile_id.is_none());
        assert!(!s.images_advanced_open);
        assert!(s.openverse_client_id.is_empty());
        assert!(s.openverse_client_secret.is_empty());
        assert_eq!(s.mcp_server.port, 3100);
        assert!(s.auto_update_enabled);
        assert!(s.focus.is_none());
        assert_eq!(s.hover_provider, usize::MAX);
        assert_eq!(s.hover_builtin_agent, usize::MAX);
    }

    #[test]
    fn tab_and_cli_arrays_cover_all_variants() {
        assert_eq!(AgentSettingsTab::ALL.len(), 4);
        assert_eq!(McpCli::ALL.len(), 6);
    }

    #[test]
    fn image_generation_profiles_follow_ts_lifecycle() {
        let mut s = AgentSettings::default();

        let first = s.add_image_gen_profile();
        assert_eq!(s.image_gen_profiles.len(), 1);
        assert_eq!(
            s.active_image_gen_profile_id.as_deref(),
            Some(first.as_str())
        );
        assert_eq!(s.image_gen_profiles[0].name, "Config 1");
        assert_eq!(s.image_gen_profiles[0].provider, ImageGenProvider::OpenAi);

        let second = s.add_image_gen_profile();
        assert_eq!(s.image_gen_profiles.len(), 2);
        assert_eq!(
            s.active_image_gen_profile_id.as_deref(),
            Some(first.as_str())
        );

        assert!(s.set_active_image_gen_profile(&second));
        assert_eq!(
            s.active_image_gen_profile_id.as_deref(),
            Some(second.as_str())
        );

        assert!(s.remove_image_gen_profile(&second));
        assert_eq!(
            s.active_image_gen_profile_id.as_deref(),
            Some(first.as_str())
        );

        assert!(s.remove_image_gen_profile(&first));
        assert!(s.image_gen_profiles.is_empty());
        assert!(s.active_image_gen_profile_id.is_none());
    }

    #[test]
    fn add_builtin_agent_prefills_coding_provider_presets_first() {
        let mut s = AgentSettings::default();

        for _ in 0..4 {
            s.add_builtin_agent();
        }

        let summary: Vec<_> = s
            .builtin_agents
            .iter()
            .map(|agent| {
                (
                    agent.display_name.as_str(),
                    agent.kind,
                    agent.model.as_str(),
                    agent.base_url.as_str(),
                    agent.api_key.as_str(),
                )
            })
            .collect();

        assert_eq!(
            summary,
            vec![
                (
                    "MINIMAX",
                    BuiltinAgentKind::OpenAiCompat,
                    "MiniMax-M2.7",
                    "https://api.minimaxi.com/v1",
                    "",
                ),
                (
                    "百炼CP",
                    BuiltinAgentKind::OpenAiCompat,
                    "qwen3-coder-plus",
                    "https://coding.dashscope.aliyuncs.com/v1",
                    "",
                ),
                (
                    "方舟CP",
                    BuiltinAgentKind::Anthropic,
                    "ark-code-latest",
                    "https://ark.cn-beijing.volces.com/api/coding",
                    "",
                ),
                (
                    "DS",
                    BuiltinAgentKind::OpenAiCompat,
                    "deepseek-v4-pro",
                    "https://api.deepseek.com/v1",
                    "",
                ),
            ]
        );
    }

    #[test]
    fn add_acp_agent_assigns_id_and_defaults_to_local_config() {
        let mut s = AgentSettings::default();

        let first = s.add_acp_agent();
        let second = s.add_acp_agent();

        assert_eq!(first, "acp-1");
        assert_eq!(second, "acp-2");
        assert_eq!(s.acp_agents.len(), 2);
        assert_eq!(s.next_acp_agent_id, 3);
        assert_eq!(s.acp_agents[0].display_name, "ACP Agent 1");
        assert_eq!(s.acp_agents[0].connection_type, AcpConnectionType::Local);
        assert!(s.acp_agents[0].command.is_empty());
        assert!(s.acp_agents[0].args.is_empty());
        assert!(s.acp_agents[0].env.is_empty());
        assert!(s.acp_agents[0].url.is_none());
        assert!(s.acp_agents[0].enabled);
    }
}
