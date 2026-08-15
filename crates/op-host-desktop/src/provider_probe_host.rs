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

use op_editor_core::agent_settings::{AgentSettings, ProviderConnectOutcome, ProviderConnectPhase};
use op_editor_core::AgentProvider;

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
        //
        // The retain stays INSIDE the success branch on purpose: a
        // transient probe failure must not wipe the provider's models
        // out of the picker (measured: "New Chat" showed "No models
        // connected" after one flaky probe). A stale-but-listed model
        // fails loudly at send time, which beats an empty picker.
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
        // Remember the connection across launches (flags only — the model
        // catalog is re-probed on restore, keys don't exist for CLIs).
        //
        // ONLY a successful probe writes. The store is the LAST KNOWN GOOD
        // list, not a snapshot of the current session: persisting failures
        // too meant a single bad probe — a GUI launch that inherited none
        // of the user's shell env, a machine offline at login — evicted the
        // provider from the startup replay list permanently, so the app
        // could never retry on its own and the user had to press Connect
        // again every launch. A failed probe now leaves the list alone and
        // the next launch retries it. Explicit Disconnect still clears the
        // entry, through `persist_connection_changes` below.
        if connected {
            let index = AgentSettings::provider_index(provider);
            if !self.remembered_connections[index] {
                self.remembered_connections[index] = true;
                crate::agent_connect_store::save(&self.remembered_connections);
            }
        }
        // Startup restore replays one silent probe per remembered
        // provider; queue the next one as each outcome lands.
        if let Some(next) = self.provider_reconnect_queue.pop() {
            self.host
                .editor_state_mut()
                .editor_ui
                .agent_settings
                .begin_provider_connect(next);
        }
        true
    }

    /// Drop providers the user explicitly disconnected from the store —
    /// the Disconnect button mutates state in the widget layer, which
    /// cannot reach the store itself. Cheap (fixed-size phase compare),
    /// called once per frame.
    ///
    /// Only `disconnect_provider` returns a card to `Idle`; a failed probe
    /// lands on `Error` and an in-flight one on `Probing`. Watching that
    /// TRANSITION rather than the `connected` flag is what separates a
    /// withdrawal from a failure — and it also spares the providers still
    /// waiting their turn in the startup replay, whose cards are `Idle`
    /// too but were never non-`Idle` to begin with.
    pub(crate) fn persist_connection_changes(&mut self) {
        let settings = &self.host.editor_state().editor_ui.agent_settings;
        let phases: [ProviderConnectPhase; 7] =
            std::array::from_fn(|index| settings.provider_connection[index].phase);
        let mut withdrawn_any = false;
        for (index, phase) in phases.iter().enumerate() {
            let withdrawn = *phase == ProviderConnectPhase::Idle
                && self.last_seen_provider_phase[index] != ProviderConnectPhase::Idle;
            if withdrawn && self.remembered_connections[index] {
                self.remembered_connections[index] = false;
                withdrawn_any = true;
            }
        }
        self.last_seen_provider_phase = phases;
        if withdrawn_any {
            crate::agent_connect_store::save(&self.remembered_connections);
        }
    }

    /// Write UI preferences through on change (same frame-observer pattern
    /// as the connection store — the picker mutates state in the widget
    /// layer, which cannot reach the store).
    pub(crate) fn persist_ui_pref_changes(&mut self) {
        let style = self.host.editor_state().editor_ui.pencil_cursor_style;
        if self.last_saved_pencil_cursor == Some(style) {
            return;
        }
        if self.last_saved_pencil_cursor.is_none() {
            self.last_saved_pencil_cursor = Some(style);
            return;
        }
        crate::ui_prefs::save_pencil_cursor(style);
        self.last_saved_pencil_cursor = Some(style);
    }

    /// Begin the startup reconnect replay for providers remembered as
    /// connected last session. The first probe starts immediately; the
    /// rest ride the landing hook above, one at a time.
    pub(crate) fn restore_remembered_connections(&mut self) {
        let mut remembered = crate::agent_connect_store::load();
        // Adopt the stored list as this session's last-known-good set
        // before any probe runs, so a probe that fails cannot be mistaken
        // for a user withdrawal and evict the provider.
        self.remembered_connections = std::array::from_fn(|index| {
            AgentProvider::ALL
                .get(index)
                .is_some_and(|provider| remembered.contains(provider))
        });
        if remembered.is_empty() {
            return;
        }
        let first = remembered.remove(0);
        remembered.reverse();
        self.provider_reconnect_queue = remembered;
        // `begin_provider_connect`, not a bare write to the request seam:
        // the landing hook drops any outcome whose card is not in the
        // Probing phase, so a seam-only request ran a real probe whose
        // result was then discarded — and with it the hook that advances
        // this queue, stranding every provider after the first.
        self.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .begin_provider_connect(first);
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

    pub(super) fn reset_settings(app: &mut DesktopApp) {
        let es = app.host.editor_state_mut();
        es.editor_ui.agent_settings.connected = [false; 7];
        es.editor_ui.agent_settings.provider_connection = Default::default();
        es.editor_ui.agent_settings.pending_provider_connect = None;
        es.editor_ui.agent_settings.builtin_agents.clear();
        es.editor_ui.agent_settings.acp_agents.clear();
        es.chat.discovered_models.clear();
        es.rebuild_chat_models();
        app.remembered_connections = [false; 7];
        app.last_seen_provider_phase = Default::default();
    }

    /// Put one provider's card in a given phase and let the frame observer
    /// adopt it, so a later transition reads as a real change rather than
    /// as first-frame noise.
    pub(super) fn settle_phase(app: &mut DesktopApp, index: usize, phase: ProviderConnectPhase) {
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .provider_connection[index]
            .phase = phase;
        app.last_seen_provider_phase[index] = phase;
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
                AgentProvider::Antigravity,
                "default",
                "Antigravity Default",
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
        // Claude's entry spliced in, sorted before the seeded
        // Antigravity entry (ALL order), which survived.
        let values: Vec<&str> = es
            .chat
            .discovered_models
            .iter()
            .map(|m| m.value.as_str())
            .collect();
        assert_eq!(values, ["claude-sonnet-4-6", "default"]);
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

#[cfg(test)]
mod remembered_connection_tests {
    use super::tests::*;
    use super::*;
    use op_ai::agent_settings_state::AgentProvider as Sc;
    use op_ai::chat_models::ModelEntry as ScModelEntry;

    #[test]
    fn connection_store_writes_land_in_a_scratch_root_not_the_user_home() {
        // Deliberately installs no redirect of its own: the store must
        // refuse the real config directory on its own, whatever order the
        // harness runs its threads in. If a future change drops that
        // guard, this fails loudly instead of silently rewriting the
        // developer's `~/.openpencil/agents.json` — the file that decides
        // which providers auto-reconnect at launch.
        let real = op_config_store::home_dir()
            .expect("home directory")
            .join(".openpencil");

        crate::agent_connect_store::save(&[true, false, false, false, false, false, false]);

        let root = op_config_store::ConfigStore::user()
            .expect("a user root")
            .root()
            .to_path_buf();
        assert_ne!(root, real, "a test must never resolve the real config dir");
        assert!(root.join("agents.json").is_file());
        assert_eq!(
            crate::agent_connect_store::load(),
            vec![AgentProvider::ClaudeCode],
            "the scratch root round-trips, so reads are isolated too"
        );
    }

    #[test]
    fn a_failed_probe_leaves_the_remembered_list_intact() {
        // The original regression: a GUI launch with a broken environment made
        // every probe fail, each failure wrote `connected = false` through
        // to `agents.json`, and the provider was gone from the startup
        // replay list for good — the app could never recover on its own.
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
        app.remembered_connections[1] = true;
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
            connected: false,
            error: Some("Codex CLI failed: env: node: No such file or directory".into()),
            ..ProbeOutcome::default()
        })
        .unwrap();

        assert!(app.drain_provider_connect());
        app.persist_connection_changes();
        assert!(
            app.remembered_connections[1],
            "a failed probe must not evict the provider from next launch's replay"
        );
        assert!(
            !app.host.editor_state().editor_ui.agent_settings.connected[1],
            "the live card still reads disconnected — only the store is spared"
        );
    }

    #[test]
    fn a_successful_probe_joins_the_remembered_list() {
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
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
                "Sonnet",
            )],
            connection_info: Some("Connected via pro (a@b.c)".into()),
            ..ProbeOutcome::default()
        })
        .unwrap();

        assert!(app.drain_provider_connect());
        assert_eq!(
            app.remembered_connections,
            [true, false, false, false, false, false, false]
        );
    }

    #[test]
    fn an_explicit_disconnect_drops_the_provider_from_the_remembered_list() {
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
        app.remembered_connections[2] = true;
        settle_phase(&mut app, 2, ProviderConnectPhase::Connected);

        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .disconnect_provider(AgentProvider::OpenCode);
        app.persist_connection_changes();

        assert_eq!(app.remembered_connections, [false; 7]);
    }

    #[test]
    fn a_provider_waiting_its_turn_in_the_startup_replay_is_not_a_disconnect() {
        // Queued providers sit at Idle with `connected = false` until their
        // probe starts — the same absolute state an explicit Disconnect
        // leaves behind. Only the transition tells them apart.
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
        app.remembered_connections = [true; 7];

        app.persist_connection_changes();
        app.persist_connection_changes();

        assert_eq!(app.remembered_connections, [true; 7]);
    }

    #[test]
    fn a_probe_failure_that_lands_on_error_is_not_read_as_a_disconnect() {
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
        app.remembered_connections[5] = true;
        settle_phase(&mut app, 5, ProviderConnectPhase::Probing);
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .provider_connection[5]
            .phase = ProviderConnectPhase::Error;

        app.persist_connection_changes();

        assert!(app.remembered_connections[5]);
    }

    #[test]
    fn the_replay_queue_advances_into_the_probing_phase() {
        // The landing hook drops any outcome whose card is not Probing, so
        // handing the next provider a bare request-seam write produced a
        // probe whose result was discarded — and the queue stalled there.
        let mut app = DesktopApp::new(None);
        reset_settings(&mut app);
        app.provider_reconnect_queue = vec![AgentProvider::OpenCode];
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
            connected: false,
            error: Some("not authenticated".into()),
            ..ProbeOutcome::default()
        })
        .unwrap();

        assert!(app.drain_provider_connect());
        // The same pump that landed Claude's outcome consumed the queued
        // request seam and spawned OpenCode's probe.
        assert!(app.provider_connect_job.is_some());
        assert!(app.provider_reconnect_queue.is_empty());
        assert_eq!(
            app.host
                .editor_state()
                .editor_ui
                .agent_settings
                .provider_connection[2]
                .phase,
            ProviderConnectPhase::Probing,
            "the queued provider must be marked in-flight, or its outcome is dropped"
        );
    }
}
