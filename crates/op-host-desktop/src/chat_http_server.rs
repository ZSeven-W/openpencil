//! HTTP-server CLI bridge — structural lifecycle for CLIs that
//! run as a local HTTP server we POST JSON requests to. Per the
//! user's architecture note ("opencode 和 codex 我们调用 http
//! server, 通过 ipc 启动本地的 server 模式") the model is: spawn
//! the binary in server mode, watch stdout for a "Listening on …"
//! line that announces the bind port, then send chat requests to
//! that local HTTP endpoint.
//!
//! **Honest scope note (2026-05-14):** the wire protocol — URL
//! path, request body JSON shape, response stream shape — is NOT
//! verified against any real `codex serve` / `opencode serve`
//! implementation:
//!
//! - As of writing, OpenAI's `codex` CLI does **not** ship a
//!   `serve` subcommand. Calling `HttpServerProvider::with_binary
//!   ("codex", ...)` will fail at spawn time.
//! - sst/opencode does ship an HTTP server (`opencode serve`), but
//!   its real request / response shapes haven't been wired up here
//!   yet. The defaults below (`/v1/chat`, `{ "message": ... }`,
//!   newline-delimited generic envelope) are placeholders.
//!
//! For that reason `HttpServerProvider::for_cli(CliName)` always
//! returns `None` today — callers can't accidentally route Codex /
//! OpenCode through an unverified protocol. Use
//! [`HttpServerProvider::with_binary`] with explicit binary + serve
//! args + chat_path once you've validated a server's real protocol.
//!
//! Lifecycle pieces that ARE structurally correct:
//! - Spawn + stderr drain + stdout listen-line discovery
//!   ([`parse_listening_line`] handles three observed formats).
//! - Receiver-drop kills the child so iterator teardown is clean.
//! - reqwest streaming response body parsing — split on `\n`, feed
//!   each non-empty line through `chat_subprocess::parse_line`
//!   (`text` / `thinking` / `tool_use` / `done` / `error` envelope).
//!
//! When a real server lands, swap the body builder + response
//! parser to match its shape; the lifecycle plumbing stays.

use std::sync::Arc;
use std::time::Duration;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, CliName, StopReason};
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::chat_runtime::{shared_runtime, BlockingRecvIter};

/// `ChatProvider` impl backed by a locally-spawned HTTP server CLI.
/// Construct via [`HttpServerProvider::for_cli`] (Codex / OpenCode)
/// or [`HttpServerProvider::with_binary`] (custom binary path).
pub struct HttpServerProvider {
    binary: String,
    serve_args: Vec<String>,
    chat_path: String,
    /// How long to wait for the spawned server to announce its
    /// listening port before declaring spawn-failure. Defaults to
    /// 10 seconds — codex/opencode usually announce within 1s, but
    /// cold first-runs that need to compile native deps can take
    /// longer.
    listen_timeout: Duration,
    label: String,
}

impl HttpServerProvider {
    /// Always returns `None` today. Both Codex and OpenCode are
    /// architecturally HttpServer-kind CLIs, but their real wire
    /// protocols haven't been wired up — see the module-level
    /// "honest scope note". Constructing through this function
    /// would pretend Codex / OpenCode just work, which they don't.
    ///
    /// Callers who've verified a server's real protocol should use
    /// [`HttpServerProvider::with_binary`] with explicit serve args
    /// + chat_path + body shape.
    #[allow(dead_code)]
    pub fn for_cli(_cli: CliName) -> Option<Self> {
        None
    }

    /// Build a provider with a user-supplied binary + serve args +
    /// chat-endpoint path. Lets the settings modal point at a
    /// non-PATH binary (e.g., `~/bin/codex-dev`).
    #[allow(dead_code)]
    pub fn with_binary(
        binary: impl Into<String>,
        serve_args: Vec<String>,
        chat_path: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            binary: binary.into(),
            serve_args,
            chat_path: chat_path.into(),
            listen_timeout: Duration::from_secs(10),
            label: label.into(),
        }
    }
}

#[derive(Serialize)]
struct ChatBody<'a> {
    message: &'a str,
}

impl ChatProvider for HttpServerProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let binary = self.binary.clone();
        let serve_args = Arc::new(self.serve_args.clone());
        let chat_path = self.chat_path.clone();
        let timeout_dur = self.listen_timeout;
        let prompt = request.user_message;
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            // Spawn `<binary> <serve_args...>`. stdin closed, stdout
            // + stderr piped so we can parse the listening line.
            let mut cmd = Command::new(&binary);
            cmd.args(serve_args.iter())
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx
                        .send(ChatDelta::Error(format!("spawn {binary} serve: {e}")))
                        .await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    return;
                }
            };

            // Drain stderr in a sibling task so the server never
            // deadlocks on a full pipe.
            if let Some(stderr) = child.stderr.take() {
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stderr).lines();
                    while let Ok(Some(_)) = lines.next_line().await {}
                });
            }

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    let _ = tx.send(ChatDelta::Error("server: no stdout".into())).await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    return;
                }
            };

            // Read stdout lines until we find one announcing the
            // bind address. Codex + OpenCode use slightly different
            // wording so we try a few patterns.
            let (port, mut stdout_lines) = match timeout(timeout_dur, discover_port(stdout)).await {
                Ok(Ok(pair)) => pair,
                Ok(Err(e)) => {
                    let _ = tx
                        .send(ChatDelta::Error(format!("server: port discovery: {e}")))
                        .await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    return;
                }
                Err(_) => {
                    let _ = tx
                        .send(ChatDelta::Error(format!(
                            "server: didn't announce listen port within {}s",
                            timeout_dur.as_secs()
                        )))
                        .await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    return;
                }
            };
            // Continue draining stdout for the rest of the server's
            // lifetime — keeps the pipe drained even if the chat
            // response comes via HTTP rather than stdout.
            tokio::spawn(async move { while let Ok(Some(_)) = stdout_lines.next_line().await {} });

            // POST the chat request and stream the response.
            let base = format!("http://127.0.0.1:{port}");
            let url = format!("{base}{chat_path}");
            let body = ChatBody { message: &prompt };
            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ChatDelta::Error(format!("http client: {e}"))).await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    return;
                }
            };
            let resp = match client.post(&url).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx
                        .send(ChatDelta::Error(format!("http POST {url}: {e}")))
                        .await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    return;
                }
            };
            if !resp.status().is_success() {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                let _ = tx
                    .send(ChatDelta::Error(format!(
                        "http {status}: {}",
                        body_text.trim()
                    )))
                    .await;
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::Aborted,
                    })
                    .await;
                let _ = child.start_kill();
                let _ = child.wait().await;
                return;
            }

            // Stream the response — assume newline-delimited JSON
            // events using the same shape `chat_subprocess::parse_line`
            // recognizes. Buffer accumulates partial lines across
            // chunk boundaries.
            let mut stream = resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();
            let mut emitted_done = false;
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                if tx.is_closed() {
                    let _ = child.start_kill();
                    break;
                }
                let bytes = match chunk {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = tx.send(ChatDelta::Error(format!("http stream: {e}"))).await;
                        let _ = tx
                            .send(ChatDelta::Done {
                                stop_reason: StopReason::Aborted,
                            })
                            .await;
                        emitted_done = true;
                        break;
                    }
                };
                buf.extend_from_slice(&bytes);
                // Drain whole lines from buf as they accumulate.
                while let Some(nl_pos) = buf.iter().position(|&b| b == b'\n') {
                    let line: Vec<u8> = buf.drain(..=nl_pos).collect();
                    let line = String::from_utf8_lossy(&line);
                    let line = line.trim_end_matches('\n').trim_end_matches('\r');
                    if line.is_empty() {
                        continue;
                    }
                    let delta = crate::chat_subprocess::parse_line(line);
                    let is_done = matches!(delta, ChatDelta::Done { .. });
                    if tx.send(delta).await.is_err() {
                        let _ = child.start_kill();
                        return;
                    }
                    if is_done {
                        emitted_done = true;
                        break;
                    }
                }
                if emitted_done {
                    break;
                }
            }
            if !emitted_done {
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::EndTurn,
                    })
                    .await;
            }
            // Server stays alive across sends (caller can re-use
            // by holding the same provider). Today we kill it
            // because the provider lives only for the duration of
            // this `send` call. Future: lift the spawn into a
            // long-lived ClientSession analog so multi-turn doesn't
            // pay the cold-start cost.
            let _ = child.start_kill();
            let _ = child.wait().await;
        });
        Box::new(BlockingRecvIter::new(rx))
    }
}

/// Read lines from `stdout` until one matches the listening
/// announcement pattern; return the parsed port plus the remaining
/// stream so the caller can keep draining (the server may write more
/// to stdout during its lifetime). Returns Err when stdout ends
/// before the listening line appears.
async fn discover_port(
    stdout: tokio::process::ChildStdout,
) -> std::io::Result<(
    u16,
    tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
)> {
    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines.next_line().await? {
        if let Some(port) = parse_listening_line(&line) {
            return Ok((port, lines));
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::UnexpectedEof,
        "server stdout ended without listening line",
    ))
}

/// Parse common variants of the "Listening on …" line. Codex emits
/// `Listening on http://127.0.0.1:8765`; opencode tends toward
/// `Server listening on port 7777`; raw frameworks sometimes just
/// write `listening 7777`. Pick the first 4-5-digit number we see
/// after any of those tokens.
pub(crate) fn parse_listening_line(line: &str) -> Option<u16> {
    let lower = line.to_ascii_lowercase();
    let mut idx = 0;
    while idx < lower.len() {
        if let Some(pos) = lower[idx..].find("listening") {
            idx += pos + "listening".len();
            // Scan for the first digit run after this token.
            let rest = &line[idx..];
            return extract_port(rest);
        } else {
            break;
        }
    }
    None
}

/// Pull the actual port (not an IP octet) out of `rest`. Strategy:
/// prefer the digit run immediately after `:` (matches the
/// `http://host:PORT` form). If no `:` is found, fall back to the
/// last digit run in the string (matches `... on port NNNN` /
/// `listening NNNN`). Either branch parses ≤ 5 digits as u16.
fn extract_port(rest: &str) -> Option<u16> {
    if let Some(colon) = rest.rfind(':') {
        let tail = &rest[colon + 1..];
        let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(p) = digits.parse::<u16>() {
            return Some(p);
        }
    }
    // No `:` (or `:` was followed by non-digits) — pick the last
    // digit run, which on the "listening on port NNNN" pattern is
    // the port.
    let mut last_run = String::new();
    let mut current = String::new();
    for c in rest.chars() {
        if c.is_ascii_digit() {
            current.push(c);
        } else if !current.is_empty() {
            last_run = std::mem::take(&mut current);
        }
    }
    if !current.is_empty() {
        last_run = current;
    }
    last_run.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_cli_returns_none_pending_real_protocol_wiring() {
        // Both Codex + OpenCode are HttpServer-kind in the
        // architecture memo, but the wire protocols (URL path,
        // body shape, response stream format) are not yet
        // verified against either real CLI — see module docs.
        // Until that lands, for_cli refuses to construct a
        // bridge so callers don't accidentally pretend it works.
        for cli in &[
            CliName::Codex,
            CliName::OpenCode,
            CliName::ClaudeCode,
            CliName::Gemini,
            CliName::Copilot,
        ] {
            assert!(
                HttpServerProvider::for_cli(*cli).is_none(),
                "for_cli should refuse {:?} until protocol is wired",
                cli
            );
        }
    }

    #[test]
    fn parse_listening_line_codex_format() {
        assert_eq!(
            parse_listening_line("Listening on http://127.0.0.1:8765"),
            Some(8765)
        );
    }

    #[test]
    fn parse_listening_line_opencode_format() {
        assert_eq!(
            parse_listening_line("Server listening on port 7777"),
            Some(7777)
        );
    }

    #[test]
    fn parse_listening_line_bare_number() {
        assert_eq!(parse_listening_line("listening 5000"), Some(5000));
    }

    #[test]
    fn parse_listening_line_returns_none_when_absent() {
        assert!(parse_listening_line("Starting codex...").is_none());
        assert!(parse_listening_line("Loading config from ~/.codex").is_none());
    }

    #[test]
    fn with_binary_constructs_as_chat_provider_trait_object() {
        // The supported construction path is `with_binary` — caller
        // supplies the verified binary + serve args + chat path.
        let p: Arc<dyn ChatProvider> = Arc::new(HttpServerProvider::with_binary(
            "echo",
            vec!["serve".into()],
            "/chat",
            "Echo",
        ));
        assert_eq!(p.provider_label(), "Echo");
    }
}
