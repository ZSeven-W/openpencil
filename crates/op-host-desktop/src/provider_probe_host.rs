//! Host pump for the connect-time provider probe.
//!
//! Follows the `iconify_host.rs` / `update_check.rs` session
//! pattern: the press handler raises
//! `agent_settings.pending_provider_connect` (the request seam in
//! op-editor-core), the redraw pump drains it onto a worker thread
//! running [`op_host_services::provider_probe::connect_provider`], and a later
//! frame polls the outcome into `agent_settings` +
//! `chat.discovered_models`.
//!
//! The job struct + outcome normalization live in
//! [`op_host_services::provider_probe_host`] (codex Issue 5 — the job is a
//! `DesktopApp` field); this residual keeps only the `impl DesktopApp`
//! pump, which drives the job through its public API.

use op_editor_core::agent_settings::ProviderConnectOutcome;

use crate::DesktopApp;
use op_host_services::model_discovery::model_entry_to_ec;
use op_host_services::provider_probe::ProbeOutcome;
use op_host_services::provider_probe_host::normalize_provider_probe_outcome;
// Re-export so `crate::provider_probe_host::ProviderConnectJob` (the
// `DesktopApp` field type in `main.rs`) still resolves with zero churn.
pub use op_host_services::provider_probe_host::ProviderConnectJob;

impl DesktopApp {
    /// Pump: poll a finished probe into editor state, then start the
    /// next requested probe. Returns true when state changed.
    pub(crate) fn drain_provider_connect(&mut self) -> bool {
        let mut changed = self.poll_provider_connect_job();
        if self
            .provider_connect_job
            .as_ref()
            .is_some_and(ProviderConnectJob::is_pending)
        {
            return changed;
        }
        let pending = self
            .host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .pending_provider_connect
            .take();
        if let Some(provider) = pending {
            self.provider_connect_job = Some(ProviderConnectJob::spawn(provider));
            changed = true;
        }
        changed
    }

    fn poll_provider_connect_job(&mut self) -> bool {
        let Some(job) = self.provider_connect_job.as_mut() else {
            return false;
        };
        let provider = job.provider();
        let Some(outcome) = job.poll() else {
            return false;
        };
        self.provider_connect_job = None;

        let es = self.host.editor_state_mut();
        // The user may have pressed Disconnect (or re-toggled) while
        // the probe ran — `disconnect_provider` resets the card to
        // Idle. A landed result must not resurrect a withdrawn card.
        if !es
            .editor_ui
            .agent_settings
            .provider_probe_in_flight(provider)
        {
            return false;
        }
        let outcome = normalize_provider_probe_outcome(provider, outcome);
        let ProbeOutcome {
            connected,
            models,
            error,
            warning,
            not_installed,
            install_command,
            connection_info,
            hint_path,
            version,
        } = outcome;
        es.editor_ui.agent_settings.apply_provider_connect_outcome(
            provider,
            ProviderConnectOutcome {
                connected,
                info: connection_info,
                warning,
                error,
                not_installed,
                install_command,
                hint_path,
                version,
            },
        );
        // Refresh this provider's slice of the discovered catalog —
        // the connect probe is also a re-discovery (a CLI installed
        // mid-session no longer needs an app restart). Stable sort
        // keeps the catalog grouped in `AgentProvider::ALL` order.
        if connected && !models.is_empty() {
            es.chat.discovered_models.retain(|m| m.provider != provider);
            es.chat
                .discovered_models
                .extend(models.into_iter().map(model_entry_to_ec));
            es.chat.discovered_models.sort_by_key(|m| {
                op_editor_core::AgentProvider::ALL
                    .iter()
                    .position(|p| *p == m.provider)
                    .unwrap_or(usize::MAX)
            });
        }
        es.rebuild_chat_models();
        self.host.mark_editor_state_dirty();
        true
    }

    /// Whether a probe worker is in flight — keeps the idle event
    /// loop waking so a result that lands while idle is drained.
    pub(crate) fn provider_connect_pending(&self) -> bool {
        self.provider_connect_job
            .as_ref()
            .is_some_and(ProviderConnectJob::is_pending)
            || self
                .host
                .editor_state()
                .editor_ui
                .agent_settings
                .pending_provider_connect
                .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::agent_settings_state::AgentProvider as Sc;
    use op_ai::chat_models::ModelEntry as ScModelEntry;
    use op_editor_core::agent_settings::ProviderConnectPhase;
    use op_editor_core::AgentProvider;

    fn reset_settings(app: &mut DesktopApp) {
        let es = app.host.editor_state_mut();
        es.editor_ui.agent_settings.connected = [false; 5];
        es.editor_ui.agent_settings.provider_connection = Default::default();
        es.editor_ui.agent_settings.pending_provider_connect = None;
        es.chat.discovered_models.clear();
        es.rebuild_chat_models();
    }

    #[test]
    fn drain_spawns_probe_from_request_seam() {
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .begin_provider_connect(AgentProvider::OpenCode);
        assert!(app.drain_provider_connect());
        assert!(app.provider_connect_pending());
        // The request seam is consumed once.
        assert_eq!(
            app.host
                .editor_state()
                .editor_ui
                .agent_settings
                .pending_provider_connect,
            None
        );
    }

    #[test]
    fn landed_outcome_updates_card_and_splices_models() {
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
        // Seed another provider's discovered models — they must
        // survive the splice.
        app.host
            .editor_state_mut()
            .chat
            .discovered_models
            .push(op_editor_core::ModelEntry::new(
                AgentProvider::GeminiCli,
                "gemini-2.5-pro",
                "Gemini 2.5 Pro",
            ));
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .begin_provider_connect(AgentProvider::ClaudeCode);
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .pending_provider_connect = None;
        let (job, tx) = ProviderConnectJob::pending_for_test(AgentProvider::ClaudeCode);
        app.provider_connect_job = Some(job);
        tx.send(ProbeOutcome {
            connected: true,
            models: vec![ScModelEntry::new(
                Sc::ClaudeCode,
                "claude-sonnet-4-6",
                "Claude Sonnet 4.6",
            )],
            connection_info: Some("Connected via pro (a@b.c)".to_string()),
            hint_path: Some("~/.claude/settings.json".to_string()),
            ..ProbeOutcome::default()
        })
        .unwrap();

        assert!(app.drain_provider_connect());
        let es = app.host.editor_state();
        let settings = &es.editor_ui.agent_settings;
        assert!(settings.connected[0]);
        assert_eq!(
            settings.provider_connection[0].phase,
            ProviderConnectPhase::Connected
        );
        assert_eq!(
            settings.provider_connection[0].info.as_deref(),
            Some("Connected via pro (a@b.c)")
        );
        // Claude's entry spliced in, sorted before the seeded Gemini
        // entry (ALL order), which survived.
        let values: Vec<&str> = es
            .chat
            .discovered_models
            .iter()
            .map(|m| m.value.as_str())
            .collect();
        assert_eq!(values, ["claude-sonnet-4-6", "gemini-2.5-pro"]);
        // Connected → the picker lists the Claude model.
        assert!(es
            .chat
            .available_models
            .iter()
            .any(|m| m.value == "claude-sonnet-4-6"));
    }

    #[test]
    fn failed_outcome_keeps_provider_disconnected_with_error() {
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .begin_provider_connect(AgentProvider::GithubCopilot);
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .pending_provider_connect = None;
        let (job, tx) = ProviderConnectJob::pending_for_test(AgentProvider::GithubCopilot);
        app.provider_connect_job = Some(job);
        tx.send(ProbeOutcome {
            connected: false,
            error: Some("No models found. Run \"copilot login\" to authenticate first.".into()),
            ..ProbeOutcome::default()
        })
        .unwrap();

        assert!(app.drain_provider_connect());
        let settings = &app.host.editor_state().editor_ui.agent_settings;
        assert!(!settings.connected[3]);
        assert_eq!(
            settings.provider_connection[3].phase,
            ProviderConnectPhase::Error
        );
        assert!(settings.provider_connection[3]
            .error
            .as_deref()
            .unwrap()
            .contains("copilot login"));
    }

    #[test]
    fn landed_connected_outcome_without_models_is_failure() {
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
        app.host
            .editor_state_mut()
            .chat
            .discovered_models
            .push(op_editor_core::ModelEntry::new(
                AgentProvider::CodexCli,
                "stale-gpt",
                "Stale GPT",
            ));
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .begin_provider_connect(AgentProvider::CodexCli);
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .pending_provider_connect = None;
        let (job, tx) = ProviderConnectJob::pending_for_test(AgentProvider::CodexCli);
        app.provider_connect_job = Some(job);
        tx.send(ProbeOutcome {
            connected: true,
            connection_info: Some("Connected via Codex CLI".into()),
            ..ProbeOutcome::default()
        })
        .unwrap();

        assert!(app.drain_provider_connect());
        let es = app.host.editor_state();
        let settings = &es.editor_ui.agent_settings;
        assert!(!settings.provider_verified_connected(AgentProvider::CodexCli));
        assert_eq!(
            settings.provider_connection[1].phase,
            ProviderConnectPhase::Error
        );
        assert!(settings.provider_connection[1]
            .error
            .as_deref()
            .is_some_and(|error| error.contains("No models found")));
        assert!(
            !es.chat
                .available_models
                .iter()
                .any(|m| m.provider == AgentProvider::CodexCli),
            "stale Codex models must not become selectable after a failed connect"
        );
    }

    #[test]
    fn outcome_for_withdrawn_card_is_dropped() {
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
        // Probe launched, then the user disconnected mid-flight —
        // the card went back to Idle.
        let (job, tx) = ProviderConnectJob::pending_for_test(AgentProvider::OpenCode);
        app.provider_connect_job = Some(job);
        tx.send(ProbeOutcome {
            connected: true,
            connection_info: Some("Connected (anthropic)".into()),
            ..ProbeOutcome::default()
        })
        .unwrap();

        assert!(!app.drain_provider_connect());
        let settings = &app.host.editor_state().editor_ui.agent_settings;
        assert!(!settings.connected[2]);
        assert_eq!(
            settings.provider_connection[2].phase,
            ProviderConnectPhase::Idle
        );
    }
}
