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

use crate::agent_settings_builtin_presets::{
    builtin_agent_preset, infer_builtin_agent_preset, BuiltinAgentPresetKey, BUILTIN_AGENT_PRESETS,
};
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
            AgentSettingsTab::System => "System",
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

impl McpServer {
    pub fn client_config_display_text(self) -> String {
        format!(
            r#"{{ "type": "http", "url": "http://127.0.0.1:{}/mcp" }}"#,
            self.port
        )
    }

    pub fn client_config_clipboard_text(self) -> String {
        format!(
            "{{\n  \"type\": \"http\",\n  \"url\": \"http://127.0.0.1:{}/mcp\"\n}}",
            self.port
        )
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
pub enum ImageTestStatus {
    Idle,
    Testing,
    Valid,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpAgentField {
    DisplayName,
    Command,
    Args,
    Env,
    Url,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFocus {
    McpPort,
    ImageSearch(ImageSearchField),
    BuiltinAgent {
        index: usize,
        field: BuiltinAgentField,
    },
    BuiltinAgentDraft(BuiltinAgentField),
    ImageGenProfile {
        index: usize,
        field: ImageGenField,
    },
    AcpAgent {
        index: usize,
        field: AcpAgentField,
    },
    AcpAgentDraft(AcpAgentField),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinAgentPresetMenuTarget {
    Agent(usize),
    Draft,
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
    pub preset: BuiltinAgentPresetKey,
    pub display_name: String,
    pub kind: BuiltinAgentKind,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub enabled: bool,
}

impl BuiltinAgentConfig {
    pub fn base_url_editable(&self) -> bool {
        !matches!(
            self.preset,
            BuiltinAgentPresetKey::Anthropic | BuiltinAgentPresetKey::OpenAi
        )
    }

    pub fn ready(&self) -> bool {
        self.enabled && !self.api_key.trim().is_empty() && !self.model.trim().is_empty()
    }

    pub fn matches_config(
        &self,
        display_name: &str,
        api_key: &str,
        model: &str,
        kind: BuiltinAgentKind,
        base_url: &str,
    ) -> bool {
        self.kind == kind
            && self.display_name.trim() == display_name.trim()
            && self.api_key.trim() == api_key.trim()
            && self.model.trim() == model.trim()
            && self.base_url.trim().trim_end_matches('/') == base_url.trim().trim_end_matches('/')
    }

    pub fn matches_add_candidate(
        &self,
        _display_name: &str,
        api_key: &str,
        model: &str,
        kind: BuiltinAgentKind,
        base_url: &str,
    ) -> bool {
        self.matches_backend(api_key, model, kind, base_url)
    }

    fn matches_backend(
        &self,
        api_key: &str,
        model: &str,
        kind: BuiltinAgentKind,
        base_url: &str,
    ) -> bool {
        self.kind == kind
            && self.api_key.trim() == api_key.trim()
            && self.model.trim() == model.trim()
            && self.base_url.trim().trim_end_matches('/') == base_url.trim().trim_end_matches('/')
    }

    pub fn apply_preset(&mut self, key: BuiltinAgentPresetKey) {
        let preset = builtin_agent_preset(key);
        self.preset = preset.key;
        self.display_name = preset.display_name.into();
        self.kind = preset.kind;
        self.model = preset.model.into();
        self.base_url = preset.base_url.into();
    }

    pub fn set_kind_for_preset(&mut self, kind: BuiltinAgentKind) {
        self.kind = kind;
        if let Some(base_url) = builtin_agent_preset(self.preset).base_url_for_kind(kind) {
            self.base_url = base_url.into();
        } else if self.preset != BuiltinAgentPresetKey::Custom {
            self.base_url = kind.default_base_url().into();
        }
        if kind == BuiltinAgentKind::OpenAiCompat && self.model.starts_with("claude-") {
            self.model = "gpt-5.4".into();
        } else if kind == BuiltinAgentKind::Anthropic && self.model.starts_with("gpt-") {
            self.model = "claude-sonnet-4-6-20250916".into();
        }
    }

    pub fn toggle_kind_for_preset(&mut self) {
        if builtin_agent_preset(self.preset).alt_kind.is_none() {
            return;
        }
        let next = match self.kind {
            BuiltinAgentKind::Anthropic => BuiltinAgentKind::OpenAiCompat,
            BuiltinAgentKind::OpenAiCompat => BuiltinAgentKind::Anthropic,
        };
        self.set_kind_for_preset(next);
    }
}

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
    pub connected: bool,
}

impl AcpAgentConfig {
    pub fn ready(&self) -> bool {
        self.enabled
            && match self.connection_type {
                AcpConnectionType::Local => !self.command.trim().is_empty(),
                AcpConnectionType::Remote => self
                    .url
                    .as_deref()
                    .map(|url| !url.trim().is_empty())
                    .unwrap_or(false),
            }
    }

    pub fn args_text(&self) -> String {
        self.args.join(", ")
    }

    pub fn env_text(&self) -> String {
        self.env
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn set_args_text(&mut self, text: &str) {
        self.args = text
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
    }

    pub fn set_env_text(&mut self, text: &str) {
        self.env = text
            .lines()
            .filter_map(|line| {
                let (key, value) = line.split_once('=')?;
                let key = key.trim();
                (!key.is_empty()).then(|| (key.to_string(), value.trim().to_string()))
            })
            .collect();
    }
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
    pub test_status: ImageTestStatus,
}

/// State for the floating agent-settings modal.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentSettings {
    pub tab: AgentSettingsTab,
    pub connected: [bool; 5],
    pub builtin_agents: Vec<BuiltinAgentConfig>,
    pub builtin_agent_draft: Option<BuiltinAgentConfig>,
    pub builtin_preset_menu_open: Option<BuiltinAgentPresetMenuTarget>,
    pub builtin_preset_menu_scroll: f32,
    pub builtin_preset_menu_hover: Option<BuiltinAgentPresetKey>,
    pub next_builtin_agent_id: u64,
    pub acp_agents: Vec<AcpAgentConfig>,
    pub acp_agent_draft: Option<AcpAgentConfig>,
    pub next_acp_agent_id: u64,
    /// Vertical scroll offset of the right content pane in px.
    pub scroll_y: f32,
    pub mcp_server: McpServer,
    pub mcp_cli_enabled: [bool; 6],
    pub mcp_client_config_copied_at_ms: Option<u64>,
    pub hover_mcp_server_button: bool,
    pub hover_mcp_client_config_copy: bool,
    pub images_advanced_open: bool,
    pub images_search_ready: bool,
    pub images_search_test_status: ImageTestStatus,
    pub hover_image_search_test_button: bool,
    pub hover_image_gen_add_button: bool,
    pub openverse_client_id: String,
    pub openverse_client_secret: String,
    pub image_gen_profiles: Vec<ImageGenProfile>,
    pub active_image_gen_profile_id: Option<String>,
    pub image_gen_provider_menu_open: Option<usize>,
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
    /// Index into `acp_agents` of the hovered ACP agent card.
    pub hover_acp_agent: usize,
    pub hover_add_provider: bool,
    pub hover_add_acp_agent: bool,
    /// Sidebar nav item under the cursor; `None` = no hover.
    pub hover_nav: Option<AgentSettingsTab>,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            tab: AgentSettingsTab::Agents,
            connected: [false; 5],
            builtin_agents: Vec::new(),
            builtin_agent_draft: None,
            builtin_preset_menu_open: None,
            builtin_preset_menu_scroll: 0.0,
            builtin_preset_menu_hover: None,
            next_builtin_agent_id: 1,
            acp_agents: Vec::new(),
            acp_agent_draft: None,
            next_acp_agent_id: 1,
            scroll_y: 0.0,
            mcp_server: McpServer::default(),
            mcp_cli_enabled: [false; 6],
            mcp_client_config_copied_at_ms: None,
            hover_mcp_server_button: false,
            hover_mcp_client_config_copy: false,
            images_advanced_open: false,
            images_search_ready: true,
            images_search_test_status: ImageTestStatus::Idle,
            hover_image_search_test_button: false,
            hover_image_gen_add_button: false,
            openverse_client_id: String::new(),
            openverse_client_secret: String::new(),
            image_gen_profiles: Vec::new(),
            active_image_gen_profile_id: None,
            image_gen_provider_menu_open: None,
            next_image_gen_profile_id: 1,
            auto_update_enabled: true,
            focus: None,
            hover_provider: usize::MAX,
            hover_builtin_agent: usize::MAX,
            hover_acp_agent: usize::MAX,
            hover_add_provider: false,
            hover_add_acp_agent: false,
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

    pub fn begin_builtin_agent_draft(&mut self) {
        if self.builtin_agent_draft.is_some() {
            return;
        }
        let preset = builtin_agent_preset(BuiltinAgentPresetKey::Anthropic);
        self.builtin_agent_draft = Some(BuiltinAgentConfig {
            id: String::new(),
            preset: preset.key,
            display_name: preset.display_name.into(),
            kind: preset.kind,
            api_key: String::new(),
            model: preset.model.into(),
            base_url: preset.base_url.into(),
            enabled: true,
        });
    }

    pub fn set_builtin_agent_preset(&mut self, index: usize, preset: BuiltinAgentPresetKey) {
        if let Some(agent) = self.builtin_agents.get_mut(index) {
            let api_key = agent.api_key.clone();
            let enabled = agent.enabled;
            agent.apply_preset(preset);
            agent.api_key = api_key;
            agent.enabled = enabled;
        }
    }

    pub fn set_builtin_agent_draft_preset(&mut self, preset: BuiltinAgentPresetKey) {
        if let Some(agent) = self.builtin_agent_draft.as_mut() {
            let api_key = agent.api_key.clone();
            agent.apply_preset(preset);
            agent.api_key = api_key;
        }
    }

    pub fn save_builtin_agent_draft(&mut self) -> Option<String> {
        if !self
            .builtin_agent_draft
            .as_ref()
            .is_some_and(|draft| draft.ready())
        {
            return None;
        }
        let draft = self.builtin_agent_draft.take()?;
        Some(self.add_builtin_agent_config(
            draft.display_name,
            draft.api_key,
            draft.model,
            draft.kind,
            draft.base_url,
        ))
    }

    pub fn cancel_builtin_agent_draft(&mut self) {
        self.builtin_agent_draft = None;
        self.builtin_preset_menu_open = None;
        self.builtin_preset_menu_scroll = 0.0;
        self.builtin_preset_menu_hover = None;
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
        let display_name = display_name.into();
        let api_key = api_key.into();
        let model = model.into();
        let base_url = base_url.into();
        if let Some(existing) = self.builtin_agents.iter().find(|agent| {
            agent.matches_add_candidate(&display_name, &api_key, &model, kind, &base_url)
        }) {
            return existing.id.clone();
        }
        let id = format!("builtin-{}", self.next_builtin_agent_id.max(1));
        self.next_builtin_agent_id = self.next_builtin_agent_id.max(1).saturating_add(1);
        self.builtin_agents.push(BuiltinAgentConfig {
            id: id.clone(),
            preset: infer_builtin_agent_preset(kind, &base_url, &model),
            display_name,
            kind,
            api_key,
            model,
            base_url,
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

    pub fn begin_acp_agent_draft(&mut self) {
        if self.acp_agent_draft.is_some() {
            return;
        }
        let n = self.next_acp_agent_id.max(1);
        self.acp_agent_draft = Some(AcpAgentConfig {
            id: String::new(),
            display_name: format!("ACP Agent {n}"),
            connection_type: AcpConnectionType::Local,
            command: String::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            url: None,
            enabled: true,
            connected: false,
        });
    }

    pub fn save_acp_agent_draft(&mut self) -> Option<String> {
        if !self
            .acp_agent_draft
            .as_ref()
            .is_some_and(|draft| draft.ready())
        {
            return None;
        }
        let draft = self.acp_agent_draft.take()?;
        Some(self.add_acp_agent_config(
            draft.display_name,
            draft.connection_type,
            draft.command,
            draft.args,
            draft.env,
            draft.url,
            draft.enabled,
        ))
    }

    pub fn cancel_acp_agent_draft(&mut self) {
        self.acp_agent_draft = None;
    }

    #[allow(clippy::too_many_arguments)]
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
            connected: false,
        });
        id
    }

    pub fn remove_acp_agent(&mut self, id: &str) -> bool {
        let before = self.acp_agents.len();
        self.acp_agents.retain(|agent| agent.id != id);
        self.acp_agents.len() != before
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
            test_status: ImageTestStatus::Idle,
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
        self.image_gen_provider_menu_open = None;
        if self.active_image_gen_profile_id.as_deref() == Some(id) {
            self.active_image_gen_profile_id =
                self.image_gen_profiles.first().map(|p| p.id.clone());
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsDrag {
    Reserved,
}
