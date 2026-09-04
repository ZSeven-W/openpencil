//! Desktop pump for built-in-provider model discovery.
//!
//! Editor-core owns the request generation and credential fingerprint. This
//! host only performs native async I/O, then hands the snapshot and result
//! back to core so a late response can never overwrite a newer account.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use op_editor_core::{
    BuiltinAgentConfig, BuiltinModelCatalogRefreshOutcome, BuiltinModelCatalogRefreshRequest,
    BuiltinModelOption,
};
use op_host_services::builtin_model_discovery::{
    discover_builtin_models, BuiltinModelAccess, BuiltinModelDiscoveryError,
};

use crate::DesktopApp;

struct BuiltinModelRefreshJob {
    request: BuiltinModelCatalogRefreshRequest,
    expected: BuiltinAgentConfig,
    rx: Receiver<BuiltinModelCatalogRefreshOutcome>,
}

/// Native jobs are deliberately separate from serializable editor state.
#[derive(Default)]
pub(crate) struct BuiltinModelRefreshHost {
    jobs: Vec<BuiltinModelRefreshJob>,
}

impl BuiltinModelRefreshHost {
    fn contains(&self, request: &BuiltinModelCatalogRefreshRequest) -> bool {
        self.jobs.iter().any(|job| job.request == *request)
    }

    fn start(
        &mut self,
        request: BuiltinModelCatalogRefreshRequest,
        expected: BuiltinAgentConfig,
    ) -> bool {
        if self.contains(&request) {
            return false;
        }
        let (tx, rx) = mpsc::channel();
        let worker_config = expected.clone();
        op_host_services::chat_runtime::shared_runtime().spawn(async move {
            let outcome = discovery_outcome(
                discover_builtin_models(&worker_config, BuiltinModelAccess::Trusted).await,
            );
            let _ = tx.send(outcome);
        });
        self.jobs.push(BuiltinModelRefreshJob {
            request,
            expected,
            rx,
        });
        true
    }

    fn poll_into(&mut self, app: &mut DesktopApp) -> bool {
        let mut changed = false;
        let mut index = 0;
        while index < self.jobs.len() {
            let outcome = match self.jobs[index].rx.try_recv() {
                Ok(outcome) => outcome,
                Err(TryRecvError::Empty) => {
                    index += 1;
                    continue;
                }
                Err(TryRecvError::Disconnected) => BuiltinModelCatalogRefreshOutcome::Error {
                    error: "model discovery worker stopped before returning a result".into(),
                },
            };
            let job = self.jobs.swap_remove(index);
            changed |= app
                .host
                .editor_state_mut()
                .editor_ui
                .agent_settings
                .apply_builtin_model_catalog_refresh_outcome_if_current(
                    &job.expected,
                    &job.request,
                    outcome,
                );
        }
        changed
    }

    pub(crate) fn is_pending(&self) -> bool {
        !self.jobs.is_empty()
    }

    #[cfg(test)]
    fn push_test_job(
        &mut self,
        request: BuiltinModelCatalogRefreshRequest,
        expected: BuiltinAgentConfig,
        rx: Receiver<BuiltinModelCatalogRefreshOutcome>,
    ) -> bool {
        if self.contains(&request) {
            return false;
        }
        self.jobs.push(BuiltinModelRefreshJob {
            request,
            expected,
            rx,
        });
        true
    }
}

fn discovery_outcome(
    result: Result<
        op_host_services::builtin_model_discovery::BuiltinModelCatalog,
        BuiltinModelDiscoveryError,
    >,
) -> BuiltinModelCatalogRefreshOutcome {
    match result {
        Ok(catalog) => BuiltinModelCatalogRefreshOutcome::Success {
            models: catalog
                .models
                .into_iter()
                .map(|model| BuiltinModelOption {
                    id: model.id,
                    display_name: model.display_name,
                })
                .collect(),
        },
        Err(error) if error.is_unsupported() => BuiltinModelCatalogRefreshOutcome::Unsupported {
            error: Some(error.to_string()),
        },
        Err(error) => BuiltinModelCatalogRefreshOutcome::Error {
            error: error.to_string(),
        },
    }
}

impl DesktopApp {
    /// Land finished jobs and consume every queued core request once.
    pub(crate) fn drain_builtin_model_refresh(&mut self) -> bool {
        // Temporarily move the host out to avoid borrowing `self` and one of
        // its fields mutably at the same time while results land into core.
        let mut refresh = std::mem::take(&mut self.builtin_model_refresh);
        let mut changed = refresh.poll_into(self);

        loop {
            let request = self
                .host
                .editor_state_mut()
                .editor_ui
                .agent_settings
                .take_pending_builtin_model_catalog_refresh();
            let Some(request) = request else {
                break;
            };
            let config = self
                .host
                .editor_state()
                .editor_ui
                .agent_settings
                .builtin_model_catalog_config_for_request(&request);
            if let Some(config) = config.filter(BuiltinAgentConfig::discovery_ready) {
                changed |= refresh.start(request, config);
            } else {
                changed |= self
                    .host
                    .editor_state_mut()
                    .editor_ui
                    .agent_settings
                    .invalidate_builtin_model_catalog(&request.target);
            }
        }

        self.builtin_model_refresh = refresh;
        if changed {
            self.host.mark_editor_state_dirty();
        }
        changed
    }

    /// Keep the idle event loop waking until every native request lands.
    pub(crate) fn builtin_model_refresh_pending(&self) -> bool {
        self.builtin_model_refresh.is_pending()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::BuiltinModelCatalogTarget;

    fn app_with_builtin(api_key: &str) -> (DesktopApp, String) {
        let mut app = DesktopApp::new(None);
        let state = app.host.editor_state_mut();
        state.editor_ui.agent_settings.builtin_agents.clear();
        state
            .editor_ui
            .agent_settings
            .clear_builtin_model_catalogs();
        state.chat.available_models.clear();
        let id = state
            .editor_ui
            .agent_settings
            .add_builtin_agent_with_defaults("Provider", api_key, "fallback-model");
        (app, id)
    }

    #[test]
    fn identical_request_gets_only_one_in_flight_job() {
        let (mut app, _) = app_with_builtin("sk-test");
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.request_ready_builtin_model_catalog_refreshes(1);
        let request = settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("ready provider queues refresh");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("request resolves its provider snapshot");
        let (_tx, rx) = mpsc::channel();

        assert!(app
            .builtin_model_refresh
            .push_test_job(request.clone(), expected.clone(), rx));
        let (_duplicate_tx, duplicate_rx) = mpsc::channel();
        assert!(!app
            .builtin_model_refresh
            .push_test_job(request, expected, duplicate_rx));
        assert_eq!(app.builtin_model_refresh.jobs.len(), 1);
    }

    #[test]
    fn disabled_request_is_invalidated_instead_of_staying_loading() {
        let (mut app, id) = app_with_builtin("sk-test");
        let target = BuiltinModelCatalogTarget::Agent(id);
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        assert!(settings
            .begin_builtin_model_catalog_refresh(target.clone(), 1)
            .is_some());
        settings.builtin_agents[0].enabled = false;

        assert!(app.drain_builtin_model_refresh());
        assert!(!app
            .host
            .editor_state()
            .editor_ui
            .agent_settings
            .builtin_model_catalogs
            .contains_key(&target));

        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.builtin_agents[0].enabled = true;
        assert!(settings
            .begin_builtin_model_catalog_refresh(target, 2)
            .is_some());
    }

    #[test]
    fn late_result_for_changed_credentials_is_ignored() {
        let (mut app, id) = app_with_builtin("sk-old");
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.request_ready_builtin_model_catalog_refreshes(1);
        let request = settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("ready provider queues refresh");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("request resolves its provider snapshot");
        let (tx, rx) = mpsc::channel();
        app.builtin_model_refresh
            .push_test_job(request, expected, rx);
        app.host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .builtin_agents[0]
            .api_key = "sk-new".into();
        assert_eq!(
            app.host
                .editor_state()
                .editor_ui
                .agent_settings
                .builtin_agents[0]
                .id,
            id
        );
        tx.send(BuiltinModelCatalogRefreshOutcome::Success {
            models: vec![BuiltinModelOption {
                id: "remote-model".into(),
                display_name: "remote-model".into(),
            }],
        })
        .expect("host owns receiver");

        assert!(!app.drain_builtin_model_refresh());
        assert!(!app
            .host
            .editor_state()
            .chat
            .available_models
            .iter()
            .any(|model| model.display_name == "remote-model"));
    }

    #[test]
    fn successful_refresh_updates_settings_catalog_without_changing_chat_picker() {
        let (mut app, id) = app_with_builtin("sk-test");
        app.host.editor_state_mut().rebuild_chat_models();
        let chat_before = app.host.editor_state().chat.available_models.clone();
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.request_ready_builtin_model_catalog_refreshes(1);
        let request = settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("ready provider queues refresh");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("request resolves its provider snapshot");
        let (tx, rx) = mpsc::channel();
        app.builtin_model_refresh
            .push_test_job(request, expected, rx);
        tx.send(BuiltinModelCatalogRefreshOutcome::Success {
            models: vec![BuiltinModelOption {
                id: "remote-model".into(),
                display_name: "Remote Model".into(),
            }],
        })
        .expect("host owns receiver");

        assert!(app.drain_builtin_model_refresh());
        assert_eq!(
            app.host.editor_state().chat.available_models,
            chat_before,
            "runtime discovery is settings-only"
        );
        assert!(!app
            .host
            .editor_state()
            .chat
            .available_models
            .iter()
            .any(|model| model.display_name == "Remote Model"));
        assert_eq!(
            app.host
                .editor_state()
                .editor_ui
                .agent_settings
                .builtin_model_catalog_options(&id),
            &[BuiltinModelOption::new("remote-model", "Remote Model")]
        );
    }

    #[test]
    fn failed_refresh_does_not_change_saved_chat_models() {
        let (mut app, _) = app_with_builtin("sk-test");
        app.host.editor_state_mut().rebuild_chat_models();
        let chat_before = app.host.editor_state().chat.available_models.clone();
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.request_ready_builtin_model_catalog_refreshes(1);
        let request = settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("ready provider queues refresh");
        let expected = settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("request resolves its provider snapshot");
        let (tx, rx) = mpsc::channel();
        app.builtin_model_refresh
            .push_test_job(request, expected, rx);
        tx.send(BuiltinModelCatalogRefreshOutcome::Error {
            error: "offline".into(),
        })
        .expect("host owns receiver");

        assert!(app.drain_builtin_model_refresh());
        assert_eq!(app.host.editor_state().chat.available_models, chat_before);
    }
}
