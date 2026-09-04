//! Reading the real reason out of Antigravity's own log file.
//!
//! Measured 2026-08-27: a turn that dies on a server-side precondition prints
//! exactly one line on stderr —
//!
//! ```text
//! Error: Agent execution terminated due to error.
//! ```
//!
//! — which carries no information at all, while the actual cause sits in the
//! CLI's log:
//!
//! ```text
//! ERROR: logging before google.Init: E0827 20:54:36.388855 360 errorreport.go:223]
//!   agent executor error: calling model: FAILED_PRECONDITION (code 400):
//!   User location is not supported for the API use.
//! ```
//!
//! Diagnosing that by hand took a log-file flag, a from-scratch repro and an
//! exit-IP comparison. The turn already writes to a private per-turn
//! directory, so pointing `--log-file` there costs nothing and lets the
//! failure explain itself the first time.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// How much of the log tail to read. The CLI logs verbosely (thousands of
/// lines per turn); the failure is always at the end.
const LOG_TAIL_BYTES: u64 = 64 * 1024;

/// How many distinct error lines to carry. One is usually the cause and the
/// next is its echo, so a small cap keeps the message readable.
const MAX_LINES: usize = 2;

/// Length cap for the whole extracted string, before redaction.
const MAX_CHARS: usize = 300;

/// The error lines Antigravity logged for this turn, redacted and capped, or
/// `None` when the log is absent, unreadable, or carries no error line.
///
/// Absence is not an error here: the log is a diagnostic aid, and a failure to
/// read it must never replace the failure the caller is already reporting.
pub(crate) fn antigravity_log_error(path: &Path) -> Option<String> {
    let text = read_tail(path)?;
    let mut seen: Vec<String> = Vec::new();
    for line in text.lines() {
        let Some(message) = error_message(line) else {
            continue;
        };
        // The CLI logs the same cause twice (executor wrapper + inner call);
        // quoting it twice would read as two separate problems.
        if !seen
            .iter()
            .any(|kept| kept == &message || kept.ends_with(&message))
        {
            seen.retain(|kept| !message.ends_with(kept.as_str()));
            seen.push(message);
        }
    }
    let joined = seen
        .into_iter()
        .rev()
        .take(MAX_LINES)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("; ");
    if joined.is_empty() {
        return None;
    }
    let redacted = op_util::cli_output::redact_secrets(&joined);
    Some(truncate_chars(&redacted, MAX_CHARS))
}

/// Append the CLI's own logged diagnosis to a failure message, when it has one.
///
/// Some CLIs keep their diagnosis out of both pipes: Antigravity's stderr says
/// "Agent execution terminated due to error." for a server-side refusal it
/// logged in full. This APPENDS rather than replaces — the exit status and the
/// child's own words stay the primary account, and the log is corroboration.
pub(crate) fn with_log_evidence(
    message: String,
    turn: Option<&crate::chat_subprocess_safety::IsolatedTurn>,
) -> String {
    match turn
        .and_then(|turn| turn.log_file())
        .as_deref()
        .and_then(antigravity_log_error)
    {
        Some(logged) => format!("{message} — CLI log: {logged}"),
        None => message,
    }
}

/// The message part of one glog-formatted ERROR line, or `None` for any line
/// that is not one.
///
/// glog severity is the first character of the stamp (`E0827 20:54:36...`).
fn error_message(line: &str) -> Option<String> {
    // Every line is prefixed with the CLI's own `ERROR: logging before
    // google.Init:` banner, so the FIRST `E` is never the severity — scan for
    // one actually followed by the 4-digit MMDD stamp.
    let mut idx = 0;
    while let Some(pos) = line[idx..].find('E') {
        let at = idx + pos;
        let rest = &line[at..];
        let is_stamp = rest
            .get(1..5)
            .is_some_and(|mmdd| mmdd.chars().all(|c| c.is_ascii_digit()));
        if is_stamp {
            // The message follows the `file.go:line]` prefix. Warnings and
            // info are deliberately excluded: a failing turn logs plenty of
            // both, and quoting them would bury the cause.
            let body = rest.split_once("] ")?.1.trim();
            return (!body.is_empty()).then(|| body.to_string());
        }
        idx = at + 1;
    }
    None
}

/// The last [`LOG_TAIL_BYTES`] of `path` as UTF-8 (lossy), or `None` when the
/// file cannot be read. The first line of the window is dropped when the file
/// was longer, since the window almost certainly cut it mid-way.
fn read_tail(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let truncated = len > LOG_TAIL_BYTES;
    if truncated {
        file.seek(SeekFrom::Start(len - LOG_TAIL_BYTES)).ok()?;
    }
    let mut buf = Vec::with_capacity(LOG_TAIL_BYTES as usize);
    file.take(LOG_TAIL_BYTES).read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    if truncated {
        text.split_once('\n').map(|(_, rest)| rest.to_string())
    } else {
        Some(text)
    }
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[cfg(test)]
#[path = "chat_subprocess_antigravity_log_tests.rs"]
mod tests;
