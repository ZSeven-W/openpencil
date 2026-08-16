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

//!
//! This file is the slim spine — the settings tabs / focus enums, the
//! `AgentSettings` struct + `Default`, and the sibling wiring. The
//! implementation lives under `agent_settings/`:
//!
//! | File                            | Purpose                              |
//! | ------------------------------- | ------------------------------------ |
//! | `agent_settings/config_types.rs`| built-in / ACP / image-gen value types |
//! | `agent_settings/mutators.rs`    | `impl AgentSettings` mutators        |

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::agent_settings_builtin_presets::BuiltinAgentPresetKey;
pub use crate::chat::AgentProvider;

mod config_types;
mod mutators;

pub use config_types::{
    normalize_builtin_models, AcpAgentConfig, AcpConnectionType, BuiltinAgentConfig,
    BuiltinAgentKind, ImageGenProfile, ImageGenProvider, MAX_BUILTIN_AGENT_MODELS,
    MAX_BUILTIN_MODEL_CHARS, OPENVERSE_AUTH_DOCS_URL,
};

/// Which section of the settings modal is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsTab {
    Agents,
    Mcp,
    Images,
    Fonts,
    System,
    /// Sign-in status + workspace identity for the planned user system.
    Account,
}

impl AgentSettingsTab {
    pub const ALL: [AgentSettingsTab; 6] = [
        AgentSettingsTab::Agents,
        AgentSettingsTab::Mcp,
        AgentSettingsTab::Images,
        AgentSettingsTab::Fonts,
        AgentSettingsTab::System,
        AgentSettingsTab::Account,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AgentSettingsTab::Agents => "Agents",
            AgentSettingsTab::Mcp => "MCP",
            AgentSettingsTab::Images => "Images",
            AgentSettingsTab::Fonts => "Fonts",
            AgentSettingsTab::System => "System",
            AgentSettingsTab::Account => "Account",
        }
    }
}

/// Terminal-side MCP integrations the user can flip on/off. Order
/// matches the TS app's MCP settings grid.
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
    /// Append-only: `mcp_cli_enabled` is indexed positionally by this
    /// array, and `migrate_mcp_cli_flags` reads persisted settings by the
    /// same index. Inserting in the middle silently reassigns a user's
    /// saved toggles to the wrong CLIs. New CLIs must be appended at the
    /// end (12 → 13) and appended to `DISPLAY` wherever the product wants
    /// the row to show.
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

    /// Row order on the MCP tab's terminal-integrations list. Deliberately
    /// NOT the persistence order — paint and hit-test both walk this array
    /// (row `i` shows `DISPLAY[i]`) while the toggle state is read through
    /// [`McpCli::index`] into the append-only `mcp_cli_enabled` layout.
    /// Kept a permutation of `ALL`; `tests_agent_settings.rs` asserts that.
    pub const DISPLAY: [McpCli; 13] = [
        McpCli::ClaudeCode,
        McpCli::Codex,
        McpCli::Dsh,
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
    ];

    /// Position in [`McpCli::ALL`] — the index of this CLI's toggle in
    /// `mcp_cli_enabled` (and in the persisted positional flag arrays).
    pub fn index(self) -> usize {
        McpCli::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .expect("every McpCli variant is registered in McpCli::ALL")
    }

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

// `McpServer` + the provider connect-lifecycle types live in
// `agent_settings_connection.rs` (800-line cap); re-exported here so
// call sites keep the `agent_settings::McpServer` path.
pub use crate::agent_settings_acp_connection::{
    AcpAgentConnectOutcome, AcpAgentConnectPhase, AcpAgentConnectRequest, AcpAgentConnection,
};
pub use crate::agent_settings_builtin_models::{
    BuiltinModelCatalog, BuiltinModelCatalogPhase, BuiltinModelCatalogRefreshOutcome,
    BuiltinModelCatalogRefreshRequest, BuiltinModelCatalogTarget, BuiltinModelOption,
};
pub use crate::agent_settings_connection::{
    McpServer, ProviderConnectOutcome, ProviderConnectPhase, ProviderConnection,
};

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

/// Which built-in provider form has its model dropdown open. The menu
/// targets the same cards as `BuiltinAgentPresetMenuTarget`, but it is
/// keyed on the Model field instead of the provider preset select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinModelMenuTarget {
    Agent(usize),
    Draft,
}

/// State for the floating agent-settings modal.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentSettings {
    pub tab: AgentSettingsTab,
    pub connected: [bool; 7],
    /// The embedding host's real MCP endpoint (e.g. the VS Code
    /// extension's McpProxy URL), delivered via the bridge `init`
    /// message. When set, the MCP tab's client-config card displays
    /// and copies THIS url — the daemon-internal `mcp_server` port
    /// would point clients at a dead endpoint in the plugin. Runtime
    /// only — never persisted.
    pub embed_mcp_url: Option<String>,
    /// Probe-derived per-provider connect status, indexed like
    /// `connected`. Runtime-only — not persisted.
    pub provider_connection: [ProviderConnection; 7],
    /// Connect-press request seam — the desktop host drains this
    /// into the async provider probe (`provider_probe_host.rs`).
    pub pending_provider_connect: Option<AgentProvider>,
    pub builtin_agents: Vec<BuiltinAgentConfig>,
    pub builtin_agent_draft: Option<BuiltinAgentConfig>,
    pub builtin_preset_menu_open: Option<BuiltinAgentPresetMenuTarget>,
    pub builtin_preset_menu_scroll: jian_core::scroll::ScrollState,
    pub builtin_preset_menu_hover: Option<BuiltinAgentPresetKey>,
    /// Dropdown of discovered models anchored to the built-in form's
    /// Model field. Runtime-only — never persisted.
    pub builtin_model_menu_open: Option<BuiltinModelMenuTarget>,
    pub builtin_model_menu_scroll: jian_core::scroll::ScrollState,
    /// Hovered row index into the visible model options.
    pub builtin_model_menu_hover: Option<usize>,
    pub next_builtin_agent_id: u64,
    /// Runtime-only provider model catalogs. Persisted settings retain only the
    /// explicitly selected `BuiltinAgentConfig::models`.
    pub builtin_model_catalogs: BTreeMap<BuiltinModelCatalogTarget, BuiltinModelCatalog>,
    pub pending_builtin_model_catalog_refreshes: VecDeque<BuiltinModelCatalogRefreshRequest>,
    pub builtin_model_catalog_generation: u64,
    /// Ids of `builtin_agents` that were auto-imported from an external
    /// CLI config (e.g. Zode's `~/.zode/config.json`). Runtime-only —
    /// NOT persisted. These agents are re-derived from their source file
    /// on every launch, so persisting them would silently duplicate the
    /// source's API keys into OpenPencil's own settings.json; the host's
    /// save path skips any agent whose id is in this set.
    pub imported_agent_ids: BTreeSet<String>,
    pub acp_agents: Vec<AcpAgentConfig>,
    pub acp_agent_draft: Option<AcpAgentConfig>,
    pub next_acp_agent_id: u64,
    /// Last issued ACP connect generation. Runtime-only and process-local.
    pub acp_agent_connect_generation: u64,
    pub pending_acp_agent_connect: Option<AcpAgentConnectRequest>,
    pub acp_agent_connection: BTreeMap<String, AcpAgentConnection>,
    /// Whether each quick-add preset's binary was found on PATH, keyed by
    /// preset id. Runtime-only and never persisted: it describes this
    /// machine right now, and a stale "missing" carried across a restart
    /// would grey out a CLI the user has since installed. Absent key =
    /// nobody looked (the browser host never can), which the UI renders as
    /// the neutral state rather than as "not installed".
    pub acp_preset_installed: BTreeMap<String, bool>,
    pub scroll_y: jian_core::scroll::ScrollState,
    pub mcp_server: McpServer,
    pub mcp_cli_enabled: [bool; 13],
    pub mcp_client_config_copied_at_ms: Option<u64>,
    pub hover_agent_settings_close: bool,
    pub hover_mcp_server_button: bool,
    pub hover_mcp_client_config_copy: bool,
    pub images_advanced_open: bool,
    pub images_search_ready: bool,
    pub images_search_test_status: ImageTestStatus,
    pub hover_image_search_test_button: bool,
    pub hover_image_search_register_link: bool,
    pub hover_image_gen_add_button: bool,
    pub openverse_client_id: String,
    pub openverse_client_secret: String,
    /// Provenance marker for Openverse credentials copied from a web
    /// deployment. `None` means the singleton is operator-managed.
    pub openverse_credential_owner: Option<String>,
    pub image_gen_profiles: Vec<ImageGenProfile>,
    pub active_image_gen_profile_id: Option<String>,
    pub image_gen_provider_menu_open: Option<usize>,
    pub hover_image_gen_provider_option: Option<(usize, ImageGenProvider)>,
    pub hover_image_gen_profile_header: Option<usize>,
    pub hover_image_gen_profile_remove: Option<usize>,
    pub hover_image_gen_profile_provider: Option<usize>,
    pub hover_image_gen_profile_test: Option<usize>,
    pub next_image_gen_profile_id: u64,
    /// Auto-check GitHub releases on startup.
    pub auto_update_enabled: bool,
    /// Opt-in gate for experimental surfaces (canvas Preview mode +
    /// the property-panel Widget section). Off by default.
    pub experimental_features_enabled: bool,
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
    /// Index into the *visible* quick-add preset rows under the cursor.
    pub hover_acp_preset: Option<usize>,
    /// Sidebar nav item under the cursor; `None` = no hover.
    pub hover_nav: Option<AgentSettingsTab>,
    /// Latest browser→daemon credential-sync failure worth showing (web
    /// host only; transient, never persisted). `None` = last sync ok.
    pub web_credential_sync_error: Option<String>,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            tab: AgentSettingsTab::Agents,
            connected: [false; 7],
            embed_mcp_url: None,
            provider_connection: Default::default(),
            pending_provider_connect: None,
            builtin_agents: Vec::new(),
            builtin_agent_draft: None,
            builtin_preset_menu_open: None,
            builtin_preset_menu_scroll: Default::default(),
            builtin_preset_menu_hover: None,
            builtin_model_menu_open: None,
            builtin_model_menu_scroll: Default::default(),
            builtin_model_menu_hover: None,
            next_builtin_agent_id: 1,
            builtin_model_catalogs: BTreeMap::new(),
            pending_builtin_model_catalog_refreshes: VecDeque::new(),
            builtin_model_catalog_generation: 0,
            imported_agent_ids: BTreeSet::new(),
            acp_agents: Vec::new(),
            acp_agent_draft: None,
            next_acp_agent_id: 1,
            acp_agent_connect_generation: 0,
            pending_acp_agent_connect: None,
            acp_agent_connection: BTreeMap::new(),
            acp_preset_installed: BTreeMap::new(),
            scroll_y: Default::default(),
            mcp_server: McpServer::default(),
            mcp_cli_enabled: [false; 13],
            mcp_client_config_copied_at_ms: None,
            hover_agent_settings_close: false,
            hover_mcp_server_button: false,
            hover_mcp_client_config_copy: false,
            images_advanced_open: false,
            images_search_ready: true,
            images_search_test_status: ImageTestStatus::Idle,
            hover_image_search_test_button: false,
            hover_image_search_register_link: false,
            hover_image_gen_add_button: false,
            openverse_client_id: String::new(),
            openverse_client_secret: String::new(),
            openverse_credential_owner: None,
            image_gen_profiles: Vec::new(),
            active_image_gen_profile_id: None,
            image_gen_provider_menu_open: None,
            hover_image_gen_provider_option: None,
            hover_image_gen_profile_header: None,
            hover_image_gen_profile_remove: None,
            hover_image_gen_profile_provider: None,
            hover_image_gen_profile_test: None,
            next_image_gen_profile_id: 1,
            auto_update_enabled: true,
            experimental_features_enabled: false,
            focus: None,
            hover_provider: usize::MAX,
            hover_builtin_agent: usize::MAX,
            hover_acp_agent: usize::MAX,
            hover_add_provider: false,
            hover_add_acp_agent: false,
            hover_acp_preset: None,
            hover_nav: None,
            web_credential_sync_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsDrag {
    Reserved,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_settings_scroll_fields_use_scroll_state() {
        let mut s = AgentSettings::default();

        s.builtin_preset_menu_scroll.offset = 18.0;
        s.scroll_y.offset = 42.0;

        assert_eq!(s.builtin_preset_menu_scroll.offset, 18.0);
        assert_eq!(s.scroll_y.offset, 42.0);
    }
}
