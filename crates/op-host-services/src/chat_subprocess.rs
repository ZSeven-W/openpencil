//! Single-shot subprocess bridge for Claude Code (fallback), Codex,
//! Antigravity, Grok Build, and DeepSeek Harness. OpenCode uses its HTTP
//! transport; Copilot uses its SDK transport.
//!
//! Codex runs `codex exec [--ephemeral] --json ... -` with an allowlisted
//! environment and the prompt on stdin. Antigravity and Grok use isolated, fail-closed turns;
//! DSH runs `dsh --profile headless <prompt>` with its narrow child
//! environment. Their exact argv and parsers live in the corresponding
//! `chat_subprocess_*` siblings.
//!
//! Multi-turn context rides an in-band history digest. Codex parse misses are
//! skipped; custom binaries fall back to plain text. Every child has stderr
//! drained, exit status interpreted, and its full process tree terminated on
//! cancellation or deadline. Binary discovery lives in [`chat_spawn`].

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, CliName, EffortLevel, StopReason,
};
use op_process_io::LineStreamChild;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use crate::chat_runtime::{prompt_with_system_prompt, shared_runtime, BlockingRecvIter};
use crate::chat_spawn::{build_command, find_binary, runtime_path_for_binary};
use crate::chat_subprocess_lifecycle::{child_env_for_cli, wait_for_terminal_exit};
use crate::chat_subprocess_quirks as quirks;
use crate::chat_subprocess_quirks::codex_reasoning_effort;
use crate::chat_subprocess_safety as safety;

pub use crate::chat_subprocess_parse::parse_line;

/// How the user's prompt reaches the CLI. Claude Code's `--print`
/// mode requires the prompt as a positional argv after `--` and
/// closes stdin immediately. Codex (`-` prompt arg) reads the
/// message off piped stdin. Generic `with_binary` callers can
/// pick either via [`SubprocessProvider::with_binary_mode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    /// Append `-- <prompt>` to argv; stdin gets closed (no input).
    PositionalArg,
    /// Argv is passed verbatim; user_message is written to stdin
    /// followed by EOF.
    Stdin,
    /// Append `<flag> <prompt>` to argv; stdin gets closed.
    FlagArg(&'static str),
    /// Append `<flag> <private-file>`; stdin gets closed.
    PromptFile(&'static str),
    /// Append `<prompt>` itself as the final argv element — the CLI's
    /// interface takes the prompt as a bare trailing argument
    /// (`dsh --profile headless "<prompt>"`). Stdin gets closed so the
    /// one-shot CLI cannot fall back into interactive mode. The prompt
    /// stays visible in argv while the child is alive (same documented
    /// tradeoff as Antigravity's `-p`).
    BareArg,
}

/// `ChatProvider` impl that bridges to a CLI binary via stdio.
/// Construct via [`SubprocessProvider::for_cli`] or
/// [`SubprocessProvider::with_binary`] for a custom binary path.
pub struct SubprocessProvider {
    binary: String,
    args: Vec<String>,
    label: String,
    prompt_mode: PromptMode,
    /// Argv flag that selects a model for this CLI (`--model` for
    /// Codex and Antigravity, `-m` for Grok Build). `None` = the transport has no model
    /// selector; `ChatRequest::model` is ignored and the CLI keeps
    /// its own default.
    model_flag: Option<&'static str>,
    /// True when the CLI accepts Codex's native reasoning knob
    /// (`--config model_reasoning_effort=<level>`). When set, the
    /// thinking + effort knobs ride that flag instead of the in-band
    /// directive line (TS parity: `codex-client.ts` never prepends a
    /// prose directive).
    native_effort_config: bool,
    /// Trailing argv appended after the per-turn flags — Codex's `-`
    /// stdin marker. TS keeps the same flag-then-marker order.
    tail_args: Vec<String>,
    /// Which known CLI this provider bridges (drives per-CLI env
    /// filtering, line parsing, stderr capture, and timeout quirks).
    /// `None` for custom `with_binary` providers — generic behavior.
    cli: Option<CliName>,
    turn_purpose: safety::TurnPurpose,
}

impl SubprocessProvider {
    /// Build a subprocess provider for a known [`CliName`]. Each CLI
    /// has its own argv template + prompt-routing mode (see module
    /// docs). Returns `None` for OpenCode (HTTP-server transport
    /// in `chat_http_server.rs`) and Copilot (official SDK transport
    /// in `chat_copilot.rs`) — neither has a stdio wire.
    pub fn for_cli(cli: CliName) -> Option<Self> {
        Self::for_cli_with_purpose(cli, safety::TurnPurpose::CanvasAgent)
    }

    /// Build a tool-free provider for orchestrator, subtask, and codegen turns.
    pub fn for_cli_generation(cli: CliName) -> Option<Self> {
        Self::for_cli_with_purpose(cli, safety::TurnPurpose::Generation)
    }

    fn for_cli_with_purpose(cli: CliName, turn_purpose: safety::TurnPurpose) -> Option<Self> {
        // Per-CLI model selector (third tuple slot): Codex takes
        // `--model <id>` — matching the TS reference. Claude Code's
        // model rides the SDK adapter (`chat_claude.rs`), so
        // its subprocess template carries no flag here.
        type Template = (Vec<String>, PromptMode, Option<&'static str>, Vec<String>);
        let (mut args, prompt_mode, model_flag, tail_args): Template = match cli {
            CliName::ClaudeCode => (
                vec![
                    "--print".into(),
                    "--verbose".into(),
                    "--output-format".into(),
                    "stream-json".into(),
                ],
                PromptMode::PositionalArg,
                None,
                Vec::new(),
            ),
            // TS `codex-client.ts` argv plus capability-gated non-persistence:
            // `exec [--ephemeral] --json
            // --skip-git-repo-check --sandbox read-only [--model]
            // [--config model_reasoning_effort=…] -` with the prompt
            // piped via stdin (the `-` marker). `--output-last-message`
            // is intentionally not ported — Rust streams the agent
            // message instead of re-reading it from a temp file.
            CliName::Codex => (
                vec![
                    "exec".into(),
                    "--json".into(),
                    "--skip-git-repo-check".into(),
                    "--sandbox".into(),
                    "read-only".into(),
                ],
                PromptMode::Stdin,
                Some("--model"),
                vec!["-".into()],
            ),
            CliName::Antigravity => (
                safety::antigravity_args(turn_purpose),
                PromptMode::FlagArg("-p"),
                Some("--model"),
                Vec::new(),
            ),
            CliName::GrokBuild => (
                safety::grok_args(turn_purpose),
                PromptMode::PromptFile("--prompt-file"),
                Some("-m"),
                Vec::new(),
            ),
            // DeepSeek Harness: one-shot subprocess, prompt as a bare
            // trailing argv element (its only verified interface), no
            // model selector. Args / parser / timeout live in the
            // `chat_subprocess_dsh` sibling (this file sits at the
            // 800-line cap).
            CliName::Dsh => (
                crate::chat_subprocess_dsh::dsh_args(),
                PromptMode::BareArg,
                None,
                Vec::new(),
            ),
            // Copilot's routed transport is the official SDK
            // (`chat_copilot.rs`); the old `gh-copilot suggest`
            // template was a stale dead end. OpenCode chats over its
            // local HTTP server (`chat_http_server.rs`).
            CliName::Copilot | CliName::OpenCode => return None,
        };
        let binary = find_binary(cli.default_binary());
        if cli == CliName::Codex {
            quirks::append_codex_ephemeral_arg(std::path::Path::new(&binary), &mut args);
        }
        Some(Self {
            binary,
            args,
            label: cli.label().into(),
            prompt_mode,
            model_flag,
            // Only Codex has the native reasoning-effort config knob.
            native_effort_config: cli == CliName::Codex,
            tail_args,
            cli: Some(cli),
            turn_purpose,
        })
    }

    /// Build a subprocess provider with a user-supplied binary path
    /// and argv (defaults to stdin prompt). Used when the settings
    /// modal needs to point at a non-PATH install.
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
            // Custom binaries carry no known model / effort flags;
            // the request's model is ignored (CLI default applies).
            model_flag: None,
            native_effort_config: false,
            tail_args: Vec::new(),
            cli: None,
            turn_purpose: safety::TurnPurpose::Generation,
        }
    }

    /// Point a known-CLI provider at a stand-in binary so the exit /
    /// stderr / stdout handling can be exercised without the real CLI.
    /// Its only callers are the unix-gated exit tests (the stand-ins are
    /// `/bin/sh` scripts), so it carries the same gate to stay live-code
    /// on Windows.
    #[cfg(all(test, unix))]
    pub(crate) fn with_test_binary(mut self, binary: impl Into<String>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Argv for one turn: the configured base args plus the model
    /// selector and (Codex) the native reasoning-effort config when
    /// the request carries them, then the trailing prompt marker
    /// (`-` / `-p ' '`). Empty / blank model ids emit no flag at all —
    /// the CLI keeps its own default.
    fn turn_args(&self, request: &ChatRequest) -> Vec<String> {
        let mut args = self.args.clone();
        if let (Some(flag), Some(model)) = (self.model_flag, request.model_id()) {
            let provider_default = model == "default"
                && matches!(self.cli, Some(CliName::Antigravity | CliName::GrokBuild));
            if !provider_default {
                args.push(flag.into());
                args.push(model.into());
            }
        }
        if self.native_effort_config {
            if let Some(level) = codex_reasoning_effort(request.thinking, request.effort) {
                args.push("--config".into());
                args.push(format!("model_reasoning_effort={level}"));
            }
        }
        args.extend(self.tail_args.iter().cloned());
        args
    }
}

impl ChatProvider for SubprocessProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn supports_cancellable_send(&self) -> bool {
        true
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, None)
    }

    fn send_cancellable(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, Some(cancel))
    }
}

impl SubprocessProvider {
    fn send_inner(
        &self,
        request: ChatRequest,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let binary = self.binary.clone();
        let cli = self.cli;
        // Build the effective prompt. Claude `--print` does not
        // expose a portable reasoning flag, so the thinking +
        // effort knobs travel in-band as a leading directive line
        // (documented beyond-TS-baseline behavior); staged attachments
        // are appended as `[attached …: <path>]` lines the CLI can
        // read by path. The guard removes the temp files when the
        // worker task ends.
        let (mut prompt, guard) = match crate::chat_attachment::prompt_with_attachments(
            &request.user_message,
            &request.attachments,
        ) {
            Ok(pair) => pair,
            Err(e) => return crate::chat_attachment::attachment_error_turn(e),
        };
        // CLIs with the native reasoning knob (Codex) get the knobs
        // as `--config model_reasoning_effort=…` via `turn_args`
        // instead of the in-band directive — sending both would
        // double-signal (TS `codex-client.ts` only uses the flag).
        if !self.native_effort_config {
            let mut directive = String::new();
            if let Some(d) = crate::chat_attachment::thinking_directive(request.thinking) {
                directive.push_str(d);
            }
            if request.effort != EffortLevel::Low {
                if !directive.is_empty() {
                    directive.push(' ');
                }
                directive.push_str(&format!(
                    "Apply {} reasoning effort.",
                    request.effort.as_str()
                ));
            }
            if !directive.is_empty() {
                prompt = format!("{directive}\n\n{prompt}");
            }
        }
        // Single-shot CLI wire: each send spawns a fresh process, so a
        // compact transcript digest carries cross-turn context in-band
        // (TS parity baseline is system prompt + last message; the
        // digest is the documented beyond-baseline extra).
        let digest = op_ai::chat_history::history_digest(
            &request.history,
            op_ai::chat_history::DEFAULT_DIGEST_CHARS,
        );
        if !digest.is_empty() {
            prompt = format!("{digest}\n\n{prompt}");
        }
        // System-prompt framing: Codex mirrors the TS GUIDELINES /
        // TASK `buildPrompt` byte-for-byte; everything
        // else keeps the generic `system\n\n---\n\nprompt` join.
        prompt = match cli {
            Some(CliName::Codex) => quirks::guidelines_task_prompt(&request.system_prompt, &prompt),
            _ => prompt_with_system_prompt(&request.system_prompt, prompt),
        };
        let attachment_paths = guard.as_ref().map(|g| g.paths()).unwrap_or(&[]);
        let prepared_turn = match self.turn_purpose {
            safety::TurnPurpose::CanvasAgent => {
                safety::IsolatedTurn::prepare(cli, &prompt, attachment_paths)
            }
            safety::TurnPurpose::Generation => {
                safety::IsolatedTurn::prepare_generation(cli, &prompt, attachment_paths)
            }
        };
        let isolation = match prepared_turn {
            Ok(turn) => turn,
            Err(e) => {
                return Box::new(
                    vec![
                        ChatDelta::Error(format!("failed to isolate CLI turn: {e}")),
                        ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        },
                    ]
                    .into_iter(),
                );
            }
        };
        if let Some(turn) = &isolation {
            prompt = turn.prompt().to_string();
        }
        let mut args_with_prompt = self.turn_args(&request);
        if let Some(turn) = &isolation {
            turn.append_cli_args(&mut args_with_prompt);
        }
        // PromptMode::PositionalArg: append `-- <prompt>` so the CLI
        // picks up the message as a CLI argument (Claude Code mode).
        // PromptMode::Stdin (default): leave argv untouched; the
        // prompt is written to stdin after spawn.
        match self.prompt_mode {
            PromptMode::PositionalArg => {
                args_with_prompt.push("--".into());
                args_with_prompt.push(prompt.clone());
            }
            PromptMode::FlagArg(flag) => {
                args_with_prompt.push(flag.into());
                args_with_prompt.push(prompt.clone());
            }
            PromptMode::BareArg => {
                // `dsh --profile headless "<prompt>"`: the prompt is the
                // final argv element, no flag. Same assembly as
                // `chat_subprocess_dsh::dsh_turn_args` (which the unit
                // tests cover).
                args_with_prompt.push(prompt.clone());
            }
            PromptMode::PromptFile(flag) => {
                args_with_prompt.push(flag.into());
                args_with_prompt.push(
                    isolation
                        .as_ref()
                        .and_then(safety::IsolatedTurn::prompt_file)
                        .expect("prompt-file mode requires isolated prompt")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
            PromptMode::Stdin => {}
        }
        let args = Arc::new(args_with_prompt);
        let prompt_mode = self.prompt_mode;
        // Per-CLI child env: Codex uses the TS allowlist so
        // unrelated secrets never reach the CLI; Claude Code + custom
        // binaries keep the scrub-the-dangerous-vars policy.
        let mut env_pairs = child_env_for_cli(cli);
        safety::append_isolated_env(&mut env_pairs, isolation.as_ref());
        let turn_timeout = match cli {
            Some(CliName::Codex) => Some(quirks::CODEX_TURN_TIMEOUT),
            Some(CliName::Antigravity) => Some(safety::ANTIGRAVITY_TIMEOUT),
            Some(CliName::GrokBuild) => Some(safety::GROK_TIMEOUT),
            Some(CliName::Dsh) => Some(crate::chat_subprocess_dsh::DSH_TIMEOUT),
            _ => None,
        };
        let label = self.label.clone();
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            // Keep staged attachment temp files alive for the turn.
            let _guard = guard;
            let _isolation = isolation;
            let deadline = tokio::time::Instant::now()
                + turn_timeout.unwrap_or(Duration::from_secs(60 * 60 * 24 * 365));
            let mut cmd = build_command(&binary, &args);
            if let Some(turn) = &_isolation {
                cmd.current_dir(turn.cwd());
            }
            // Set the child's env from the per-CLI policy. We
            // env_clear first because tokio::process Command
            // otherwise inherits the parent env verbatim.
            cmd.env_clear();
            cmd.envs(env_pairs);
            // Reapply PATH after `env_clear`, pinned to the resolved wrapper's
            // directory. Codex's npm launcher uses `#!/usr/bin/env node`; a
            // different Node earlier in the GUI process PATH can select the
            // wrong architecture-specific optional package.
            cmd.env(
                "PATH",
                runtime_path_for_binary(std::path::Path::new(&binary)),
            );
            let mut child = match LineStreamChild::spawn_command(cmd) {
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

            // Drain stderr on a sibling task so the CLI never
            // deadlocks on a full stderr pipe (codex BLOCK 1), and keep
            // a BOUNDED tail of what it said. The tail used to be kept
            // only for Codex (the TS `extractCodexCliError` port) and
            // discarded for everyone else — which is how a failing
            // Antigravity turn reached the user as a bare
            // `CLI exited with status 1` while the child had printed a
            // full explanation on stderr (measured 2026-08-07: piped
            // `agy` writes its whole unauthenticated block to stderr
            // and leaves stdout empty). Every CLI now keeps it.
            let stderr_tail = Arc::new(std::sync::Mutex::new(op_util::cli_output::BoundedTail::new(
                crate::chat_subprocess_exit::STDERR_TAIL_CAP,
                crate::chat_subprocess_exit::STDERR_TAIL_LINES,
            )));
            let stderr_drain = child.take_stderr().map(|stderr| {
                let capture = Arc::clone(&stderr_tail);
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stderr).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if let Ok(mut buf) = capture.lock() {
                            buf.push_line(&line);
                        }
                    }
                })
            });

            // Bound both a backpressured prompt write and EOF delivery by the
            // same receiver/deadline contract as stdout. Dropping this future
            // releases its child borrow before tree-aware cleanup below.
            let stdin_result = {
                let prepare_stdin = async {
                    if prompt_mode == PromptMode::Stdin {
                        child.feed(prompt.as_bytes()).await?;
                    }
                    let _ = child.close_stdin().await;
                    Ok::<(), std::io::Error>(())
                };
                tokio::pin!(prepare_stdin);
                tokio::select! {
                    biased;
                    _ = tx.closed() => None,
                    _ = tokio::time::sleep_until(deadline), if turn_timeout.is_some() => {
                        let secs = turn_timeout.map(|d| d.as_secs()).unwrap_or_default();
                        let _ = tx.send(ChatDelta::Error(format!(
                            "{label} request timed out after {secs}s."
                        ))).await;
                        let _ = tx.send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        }).await;
                        None
                    }
                    result = &mut prepare_stdin => Some(result),
                }
            };
            if let Some(Err(error)) = &stdin_result {
                let _ = tx
                    .send(ChatDelta::Error(format!("stdin write: {error}")))
                    .await;
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::Aborted,
                    })
                    .await;
            }
            if !matches!(stdin_result, Some(Ok(()))) {
                let _ = child.start_kill();
                let _ = tokio::time::timeout(safety::EXIT_GRACE, child.wait()).await;
                return;
            }

            let mut lines = match child.take_lines() {
                Some(lines) => lines,
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

            let mut emitted_done = false;
            let mut terminal_error = false;
            let mut emitted_text = false;
            let mut emitted_error = false;
            // Bounded record of what the child put on stdout, so a CLI
            // that reports its failure there instead of on stderr still
            // has evidence to show when the exit status is all we get.
            let mut stdout_tail =
                op_util::cli_output::BoundedTail::new(
                crate::chat_subprocess_exit::STDOUT_TAIL_CAP,
                crate::chat_subprocess_exit::STDOUT_TAIL_LINES,
            );
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
                    // TS wall clock: SIGTERM + "request timed out"
                    // (`codex-client.ts` rejects text).
                    _ = tokio::time::sleep_until(deadline), if turn_timeout.is_some() => {
                        let secs = turn_timeout.map(|d| d.as_secs()).unwrap_or_default();
                        let _ = tx
                            .send(ChatDelta::Error(format!(
                                "{label} request timed out after {secs}s."
                            )))
                            .await;
                        let _ = tx
                            .send(ChatDelta::Done {
                                stop_reason: StopReason::Aborted,
                            })
                            .await;
                        terminal_error = true;
                        let _ = child.start_kill();
                        break;
                    }
                    result = lines.next_line() => match result {
                        Ok(Some(line)) => {
                            stdout_tail.push_line(&line);
                            let codex_terminal_error = cli == Some(CliName::Codex)
                                && quirks::is_codex_terminal_error(&line);
                            if let Some(message) = safety::friendly_stdout_error(cli, &line) {
                                // Same rule as the exit path: a verdict
                                // never travels without the child's own
                                // words. stderr is best-effort here (the
                                // child is still alive, its drain task
                                // may be mid-flight); stdout at least
                                // carries the line that just matched.
                                let errs = stderr_tail.lock().map(|b| b.text()).unwrap_or_default();
                                let message = crate::chat_subprocess_exit::with_classified_tail(
                                    message, &errs, &stdout_tail.text(),
                                );
                                let _ = tx.send(ChatDelta::Error(message)).await;
                                let _ = tx.send(ChatDelta::Done { stop_reason: StopReason::Aborted }).await;
                                terminal_error = true;
                                let _ = child.start_kill();
                                break;
                            }
                            // Per-CLI parse: Codex skips unparsed
                            // lines (TS parity); generic
                            // CLIs degrade them to raw text.
                            let delta = match cli {
                                Some(CliName::Codex) => quirks::parse_codex_line(&line),
                                Some(CliName::GrokBuild) => {
                                    crate::chat_grok_stream::parse_grok_stream_line(&line)
                                }
                                // dsh's whole stdout is the answer: every
                                // line is text, never an event envelope.
                                Some(CliName::Dsh) => {
                                    Some(crate::chat_subprocess_dsh::parse_dsh_line(&line))
                                }
                                _ => Some(parse_line(&line)),
                            };
                            let Some(delta) = delta else { continue };
                            match &delta {
                                ChatDelta::TextDelta(_) => emitted_text = true,
                                ChatDelta::Error(_) => emitted_error = true,
                                _ => {}
                            }
                            // TS `runCodexExec`: a turn that completes
                            // with no text and no errors is an explicit
                            // failure ("Codex returned no output."), not
                            // a silent empty bubble. Codex signals turn
                            // end in-stream (`turn.completed`), so the
                            // check rides the Done conversion here; the
                            // post-EOF branch below covers streams that
                            // end without a turn event.
                            let delta = match delta {
                                ChatDelta::Done { .. }
                                    if emitted_error && safety::is_guarded_cli(cli) =>
                                {
                                    ChatDelta::Done {
                                        stop_reason: StopReason::Aborted,
                                    }
                                }
                                ChatDelta::Done { .. }
                                    if cli == Some(CliName::Codex)
                                        && !emitted_text
                                        && !emitted_error =>
                                {
                                    emitted_error = true;
                                    let _ = tx
                                        .send(ChatDelta::Error(
                                            "Codex returned no output.".into(),
                                        ))
                                        .await;
                                    ChatDelta::Done {
                                        stop_reason: StopReason::Aborted,
                                    }
                                }
                                other => other,
                            };
                            let is_done = matches!(delta, ChatDelta::Done { .. });
                            if tx.send(delta).await.is_err() {
                                let _ = child.start_kill();
                                break;
                            }
                            if codex_terminal_error {
                                let _ = tx
                                    .send(ChatDelta::Done {
                                        stop_reason: StopReason::Aborted,
                                    })
                                    .await;
                                emitted_done = true;
                                terminal_error = true;
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
            // Aborted instead of an unrelated `EndTurn`. Codex routes
            // stderr through the TS extractor and surfaces the TS
            // "returned no output" error on an empty success.
            // A guarded CLI may emit its terminal event before its
            // process actually exits. Bound that final reap as well,
            // otherwise a well-formed `Done` could bypass the turn
            // wall clock and leave the chat blocked indefinitely.
            let terminal_deadline =
                deadline.min(tokio::time::Instant::now() + safety::EXIT_GRACE);
            let status = wait_for_terminal_exit(&mut child, terminal_deadline, &tx).await;
            // Let the stderr drain finish before anyone reads its tail.
            // The child exiting ends BOTH pipes at once, so the drain
            // task is routinely still mid-flight here — under load it
            // may not have been polled at all, and the tail then reads
            // back EMPTY, reporting a child that explained itself as
            // `(no output captured)`. Reproduced 2026-08-07 under four
            // concurrent test binaries. Bounded: the child is already
            // reaped, so its pipe is at EOF and this returns at once.
            if let Some(drain) = stderr_drain {
                let _ = tokio::time::timeout(crate::chat_subprocess_exit::STDERR_DRAIN_GRACE, drain).await;
            }
            if !emitted_done && !terminal_error {
                // Unknown status counts as failure here — see
                // `unfinished_child_is_failure` for why.
                if crate::chat_subprocess_exit::unfinished_child_is_failure(status.as_ref()) {
                    // A live-streamed Error already explained the
                    // failure — don't stack a second message on it
                    // (TS surfaces exactly one error per turn).
                    if !emitted_error {
                        let stderr_text = stderr_tail
                            .lock()
                            .map(|buf| buf.text())
                            .unwrap_or_default();
                        let stdout_text = stdout_tail.text();
                        let msg = crate::chat_subprocess_exit::exit_failure_message(
                            cli,
                            status.as_ref(),
                            &stderr_text,
                            &stdout_text,
                        );
                        let msg = crate::chat_subprocess_antigravity_log::with_log_evidence(
                            msg,
                            _isolation.as_ref(),
                        );
                        let _ = tx.send(ChatDelta::Error(msg)).await;
                    }
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                } else if safety::is_guarded_cli(cli) && emitted_error {
                    let _ = tx
                        .send(ChatDelta::Done {
                            stop_reason: StopReason::Aborted,
                        })
                        .await;
                } else if (cli == Some(CliName::Codex) || safety::is_guarded_cli(cli))
                    && !emitted_text
                    && !emitted_error
                {
                    // TS `runCodexExec`: empty final text + no errors
                    // = explicit failure, not a silent empty bubble.
                    let _ = tx
                        .send(ChatDelta::Error(format!("{label} returned no output.")))
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
        match cancel {
            Some(cancel) => Box::new(BlockingRecvIter::cooperative(rx, cancel)),
            None => Box::new(BlockingRecvIter::new(rx)),
        }
    }
}

#[cfg(test)]
#[path = "chat_subprocess_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chat_subprocess_exit_tests.rs"]
mod exit_tests;
