//! Subprocess CLI bridge — spawns an external CLI binary
//! (Claude Code / Gemini / Copilot) and bridges its stdio into the
//! shell-core `ChatProvider` shape.
//!
//! Wire protocol (default, generic — actual per-CLI templates land in
//! follow-up commits once each CLI's headless flag set is wired into
//! the settings modal):
//!
//! 1. Spawn `<binary> <args...>` with stdin / stdout piped.
//! 2. Write `request.user_message` to stdin, followed by EOF (close).
//! 3. Read stdout line by line. For each line, attempt to parse as a
//!    JSON object:
//!    - `{"type":"text","delta":"..."}` → `ChatDelta::TextDelta`
//!    - `{"type":"thinking","delta":"..."}` → `ChatDelta::Thinking`
//!    - `{"type":"tool_use","name":"...","args":...}` → `ChatDelta::ToolUse`
//!    - `{"type":"done","stop_reason":"..."}` → `ChatDelta::Done`
//!    - `{"type":"error","message":"..."}` → `ChatDelta::Error`
//!    Any other shape, or non-JSON line → treated as a plain
//!    `TextDelta` carrying the raw line + `"\n"`. This keeps CLIs that
//!    don't speak the structured format from producing a silent chat
//!    box (they still echo the model's response).
//! 4. On stdout EOF the bridge emits `Done { EndTurn }` if the CLI
//!    didn't already, then closes the channel.
//!
//! The `tokio::process::Child` is held by the spawned task so it lives
//! for the duration of stdout pumping; killed by drop on early channel
//! close (user navigated away mid-stream).

use std::sync::Arc;

use openpencil_shell_core::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, CliName, StopReason,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::chat_runtime::{shared_runtime, BlockingRecvIter};

/// Build a `tokio::process::Command` that spawns `binary` with `args`.
/// Handles the three desktop platforms identically wherever possible,
/// and papers over the well-known cross-platform binary-lookup gaps:
///
/// - **macOS / Linux**: a bare command name resolves via the usual
///   PATH execvp lookup. We forward straight to `Command::new`.
/// - **Windows**: Win32 `CreateProcessW` does **not** honor PATHEXT,
///   so a bare `claude` only spawns when an exact `claude` (no
///   extension) is on PATH. npm / bun / Volta / scoop / winget all
///   ship Node-based CLIs as `claude.cmd` / `claude.bat` / `claude.ps1`
///   shims. To make those work we route through `cmd /c <binary>`
///   when the binary doesn't already look like a fully-resolved path
///   ending in `.exe`. The binary names we ship from `for_cli` are
///   hardcoded constants (no user-controlled metacharacters) so this
///   is safe from shell injection. Users passing a custom binary via
///   `with_binary` are responsible for not embedding shell payload.
///
/// On every platform stdin / stdout / stderr are piped, and the child
/// is detached from any controlling terminal (`process_group(0)` on
/// Unix so Ctrl-C in the OP terminal doesn't kill the CLI; on Windows
/// `creation_flags(CREATE_NO_WINDOW)` so spawning the CLI doesn't
/// pop a console window for users running the GUI build).
fn build_command(binary: &str, args: &[String]) -> Command {
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW from winbase.h — keeps the console hidden
        // when OpenPencil launches from a non-console parent (e.g.,
        // double-click on the .exe from Explorer).
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let bare = std::path::Path::new(binary);
        let has_path_sep = bare.parent().map(|p| !p.as_os_str().is_empty()).unwrap_or(false);
        let looks_like_exe = bare
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
        if has_path_sep || looks_like_exe {
            let mut cmd = Command::new(binary);
            cmd.args(args);
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd
        } else {
            // Route through `cmd /c` so PATHEXT (.cmd / .bat / .ps1)
            // expansion kicks in. /c runs the command and exits.
            let mut cmd = Command::new("cmd");
            cmd.arg("/c").arg(binary).args(args);
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd
        }
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new(binary);
        cmd.args(args);
        // process_group(0) puts the child in its own group so signals
        // sent to OP's pgroup (e.g., Ctrl-C in the terminal that
        // launched the GUI) don't propagate to the CLI mid-stream.
        // The chat bridge has its own kill-on-receiver-drop path, so
        // we never depend on signal propagation for cleanup.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        cmd
    }
}

/// Stringify an `ExitStatus` for chat error reporting. Cross-platform:
/// on Unix `.code()` is `None` when killed by signal — show the signal
/// number instead; on Windows `.code()` is always populated.
fn exit_status_label(status: &std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        return code.to_string();
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return format!("signal {sig}");
        }
    }
    "?".into()
}

/// `ChatProvider` impl that bridges to a CLI binary via stdio.
/// Construct via [`SubprocessProvider::for_cli`] or
/// [`SubprocessProvider::with_binary`] for a custom binary path.
pub struct SubprocessProvider {
    binary: String,
    args: Vec<String>,
    label: String,
}

impl SubprocessProvider {
    /// Build a subprocess provider for a known [`CliName`]. Uses
    /// `cli.default_binary()` as the executable name + the canonical
    /// `--print` / `--stream-json` flags (best-effort defaults — each
    /// CLI's real flag set lands as user-tunable in the settings
    /// modal). The returned provider's label is `cli.label()`.
    ///
    /// Returns `None` when `cli` is in the HttpServer category
    /// (Codex / OpenCode) — those go through the dedicated
    /// HttpServerProvider, not this subprocess bridge. Callers who
    /// truly want a generic stdio pipe to a `codex` / `opencode`
    /// binary can use [`SubprocessProvider::with_binary`] instead
    /// (codex CONCERN 5: don't silently accept the wrong backend).
    pub fn for_cli(cli: CliName) -> Option<Self> {
        let args: Vec<String> = match cli {
            CliName::ClaudeCode => vec![
                "--print".into(),
                "--output-format".into(),
                "stream-json".into(),
            ],
            CliName::Gemini => vec!["--quiet".into()],
            CliName::Copilot => vec!["suggest".into()],
            CliName::Codex | CliName::OpenCode => return None,
        };
        Some(Self {
            binary: cli.default_binary().into(),
            args,
            label: cli.label().into(),
        })
    }

    /// Build a subprocess provider with a user-supplied binary path
    /// and argv. Used when the settings modal needs to point at a
    /// non-PATH install (e.g., a beta build at `~/bin/claude-beta`).
    #[allow(dead_code)]
    pub fn with_binary(
        binary: impl Into<String>,
        args: Vec<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            binary: binary.into(),
            args,
            label: label.into(),
        }
    }
}

impl ChatProvider for SubprocessProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn send(
        &self,
        request: ChatRequest,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let binary = self.binary.clone();
        let args = Arc::new(self.args.clone());
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            let mut cmd = build_command(&binary, &args);
            cmd.stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx
                        .send(ChatDelta::Error(format!("spawn {binary}: {e}")))
                        .await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    return;
                }
            };

            // Drain stderr to /dev/null on a sibling task so the CLI
            // never deadlocks on a full stderr pipe (codex BLOCK 1).
            // Future work could route it into a status-bar notice
            // channel; today we just keep the pipe drained.
            if let Some(stderr) = child.stderr.take() {
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stderr).lines();
                    while let Ok(Some(_)) = lines.next_line().await {}
                });
            }

            if let Some(mut stdin) = child.stdin.take() {
                // Feed the user message + close stdin so the CLI
                // sees EOF and starts responding instead of waiting
                // for more input. Stdin write errors surface as a
                // chat error so the user sees the broken-pipe instead
                // of silent normal completion (codex CONCERN 3).
                if let Err(e) = stdin.write_all(request.user_message.as_bytes()).await {
                    let _ = tx
                        .send(ChatDelta::Error(format!("stdin write: {e}")))
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
                let _ = stdin.shutdown().await; // EOF; ignore close error
            }

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    let _ = tx
                        .send(ChatDelta::Error("no stdout from CLI".into()))
                        .await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                    return;
                }
            };

            let mut lines = BufReader::new(stdout).lines();
            let mut emitted_done = false;
            let mut terminal_error = false;
            loop {
                // Race the line read against channel closure so the
                // bridge notices an idle receiver-drop without
                // waiting for the next CLI output (codex BLOCK 2).
                // `tokio::sync::mpsc::Sender::closed()` returns a
                // future that resolves when every receiver has been
                // dropped — no polling required.
                tokio::select! {
                    biased;
                    _ = tx.closed() => {
                        let _ = child.start_kill();
                        break;
                    }
                    result = lines.next_line() => match result {
                        Ok(Some(line)) => {
                            let delta = parse_line(&line);
                            let is_done = matches!(delta, ChatDelta::Done { .. });
                            if tx.send(delta).await.is_err() {
                                let _ = child.start_kill();
                                break;
                            }
                            if is_done {
                                // CLI signaled turn end — stop reading
                                // even if stdout stays open (codex
                                // BLOCK 3).
                                emitted_done = true;
                                break;
                            }
                        }
                        Ok(None) => break, // stdout EOF
                        Err(e) => {
                            let _ = tx.send(ChatDelta::Error(e.to_string())).await;
                            // Terminal: don't paper over an I/O error
                            // with `EndTurn` (codex BLOCK 5).
                            let _ = tx
                                .send(ChatDelta::Done {
                                    stop_reason: StopReason::Aborted,
                                })
                                .await;
                            terminal_error = true;
                            break;
                        }
                    },
                }
            }

            // Reap the child + interpret exit status (codex BLOCK 4):
            // non-zero exit with no prior Done surfaces as Error +
            // Aborted instead of an unrelated `EndTurn`.
            let status = child.wait().await.ok();
            if !emitted_done && !terminal_error {
                let nonzero = status.as_ref().map(|s| !s.success()).unwrap_or(false);
                if nonzero {
                    let label = status
                        .as_ref()
                        .map(exit_status_label)
                        .unwrap_or_else(|| "?".into());
                    let _ = tx
                        .send(ChatDelta::Error(format!("CLI exited with status {label}")))
                        .await;
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                } else {
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::EndTurn,
                        })
                        .await;
                }
            }
        });
        Box::new(BlockingRecvIter::new(rx))
    }
}

/// Parse a single CLI stdout line into a `ChatDelta`. Recognized
/// shapes documented in module header; everything else degrades to a
/// raw text delta carrying the line + a trailing newline.
fn parse_line(line: &str) -> ChatDelta {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('{') {
        // Not JSON — surface as raw text so CLIs that just stream
        // plain stdout (e.g., `gh copilot suggest`) still produce a
        // visible response in the chat panel.
        let mut s = line.to_string();
        s.push('\n');
        return ChatDelta::TextDelta(s);
    }
    let val: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => {
            let mut s = line.to_string();
            s.push('\n');
            return ChatDelta::TextDelta(s);
        }
    };
    let ty = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
    // Strict shape per type — missing or wrong-type required fields
    // become `Error` deltas instead of silent empty deltas (codex
    // BLOCK 6). Better to surface a parse problem than to feed empty
    // strings into the chat panel.
    match ty {
        "text" => match val.get("delta").and_then(|v| v.as_str()) {
            Some(s) => ChatDelta::TextDelta(s.to_string()),
            None => ChatDelta::Error(format!("malformed text event: {trimmed}")),
        },
        "thinking" => match val.get("delta").and_then(|v| v.as_str()) {
            Some(s) => ChatDelta::Thinking(s.to_string()),
            None => ChatDelta::Error(format!("malformed thinking event: {trimmed}")),
        },
        "tool_use" => match (
            val.get("name").and_then(|v| v.as_str()),
            val.get("args"),
        ) {
            (Some(name), Some(args)) => ChatDelta::ToolUse {
                name: name.to_string(),
                args: args.to_string(),
            },
            _ => ChatDelta::Error(format!("malformed tool_use event: {trimmed}")),
        },
        "done" => {
            let reason = val.get("stop_reason").and_then(|v| v.as_str());
            ChatDelta::Done {
                stop_reason: map_stop_reason(reason),
            }
        }
        "error" => match val.get("message").and_then(|v| v.as_str()) {
            Some(msg) => ChatDelta::Error(msg.to_string()),
            None => ChatDelta::Error(format!("malformed error event: {trimmed}")),
        },
        _ => {
            // Unknown structured event — surface the raw line so the
            // user can debug what their CLI is emitting.
            let mut s = line.to_string();
            s.push('\n');
            ChatDelta::TextDelta(s)
        }
    }
}

fn map_stop_reason(s: Option<&str>) -> StopReason {
    match s.unwrap_or("") {
        "max_tokens" => StopReason::MaxTokens,
        "tool_use" => StopReason::ToolUse,
        "aborted" | "user_abort" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line_text_delta() {
        match parse_line(r#"{"type":"text","delta":"Hello"}"#) {
            ChatDelta::TextDelta(s) => assert_eq!(s, "Hello"),
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_thinking_delta() {
        match parse_line(r#"{"type":"thinking","delta":"reasoning..."}"#) {
            ChatDelta::Thinking(s) => assert_eq!(s, "reasoning..."),
            other => panic!("expected Thinking, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_tool_use() {
        match parse_line(r#"{"type":"tool_use","name":"bash","args":{"cmd":"ls"}}"#) {
            ChatDelta::ToolUse { name, args } => {
                assert_eq!(name, "bash");
                assert!(args.contains("ls"));
            }
            other => panic!("expected ToolUse, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_done_with_stop_reason() {
        match parse_line(r#"{"type":"done","stop_reason":"max_tokens"}"#) {
            ChatDelta::Done { stop_reason } => {
                assert!(matches!(stop_reason, StopReason::MaxTokens));
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_error() {
        match parse_line(r#"{"type":"error","message":"rate limited"}"#) {
            ChatDelta::Error(s) => assert_eq!(s, "rate limited"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_plain_text_falls_through_to_text_delta() {
        match parse_line("Just some plain text") {
            ChatDelta::TextDelta(s) => assert_eq!(s, "Just some plain text\n"),
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_malformed_json_falls_through() {
        // Looks like JSON ({ ... }) but isn't valid → raw text.
        match parse_line("{not json") {
            ChatDelta::TextDelta(s) => assert_eq!(s, "{not json\n"),
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_unknown_type_falls_through() {
        match parse_line(r#"{"type":"frobnicate","payload":42}"#) {
            ChatDelta::TextDelta(s) => assert!(s.contains("frobnicate")),
            other => panic!("expected TextDelta fallback, got {other:?}"),
        }
    }

    #[test]
    fn for_cli_claude_code_seeds_print_stream_json_flags() {
        let p = SubprocessProvider::for_cli(CliName::ClaudeCode).unwrap();
        assert_eq!(p.binary, "claude");
        assert!(p.args.iter().any(|a| a == "--print"));
        assert!(p.args.iter().any(|a| a == "stream-json"));
        assert_eq!(p.label, "Claude Code");
    }

    #[test]
    fn for_cli_uses_default_binary_per_cli_name() {
        assert_eq!(
            SubprocessProvider::for_cli(CliName::Gemini).unwrap().binary,
            "gemini"
        );
        assert_eq!(
            SubprocessProvider::for_cli(CliName::Copilot).unwrap().binary,
            "gh-copilot"
        );
    }

    #[test]
    fn for_cli_rejects_http_server_kinds() {
        assert!(SubprocessProvider::for_cli(CliName::Codex).is_none());
        assert!(SubprocessProvider::for_cli(CliName::OpenCode).is_none());
    }

    #[test]
    fn parse_line_malformed_structured_event_is_error() {
        // "text" with no "delta" → Error
        match parse_line(r#"{"type":"text"}"#) {
            ChatDelta::Error(s) => assert!(s.contains("malformed text")),
            other => panic!("expected Error, got {other:?}"),
        }
        // "tool_use" missing "name" → Error
        match parse_line(r#"{"type":"tool_use","args":{}}"#) {
            ChatDelta::Error(s) => assert!(s.contains("malformed tool_use")),
            other => panic!("expected Error, got {other:?}"),
        }
        // "error" missing "message" → Error
        match parse_line(r#"{"type":"error"}"#) {
            ChatDelta::Error(s) => assert!(s.contains("malformed error")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn spawn_failure_surfaces_error_and_done() {
        // Use a binary path that's guaranteed not to exist on PATH.
        // Cross-platform expectation:
        //   - macOS / Linux: `Command::new` returns spawn-time Err
        //     because execvp fails → `ChatDelta::Error("spawn ...")`
        //     emitted before EOF.
        //   - Windows: `build_command` routes through `cmd /c`, which
        //     successfully spawns. The inner CLI then exits non-zero
        //     ("not recognized as an internal or external command")
        //     → `ChatDelta::Error("CLI exited with status N")` from
        //     the exit-status path.
        // Either way the bridge must emit at least one Error delta
        // plus a terminal Done.
        let p = SubprocessProvider::with_binary(
            "definitely-not-a-binary-3kf9j2-xyz",
            Vec::new(),
            "bogus",
        );
        let deltas: Vec<ChatDelta> = p
            .send(ChatRequest {
                system_prompt: String::new(),
                user_message: "hi".into(),
                max_output_tokens: 64,
            })
            .collect();
        assert!(
            deltas.iter().any(|d| matches!(d, ChatDelta::Error(_))),
            "expected at least one Error delta, got {:?}",
            deltas
        );
        assert!(
            deltas.iter().any(|d| matches!(d, ChatDelta::Done { .. })),
            "expected terminal Done in deltas, got {:?}",
            deltas
        );
    }
}
