//! Per-provider connect-lifecycle state for the agent-settings modal.
//!
//! Mirrors the TS provider connect flow (`apps/web/server/api/ai/
//! connect-agent.ts` + `apps/web/src/components/shared/
//! agent-settings-providers-tab.tsx`): pressing Connect runs a real
//! probe against the local CLI (installed? responsive?
//! authenticated?) and surfaces a human-readable connection status
//! on the provider card instead of flipping a bare boolean. The
//! probe itself is transport-bound and lives host-side
//! (`op-host-desktop/src/provider_probe*.rs`); this module carries
//! the wasm-clean state types plus the request seam the host
//! drains.
//!
//! `McpServer` also lives here (moved from `agent_settings.rs` to
//! honor the 800-line file cap); it is re-exported from there so
//! call sites keep the `agent_settings::McpServer` path.

use crate::agent_settings::AgentSettings;
use crate::chat::AgentProvider;

/// Lifecycle phase of one provider card's connect probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProviderConnectPhase {
    /// No probe has run (or the provider was disconnected).
    #[default]
    Idle,
    /// A probe is in flight — the card shows "Connecting…".
    Probing,
    /// The last probe succeeded.
    Connected,
    /// The last probe failed (including the not-installed case).
    Error,
}

/// Probe-derived status of one provider card. Runtime-only — never
/// persisted (`settings_io` snapshots only `connected`); a restart
/// reverts to `Idle` until the next Connect press.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderConnection {
    pub phase: ProviderConnectPhase,
    /// Human-readable status, e.g. `Connected via pro (a@b.c)` —
    /// the TS `ConnectResult.connectionInfo` string.
    pub info: Option<String>,
    /// Non-fatal advisory (rendered amber), e.g. the Codex
    /// empty-model-cache hint.
    pub warning: Option<String>,
    /// Probe failure text (TS `ConnectResult.error`, after the
    /// friendly-error mapping).
    pub error: Option<String>,
    /// CLI binary was not found (TS `notInstalled`) — the card
    /// shows install guidance instead of an error.
    pub not_installed: bool,
    /// Manual install command for the not-installed state (TS
    /// `install-agent.ts::getInstallInfo().command`).
    pub install_command: Option<String>,
    /// Config-file path hint (TS `hintPath`).
    pub hint_path: Option<String>,
    /// CLI version string when the probe ran a version check
    /// (for example, `codex --version`).
    pub version: Option<String>,
}

/// Result payload the host-side probe writes back through
/// [`AgentSettings::apply_provider_connect_outcome`]. Discovered
/// model entries travel separately through `chat.discovered_models`
/// (the same seam startup discovery uses).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderConnectOutcome {
    pub connected: bool,
    pub info: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
    pub not_installed: bool,
    pub install_command: Option<String>,
    pub hint_path: Option<String>,
    pub version: Option<String>,
}

/// MCP server status — surfaced on the MCP tab's top card. Default
/// port mirrors the TS app (`pen-mcp` default 3100).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpServer {
    pub running: bool,
    pub port: u16,
}

/// Default HTTP MCP / serve-web daemon port, mirroring the retired TS
/// `@zseven-w/pen-mcp` default. Single source of truth for the CLI,
/// the settings modal, and the web shell's daemon fallback.
pub const DEFAULT_MCP_PORT: u16 = 3100;

/// Default local daemon origin (`local_daemon_origin(DEFAULT_MCP_PORT)`
/// as a `const` — the web shell needs a `&'static str` fallback).
pub const DEFAULT_LOCAL_DAEMON_ORIGIN: &str = "http://127.0.0.1:3100";

/// Local daemon origin for `port` (`http://127.0.0.1:<port>`).
pub fn local_daemon_origin(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// Local HTTP MCP endpoint for `port` (`http://127.0.0.1:<port>/mcp`).
pub fn local_mcp_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

/// Canonical "connected but returned no models" error for a provider,
/// localized via the `providerProbe.*` catalog. Single source for the
/// web host (real UI locale) and the services probe pump (pinned EnUs).
pub fn missing_models_connect_error(locale: crate::Locale, provider: AgentProvider) -> String {
    let name = match provider {
        AgentProvider::ClaudeCode => "Claude Code",
        AgentProvider::CodexCli => "Codex CLI",
        AgentProvider::OpenCode => "OpenCode",
        AgentProvider::GithubCopilot => "GitHub Copilot",
        AgentProvider::Antigravity => {
            return op_i18n::translate(locale, "providerProbe.noDefaultModelAntigravity")
                .to_string();
        }
        AgentProvider::GrokBuild => "Grok Build",
        AgentProvider::DeepSeekHarness => "DeepSeek Harness",
    };
    op_i18n::translate_with(locale, "providerProbe.noModelList", &[("name", name)])
}

impl Default for McpServer {
    fn default() -> Self {
        Self {
            running: false,
            port: DEFAULT_MCP_PORT,
        }
    }
}

impl McpServer {
    pub fn client_config_display_text(self) -> String {
        format!(
            r#"{{ "type": "http", "url": "{}" }}"#,
            local_mcp_url(self.port)
        )
    }

    pub fn client_config_clipboard_text(self) -> String {
        format!(
            "{{\n  \"type\": \"http\",\n  \"url\": \"{}\"\n}}",
            local_mcp_url(self.port)
        )
    }
}

impl AgentSettings {
    /// True when an embedding host owns the MCP lifecycle
    /// (`embed_mcp_url` set): the server card then reads as
    /// always-running at the host's port, with no start/stop button
    /// and no port editing — the daemon-internal `mcp_server` state
    /// is meaningless in that surface.
    pub fn mcp_host_managed(&self) -> bool {
        self.embed_mcp_url.is_some()
    }

    /// The port component of `embed_mcp_url`, for the read-only port
    /// display (e.g. `"63655"` from `http://127.0.0.1:63655/mcp`).
    pub fn embed_mcp_port_text(&self) -> Option<&str> {
        let url = self.embed_mcp_url.as_deref()?;
        let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
        let authority = after_scheme
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default();
        authority.rsplit_once(':').map(|(_, port)| port)
    }

    /// The client-config card's one-line display text. Prefers the
    /// embedding host's real endpoint (`embed_mcp_url`, set via the
    /// bridge init) over the daemon-internal `mcp_server` port.
    pub fn mcp_client_config_display_text(&self) -> String {
        match &self.embed_mcp_url {
            Some(url) => format!(r#"{{ "type": "http", "url": "{url}" }}"#),
            None => self.mcp_server.client_config_display_text(),
        }
    }

    /// The client-config card's copied (pretty-printed) text — same
    /// endpoint preference as `mcp_client_config_display_text`.
    pub fn mcp_client_config_clipboard_text(&self) -> String {
        match &self.embed_mcp_url {
            Some(url) => format!("{{\n  \"type\": \"http\",\n  \"url\": \"{url}\"\n}}"),
            None => self.mcp_server.client_config_clipboard_text(),
        }
    }

    /// Index into the `connected` / `provider_connection` arrays —
    /// both are ordered by `AgentProvider::ALL`.
    pub fn provider_index(provider: AgentProvider) -> usize {
        AgentProvider::ALL
            .iter()
            .position(|candidate| *candidate == provider)
            .unwrap_or(0)
    }

    pub fn provider_verified_connected_at(&self, index: usize) -> bool {
        self.connected.get(index).copied().unwrap_or(false)
            && self
                .provider_connection
                .get(index)
                .is_some_and(|conn| conn.phase == ProviderConnectPhase::Connected)
    }

    pub fn provider_verified_connected(&self, provider: AgentProvider) -> bool {
        self.provider_verified_connected_at(Self::provider_index(provider))
    }

    pub fn verified_connected_mask(&self) -> [bool; 7] {
        std::array::from_fn(|i| self.provider_verified_connected_at(i))
    }

    /// Connect pressed on a disconnected card — flip the card to the
    /// in-flight state and raise the request seam the desktop host
    /// drains into the async probe. Mirrors the TS `handleConnect`
    /// prologue (clear error/warning/notInstalled, show spinner).
    pub fn begin_provider_connect(&mut self, provider: AgentProvider) {
        let idx = Self::provider_index(provider);
        self.connected[idx] = false;
        self.provider_connection[idx] = ProviderConnection {
            phase: ProviderConnectPhase::Probing,
            ..ProviderConnection::default()
        };
        self.pending_provider_connect = Some(provider);
    }

    /// Disconnect pressed — mirror the TS `disconnectProvider` store
    /// action, which resets the provider entry to its default
    /// (disconnected, no status). Any in-flight probe request for
    /// this provider is withdrawn.
    pub fn disconnect_provider(&mut self, provider: AgentProvider) {
        let idx = Self::provider_index(provider);
        self.connected[idx] = false;
        self.provider_connection[idx] = ProviderConnection::default();
        if self.pending_provider_connect == Some(provider) {
            self.pending_provider_connect = None;
        }
    }

    /// Write a finished probe result onto the card. Connected
    /// outcomes also flip the persisted `connected` flag (the model
    /// catalog mask); failures leave it off so the picker never
    /// lists a provider that did not verify.
    pub fn apply_provider_connect_outcome(
        &mut self,
        provider: AgentProvider,
        outcome: ProviderConnectOutcome,
    ) {
        let idx = Self::provider_index(provider);
        self.connected[idx] = outcome.connected;
        self.provider_connection[idx] = ProviderConnection {
            phase: if outcome.connected {
                ProviderConnectPhase::Connected
            } else {
                ProviderConnectPhase::Error
            },
            info: outcome.info,
            warning: outcome.warning,
            error: outcome.error,
            not_installed: outcome.not_installed,
            install_command: outcome.install_command,
            hint_path: outcome.hint_path,
            version: outcome.version,
        };
    }

    /// Whether a probe is in flight for `provider` — the press
    /// handler uses this to ignore re-presses while connecting (TS
    /// disables the button via `isConnecting`).
    pub fn provider_probe_in_flight(&self, provider: AgentProvider) -> bool {
        self.provider_connection[Self::provider_index(provider)].phase
            == ProviderConnectPhase::Probing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_provider_connect_sets_probing_and_request_seam() {
        let mut s = AgentSettings::default();
        s.provider_connection[1].error = Some("stale".into());
        s.begin_provider_connect(AgentProvider::CodexCli);
        assert_eq!(
            s.provider_connection[1].phase,
            ProviderConnectPhase::Probing
        );
        // The TS handleConnect prologue clears prior status.
        assert_eq!(s.provider_connection[1].error, None);
        assert_eq!(s.pending_provider_connect, Some(AgentProvider::CodexCli));
        assert!(s.provider_probe_in_flight(AgentProvider::CodexCli));
        // A fresh probe withdraws any stale persisted connection
        // until the probe lands successfully.
        assert!(!s.connected[1]);
    }

    #[test]
    fn apply_connected_outcome_flips_flag_and_stores_info() {
        let mut s = AgentSettings::default();
        s.begin_provider_connect(AgentProvider::ClaudeCode);
        s.apply_provider_connect_outcome(
            AgentProvider::ClaudeCode,
            ProviderConnectOutcome {
                connected: true,
                info: Some("Connected via pro (a@b.c)".into()),
                hint_path: Some("~/.claude/settings.json".into()),
                ..ProviderConnectOutcome::default()
            },
        );
        assert!(s.connected[0]);
        let conn = &s.provider_connection[0];
        assert_eq!(conn.phase, ProviderConnectPhase::Connected);
        assert_eq!(conn.info.as_deref(), Some("Connected via pro (a@b.c)"));
        assert_eq!(conn.hint_path.as_deref(), Some("~/.claude/settings.json"));
    }

    #[test]
    fn apply_failed_outcome_keeps_provider_disconnected() {
        let mut s = AgentSettings::default();
        s.begin_provider_connect(AgentProvider::GithubCopilot);
        s.apply_provider_connect_outcome(
            AgentProvider::GithubCopilot,
            ProviderConnectOutcome {
                connected: false,
                error: Some("Not authenticated.".into()),
                ..ProviderConnectOutcome::default()
            },
        );
        assert!(!s.connected[3]);
        let conn = &s.provider_connection[3];
        assert_eq!(conn.phase, ProviderConnectPhase::Error);
        assert_eq!(conn.error.as_deref(), Some("Not authenticated."));
    }

    #[test]
    fn not_installed_outcome_carries_install_guidance() {
        let mut s = AgentSettings::default();
        s.apply_provider_connect_outcome(
            AgentProvider::CodexCli,
            ProviderConnectOutcome {
                connected: false,
                not_installed: true,
                install_command: Some("npm install -g @openai/codex".into()),
                error: Some("Codex CLI not found".into()),
                ..ProviderConnectOutcome::default()
            },
        );
        let conn = &s.provider_connection[1];
        assert!(conn.not_installed);
        assert_eq!(
            conn.install_command.as_deref(),
            Some("npm install -g @openai/codex")
        );
    }

    #[test]
    fn disconnect_resets_card_and_withdraws_pending_probe() {
        let mut s = AgentSettings::default();
        s.connected[2] = true;
        s.provider_connection[2].phase = ProviderConnectPhase::Connected;
        s.provider_connection[2].info = Some("Connected (anthropic)".into());
        s.pending_provider_connect = Some(AgentProvider::OpenCode);
        s.disconnect_provider(AgentProvider::OpenCode);
        assert!(!s.connected[2]);
        assert_eq!(s.provider_connection[2], ProviderConnection::default());
        assert_eq!(s.pending_provider_connect, None);
    }

    #[test]
    fn verified_connected_requires_connected_phase() {
        let mut s = AgentSettings::default();
        s.connected[0] = true;
        assert!(!s.provider_verified_connected(AgentProvider::ClaudeCode));
        s.provider_connection[0].phase = ProviderConnectPhase::Connected;
        assert!(s.provider_verified_connected(AgentProvider::ClaudeCode));
        assert_eq!(
            s.verified_connected_mask(),
            [true, false, false, false, false, false, false]
        );
    }
}

#[cfg(test)]
mod embed_mcp_url_tests {
    use super::*;

    #[test]
    fn client_config_prefers_the_embed_url_over_the_daemon_port() {
        let mut settings = AgentSettings::default();
        assert!(settings
            .mcp_client_config_display_text()
            .contains("http://127.0.0.1:3100/mcp"));
        settings.embed_mcp_url = Some("http://127.0.0.1:63655/mcp".into());
        assert_eq!(
            settings.mcp_client_config_display_text(),
            r#"{ "type": "http", "url": "http://127.0.0.1:63655/mcp" }"#
        );
        assert!(settings
            .mcp_client_config_clipboard_text()
            .contains("\"url\": \"http://127.0.0.1:63655/mcp\""));
    }
}

#[cfg(test)]
mod mcp_defaults_tests {
    use super::*;

    #[test]
    fn default_origin_const_matches_the_port_const() {
        assert_eq!(
            DEFAULT_LOCAL_DAEMON_ORIGIN,
            local_daemon_origin(DEFAULT_MCP_PORT)
        );
        assert_eq!(
            local_mcp_url(DEFAULT_MCP_PORT),
            format!("{DEFAULT_LOCAL_DAEMON_ORIGIN}/mcp")
        );
    }
}

#[cfg(test)]
mod embed_mcp_host_tests {
    use super::*;

    #[test]
    fn host_managed_state_and_port_derive_from_the_embed_url() {
        let mut settings = AgentSettings::default();
        assert!(!settings.mcp_host_managed());
        assert_eq!(settings.embed_mcp_port_text(), None);
        settings.embed_mcp_url = Some("http://127.0.0.1:63655/mcp".into());
        assert!(settings.mcp_host_managed());
        assert_eq!(settings.embed_mcp_port_text(), Some("63655"));
    }
}
