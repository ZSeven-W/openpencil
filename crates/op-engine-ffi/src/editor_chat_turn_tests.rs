//! Buffer-safety contracts for the shared mobile SSE parser.

use super::*;

#[test]
fn unterminated_single_line_over_the_limit_is_rejected_without_its_body() {
    let error = ensure_sse_size(MAX_SSE_EVENT_BYTES + 1).expect_err("oversized line");
    let message = error.to_string();
    assert!(message.contains("safety limit"));
    assert!(!message.contains("provider-body-marker"));
}

#[test]
fn accumulated_multi_data_event_cannot_cross_the_limit() {
    let mut event = String::new();
    let first = "a".repeat(MAX_SSE_EVENT_BYTES / 2);
    let second = "b".repeat(MAX_SSE_EVENT_BYTES / 2);
    append_sse_data(&mut event, &first).expect("first data line fits");
    let error = append_sse_data(&mut event, &second).expect_err("separator crosses limit");
    assert!(error.to_string().contains("safety limit"));
    assert_eq!(event.len(), first.len(), "failed append is atomic");
}

#[test]
fn exactly_bounded_event_remains_accepted() {
    let mut event = String::new();
    append_sse_data(&mut event, &"x".repeat(MAX_SSE_EVENT_BYTES)).expect("exact boundary");
    assert_eq!(event.len(), MAX_SSE_EVENT_BYTES);
}
