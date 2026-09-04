//! What a failed CLI turn tells the user.
//!
//! Split out of `chat_subprocess.rs` (800-line cap) because the exit
//! path grew a real job: it used to pick between two canned sentences,
//! and now it also has to decide what of the child's own output is safe
//! and useful to quote.
//!
//! The rule the whole module exists to enforce: **a failing child never
//! gets summarised down to its exit code alone**. `CLI exited with
//! status 1` with the stderr buffer thrown away is what a user and a
//! log both received for an Antigravity turn that had, in fact,
//! explained itself in full on stderr.

use std::process::ExitStatus;

use op_ai::chat_provider::CliName;
use op_util::cli_output::{diagnostic_tail, diagnostic_tail_capped};

use crate::chat_spawn::exit_status_label;
use crate::chat_subprocess_quirks as quirks;
use crate::chat_subprocess_safety as safety;

/// Cap on the retained Codex stderr tail used for error extraction.
/// Lives here (rather than in the bridge spine) so `chat_subprocess.rs`
/// stays under the 800-line cap — pure code motion.
pub(crate) const STDERR_TAIL_CAP: usize = 64 * 1024;

/// Line cap paired with [`STDERR_TAIL_CAP`]. Generous: the Codex error
/// extractor scans the whole retained blob, and a stack trace is worth
/// keeping in full when it fits inside the byte budget.
pub(crate) const STDERR_TAIL_LINES: usize = 2048;

/// Cap on the retained stdout tail — read only on the failure path, for
/// CLIs that report their diagnosis on stdout. Smaller than the stderr
/// budget: a healthy turn's stdout is the answer, not diagnostics.
pub(crate) const STDOUT_TAIL_CAP: usize = 8 * 1024;

/// Line cap paired with [`STDOUT_TAIL_CAP`].
pub(crate) const STDOUT_TAIL_LINES: usize = 256;

/// How long a reaped turn waits for the stderr drain to reach EOF
/// before formatting its failure message. The child is already reaped so
/// its pipe is at EOF and the drain finishes the instant it is polled —
/// the wait is really for the drain TASK to be scheduled, not for I/O.
/// Under a saturated runtime (the orchestrator's parallel turns, or the
/// concurrent stress test) that scheduling latency occasionally ran past
/// a two-second bound and the tail read back empty, so this is generous:
/// it only has to outlast scheduler starvation, while still capping a
/// genuinely wedged reader (a grandchild holding the pipe open).
pub(crate) const STDERR_DRAIN_GRACE: std::time::Duration = std::time::Duration::from_secs(30);

/// Budget for the tail appended to a *classified* failure. Shorter than
/// the unclassified one: the friendly sentence already carries the
/// meaning, and the quote is there as corroboration — so it must be
/// present, but subordinate.
const CLASSIFIED_TAIL_MAX_CHARS: usize = 300;

/// Whether a child that finished WITHOUT a terminal event and WITHOUT a
/// streamed error should be reported as a failure.
///
/// The subtlety is the unknown case. Reaching this decision at all means the
/// child did not finish cleanly, so an exit status we could not read is the
/// least explicable outcome available — not a quiet success. Treating unknown
/// as success is what let a Windows `cmd /c <missing-binary>` reach the user
/// as a silent `Done { EndTurn }`: the agent appearing to say nothing and
/// stop, with no error anywhere to explain it.
pub(crate) fn unfinished_child_is_failure(status: Option<&ExitStatus>) -> bool {
    status.map(|s| !s.success()).unwrap_or(true)
}

/// The `ChatDelta::Error` text for a child that exited non-zero without
/// having streamed an error of its own.
///
/// Three tiers, most specific first: Codex's ported TS extractor, the
/// per-CLI friendly classifier over BOTH streams, then the exit status
/// with the child's last words attached.
pub(crate) fn exit_failure_message(
    cli: Option<CliName>,
    status: Option<&ExitStatus>,
    stderr_text: &str,
    stdout_text: &str,
) -> String {
    if cli == Some(CliName::Codex) {
        return quirks::extract_codex_cli_error(stderr_text).unwrap_or_else(|| {
            let code = status
                .map(exit_status_label)
                .unwrap_or_else(|| "unknown".into());
            with_output_evidence(
                format!("Codex exited with code {code}."),
                stderr_text,
                stdout_text,
            )
        });
    }
    // Classify against BOTH streams. The stdout matcher used to see only
    // live lines, so a CLI that exits before its terminal event — or
    // prints its diagnosis after the last parsable line — fell through
    // to the bare exit status.
    if let Some(message) = safety::friendly_cli_error(cli, stderr_text, stdout_text) {
        return with_classified_tail(message, stderr_text, stdout_text);
    }
    let code = status.map(exit_status_label).unwrap_or_else(|| "?".into());
    with_output_evidence(
        format!("CLI exited with status {code}"),
        stderr_text,
        stdout_text,
    )
}

/// Attach the child's own last words to a CLASSIFIED failure.
///
/// A classification is a keyword guess, and "classified" used to mean
/// "no evidence shown" — the same defect as the bare exit status, just
/// on the other branch. The real cost of a misclassification is never
/// the wrong label; it is the wrong label *plus* the child's actual
/// words being swallowed, which leaves nothing to notice the mistake
/// with. So the sentence and the quote always travel together.
pub(crate) fn with_classified_tail(
    message: String,
    stderr_text: &str,
    stdout_text: &str,
) -> String {
    let tail = diagnostic_tail_capped(stderr_text, CLASSIFIED_TAIL_MAX_CHARS)
        .or_else(|| diagnostic_tail_capped(stdout_text, CLASSIFIED_TAIL_MAX_CHARS));
    match tail {
        // No quote at all rather than an empty one: the classifier fired
        // on something, and if that something is gone by now, saying so
        // is more honest than an empty pair of quotes.
        None => message,
        Some(tail) => format!("{message} — CLI output: {tail}"),
    }
}

/// Attach the child's own last words to an unclassified exit message.
///
/// stderr wins when both streams spoke, because a CLI that writes to
/// both puts its answer on stdout and its diagnosis on stderr.
///
/// The quoted text is redacted and length-capped by
/// [`op_util::cli_output::diagnostic_tail`] — child output routinely
/// carries OAuth URLs and API keys, and this string lands in a log file
/// and a chat bubble.
fn with_output_evidence(base: String, stderr_text: &str, stdout_text: &str) -> String {
    match diagnostic_tail(stderr_text).or_else(|| diagnostic_tail(stdout_text)) {
        // "no output captured" is itself a finding: it separates "the
        // CLI explained itself and we hid it" from "the CLI died
        // silently", which need different next steps.
        None => format!("{base} (no output captured)"),
        Some(tail) => format!("{base}: {tail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unclassified_exit_quotes_the_child_and_a_silent_one_says_so() {
        assert_eq!(
            exit_failure_message(Some(CliName::GrokBuild), None, "", ""),
            "CLI exited with status ? (no output captured)"
        );
        let message = exit_failure_message(
            Some(CliName::GrokBuild),
            None,
            "sandbox policy refused tool `write_file`",
            "",
        );
        assert!(message.contains("sandbox policy refused"), "{message}");
    }

    #[test]
    fn stderr_is_quoted_in_preference_to_stdout() {
        let message = exit_failure_message(
            Some(CliName::GrokBuild),
            None,
            "the real diagnosis",
            "a partial answer",
        );
        assert!(message.contains("the real diagnosis"), "{message}");
        assert!(!message.contains("a partial answer"), "{message}");
    }

    #[test]
    fn a_classified_failure_carries_both_the_verdict_and_the_evidence() {
        // A classification is a keyword guess. Shipping it WITHOUT the
        // child's own words is the same defect as the bare exit status:
        // there is then nothing to notice a misclassification with.
        let message = exit_failure_message(
            Some(CliName::Antigravity),
            None,
            "Authentication required. Please visit the URL to log in:\n\
             Error: authentication failed or timed out",
            "",
        );
        assert!(
            message.starts_with("Antigravity is not authenticated. Run `agy` once in a terminal."),
            "verdict leads: {message}"
        );
        assert!(message.contains("Authentication required"), "{message}");
        assert!(
            message.contains("authentication failed or timed out"),
            "{message}"
        );
    }

    #[test]
    fn a_classified_tail_is_redacted_and_capped_tighter_than_an_unclassified_one() {
        let message = exit_failure_message(
            Some(CliName::Antigravity),
            None,
            &format!(
                "Authentication required. Please visit the URL to log in:\n  \
                 https://accounts.google.com/o/oauth2/auth?client_id=FAKEID&state=FAKESTATE\n{}",
                "noise line that goes on and on and on\n".repeat(50)
            ),
            "",
        );
        for secret in ["client_id=FAKEID", "state=FAKESTATE"] {
            assert!(!message.contains(secret), "leaked {secret:?} in {message}");
        }
        assert!(
            message.chars().count() <= 96 + CLASSIFIED_TAIL_MAX_CHARS,
            "message was {} chars: {message}",
            message.chars().count()
        );
    }

    #[test]
    fn a_classification_with_nothing_left_to_quote_stays_a_bare_verdict() {
        // `friendly_stdout_error` can fire on a live line whose text is
        // gone from both retained tails by the time we format. An empty
        // pair of quotes would be worse than none.
        let message = with_classified_tail("Grok Build is not authenticated.".into(), "", "");
        assert_eq!(message, "Grok Build is not authenticated.");
    }
}
