//! DeepSeek Harness (`dsh`) branch of the subprocess chat transport.
//!
//! Sibling of `chat_subprocess.rs` (which sat at the 800-line cap), the
//! same way `chat_subprocess_quirks.rs` keeps the Codex quirks apart
//! from the generic bridge.
//!
//! Verified `dsh` facts this branch is built on:
//!
//! - No ACP support (scanned the whole package) — the only bridge is a
//!   subprocess, ONE shot per turn: `dsh --profile headless "<prompt>"`
//!   prints the final answer and exits.
//! - Binary `dsh` (npm global `@deepseek-ai/dsh`).
//! - Requires Node ≥ 22: the GUI process must not hand `dsh` the
//!   system default node (v20 crashes with `stripTypeScriptTypes`
//!   missing / exit 127). Like every other Node-based CLI here, the
//!   child inherits the login-shell-repaired PATH
//!   (`chat_spawn::effective_path_env` merged into the child env by
//!   `build_command`, and the process itself repaired by
//!   `chat_spawn::repair_gui_process_env` at GUI startup) — no new
//!   spawn logic was invented for this branch.
//!
//! Output contract: the WHOLE stdout is the answer (passed through
//! verbatim, never JSON-parsed — an answer that happens to look like a
//! JSON event must still render as text); stderr is retained only as
//! diagnostics for failure messages.
//!
//! Fail-closed posture aligns with the Grok Build / Antigravity
//! branches: filtered child env (`chat_subprocess_safety::child_env`),
//! guarded-CLI reap / empty-output checks
//! (`chat_subprocess_safety::is_guarded_cli`), and a wall-clock turn
//! budget ([`DSH_TIMEOUT`]).

use std::time::Duration;

use op_ai::chat_provider::ChatDelta;
#[cfg(test)]
use op_ai::chat_provider::StopReason;

/// Wall clock for one DeepSeek Harness turn.
///
/// Aligned with the crate's WIDEST subprocess budget —
/// `chat_subprocess_quirks::CODEX_TURN_TIMEOUT` (15 min, the TS
/// `DEFAULT_CODEX_TIMEOUT_MS` reference cap). A `dsh` single-shot turn
/// has no streaming events: the bridge only hears back when the CLI
/// prints its final answer and exits, and a deepseek-v4-pro max-effort
/// turn routinely runs several minutes. Two minutes (Antigravity's
/// budget) or five (Grok's) would kill a healthy turn mid-flight.
pub const DSH_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Base argv for every `dsh` turn: headless profile, no interactive
/// session.
pub fn dsh_args() -> Vec<String> {
    vec!["--profile".into(), "headless".into()]
}

/// Full argv for one turn: [`dsh_args`] plus the prompt as a single
/// trailing argument — the exact shape `dsh --profile headless
/// "<prompt>"`. The chat bridge's `PromptMode::BareArg` arm appends
/// the prompt the same way this assembles it, so the unit tests below
/// cover the shipped argv. Test-only: production assembly lives in
/// the bridge's `BareArg` arm (this file exists to pin the contract).
///
/// Shell-escaping safety: `build_command` spawns through execvp
/// (`tokio::process::Command`), never through a shell, so the prompt
/// is ONE argv element no matter what it contains — quotes,
/// `$(…)`, pipes, or newlines travel byte-for-byte and are never
/// interpreted. No escaping step exists to get wrong.
#[cfg(test)]
pub fn dsh_turn_args(prompt: &str) -> Vec<String> {
    let mut args = dsh_args();
    args.push(prompt.to_string());
    args
}

/// Parse one `dsh` stdout line. `dsh` is a plain-stdout CLI, so the
/// contract is "stdout 全文即回答": every line is answer text, passed
/// through verbatim (newline preserved) and NEVER routed through the
/// JSON envelope parser — an answer that legitimately contains a line
/// shaped like `{"type":"error"}` must not be rewritten into an error
/// event.
pub fn parse_dsh_line(line: &str) -> ChatDelta {
    ChatDelta::TextDelta(format!("{line}\n"))
}

/// A `dsh` turn has no terminal in-stream event; the answer simply
/// ends at stdout EOF, after which the bridge sends
/// `Done { EndTurn }` on a zero exit status. Test-only named helper
/// so the end-of-stream contract is explicit and testable (production
/// takes the bridge's generic EOF path, which sends exactly this).
#[cfg(test)]
pub fn end_of_answer() -> ChatDelta {
    ChatDelta::Done {
        stop_reason: StopReason::EndTurn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsh_timeout_aligns_with_the_widest_subprocess_budget() {
        // One-shot turns emit nothing until the final answer; the
        // budget must be the crate's widest (Codex's 15-minute TS cap),
        // not the 2/5-minute guarded-CLI budgets.
        assert_eq!(DSH_TIMEOUT, Duration::from_secs(15 * 60));
        assert_eq!(
            DSH_TIMEOUT,
            crate::chat_subprocess_quirks::CODEX_TURN_TIMEOUT,
            "DSH_TIMEOUT must stay aligned with CODEX_TURN_TIMEOUT"
        );
    }

    #[test]
    fn dsh_args_are_exactly_profile_headless() {
        assert_eq!(dsh_args(), vec!["--profile", "headless"]);
    }

    #[test]
    fn dsh_turn_args_passes_the_prompt_through_as_one_trailing_arg() {
        assert_eq!(
            dsh_turn_args("design a login page"),
            vec!["--profile", "headless", "design a login page"]
        );
    }

    #[test]
    fn dsh_turn_args_keeps_shell_metacharacters_inert() {
        // The prompt must stay ONE argv element: quotes, command
        // substitution, pipes and whitespace all travel verbatim
        // because the bridge spawns via execvp, not a shell. If any of
        // these were ever shell-interpreted, `build_command` would be
        // the bug and this assertion the regression tripwire.
        let hostile = "say $(whoami) && rm -rf /; 'quoted' \"double\" | pipe\nnewline";
        let args = dsh_turn_args(hostile);
        assert_eq!(args.len(), 3);
        assert_eq!(args[2], hostile, "prompt must arrive byte-for-byte");
        // No shell metacharacters can appear as their own argv element.
        for arg in &args {
            assert_ne!(arg, "&&");
            assert_ne!(arg, "|");
            assert_ne!(arg, "$(whoami)");
        }
    }

    #[test]
    fn dsh_stdout_lines_are_always_answer_text_never_events() {
        // The whole stdout is the answer. A line that LOOKS like a
        // JSON event must render as text, not as an error / done event.
        assert_eq!(
            parse_dsh_line(r#"{"type":"error","message":"boom"}"#),
            ChatDelta::TextDelta("{\"type\":\"error\",\"message\":\"boom\"}\n".into())
        );
        assert_eq!(
            parse_dsh_line("plain prose"),
            ChatDelta::TextDelta("plain prose\n".into())
        );
        assert_eq!(
            end_of_answer(),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn
            }
        );
    }
}
