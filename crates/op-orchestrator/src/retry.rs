//! 重试辅助函数 —— S3b-1b Task A1。
//!
//! 对齐 TS `orchestrator-sub-agent.ts:150-152`(不可重试错误检测)。
//! 规划阶段的 tier→mode 序列(`attempt_modes`)已随单档规划收敛移除。

/// Provider account/session-quota substrings (motion50 fix 1). These come from
/// the Claude Code CLI's plain-text quota message —
/// "You've hit your session limit · resets 2:50pm (Asia/Shanghai)" — which
/// matched nothing in the old table, so lane1/web-10 burned the full
/// 3-attempt ladder PLUS the salvage pass on all 13 subtasks and still exited
/// 0. Unlike a per-request HTTP 429, a quota with a wall-clock reset time
/// cannot recover within a run's seconds-scale retry window.
const PROVIDER_LIMIT_SUBSTRINGS: &[&str] = &[
    "session limit",
    "hit your session limit", // subsumed by "session limit"; kept greppable
    "usage limit",
];

/// True when the error reports the provider's account/session quota is
/// exhausted until a reset time. Consumed twice: `is_non_retryable` (stop the
/// per-subtask ladder) and the run-level circuit breaker in `run.rs`'s
/// sequential loop / the concurrent executor (two consecutive quota failures
/// set the `AbortFlag` so no further model calls are spent).
pub(crate) fn is_provider_limit(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    // "resets <time>" on its own is too loose (a transport error can say the
    // connection "resets"); it only counts next to a limit word.
    PROVIDER_LIMIT_SUBSTRINGS
        .iter()
        .any(|needle| lower.contains(needle))
        || (lower.contains("resets ") && lower.contains("limit"))
}

/// 判断错误消息是否为不可重试的终止条件。
///
/// Port of `orchestrator-sub-agent.ts:150-152`:
/// ```ts
/// const isNonRetryable = (msg: string) =>
///   /HTTP 4(0[01]|29|51)|content blocked|authentication failed|censorship/i.test(msg);
/// ```
///
/// 用简单的 `to_lowercase() + contains` 替代 regex crate,语义完全等价。
pub(crate) fn is_non_retryable(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("http 400")
        || lower.contains("http 401")
        || lower.contains("http 429")
        || lower.contains("http 451")
        || lower.contains("content blocked")
        || lower.contains("authentication failed")
        || lower.contains("censorship")
        // A CLI's own transport-config failure is deterministic — codex's
        // stream-reconnect rejects the macOS system proxy with "Invalid
        // proxy configuration: http://127.0.0.1:7897" on EVERY attempt;
        // burning the 3-attempt ladder + the salvage pass on it costs
        // minutes and ends the same way. Fail fast with the message intact.
        || lower.contains("invalid proxy configuration")
        // Provider session/usage quota messages (Claude Code CLI) — a quota
        // with a wall-clock reset never recovers inside one run's retry
        // ladder, so fail the subtask fast (motion50 fix 1).
        || is_provider_limit(msg)
}

/// True when a subtask failed because our own post-generation self-check
/// (`orchestration_self_check`) rejected otherwise-parsed, otherwise-real
/// content — a geometry/quality judgment on THIS model's output — rather
/// than a transport, parsing, or capability failure (stream error, script
/// syntax error, blank output, `InsertSubtree` rejection).
///
/// The distinction matters for the retry ladder
/// (`concurrent::run_subtask_retry_ladder`): a quality rejection is not
/// evidence the model needs a narrower skill set to succeed — the content
/// was otherwise fine, so throwing away skills on attempt 2 only makes the
/// *rest* of the design worse while doing nothing to fix the one flagged
/// issue. Skill-tier downgrade should stay reserved for the failures that
/// actually suggest the model is struggling with the full prompt (timeouts,
/// stream errors, safety-scanner stalls) — see `subagent::run_subtask_with_reveal_at`'s
/// `fail(format!("self-check failed: {message}"))` call site for the exact
/// prefix this matches.
pub(crate) fn is_self_check_rejection(msg: &str) -> bool {
    msg.starts_with("self-check failed: ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_non_retryable — true cases ───────────────────────────────────────

    #[test]
    fn http_400_is_non_retryable() {
        assert!(is_non_retryable("HTTP 400 Bad Request"));
    }

    #[test]
    fn http_401_is_non_retryable() {
        assert!(is_non_retryable("HTTP 401 Unauthorized"));
    }

    #[test]
    fn http_429_is_non_retryable() {
        assert!(is_non_retryable("HTTP 429 rate limited"));
    }

    #[test]
    fn http_451_is_non_retryable() {
        assert!(is_non_retryable("HTTP 451 Unavailable For Legal Reasons"));
    }

    #[test]
    fn content_blocked_is_non_retryable() {
        assert!(is_non_retryable("content blocked by policy"));
    }

    #[test]
    fn authentication_failed_is_non_retryable() {
        assert!(is_non_retryable("authentication failed: invalid key"));
    }

    #[test]
    fn censorship_lowercase_is_non_retryable() {
        assert!(is_non_retryable("censorship filter triggered"));
    }

    #[test]
    fn censorship_uppercase_is_non_retryable() {
        assert!(is_non_retryable("CENSORSHIP detected"));
    }

    // ── is_non_retryable — false cases ──────────────────────────────────────

    #[test]
    fn http_500_is_retryable() {
        assert!(!is_non_retryable("HTTP 500 Internal Server Error"));
    }

    #[test]
    fn timed_out_is_retryable() {
        assert!(!is_non_retryable("timed out after 30s"));
    }

    #[test]
    fn socket_closed_is_retryable() {
        assert!(!is_non_retryable("socket closed unexpectedly"));
    }

    // ── is_self_check_rejection ─────────────────────────────────────────────

    #[test]
    fn self_check_failure_message_is_a_self_check_rejection() {
        assert!(is_self_check_rejection(
            "self-check failed: radial-stack-not-concentric at n14: ..."
        ));
    }

    #[test]
    fn stream_error_is_not_a_self_check_rejection() {
        assert!(!is_self_check_rejection(
            "stream disconnected before completion"
        ));
    }

    #[test]
    fn script_parse_error_is_not_a_self_check_rejection() {
        assert!(!is_self_check_rejection(
            "script error: unexpected end of string"
        ));
    }

    #[test]
    fn blank_container_is_not_a_self_check_rejection() {
        assert!(!is_self_check_rejection(
            "blank container root produced no content nodes"
        ));
    }
}

#[cfg(test)]
mod proxy_tests {
    use super::*;

    #[test]
    fn invalid_proxy_configuration_is_non_retryable() {
        assert!(is_non_retryable(
            "stream disconnected before completion: URL error: Invalid proxy configuration: http://127.0.0.1:7897"
        ));
    }

    // ── motion50 fix 1: provider session/usage quota exhaustion ─────────────

    /// The exact lane1/web-10 message: every subtask burned the 3-attempt
    /// ladder + salvage on it because the plain-text CLI quota message matched
    /// nothing in the old table.
    #[test]
    fn claude_cli_session_limit_message_is_non_retryable() {
        assert!(is_non_retryable(
            "You've hit your session limit · resets 2:50pm (Asia/Shanghai)"
        ));
    }

    #[test]
    fn usage_limit_and_reset_wording_is_non_retryable() {
        assert!(is_non_retryable(
            "You've hit your usage limit · resets 3pm (Asia/Shanghai)"
        ));
        assert!(is_non_retryable("session limit reached, resets 09:00"));
        assert!(is_non_retryable("USAGE LIMIT exceeded"));
    }

    #[test]
    fn plain_errors_are_not_provider_limits() {
        assert!(!is_non_retryable("stream disconnected before completion"));
        assert!(!is_non_retryable("HTTP 500 upstream"));
    }
}
