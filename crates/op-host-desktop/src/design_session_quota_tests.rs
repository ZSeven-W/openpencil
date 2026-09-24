//! Quota-error rendering tests, split out of `design_session.rs` at the
//! 800-line cap (pure code motion).

use super::friendly_quota_error;

#[test]
fn ark_quota_json_renders_one_friendly_sentence_with_reset_time() {
    let raw = r#"orchestration failed: openai-compatible http 429 Too Many Requests: {"error":{"code":"AccountQuotaExceeded","message":"You have exceeded the 5-hour usage quota. It will reset at 2026-07-10 16:59:53 +0800 CST. We recommend upgrading your plan for more quota, or waiting for the reset. Request id: 0217","param":"","type":"TooManyRequests"}}"#;
    let friendly = friendly_quota_error(raw).expect("quota-shaped error");
    assert!(
        friendly.contains("2026-07-10 16:59:53 +0800 CST"),
        "{friendly}"
    );
    assert!(
        !friendly.contains('{'),
        "no raw JSON in the friendly line: {friendly}"
    );
}

#[test]
fn non_quota_errors_pass_through() {
    assert!(friendly_quota_error("orchestration failed: http 500 internal").is_none());
    assert!(friendly_quota_error("parse error in subtask").is_none());
}
