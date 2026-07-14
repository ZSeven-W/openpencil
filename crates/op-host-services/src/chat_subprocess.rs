//! Subprocess CLI bridge — spawns an external CLI binary
//! (Claude Code / Codex / Gemini / Antigravity / Grok Build) and bridges its stdio into the
//! shell-core `ChatProvider` shape.
//!
//! Per-CLI wire protocol (single-shot mode; multi-turn context rides
//! the history digest — see below):
//!
//! - **Claude Code (`claude`)** — invoked as
//!   `claude --print --verbose --output-format stream-json -- <prompt>`.
//!   Prompt is a positional argv after `--`. Stdin closes immediately.
//!   Stdout is line-delimited JSON parsed by the generic envelope
//!   parser ([`parse_line`]). The routed chat path uses the SDK
//!   adapter (`chat_claude.rs`); this template is the fallback.
//! - **Codex (`codex`)** — `codex exec --json --skip-git-repo-check
//!   --sandbox read-only [--model <id>] [--config
//!   model_reasoning_effort=<level>] -` with the prompt piped via
//!   stdin (TS parity: `codex-client.ts` uses the `-` stdin prompt on
//!   all platforms to dodge Windows shell-escaping + argv length
//!   limits). Child env is allowlist-filtered
//!   ([`chat_subprocess_quirks::codex_child_env`]); stderr is captured
//!   for the TS error extraction (`extractCodexCliError`).
//!   Divergence from TS (documented): TS buffers the whole turn and
//!   reads `--output-last-message`; Rust streams `item.completed`
//!   agent messages live — same final text, no temp file.
//! - **Gemini CLI (`gemini`)** — `gemini -o stream-json
//!   --approval-mode plan [-m <id>] -p ' '` with the prompt piped via
//!   stdin (TS parity: `gemini-client.ts::streamGeminiExec`; the
//!   `-p ' '` marker is the TS quirk — Gemini appends the `-p` value
//!   after stdin content). Lines parse via
//!   [`chat_subprocess_quirks::parse_gemini_stream_line`]; child env
//!   is allowlist-filtered.
//! - **GitHub Copilot** — no subprocess template. The routed path is
//!   the official SDK (`chat_copilot.rs`); the old `gh-copilot
//!   suggest` fallback was stale (the `gh` extension was retired in
//!   favor of the standalone `copilot` CLI) so `for_cli` now refuses
//!   instead of shipping a dead argv.
//! - **OpenCode** — served by the HTTP-server transport
//!   (`chat_http_server.rs`), not a stdio subprocess.
//! - **Antigravity (`agy`)** — isolated-cwd `--sandbox` print mode plus a
//!   private per-turn HOME containing a strict, no-write/no-command/no-web
//!   policy and only this editor process's loopback OpenPencil MCP entry.
//!   The OS keyring remains available for authentication. Its verified
//!   one-shot API only accepts `-p <prompt>`, so the prompt remains visible in
//!   argv while that child is alive.
//! - **Grok Build (`grok`)** — private `--prompt-file`, fail-closed
//!   `dontAsk` permissions in both argv and a private per-turn
//!   `CLAUDE_CONFIG_DIR` compatibility policy, strict sandbox, read-only
//!   built-ins, and an explicit `MCPTool(openpencil__*)` pre-approval. Other
//!   MCP servers merged into the user's global config are not pre-approved and
//!   are silently denied; the prompt guard independently forbids them.
//!   Its NDJSON `text`, `thought`, `end`, and `error` events parse through
//!   [`crate::chat_grok_stream::parse_grok_stream_line`].
//!
//! Codex / Gemini parse-misses are skipped silently (TS parity — TS
//! drops unparsed lines); generic custom binaries degrade unparsed
//! lines to raw `TextDelta` so plain-stdout CLIs still show output.
//! On stdout EOF the bridge reaps the child + interprets exit status —
//! non-zero exit surfaces as `Error + Done { Aborted }` (codex BLOCK
//! 4 / 5), with Codex stderr routed through the TS error extractor.
//! Codex / Gemini turns also carry the TS 15-minute wall clock
//! (`<CLI> request timed out after 900s.`). On receiver drop the
//! bridge `child.start_kill()` so the user can navigate away without
//! leaking processes.
//!
//! Binary lookup falls back through PATH then per-platform default
//! install paths (npm / yarn / bun globals) — see
//! `chat_spawn::find_binary`.

use std::sync::Arc;
use std::time::Duration;

use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, CliName, EffortLevel, StopReason, ThinkingMode,
};
use op_process_io::LineStreamChild;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use crate::chat_runtime::{prompt_with_system_prompt, shared_runtime, BlockingRecvIter};
use crate::chat_spawn::{build_command, exit_status_label, find_binary};
use crate::chat_subprocess_quirks as quirks;
use crate::chat_subprocess_safety as safety;

pub use crate::chat_subprocess_parse::parse_line;

/// How the user's prompt reaches the CLI. Claude Code's `--print`
/// mode requires the prompt as a positional argv after `--` and
/// closes stdin immediately. Codex (`-` prompt arg) and Gemini read
/// the message off piped stdin. Generic `with_binary` callers can
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
}

/// TS `DEFAULT_CODEX_TIMEOUT_MS` / `DEFAULT_GEMINI_TIMEOUT_MS` —
/// both reference clients cap a turn at 15 minutes then SIGTERM.
const CLI_TURN_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Cap on the retained Codex stderr tail used for error extraction.
const STDERR_TAIL_CAP: usize = 64 * 1024;

/// `ChatProvider` impl that bridges to a CLI binary via stdio.
/// Construct via [`SubprocessProvider::for_cli`] or
/// [`SubprocessProvider::with_binary`] for a custom binary path.
pub struct SubprocessProvider {
    binary: String,
    args: Vec<String>,
    label: String,
    prompt_mode: PromptMode,
    /// Argv flag that selects a model for this CLI (`--model` for
    /// Codex, `-m` for Gemini — both verified against the TS
    /// reference clients). `None` = the transport has no model
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
    /// stdin marker, Gemini's `-p ' '` marker. TS keeps the same
    /// flag-then-marker order (`codex-client.ts` / `gemini-client.ts`).
    tail_args: Vec<String>,
    /// Which known CLI this provider bridges (drives per-CLI env
    /// filtering, line parsing, stderr capture, and timeout quirks).
    /// `None` for custom `with_binary` providers — generic behavior.
    cli: Option<CliName>,
}

impl SubprocessProvider {
    /// Build a subprocess provider for a known [`CliName`]. Each CLI
    /// has its own argv template + prompt-routing mode (see module
    /// docs). Returns `None` for OpenCode (HTTP-server transport
    /// in `chat_http_server.rs`) and Copilot (official SDK transport
    /// in `chat_copilot.rs`) — neither has a stdio wire.
    pub fn for_cli(cli: CliName) -> Option<Self> {
        // Per-CLI model selector (third tuple slot): Codex takes
        // `--model <id>` and Gemini `-m <id>` — matching the TS
        // reference (`codex-client.ts` / `gemini-client.ts`). Claude
        // Code's model rides the SDK adapter (`chat_claude.rs`), so
        // its subprocess template carries no flag here.
        type Template = (Vec<String>, PromptMode, Option<&'static str>, Vec<String>);
        let (args, prompt_mode, model_flag, tail_args): Template = match cli {
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
            // TS `gemini-client.ts::streamGeminiExec` argv order:
            // `-o stream-json --approval-mode plan [-m model] -p ' '`,
            // prompt via stdin.
            CliName::Gemini => (
                vec![
                    "-o".into(),
                    "stream-json".into(),
                    "--approval-mode".into(),
                    "plan".into(),
                ],
                PromptMode::Stdin,
                Some("-m"),
                vec!["-p".into(), " ".into()],
            ),
            // TS `codex-client.ts` argv: `exec --json
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
                safety::antigravity_args(),
                PromptMode::FlagArg("-p"),
                Some("--model"),
                Vec::new(),
            ),
            CliName::GrokBuild => (
                safety::grok_args(),
                PromptMode::PromptFile("--prompt-file"),
                Some("-m"),
                Vec::new(),
            ),
            // Copilot's routed transport is the official SDK
            // (`chat_copilot.rs`); the old `gh-copilot suggest`
            // template was a stale dead end. OpenCode chats over its
            // local HTTP server (`chat_http_server.rs`).
            CliName::Copilot | CliName::OpenCode => return None,
        };
        let binary = find_binary(cli.default_binary());
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
            // Custom binaries carry no known model / effort flags;
            // the request's model is ignored (CLI default applies).
            model_flag: None,
            native_effort_config: false,
            tail_args: Vec::new(),
            cli: None,
        }
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

/// Map the per-turn thinking + effort knobs onto Codex's
/// `model_reasoning_effort` config value. Mirrors TS
/// `codex-client.ts::resolveCodexEffort`: disabled thinking forces
/// `low`; `max` folds to Codex's top tier `high`; otherwise the
/// effort token passes through. The defaulted pair (`Adaptive` +
/// `Low`) emits no flag so an untouched panel keeps the CLI's own
/// default — same convention as the in-band directive path.
fn codex_reasoning_effort(thinking: ThinkingMode, effort: EffortLevel) -> Option<&'static str> {
    if thinking == ThinkingMode::Disabled {
        return Some("low");
    }
    match (thinking, effort) {
        (ThinkingMode::Adaptive, EffortLevel::Low) => None,
        (_, EffortLevel::Max) => Some("high"),
        (_, e) => Some(e.as_str()),
    }
}

/// Dangerous environment variables that should never be propagated to
/// the spawned CLI: linker preload paths can hijack execution; PATH
/// can substitute a malicious binary; runtime-library paths
/// (NODE_OPTIONS, PYTHONPATH, etc.) can inject code into Node-based
/// CLIs. Mirrors bartolli/anthropic-agent-sdk's `DANGEROUS_ENV_VARS`.
/// Used for Claude Code + custom binaries; Codex / Gemini get the
/// stricter TS allowlists (`chat_subprocess_quirks`). Returns the
/// env-var pairs the child will receive (parent env minus the
/// dangerous names — preserving every safe var so node version
/// managers like nvm / volta still pick the right Node).
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
        let cli = self.cli;
        // Build the effective prompt. Neither claude `--print` nor
        // gemini exposes a portable reasoning flag, so the thinking +
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
        // System-prompt framing: Codex + Gemini mirror the TS
        // GUIDELINES / TASK `buildPrompt` byte-for-byte; everything
        // else keeps the generic `system\n\n---\n\nprompt` join.
        prompt = match cli {
            Some(CliName::Codex) | Some(CliName::Gemini) => {
                quirks::guidelines_task_prompt(&request.system_prompt, &prompt)
            }
            _ => prompt_with_system_prompt(&request.system_prompt, prompt),
        };
        let attachment_paths = guard.as_ref().map(|g| g.paths()).unwrap_or(&[]);
        let isolation = match safety::IsolatedTurn::prepare(cli, &prompt, attachment_paths) {
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
        // Per-CLI child env: Codex / Gemini use the TS allowlists so
        // unrelated secrets never reach the CLI; Claude Code + custom
        // binaries keep the scrub-the-dangerous-vars policy.
        let mut env_pairs = match cli {
            Some(CliName::Codex) => quirks::codex_child_env(),
            Some(CliName::Gemini) => quirks::gemini_child_env(),
            Some(CliName::Antigravity | CliName::GrokBuild) => {
                safety::child_env(cli).unwrap_or_default()
            }
            _ => scrubbed_child_env(),
        };
        safety::append_isolated_env(&mut env_pairs, isolation.as_ref());
        let turn_timeout = match cli {
            Some(CliName::Codex | CliName::Gemini) => Some(CLI_TURN_TIMEOUT),
            Some(CliName::Antigravity) => Some(safety::ANTIGRAVITY_TIMEOUT),
            Some(CliName::GrokBuild) => Some(safety::GROK_TIMEOUT),
            _ => None,
        };
        let label = self.label.clone();
        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            // Keep staged attachment temp files alive for the turn.
            let _guard = guard;
            let _isolation = isolation;
            let mut cmd = build_command(&binary, &args);
            if let Some(turn) = &_isolation {
                cmd.current_dir(turn.cwd());
            }
            // Set the child's env from the per-CLI policy. We
            // env_clear first because tokio::process Command
            // otherwise inherits the parent env verbatim.
            cmd.env_clear();
            cmd.envs(env_pairs);
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
            // deadlocks on a full stderr pipe (codex BLOCK 1). Codex
            // keeps a bounded tail for the TS error extraction
            // (`extractCodexCliError`); other CLIs discard it (TS
            // parity: gemini stream path discards stderr).
            let stderr_tail: Arc<std::sync::Mutex<String>> = Arc::default();
            if let Some(stderr) = child.take_stderr() {
                let capture = (cli == Some(CliName::Codex) || safety::is_guarded_cli(cli))
                    .then(|| Arc::clone(&stderr_tail));
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stderr).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if let Some(tail) = &capture {
                            if let Ok(mut buf) = tail.lock() {
                                buf.push_str(&line);
                                buf.push('\n');
                                if buf.len() > STDERR_TAIL_CAP {
                                    let cut = buf.len() - STDERR_TAIL_CAP;
                                    buf.drain(..cut);
                                }
                            }
                        }
                    }
                });
            }

            match prompt_mode {
                PromptMode::Stdin => {
                    // Feed the user message + close stdin so the CLI
                    // sees EOF and starts responding. Stdin write
                    // errors surface as a chat error.
                    if let Err(e) = child.feed(prompt.as_bytes()).await {
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
                    // No stdin write — prompt is in argv. Close stdin
                    // immediately so the CLI doesn't sit waiting on it
                    // (Claude Code's `--print` mode exits if stdin
                    // stays open with no input).
                }
                PromptMode::FlagArg(_) => {
                    // Prompt is carried in argv; closing stdin prevents a
                    // one-shot CLI from falling back into interactive mode.
                }
                PromptMode::PromptFile(_) => {
                    // Prompt lives in the private isolated workspace.
                }
            }
            let _ = child.close_stdin().await; // EOF; ignore close error

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

            let deadline = tokio::time::Instant::now()
                + turn_timeout.unwrap_or(Duration::from_secs(60 * 60 * 24 * 365));
            let mut emitted_done = false;
            let mut terminal_error = false;
            let mut emitted_text = false;
            let mut emitted_error = false;
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
                    // (codex-client.ts / gemini-client.ts reject text).
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
                            // Per-CLI parse: Codex / Gemini skip
                            // unparsed lines (TS parity); generic
                            // CLIs degrade them to raw text.
                            let delta = match cli {
                                Some(CliName::Codex) => quirks::parse_codex_line(&line),
                                Some(CliName::Gemini) => quirks::parse_gemini_stream_line(&line),
                                Some(CliName::GrokBuild) => {
                                    crate::chat_grok_stream::parse_grok_stream_line(&line)
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
            let status = if safety::is_guarded_cli(cli) {
                child.kill_graceful(safety::EXIT_GRACE).await.ok()
            } else {
                child.wait().await.ok()
            };
            if !emitted_done && !terminal_error {
                let nonzero = status.as_ref().map(|s| !s.success()).unwrap_or(false);
                if nonzero {
                    // A live-streamed Error already explained the
                    // failure — don't stack a second message on it
                    // (TS surfaces exactly one error per turn).
                    if !emitted_error {
                        let tail = stderr_tail
                            .lock()
                            .map(|buf| buf.clone())
                            .unwrap_or_default();
                        let msg = if cli == Some(CliName::Codex) {
                            quirks::extract_codex_cli_error(&tail).unwrap_or_else(|| {
                                let code = status
                                    .as_ref()
                                    .map(exit_status_label)
                                    .unwrap_or_else(|| "unknown".into());
                                format!("Codex exited with code {code}.")
                            })
                        } else if let Some(message) = safety::friendly_stderr_error(cli, &tail) {
                            message
                        } else {
                            let code = status
                                .as_ref()
                                .map(exit_status_label)
                                .unwrap_or_else(|| "?".into());
                            format!("CLI exited with status {code}")
                        };
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
        Box::new(BlockingRecvIter::new(rx))
    }
}

#[cfg(test)]
#[path = "chat_subprocess_tests.rs"]
mod tests;
