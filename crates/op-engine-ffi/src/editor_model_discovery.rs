//! Mobile-editor pump for built-in-provider model discovery.
//!
//! Provider I/O runs on a shared Tokio runtime. Workers own only a cloned
//! provider snapshot and an `mpsc::Sender`; they never call an FFI callback or
//! touch [`op_host_native::WidgetHostNative`]. Results land on the engine
//! thread during `op_frame`, where editor-core's generation and credential
//! fingerprint checks reject stale responses.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::OnceLock;

use op_editor_core::{
    BuiltinAgentConfig, BuiltinModelCatalogRefreshOutcome, BuiltinModelCatalogRefreshRequest,
    BuiltinModelOption,
};
use op_host_native::WidgetHostNative;
use tokio::runtime::{Builder, Runtime};
use tokio::task::AbortHandle;

use crate::lifecycle::Session;

/// The shell already supports timed redraw wakes for caret animation. Reusing
/// that channel keeps every callback on the engine thread while a network job
/// is outstanding, instead of calling platform UI from a Tokio worker.
const POLL_INTERVAL_MS: u64 = 150;

struct ModelDiscoveryJob {
    request: BuiltinModelCatalogRefreshRequest,
    expected: BuiltinAgentConfig,
    rx: Receiver<BuiltinModelCatalogRefreshOutcome>,
    abort: Option<AbortHandle>,
}

/// Runtime-only jobs stay out of serializable editor state. Dropping an engine
/// aborts its outstanding requests so credential snapshots are not retained
/// until the network timeout after the platform has torn the editor down.
#[derive(Default)]
pub(crate) struct MobileModelDiscoveryHost {
    jobs: Vec<ModelDiscoveryJob>,
}

impl Drop for MobileModelDiscoveryHost {
    fn drop(&mut self) {
        for job in &self.jobs {
            if let Some(abort) = &job.abort {
                abort.abort();
            }
        }
    }
}

impl MobileModelDiscoveryHost {
    fn contains(&self, request: &BuiltinModelCatalogRefreshRequest) -> bool {
        self.jobs.iter().any(|job| job.request == *request)
    }

    fn start(
        &mut self,
        request: BuiltinModelCatalogRefreshRequest,
        expected: BuiltinAgentConfig,
    ) -> Result<bool, &'static str> {
        if self.contains(&request) {
            return Ok(false);
        }
        let runtime = model_discovery_runtime()?;
        let (tx, rx) = mpsc::channel();
        let worker_config = expected.clone();
        let task = runtime.spawn(async move {
            let outcome = discovery_outcome(
                op_builtin_model_discovery::discover_builtin_models(&worker_config).await,
            );
            let _ = tx.send(outcome);
        });
        self.jobs.push(ModelDiscoveryJob {
            request,
            expected,
            rx,
            abort: Some(task.abort_handle()),
        });
        Ok(true)
    }

    fn poll_into(&mut self, host: &mut WidgetHostNative) -> bool {
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
            changed |= host
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

    /// Land finished jobs, then consume every settings-owned request exactly
    /// once. Returns the next engine-thread poll deadline while work remains.
    pub(crate) fn pump(&mut self, host: &mut WidgetHostNative, now_ms: u64) -> Option<u64> {
        let mut changed = self.poll_into(host);

        loop {
            let request = host
                .editor_state_mut()
                .editor_ui
                .agent_settings
                .take_pending_builtin_model_catalog_refresh();
            let Some(request) = request else {
                break;
            };
            let expected = host
                .editor_state()
                .editor_ui
                .agent_settings
                .builtin_model_catalog_config_for_request(&request);
            let Some(expected) = expected.filter(BuiltinAgentConfig::discovery_ready) else {
                changed |= host
                    .editor_state_mut()
                    .editor_ui
                    .agent_settings
                    .invalidate_builtin_model_catalog(&request.target);
                continue;
            };
            match self.start(request.clone(), expected.clone()) {
                Ok(started) => changed |= started,
                Err(error) => {
                    changed |= host
                        .editor_state_mut()
                        .editor_ui
                        .agent_settings
                        .apply_builtin_model_catalog_refresh_outcome_if_current(
                            &expected,
                            &request,
                            BuiltinModelCatalogRefreshOutcome::Error {
                                error: error.into(),
                            },
                        );
                }
            }
        }

        if changed {
            host.mark_editor_state_dirty();
        }
        (!self.jobs.is_empty()).then(|| now_ms.saturating_add(POLL_INTERVAL_MS))
    }

    #[cfg(test)]
    fn consume_queued_test_job(
        &mut self,
        host: &mut WidgetHostNative,
        rx: Receiver<BuiltinModelCatalogRefreshOutcome>,
    ) -> BuiltinModelCatalogRefreshRequest {
        let request = host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .take_pending_builtin_model_catalog_refresh()
            .expect("settings press queued a discovery request");
        let expected = host
            .editor_state()
            .editor_ui
            .agent_settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("queued request owns a current provider snapshot");
        assert!(!self.contains(&request));
        self.jobs.push(ModelDiscoveryJob {
            request: request.clone(),
            expected,
            rx,
            abort: None,
        });
        request
    }
}

impl Session {
    pub(crate) fn pump_editor_model_discovery(&mut self, now_ms: u64) -> Option<u64> {
        let Session {
            editor,
            model_discovery,
            ..
        } = self;
        editor
            .as_mut()
            .and_then(|host| model_discovery.pump(host, now_ms))
    }
}

fn model_discovery_runtime() -> Result<&'static Runtime, &'static str> {
    static RUNTIME: OnceLock<Result<Runtime, ()>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .thread_name("op-mobile-model-discovery")
                .build()
                .map_err(|_| ())
        })
        .as_ref()
        .map_err(|_| "model discovery runtime is unavailable")
}

fn discovery_outcome(
    result: Result<
        op_builtin_model_discovery::BuiltinModelCatalog,
        op_builtin_model_discovery::BuiltinModelDiscoveryError,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    use op_editor_core::agent_settings::BuiltinAgentField;
    use op_editor_core::host_settings_commit::SettingsCommitScope;
    use op_editor_core::{BuiltinAgentKind, BuiltinAgentPresetKey, BuiltinModelCatalogPhase};
    use op_editor_ui::widgets::agent_settings_panel::AgentSettingsHit;
    use op_editor_ui::widgets::agent_settings_press_flow::apply_agent_settings_hit;

    fn host_with_provider() -> (WidgetHostNative, String) {
        let mut host = WidgetHostNative::new();
        let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
        settings.builtin_agents.clear();
        settings.clear_builtin_model_catalogs();
        let id = settings.add_builtin_agent_config(
            "Provider",
            "sk-mobile",
            "saved-model",
            BuiltinAgentKind::OpenAiCompat,
            "https://api.example.com/v1",
        );
        host.editor_state_mut().rebuild_chat_models();
        (host, id)
    }

    fn press_model_field(host: &mut WidgetHostNative) {
        apply_agent_settings_hit(
            host.editor_state_mut(),
            AgentSettingsHit::FocusBuiltinAgent {
                index: 0,
                field: BuiltinAgentField::Model,
            },
            SettingsCommitScope::Operator,
            10,
        );
    }

    fn spawn_catalog_server(body: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local catalog server");
        let address = listener.local_addr().expect("local catalog address");
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept discovery request");
            let mut request = [0_u8; 4096];
            let length = stream.read(&mut request).expect("read discovery request");
            let request = String::from_utf8_lossy(&request[..length]).into_owned();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write discovery response");
            request
        });
        (format!("http://{address}/v1"), handle)
    }

    #[test]
    fn settings_press_pump_runs_portable_discovery_off_the_engine_thread() {
        let (base_url, server) =
            spawn_catalog_server(r#"{"data":[{"id":"mobile-remote","name":"Mobile Remote"}]}"#);
        let (mut host, id) = host_with_provider();
        {
            let provider = &mut host
                .editor_state_mut()
                .editor_ui
                .agent_settings
                .builtin_agents[0];
            provider.preset = BuiltinAgentPresetKey::Custom;
            provider.base_url = base_url;
        }
        press_model_field(&mut host);

        let mut discovery = MobileModelDiscoveryHost::default();
        assert_eq!(discovery.pump(&mut host, 10), Some(160));
        for now_ms in (160..=6_160).step_by(POLL_INTERVAL_MS as usize) {
            std::thread::sleep(Duration::from_millis(5));
            if discovery.pump(&mut host, now_ms).is_none() {
                break;
            }
        }

        assert_eq!(
            host.editor_state()
                .editor_ui
                .agent_settings
                .builtin_model_catalog_options(&id),
            &[BuiltinModelOption::new("mobile-remote", "Mobile Remote")]
        );
        let request = server.join().expect("catalog server exits");
        assert!(request.starts_with("GET /v1/models HTTP/1.1"));
        assert!(request.contains("authorization: Bearer sk-mobile"));
    }

    #[test]
    fn settings_press_queue_reaches_mobile_consumer_and_success_lands() {
        let (mut host, id) = host_with_provider();
        let chat_before = host.editor_state().chat.available_models.clone();
        press_model_field(&mut host);

        let (tx, rx) = mpsc::channel();
        let mut discovery = MobileModelDiscoveryHost::default();
        let request = discovery.consume_queued_test_job(&mut host, rx);
        assert_eq!(
            request.target,
            op_editor_core::BuiltinModelCatalogTarget::Agent(id.clone())
        );
        tx.send(BuiltinModelCatalogRefreshOutcome::Success {
            models: vec![BuiltinModelOption::new("remote-model", "Remote Model")],
        })
        .unwrap();

        assert!(discovery.poll_into(&mut host));
        let settings = &host.editor_state().editor_ui.agent_settings;
        assert_eq!(
            settings.builtin_model_catalog_options(&id),
            &[BuiltinModelOption::new("remote-model", "Remote Model")]
        );
        assert_eq!(
            host.editor_state().chat.available_models,
            chat_before,
            "settings discovery must not rebuild or fetch for the chat picker"
        );
    }

    #[test]
    fn current_generation_error_exits_loading() {
        let (mut host, id) = host_with_provider();
        press_model_field(&mut host);
        let (tx, rx) = mpsc::channel();
        let mut discovery = MobileModelDiscoveryHost::default();
        discovery.consume_queued_test_job(&mut host, rx);
        tx.send(BuiltinModelCatalogRefreshOutcome::Error {
            error: "offline".into(),
        })
        .unwrap();

        assert!(discovery.poll_into(&mut host));
        let catalog = host
            .editor_state()
            .editor_ui
            .agent_settings
            .builtin_model_catalog(&op_editor_core::BuiltinModelCatalogTarget::Agent(id))
            .expect("error state remains visible for manual entry and retry");
        assert_eq!(catalog.phase, BuiltinModelCatalogPhase::Error);
        assert_eq!(catalog.error.as_deref(), Some("offline"));
    }

    #[test]
    fn changed_credential_rejects_late_mobile_result() {
        let (mut host, id) = host_with_provider();
        press_model_field(&mut host);
        let (tx, rx) = mpsc::channel();
        let mut discovery = MobileModelDiscoveryHost::default();
        discovery.consume_queued_test_job(&mut host, rx);
        host.editor_state_mut()
            .editor_ui
            .agent_settings
            .builtin_agents[0]
            .api_key = "sk-replaced".into();
        tx.send(BuiltinModelCatalogRefreshOutcome::Success {
            models: vec![BuiltinModelOption::new("stale-model", "Stale Model")],
        })
        .unwrap();

        assert!(!discovery.poll_into(&mut host));
        let settings = &host.editor_state().editor_ui.agent_settings;
        assert!(settings.builtin_model_catalog_options(&id).is_empty());
        assert!(!settings
            .builtin_model_catalogs
            .contains_key(&op_editor_core::BuiltinModelCatalogTarget::Agent(id)));
    }

    #[test]
    fn old_generation_cannot_replace_a_new_mobile_request() {
        let (mut host, id) = host_with_provider();
        press_model_field(&mut host);
        let (tx, rx) = mpsc::channel();
        let mut discovery = MobileModelDiscoveryHost::default();
        let old = discovery.consume_queued_test_job(&mut host, rx);

        let target = op_editor_core::BuiltinModelCatalogTarget::Agent(id);
        let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
        assert!(settings.invalidate_builtin_model_catalog(&target));
        let new = settings
            .begin_builtin_model_catalog_refresh(target.clone(), 20)
            .expect("new generation starts");
        assert!(new.generation > old.generation);
        tx.send(BuiltinModelCatalogRefreshOutcome::Success {
            models: vec![BuiltinModelOption::new("old-model", "Old Model")],
        })
        .unwrap();

        assert!(!discovery.poll_into(&mut host));
        let catalog = host
            .editor_state()
            .editor_ui
            .agent_settings
            .builtin_model_catalog(&target)
            .expect("new request remains current");
        assert_eq!(catalog.phase, BuiltinModelCatalogPhase::Loading);
        assert_eq!(catalog.generation, new.generation);
        assert!(catalog.models.is_empty());
    }

    #[test]
    fn portable_errors_map_without_credentials_or_endpoint_text() {
        let outcome = discovery_outcome(Err(
            op_builtin_model_discovery::BuiltinModelDiscoveryError::RequestFailed,
        ));
        assert_eq!(
            outcome,
            BuiltinModelCatalogRefreshOutcome::Error {
                error: "model discovery request failed".into()
            }
        );
    }
}
