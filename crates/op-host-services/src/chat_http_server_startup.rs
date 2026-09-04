//! Cancellation-safe OpenCode server startup and loopback-port selection.

use std::io;
use std::net::{Ipv4Addr, TcpListener};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use super::probe::probe_server_while_open;
use super::{parse_server_url, OpenCodeError};
use crate::chat_spawn::build_command;

const MAX_DIAGNOSTIC_BYTES: usize = 16 * 1024;
const STDERR_SETTLE: Duration = Duration::from_millis(100);
pub(super) const SERVER_BIND_ATTEMPTS: usize = 3;

/// TS `server.ts` listen timeout: 5s, 15s on Windows
/// (`opencode-client.ts:92`).
fn listen_timeout() -> Duration {
    if cfg!(windows) {
        Duration::from_secs(15)
    } else {
        Duration::from_secs(5)
    }
}

/// Reserve an unused IPv4 loopback port, then release the reservation
/// immediately before spawning OpenCode. The bind-to-zero pattern asks the
/// OS for a real ephemeral port; passing zero to OpenCode itself does not.
pub(super) fn reserve_loopback_port() -> io::Result<u16> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    Ok(listener.local_addr()?.port())
}

pub(super) fn opencode_server_args(port: u16) -> Vec<String> {
    vec![
        "serve".into(),
        "--hostname=127.0.0.1".into(),
        format!("--port={port}"),
    ]
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ServerResolution {
    Ready(String),
    Cancelled,
    IdentityFailed,
}

/// Resolve the default server or start an integration-owned one. Both health
/// probes and the listen handshake observe receiver closure. Because the
/// spawned child is written through the caller-owned slot before either await,
/// cancellation after spawn cannot detach it.
pub(super) async fn resolve_opencode_server<T>(
    tx: &mpsc::Sender<T>,
    client: &reqwest::Client,
    binary: &str,
    default_url: &str,
    spawned: &mut Option<tokio::process::Child>,
) -> Result<ServerResolution, OpenCodeError> {
    resolve_opencode_server_with(tx, client, binary, default_url, spawned, build_command).await
}

async fn resolve_opencode_server_with<T>(
    tx: &mpsc::Sender<T>,
    client: &reqwest::Client,
    binary: &str,
    default_url: &str,
    spawned: &mut Option<tokio::process::Child>,
    command_builder: fn(&str, &[String]) -> tokio::process::Command,
) -> Result<ServerResolution, OpenCodeError> {
    let Some(default_healthy) = probe_server_while_open(tx, client, default_url).await else {
        return Ok(ServerResolution::Cancelled);
    };
    if default_healthy {
        return Ok(ServerResolution::Ready(default_url.to_string()));
    }

    let Some(url) = spawn_opencode_server(tx, binary, spawned, command_builder).await? else {
        return Ok(ServerResolution::Cancelled);
    };
    let Some(spawned_healthy) = probe_server_while_open(tx, client, &url).await else {
        return Ok(ServerResolution::Cancelled);
    };
    if spawned_healthy {
        Ok(ServerResolution::Ready(url))
    } else {
        Ok(ServerResolution::IdentityFailed)
    }
}

/// Spawn an OpenCode server on an explicitly reserved port. A small retry
/// budget handles the unavoidable close-reservation/spawn TOCTOU race when a
/// different process claims the port first.
///
/// The child is installed in `spawned` immediately after `spawn()`, before
/// waiting for stdout. On receiver cancellation this returns `Ok(None)` while
/// deliberately leaving that handle in place for the caller's final
/// tree-aware cleanup.
async fn spawn_opencode_server<T>(
    tx: &mpsc::Sender<T>,
    binary: &str,
    spawned: &mut Option<tokio::process::Child>,
    command_builder: fn(&str, &[String]) -> tokio::process::Command,
) -> Result<Option<String>, OpenCodeError> {
    for attempt in 0..SERVER_BIND_ATTEMPTS {
        if tx.is_closed() {
            return Ok(None);
        }
        let port = reserve_loopback_port().map_err(|error| OpenCodeError::Spawn {
            binary: binary.to_string(),
            message: format!("reserve loopback port: {error}"),
        })?;
        match spawn_opencode_server_once(tx, binary, port, spawned, command_builder).await {
            AttemptOutcome::Ready(url) => return Ok(Some(url)),
            AttemptOutcome::Cancelled => return Ok(None),
            AttemptOutcome::Failed {
                error: _,
                address_in_use: true,
            } if attempt + 1 < SERVER_BIND_ATTEMPTS => {
                discard_failed_attempt(spawned).await;
            }
            AttemptOutcome::Failed { error, .. } => return Err(error),
        }
    }
    unreachable!("the bounded server-start loop always returns")
}

enum AttemptOutcome {
    Ready(String),
    Cancelled,
    Failed {
        error: OpenCodeError,
        address_in_use: bool,
    },
}

enum ListenScan {
    Ready(String),
    UnexpectedUrl(String),
    Ended,
}

async fn spawn_opencode_server_once<T>(
    tx: &mpsc::Sender<T>,
    binary: &str,
    port: u16,
    spawned: &mut Option<tokio::process::Child>,
    command_builder: fn(&str, &[String]) -> tokio::process::Command,
) -> AttemptOutcome {
    let args = opencode_server_args(port);
    let mut cmd = command_builder(binary, &args);
    // Last-resort guard when task/runtime abort bypasses tree-aware shutdown.
    cmd.kill_on_drop(true);
    cmd.env("OPENCODE_CONFIG_CONTENT", "{}");
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(error) => {
            return AttemptOutcome::Failed {
                error: OpenCodeError::Spawn {
                    binary: binary.to_string(),
                    message: error.to_string(),
                },
                address_in_use: false,
            };
        }
    };
    *spawned = Some(child);

    let child = spawned.as_mut().expect("child was just installed");
    let Some(stdout) = child.stdout.take() else {
        return AttemptOutcome::Failed {
            error: OpenCodeError::NoStdout,
            address_in_use: false,
        };
    };
    let stderr = child.stderr.take();
    let output: Arc<Mutex<String>> = Arc::default();
    let stderr_output = Arc::clone(&output);
    let stderr_task = tokio::spawn(async move {
        let Some(stderr) = stderr else { return };
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            append_diagnostic(&stderr_output, &line);
        }
    });

    let stdout_output = Arc::clone(&output);
    let scan = async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(url) = parse_server_url(&line) {
                if announced_port(&url) != Some(port) {
                    return ListenScan::UnexpectedUrl(url);
                }
                // Keep draining stdout for the server's lifetime so a chatty
                // server cannot block on a full pipe after readiness.
                tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });
                return ListenScan::Ready(url);
            }
            append_diagnostic(&stdout_output, &line);
        }
        ListenScan::Ended
    };

    let timeout = listen_timeout();
    let scan_result = tokio::select! {
        biased;
        _ = tx.closed() => return AttemptOutcome::Cancelled,
        result = tokio::time::timeout(timeout, scan) => result,
    };
    match scan_result {
        Ok(ListenScan::Ready(url)) => AttemptOutcome::Ready(url),
        Ok(ListenScan::UnexpectedUrl(announced)) => AttemptOutcome::Failed {
            error: OpenCodeError::UnexpectedListenUrl {
                expected: format!("http://127.0.0.1:{port}"),
                announced,
            },
            address_in_use: false,
        },
        Ok(ListenScan::Ended) => {
            // Give the stderr drain a short, bounded opportunity to capture
            // EADDRINUSE before deciding whether this attempt can be retried.
            let _ = tokio::time::timeout(STDERR_SETTLE, stderr_task).await;
            let status = spawned
                .as_mut()
                .and_then(|child| child.try_wait().ok().flatten());
            let code = status
                .and_then(|status| status.code())
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".into());
            let output = output.lock().map(|text| text.clone()).unwrap_or_default();
            let address_in_use = reports_address_in_use(&output);
            AttemptOutcome::Failed {
                error: OpenCodeError::ServerExited { code, output },
                address_in_use,
            }
        }
        Err(_) => AttemptOutcome::Failed {
            error: OpenCodeError::ListenTimeout {
                millis: timeout.as_millis(),
            },
            address_in_use: false,
        },
    }
}

async fn discard_failed_attempt(spawned: &mut Option<tokio::process::Child>) {
    let Some(mut child) = spawned.take() else {
        return;
    };
    if !matches!(child.try_wait(), Ok(Some(_))) {
        let _ = op_process_io::terminate_tokio_process_tree(&mut child, Duration::ZERO).await;
    }
}

fn append_diagnostic(output: &Mutex<String>, line: &str) {
    if let Ok(mut output) = output.lock() {
        if output.len() < MAX_DIAGNOSTIC_BYTES {
            output.push_str(line);
            output.push('\n');
        }
    }
}

fn announced_port(raw: &str) -> Option<u16> {
    reqwest::Url::parse(raw).ok()?.port_or_known_default()
}

pub(super) fn reports_address_in_use(output: &str) -> bool {
    let lowercase = output.to_ascii_lowercase();
    lowercase.contains("eaddrinuse")
        || lowercase.contains("address already in use")
        || lowercase.contains("address in use")
}

#[cfg(test)]
#[path = "chat_http_server_startup_tests.rs"]
mod tests;
