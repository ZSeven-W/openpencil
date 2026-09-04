//! Background startup model-catalog probe.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use op_ai::chat_models::ModelEntry;

use crate::model_discovery::{discover_models_for_connected, model_entry_to_ec};

pub struct ModelProbe {
    rx: Option<Receiver<Vec<ModelEntry>>>,
    providers: [bool; 7],
}

impl ModelProbe {
    pub fn idle() -> Self {
        Self {
            rx: None,
            providers: [false; 7],
        }
    }

    /// Spawn a full-catalog discovery worker for compatibility with callers
    /// that explicitly need every locally installed provider.
    pub fn spawn() -> Self {
        Self::spawn_for_connected([true; 7])
    }

    /// Probe only providers connected at startup. Provider queries execute
    /// concurrently, so their individual timeouts do not stack serially.
    pub fn spawn_for_connected(connected: [bool; 7]) -> Self {
        if !connected.iter().any(|is_connected| *is_connected) {
            return Self::idle();
        }
        let (tx, rx) = mpsc::channel();
        // Detached one-shot: the worker sends exactly one catalog and returns.
        // `discover_models_for_connected` is itself deadline-bounded (every CLI
        // probe under it runs against a timeout), so the thread cannot outlive
        // its purpose — dropping `rx` on the UI side just makes the final send
        // a no-op. Nothing to join, nothing to signal.
        std::thread::spawn(move || {
            let _ = tx.send(discover_models_for_connected(connected));
        });
        Self {
            rx: Some(rx),
            providers: connected,
        }
    }

    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    #[cfg(test)]
    pub(crate) fn pending_for_test() -> (Self, mpsc::Sender<Vec<ModelEntry>>) {
        Self::pending_for_providers_for_test([true; 7])
    }

    #[cfg(test)]
    fn pending_for_providers_for_test(
        providers: [bool; 7],
    ) -> (Self, mpsc::Sender<Vec<ModelEntry>>) {
        let (tx, rx) = mpsc::channel();
        (
            Self {
                rx: Some(rx),
                providers,
            },
            tx,
        )
    }

    /// Merge completed provider slices into the live catalog. Unprobed slices
    /// stay intact in case the user connected another provider during startup.
    pub fn poll_into(&mut self, state: &mut op_editor_core::EditorState) -> bool {
        let Some(rx) = self.rx.as_ref() else {
            return false;
        };
        match rx.try_recv() {
            Ok(models) => {
                for (index, provider) in op_editor_core::AgentProvider::ALL.iter().enumerate() {
                    if self.providers[index] {
                        state
                            .chat
                            .discovered_models
                            .retain(|model| model.provider != *provider);
                    }
                }
                state
                    .chat
                    .discovered_models
                    .extend(models.into_iter().map(model_entry_to_ec));
                state.chat.discovered_models.sort_by_key(|model| {
                    op_editor_core::AgentProvider::ALL
                        .iter()
                        .position(|provider| *provider == model.provider)
                        .unwrap_or(usize::MAX)
                });
                state.rebuild_chat_models();
                self.rx = None;
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::agent_settings_state::AgentProvider as AiProvider;
    use op_editor_core::{AgentProvider, EditorState, ModelEntry as EditorModelEntry};

    #[test]
    fn empty_connected_mask_stays_idle() {
        assert!(!ModelProbe::spawn_for_connected([false; 7]).is_pending());
    }

    #[test]
    fn filtered_result_does_not_erase_unprobed_provider() {
        let mut providers = [false; 7];
        providers[4] = true;
        let (mut probe, tx) = ModelProbe::pending_for_providers_for_test(providers);
        let mut state = EditorState::default();
        state.chat.discovered_models.push(EditorModelEntry::new(
            AgentProvider::GrokBuild,
            "grok-existing",
            "Grok existing",
        ));
        tx.send(vec![ModelEntry::new(
            AiProvider::Antigravity,
            "agy-new",
            "Antigravity new",
        )])
        .expect("probe result");

        assert!(probe.poll_into(&mut state));
        assert_eq!(state.chat.discovered_models.len(), 2);
        assert_eq!(
            state.chat.discovered_models[0].provider,
            AgentProvider::Antigravity
        );
        assert_eq!(
            state.chat.discovered_models[1].provider,
            AgentProvider::GrokBuild
        );
    }
}
