//! Shared GitHub Copilot CLI probe backed by the official SDK.
//!
//! The shared probe applies the SDK server argv (`--server --stdio`) and lets
//! the SDK own Content-Length framing and request correlation. Keep connect-
//! time checks and background discovery on this path so they cannot drift.

use std::fmt;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use github_copilot_sdk::rpc::ModelPolicyState;
use github_copilot_sdk::types::{GetAuthStatusResponse, Model};
use github_copilot_sdk::Client;
use op_ai::agent_settings_state::AgentProvider;
use op_ai::chat_models::ModelEntry;
use tokio::process::{Child, Command};

const COPILOT_PROBE_TIMEOUT: Duration = Duration::from_secs(15);
const COPILOT_AUTH_STATUS_TIMEOUT: Duration = Duration::from_secs(2);
const COPILOT_SERVER_ARGS: [&str; 5] = [
    "--server",
    "--stdio",
    "--no-auto-update",
    "--log-level",
    "info",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CopilotAuth {
    pub(crate) login: Option<String>,
    pub(crate) auth_type: Option<String>,
    pub(crate) status_message: Option<String>,
}

impl From<GetAuthStatusResponse> for CopilotAuth {
    fn from(status: GetAuthStatusResponse) -> Self {
        Self {
            login: status.login,
            auth_type: status.auth_type,
            status_message: status.status_message,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CopilotProbe {
    pub(crate) models: Vec<ModelEntry>,
    pub(crate) auth: Option<CopilotAuth>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CopilotProbeError {
    TimedOut,
    Sdk(String),
}

impl fmt::Display for CopilotProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimedOut => f.write_str("Connection timed out"),
            Self::Sdk(message) => f.write_str(message),
        }
    }
}

/// Start the explicitly resolved CLI in the official SDK server mode, verify
/// auth, and return the live model catalog. The process stays in a local guard
/// rather than inside `Client::start`: SDK 1.0.11's lifecycle task retains the
/// client after a cancelled startup, which can otherwise strand the child.
pub(crate) fn probe_copilot_cli(exe: &Path) -> Result<CopilotProbe, CopilotProbeError> {
    probe_copilot_cli_with_timeout(exe, COPILOT_PROBE_TIMEOUT)
}

fn probe_copilot_cli_with_timeout(
    exe: &Path,
    timeout: Duration,
) -> Result<CopilotProbe, CopilotProbeError> {
    let exe = exe.to_path_buf();
    std::thread::Builder::new()
        .name("openpencil-copilot-probe".to_string())
        .spawn(move || {
            // SDK 1.0.11's lifecycle dispatcher retains its ClientInner. A
            // disposable runtime cancels that task after every one-shot probe
            // instead of accumulating one retained task per catalog refresh.
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| CopilotProbeError::Sdk(error.to_string()))?;
            runtime.block_on(async move {
                let server = CopilotServer::spawn(&exe).await?;
                probe_started_server(server, timeout).await
            })
        })
        .map_err(|error| CopilotProbeError::Sdk(error.to_string()))?
        .join()
        .map_err(|_| CopilotProbeError::Sdk("Copilot probe worker panicked".to_string()))?
}

async fn probe_started_server(
    mut server: CopilotServer,
    timeout: Duration,
) -> Result<CopilotProbe, CopilotProbeError> {
    // Keep the child-owning guard outside the timed future. After cancellation
    // we can synchronously signal it and await the reap before its disposable
    // runtime shuts down; `kill_on_drop` remains the panic/early-return guard.
    let models = match tokio::time::timeout(timeout, async {
        server
            .client
            .verify_protocol_version()
            .await
            .map_err(|error| CopilotProbeError::Sdk(error.to_string()))?;
        query_copilot_models(&server.client).await
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Err(CopilotProbeError::TimedOut),
    };
    // Authentication only enriches the card and owns a separate budget. A
    // valid catalog that arrives near the essential deadline must stay valid.
    let result = match models {
        Ok(models) => Ok(CopilotProbe {
            models,
            auth: query_copilot_auth(&server.client).await,
        }),
        Err(error) => Err(error),
    };
    server.shutdown().await;
    result
}

/// SDK-framed client plus an independently owned CLI child. Keeping the child
/// outside SDK 1.0.11 makes an outer timeout cancellation-safe: this guard's
/// Drop always sends a kill even when the SDK is still awaiting its handshake.
pub(crate) struct CopilotServer {
    client: Client,
    child: Option<Child>,
}

impl CopilotServer {
    pub(crate) async fn spawn(exe: &Path) -> Result<Self, CopilotProbeError> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let mut command = Command::new(exe);
        command
            .args(COPILOT_SERVER_ARGS)
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(not(windows))]
        {
            command.env("PATH", crate::chat_spawn::effective_path_env());
        }
        #[cfg(unix)]
        {
            command.process_group(0);
        }
        crate::chat_spawn::hide_console_window(command.as_std_mut());

        let mut child = command
            .spawn()
            .map_err(|error| CopilotProbeError::Sdk(error.to_string()))?;
        let client = match (child.stdin.take(), child.stdout.take()) {
            (Some(stdin), Some(stdout)) => Client::from_streams(stdout, stdin, cwd)
                .map_err(|error| CopilotProbeError::Sdk(error.to_string())),
            (None, _) => Err(CopilotProbeError::Sdk(
                "Copilot CLI started without stdin".to_string(),
            )),
            (_, None) => Err(CopilotProbeError::Sdk(
                "Copilot CLI started without stdout".to_string(),
            )),
        };
        match client {
            Ok(client) => Ok(Self {
                client,
                child: Some(child),
            }),
            Err(error) => {
                stop_and_reap_child(&mut child).await;
                Err(error)
            }
        }
    }

    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    pub(crate) async fn shutdown(&mut self) {
        self.client.force_stop();
        if let Some(child) = self.child.as_mut() {
            // Keep ownership in `self` across the await. If the shutdown
            // future is cancelled, Drop can still force-stop the server.
            stop_and_reap_child(child).await;
        }
        if self
            .child
            .as_ref()
            .is_some_and(|child| child.id().is_none())
        {
            self.child.take();
        }
    }
}

impl Drop for CopilotServer {
    fn drop(&mut self) {
        self.client.force_stop();
        if let Some(child) = self.child.as_mut() {
            // `try_wait` reaps an already-exited leader and prevents a numeric
            // PID tree signal from racing a reused Windows PID. An active
            // child gets the shared nonblocking tree-aware kill path.
            if !matches!(child.try_wait(), Ok(Some(_))) {
                let _ = op_process_io::kill_tokio_process_tree(child);
            }
        }
    }
}

async fn stop_and_reap_child(child: &mut Child) {
    // Avoid signaling an already-exited numeric PID. Otherwise the shared
    // helper signals descendants before reaping and does not wait forever if
    // neither the tree nor the exact leader accepted termination.
    if !matches!(child.try_wait(), Ok(Some(_))) {
        let _ = op_process_io::terminate_tokio_process_tree(child, Duration::ZERO).await;
    }
}

/// Query an already-connected SDK client. Keeping this boundary independent
/// of process spawning lets the framing/auth/model contract be tested over an
/// in-memory transport, including a terminal frame with no trailing newline.
async fn query_copilot_models(client: &Client) -> Result<Vec<ModelEntry>, CopilotProbeError> {
    // The live catalog is the connection authority. Query it first so a slow
    // best-effort auth status cannot consume the essential probe budget; this
    // also keeps BYOK Copilot servers usable when GitHub auth is unnecessary.
    // SDK 1.0.11 models current billing metadata directly: legacy multiplier
    // and tokenPrices are both optional typed fields.
    let models = client
        .list_models()
        .await
        .map_err(|error| CopilotProbeError::Sdk(error.to_string()))?;
    Ok(copilot_model_entries(models))
}

async fn query_copilot_auth(client: &Client) -> Option<CopilotAuth> {
    // Auth text only enriches the connected card. An older server without the
    // method, or a BYOK server without GitHub auth, can still connect when its
    // live model catalog succeeded.
    tokio::time::timeout(COPILOT_AUTH_STATUS_TIMEOUT, client.get_auth_status())
        .await
        .ok()
        .and_then(Result::ok)
        .map(Into::into)
}

/// Preserve the retired TS policy rule: a model with no policy is visible,
/// while a model carrying a policy is visible only when explicitly enabled.
fn copilot_model_entries(models: Vec<Model>) -> Vec<ModelEntry> {
    models
        .into_iter()
        .filter(|model| {
            model
                .policy
                .as_ref()
                .is_none_or(|policy| policy.state == ModelPolicyState::Enabled)
        })
        .filter_map(|model| {
            let id = model.id.trim().to_string();
            if id.is_empty() {
                return None;
            }
            let name = model.name.trim();
            let name = if name.is_empty() {
                id.clone()
            } else {
                name.to_string()
            };
            Some(ModelEntry::new(AgentProvider::GithubCopilot, id, name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use github_copilot_sdk::types::{ModelBilling, ModelBillingTokenPrices, ModelPolicy};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

    fn sdk_model(id: &str, name: &str, policy: Option<ModelPolicyState>) -> Model {
        Model {
            id: id.to_string(),
            name: name.to_string(),
            policy: policy.map(|state| ModelPolicy { state, terms: None }),
            ..Default::default()
        }
    }

    #[test]
    fn typed_models_accept_token_prices_and_keep_only_policy_enabled_entries() {
        let mut current_billing = sdk_model("no-policy", "No policy", None);
        current_billing.billing = Some(ModelBilling {
            multiplier: None,
            token_prices: Some(ModelBillingTokenPrices::default()),
            ..Default::default()
        });
        let models = copilot_model_entries(vec![
            current_billing,
            sdk_model("null-policy", "Null policy", None),
            sdk_model("enabled", "Enabled", Some(ModelPolicyState::Enabled)),
            sdk_model("disabled", "Disabled", Some(ModelPolicyState::Disabled)),
        ]);

        let values: Vec<&str> = models.iter().map(|model| model.value.as_str()).collect();
        assert_eq!(values, ["no-policy", "null-policy", "enabled"]);
    }

    #[test]
    fn sdk_models_drop_blank_ids_and_fall_back_from_blank_names() {
        let models = copilot_model_entries(vec![
            sdk_model("  ", "Missing id", None),
            sdk_model("gpt-5-mini", "  ", None),
        ]);

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].value, "gpt-5-mini");
        assert_eq!(models[0].display_name, "gpt-5-mini");
    }

    #[cfg(unix)]
    #[test]
    fn dropping_guard_kills_a_hanging_server_process_tree() {
        use std::os::unix::fs::PermissionsExt as _;

        let script = std::env::temp_dir().join(format!(
            "openpencil-copilot-probe-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let ready = std::path::PathBuf::from(format!("{}.ready", script.display()));
        let survived = std::path::PathBuf::from(format!("{}.survived", script.display()));
        std::fs::write(
            &script,
            "#!/bin/sh\n(printf ready > \"$0.ready\"; sleep 1; printf survived > \"$0.survived\") &\nwait\n",
        )
        .expect("write fake Copilot server");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("make fake server executable");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        runtime.block_on(async {
            let server = CopilotServer::spawn(&script)
                .await
                .expect("spawn fake server");
            let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
            while !ready.exists() {
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "fake server did not report readiness"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            drop(server);
        });
        drop(runtime);

        std::thread::sleep(Duration::from_millis(1_500));
        let child_survived = survived.exists();
        let _ = std::fs::remove_file(&script);
        let _ = std::fs::remove_file(&ready);
        let _ = std::fs::remove_file(&survived);
        assert!(!child_survived, "server survived cancellation");
    }

    #[cfg(unix)]
    #[test]
    fn hanging_server_returns_the_bounded_timeout_error() {
        use std::os::unix::fs::PermissionsExt as _;

        let script = std::env::temp_dir().join(format!(
            "openpencil-copilot-timeout-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::write(&script, "#!/bin/sh\nexec sleep 5\n").expect("write fake server");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("make fake server executable");

        let result = probe_copilot_cli_with_timeout(&script, Duration::from_millis(50));
        let _ = std::fs::remove_file(&script);
        assert!(matches!(result, Err(CopilotProbeError::TimedOut)));
    }

    #[test]
    fn sdk_client_reads_terminal_framed_model_reply_and_maps_auth() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        runtime.block_on(async {
            let (client_stream, server_stream) = tokio::io::duplex(32 * 1024);
            let (client_reader, client_writer) = tokio::io::split(client_stream);
            let (server_reader, mut server_writer) = tokio::io::split(server_stream);
            let mut server_reader = tokio::io::BufReader::new(server_reader);
            let client = Client::from_streams(
                client_reader,
                client_writer,
                std::env::current_dir().expect("current directory"),
            )
            .expect("in-memory SDK client");

            let server = tokio::spawn(async move {
                let models_request = read_frame(&mut server_reader).await;
                assert_eq!(models_request["method"], "models.list");
                // `write_frame` deliberately adds no newline after the body.
                // The server then waits for the auth request, so the SDK must
                // decode this model reply by Content-Length before any later
                // frame could accidentally delimit it.
                write_frame(
                    &mut server_writer,
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": models_request["id"],
                        "result": {
                            "models": [
                                {
                                    "id": "gpt-5-mini",
                                    "name": "GPT-5 mini",
                                    "capabilities": {}
                                },
                                {
                                    "id": "disabled-model",
                                    "name": "Disabled",
                                    "capabilities": {},
                                    "policy": { "state": "disabled" }
                                }
                            ]
                        }
                    }),
                )
                .await;

                let auth_request = read_frame(&mut server_reader).await;
                assert_eq!(auth_request["method"], "auth.getStatus");
                write_frame(
                    &mut server_writer,
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": auth_request["id"],
                        "result": {
                            "isAuthenticated": true,
                            "authType": "oauth",
                            "login": "octocat",
                            "statusMessage": "Authenticated"
                        }
                    }),
                )
                .await;
            });

            let models = query_copilot_models(&client)
                .await
                .expect("SDK-backed model query succeeds");
            let probe = CopilotProbe {
                models,
                auth: query_copilot_auth(&client).await,
            };
            server.await.expect("fake Copilot server completes");
            client.force_stop();

            assert_eq!(probe.models.len(), 1);
            assert_eq!(probe.models[0].value, "gpt-5-mini");
            let auth = probe.auth.expect("auth status mapped");
            assert_eq!(auth.login.as_deref(), Some("octocat"));
            assert_eq!(auth.auth_type.as_deref(), Some("oauth"));
        });
    }

    async fn read_frame<R>(reader: &mut R) -> serde_json::Value
    where
        R: tokio::io::AsyncBufRead + Unpin,
    {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            let read = reader.read_line(&mut line).await.expect("frame header");
            assert_ne!(read, 0, "unexpected EOF before frame body");
            if line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(value) = line.trim().strip_prefix("Content-Length:").map(str::trim) {
                content_length = Some(value.parse::<usize>().expect("content length"));
            }
        }
        let mut body = vec![0; content_length.expect("Content-Length header")];
        reader.read_exact(&mut body).await.expect("frame body");
        serde_json::from_slice(&body).expect("JSON-RPC body")
    }

    async fn write_frame<W>(writer: &mut W, value: serde_json::Value)
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        let body = serde_json::to_vec(&value).expect("serialize JSON-RPC response");
        writer
            .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
            .await
            .expect("frame header write");
        writer.write_all(&body).await.expect("frame body write");
        writer.flush().await.expect("frame flush");
    }
}
