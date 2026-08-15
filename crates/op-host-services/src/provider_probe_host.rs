//! Connect-time provider-probe job + outcome normalization — the
//! host-free half carved out of `op-host-desktop`'s
//! `provider_probe_host.rs` (codex Issue 5: the job struct is a
//! `DesktopApp` field, so it must live here for both crates to name
//! it). The `impl DesktopApp` pump stays desktop-side and drives this
//! job through its public API (`spawn` / `is_pending` / `provider` /
//! `poll`).

use std::sync::mpsc::{self, Receiver, TryRecvError};

use crate::provider_probe::{connect_provider, ProbeOutcome};

/// One in-flight connect probe. A single slot suffices — the modal
/// shows one Connect press at a time and the press handler ignores
/// re-presses while a card is `Probing`.
pub struct ProviderConnectJob {
    provider: op_editor_core::AgentProvider,
    rx: Option<Receiver<ProbeOutcome>>,
}

impl ProviderConnectJob {
    pub fn spawn(provider: op_editor_core::AgentProvider) -> Self {
        let (tx, rx) = mpsc::channel();
        let sc_provider = provider_to_sc(provider);
        // Detached one-shot: `connect_provider` runs deadline-bounded CLI
        // probes and returns a single outcome, so the thread always finishes on
        // its own. A dropped `rx` (modal closed) only makes the send a no-op.
        std::thread::spawn(move || {
            // Resolved on the worker (settings file + OS locale) so the
            // probe's status/error strings match the chrome language.
            let locale = crate::provider_probe::resolved_ui_locale();
            let _ = tx.send(connect_provider(sc_provider, locale));
        });
        Self {
            provider,
            rx: Some(rx),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    /// The provider this job is probing. Public accessor for the
    /// desktop-residual pump (which can't read the private field across
    /// the crate boundary).
    pub fn provider(&self) -> op_editor_core::AgentProvider {
        self.provider
    }

    /// Test seam: construct a pending job + the sender that feeds it a
    /// fake outcome. Public (not `#[cfg(test)]`) so the desktop residual's
    /// `impl DesktopApp` tests can build one across the crate boundary.
    #[doc(hidden)]
    pub fn pending_for_test(
        provider: op_editor_core::AgentProvider,
    ) -> (Self, mpsc::Sender<ProbeOutcome>) {
        let (tx, rx) = mpsc::channel();
        (
            Self {
                provider,
                rx: Some(rx),
            },
            tx,
        )
    }

    pub fn poll(&mut self) -> Option<ProbeOutcome> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(outcome) => {
                self.rx = None;
                Some(outcome)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                None
            }
        }
    }
}

/// op-editor-core's `AgentProvider` → the shell-core twin the probe
/// layer speaks (same variants, same order).
fn provider_to_sc(p: op_editor_core::AgentProvider) -> op_ai::agent_settings_state::AgentProvider {
    use op_ai::agent_settings_state::AgentProvider as Sc;
    match p {
        op_editor_core::AgentProvider::ClaudeCode => Sc::ClaudeCode,
        op_editor_core::AgentProvider::CodexCli => Sc::CodexCli,
        op_editor_core::AgentProvider::OpenCode => Sc::OpenCode,
        op_editor_core::AgentProvider::GithubCopilot => Sc::GithubCopilot,
        op_editor_core::AgentProvider::Antigravity => Sc::Antigravity,
        op_editor_core::AgentProvider::GrokBuild => Sc::GrokBuild,
        op_editor_core::AgentProvider::DeepSeekHarness => Sc::DeepSeekHarness,
    }
}

pub fn normalize_provider_probe_outcome(
    provider: op_editor_core::AgentProvider,
    mut outcome: ProbeOutcome,
) -> ProbeOutcome {
    if outcome.connected && outcome.models.is_empty() {
        outcome.connected = false;
        outcome.connection_info = None;
        outcome.error = Some(missing_models_connect_error(provider));
    }
    outcome
}

pub fn missing_models_connect_error(provider: op_editor_core::AgentProvider) -> String {
    // Pinned to English: this normalization is a belt-and-suspenders
    // guard the pump applies on the UI thread (its callers pass no
    // locale, and reading the settings file here would make the desktop
    // pump tests depend on the machine's ambient locale). The probe
    // itself already produces fully localized no-model errors in
    // `connect_provider`.
    op_editor_core::missing_models_connect_error(op_i18n::Locale::EnUs, provider)
}
