//! 重试辅助函数 —— S3b-1b Task A1。
//!
//! 对齐 TS `orchestrator.ts:1323-1329`(tier→mode 序列)
//! 与 `orchestrator-sub-agent.ts:150-152`(不可重试错误检测)。

// 这两个函数的调用方在 Task C2(`run.rs`)中添加;此处暂无调用方。
#![allow(dead_code)]

use crate::model_profile::ModelTier;
use crate::types::PlanningMode;

/// 按照 tier 返回规划模式的重试序列(静态切片,首个失败 → 下一档)。
///
/// Port of `orchestrator.ts:1323-1329`:
/// ```ts
/// const ATTEMPT_MODES = {
///   Full:     [Rich],
///   Standard: [Rich, Minimal],
///   Basic:    [Rich, Minimal, Compact],
/// }
/// ```
pub(crate) fn attempt_modes(tier: ModelTier) -> &'static [PlanningMode] {
    match tier {
        ModelTier::Full => &[PlanningMode::Rich],
        ModelTier::Standard => &[PlanningMode::Rich, PlanningMode::Minimal],
        ModelTier::Basic => &[
            PlanningMode::Rich,
            PlanningMode::Minimal,
            PlanningMode::Compact,
        ],
    }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── attempt_modes ───────────────────────────────────────────────────────

    #[test]
    fn full_tier_only_rich() {
        assert_eq!(attempt_modes(ModelTier::Full), &[PlanningMode::Rich]);
    }

    #[test]
    fn standard_tier_rich_then_minimal() {
        assert_eq!(
            attempt_modes(ModelTier::Standard),
            &[PlanningMode::Rich, PlanningMode::Minimal]
        );
    }

    #[test]
    fn basic_tier_all_three() {
        assert_eq!(
            attempt_modes(ModelTier::Basic),
            &[
                PlanningMode::Rich,
                PlanningMode::Minimal,
                PlanningMode::Compact
            ]
        );
    }

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
}
