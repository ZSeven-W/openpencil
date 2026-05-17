//! Subprocess CLI bridge — spawns an external CLI binary
//! (Claude Code / Gemini / Copilot) and bridges its stdio into the
//! shell-core `ChatProvider` shape.
//!
//! Per-CLI wire protocol (single-shot mode — multi-turn via
//! `--resume <session>` lands in a follow-up):
//!
//! - **Claude Code (`claude`)** — invoked as
//!   `claude --print --verbose --output-format stream-json -- <prompt>`.
//!   Prompt is a positional argv after `--`. Stdin closes immediately.
//!   Stdout is line-delimited JSON; we recognize Claude's
//!   `system` / `assistant` / `result` shapes alongside the generic
//!   `text` / `thinking` / `tool_use` / `done` / `error` envelope.
//!   Flag set follows bartolli/anthropic-agent-sdk + Claude Code's
//!   own documented headless mode (CLAUDE_CODE_ENTRYPOINT env var,
//!   `--verbose` for full stream-json detail).
//! - **Gemini CLI (`gemini`)** — `gemini --quiet`; prompt via stdin
//!   (gemini reads piped stdin as the message).
//! - **GitHub Copilot CLI (`gh-copilot`)** — `gh-copilot suggest`;
//!   prompt via stdin.
//!
//! Stdout is read line-by-line; recognized structured shapes mapped
//! to `ChatDelta`; unrecognized lines surface as raw `TextDelta` so
//! plain-stdout CLIs still produce visible output. On stdout EOF the
//! bridge reaps the child + interprets exit status — non-zero exit
//! surfaces as `Error + Done { Aborted }` (codex BLOCK 4 / 5). On
//! receiver drop the bridge `child.start_kill()` so the user can
//! navigate away without leaking processes.
//!
//! Binary lookup falls back through PATH then per-platform default
//! install paths (npm / yarn / bun globals) — see `find_binary`.

use std::path::PathBuf;
use std::sync::Arc;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, CliName, StopReason};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::chat_runtime::{shared_runtime, BlockingRecvIter};

/// How the user's prompt reaches the CLI. Claude Code's `--print`
/// mode requires the prompt as a positional argv after `--` and
/// closes stdin immediately. Gemini / Copilot read the message off
/// piped stdin. Generic `with_binary` callers can pick either via
/// [`SubprocessProvider::with_binary_mode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    /// Append `-- <prompt>` to argv; stdin gets closed (no input).
    PositionalArg,
    /// Argv is passed verbatim; user_message is written to stdin
    /// followed by EOF.
    Stdin,
}

/// Search for `name` on PATH, then in well-known per-platform install
/// locations for Node-based CLIs (npm / pnpm / yarn / bun globals,
/// nvm, volta). Returns the resolved absolute path, or `name` itself
/// as a fallback so `build_command` can still attempt a bare-name
/// spawn (errors surface as a normal spawn-failure `ChatDelta::Error`).
///
/// Cross-platform: each branch only probes paths that exist on that
/// OS so we don't pay for filesystem-stat misses on the wrong OS.
fn find_binary(name: &str) -> String {
    // PATH-relative entries first (cross-platform).
    if let Ok(path_env) = std::env::var("PATH") {
        let sep = if cfg!(windows) { ';' } else { ':' };
        for dir in path_env.split(sep).filter(|s| !s.is_empty()) {
            let candidate = std::path::Path::new(dir).join(name);
            if candidate.is_file() {
                return candidate.to_string_lossy().into();
            }
            // Windows: PATHEXT-style suffix probe so we find
            // `claude.cmd` / `claude.exe` / `claude.bat` even when the
            // user typed the bare name.
            #[cfg(windows)]
            {
                for ext in &[".exe", ".cmd", ".bat", ".ps1"] {
                    let mut with_ext = candidate.clone();
                    with_ext.set_extension(&ext[1..]);
                    if with_ext.is_file() {
                        return with_ext.to_string_lossy().into();
                    }
                }
            }
        }
    }
    // Fall back through well-known install locations. Mirrors
    // bartolli/anthropic-agent-sdk's `find_cli` for parity with the
    // reference implementation.
    let candidates = well_known_install_paths(name);
    for path in candidates {
        if path.is_file() {
            return path.to_string_lossy().into();
        }
    }
    name.into()
}

fn well_known_install_paths(name: &str) -> Vec<PathBuf> {
    let home = dirs::home_dir();
    let mut out: Vec<PathBuf> = Vec::new();
    #[cfg(unix)]
    {
        if let Some(h) = home.clone() {
            out.push(h.join(".npm-global/bin").join(name));
            out.push(h.join(".local/bin").join(name));
            out.push(h.join(".bun/bin").join(name));
            out.push(h.join(".volta/bin").join(name));
            out.push(h.join("node_modules/.bin").join(name));
            out.push(h.join(".yarn/bin").join(name));
        }
        out.push(PathBuf::from("/usr/local/bin").join(name));
        out.push(PathBuf::from("/opt/homebrew/bin").join(name));
    }
    #[cfg(windows)]
    {
        let _ = home; // not used directly on Windows
        if let Ok(appdata) = std::env::var("APPDATA") {
            for ext in &["cmd", "exe", "bat", "ps1"] {
                out.push(
                    PathBuf::from(&appdata)
                        .join("npm")
                        .join(format!("{name}.{ext}")),
                );
            }
        }
        if let Ok(localapp) = std::env::var("LOCALAPPDATA") {
            for ext in &["cmd", "exe", "bat", "ps1"] {
                out.push(
                    PathBuf::from(&localapp)
                        .join("Programs")
                        .join(name)
                        .join(format!("{name}.{ext}")),
                );
            }
        }
    }
    out
}

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
        let has_path_sep = bare
            .parent()
            .map(|p| !p.as_os_str().is_empty())
            .unwrap_or(false);
        let looks_like_exe = bare
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
        if has_path_sep || looks_like_exe {
            let mut cmd = Command::new(binary);
            cmd.args(args);
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd
        } else {
            // Route through `cmd /c` so PATHEXT (.cmd / .bat / .ps1)
            // expansion kicks in. /c runs the command and exits.
            let mut cmd = Command::new("cmd");
            cmd.arg("/c").arg(binary).args(args);
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
    prompt_mode: PromptMode,
}

impl SubprocessProvider {
    /// Build a subprocess provider for a known [`CliName`]. Each CLI
    /// has its own argv template + prompt-routing mode:
    ///
    /// - **Claude Code**: `--print --verbose --output-format
    ///   stream-json`; prompt as positional argv after `--`. Stdin
    ///   stays closed. Matches `bartolli/anthropic-agent-sdk`
    ///   reference and Claude Code's own documented headless mode.
    /// - **Gemini**: `--quiet`; prompt via stdin.
    /// - **Copilot**: `suggest`; prompt via stdin.
    ///
    /// Returns `None` when `cli` is in the HttpServer category
    /// (Codex / OpenCode) — those go through the dedicated
    /// HttpServerProvider, not this subprocess bridge.
    pub fn for_cli(cli: CliName) -> Option<Self> {
        let (args, prompt_mode): (Vec<String>, PromptMode) = match cli {
            CliName::ClaudeCode => (
                vec![
                    "--print".into(),
                    "--verbose".into(),
                    "--output-format".into(),
                    "stream-json".into(),
                ],
                PromptMode::PositionalArg,
            ),
            CliName::Gemini => (vec!["--quiet".into()], PromptMode::Stdin),
            CliName::Copilot => (vec!["suggest".into()], PromptMode::Stdin),
            CliName::Codex | CliName::OpenCode => return None,
        };
        let binary = find_binary(cli.default_binary());
        Some(Self {
            binary,
            args,
            label: cli.label().into(),
            prompt_mode,
        })
    }

    /// Build a subprocess provider with a user-supplied binary path
    /// and argv (defaults to stdin prompt). Used when the settings
    /// modal needs to point at a non-PATH install.
    #[allow(dead_code)]
    pub fn with_binary(
        binary: impl Into<String>,
        args: Vec<String>,
        label: impl Into<String>,
    ) -> Self {
        Self::with_binary_mode(binary, args, label, PromptMode::Stdin)
    }

    /// Build a subprocess provider with an explicit prompt-routing
    /// mode. Required for CLIs like Claude Code that want the prompt
    /// as a positional argv rather than via stdin.
    #[allow(dead_code)]
    pub fn with_binary_mode(
        binary: impl Into<String>,
        args: Vec<String>,
        label: impl Into<String>,
        prompt_mode: PromptMode,
    ) -> Self {
        Self {
            binary: binary.into(),
            args,
            label: label.into(),
            prompt_mode,
        }
    }
}

/// Dangerous environment variables that should never be propagated to
/// the spawned CLI: linker preload paths can hijack execution; PATH
/// can substitute a malicious binary; runtime-library paths
/// (NODE_OPTIONS, PYTHONPATH, etc.) can inject code into Node-based
/// CLIs. Mirrors bartolli/anthropic-agent-sdk's `DANGEROUS_ENV_VARS`.
/// Today we don't accept user-supplied env vars from the settings
/// modal; the constant + scrub is here so the moment we do, the
/// dangerous-var check is already in place. Returns the env-var
/// pairs the child will receive (parent env minus the dangerous
/// names — preserving every safe var so node version managers like
/// nvm / volta still pick the right Node).
fn scrubbed_child_env() -> Vec<(String, String)> {
    const DANGEROUS: &[&str] = &[
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "DYLD_INSERT_LIBRARIES",
        "DYLD_LIBRARY_PATH",
        "NODE_OPTIONS",
        "PYTHONPATH",
        "PERL5LIB",
        "RUBYLIB",
    ];
    std::env::vars()
        .filter(|(k, _)| !DANGEROUS.iter().any(|d| k.eq_ignore_ascii_case(d)))
        .collect()
}

impl ChatProvider for SubprocessProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let binary = self.binary.clone();
        let mut args_with_prompt = self.args.clone();
        // PromptMode::PositionalArg: append `-- <prompt>` so the CLI
        // picks up the message as a CLI argument (Claude Code mode).
        // PromptMode::Stdin (default): leave argv untouched; the
        // prompt is written to stdin after spawn.
        if self.prompt_mode == PromptMode::PositionalArg {
            args_with_prompt.push("--".into());
            args_with_prompt.push(request.user_message.clone());
        }
        let args = Arc::new(args_with_prompt);
        let prompt_mode = self.prompt_mode;
        let env_pairs = scrubbed_child_env();
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            let mut cmd = build_command(&binary, &args);
            cmd.stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            // Set the child's env from the scrubbed parent env so
            // dangerous interposition vars (LD_PRELOAD / NODE_OPTIONS
            // / DYLD_INSERT_LIBRARIES / ...) never propagate. We
            // env_clear first because tokio::process Command
            // otherwise inherits the parent env verbatim.
            cmd.env_clear();
            cmd.envs(env_pairs);
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
                match prompt_mode {
                    PromptMode::Stdin => {
                        // Feed the user message + close stdin so the
                        // CLI sees EOF and starts responding. Stdin
                        // write errors surface as a chat error.
                        if let Err(e) = stdin.write_all(request.user_message.as_bytes()).await {
                            let _ = tx.send(ChatDelta::Error(format!("stdin write: {e}"))).await;
                            let _ = tx
                                .send(ChatDelta::Done {
                                    stop_reason: StopReason::Aborted,
                                })
                                .await;
                            let _ = child.start_kill();
                            let _ = child.wait().await;
                            return;
                        }
                    }
                    PromptMode::PositionalArg => {
                        // No stdin write — prompt is in argv. Close
                        // stdin immediately so the CLI doesn't sit
                        // waiting on it (Claude Code's `--print` mode
                        // exits if stdin stays open with no input).
                    }
                }
                let _ = stdin.shutdown().await; // EOF; ignore close error
            }

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    let _ = tx.send(ChatDelta::Error("no stdout from CLI".into())).await;
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
pub(crate) fn parse_line(line: &str) -> ChatDelta {
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
        "tool_use" => match (val.get("name").and_then(|v| v.as_str()), val.get("args")) {
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
        // `binary` is the resolved absolute path when claude is on
        // PATH (or one of the npm-global / nvm fallback locations);
        // otherwise it falls back to the bare name. Either way the
        // file name component must match.
        assert!(p.binary.ends_with("claude"), "binary={}", p.binary);
        assert!(p.args.iter().any(|a| a == "--print"));
        assert!(p.args.iter().any(|a| a == "--verbose"));
        assert!(p.args.iter().any(|a| a == "stream-json"));
        assert_eq!(p.label, "Claude Code");
        assert_eq!(p.prompt_mode, PromptMode::PositionalArg);
    }

    #[test]
    fn for_cli_uses_default_binary_per_cli_name() {
        // Resolved path may be bare or absolute depending on what's
        // installed on the test host; check the basename only.
        let gemini = SubprocessProvider::for_cli(CliName::Gemini).unwrap();
        assert!(
            gemini.binary.ends_with("gemini"),
            "binary={}",
            gemini.binary
        );
        assert_eq!(gemini.prompt_mode, PromptMode::Stdin);
        let copilot = SubprocessProvider::for_cli(CliName::Copilot).unwrap();
        assert!(
            copilot.binary.ends_with("gh-copilot"),
            "binary={}",
            copilot.binary
        );
        assert_eq!(copilot.prompt_mode, PromptMode::Stdin);
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
                ..Default::default()
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
