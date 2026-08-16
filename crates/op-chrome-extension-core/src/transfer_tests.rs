//! Tests for [`crate::transfer`] — chunk planning, integrity, truncation.

use crate::transfer::{
    mcp_envelope_too_large, snapshot_too_large, ChunkPlan, CHUNK_CHARS, MAX_SNAPSHOT_MB,
    MCP_FALLBACK_BODY_MB,
};

/// Feed a plan one slice whose text is irrelevant to the assertion.
fn accept_len(plan: &mut ChunkPlan, len: u32) -> bool {
    plan.accept("", f64::from(len))
}

#[test]
fn a_single_slice_completes_a_small_snapshot() {
    let mut plan = ChunkPlan::new(10);
    assert_eq!(plan.next_offset(), 0);
    assert_eq!(plan.next_length(), CHUNK_CHARS);
    assert!(!plan.done());
    assert!(plan.accept("0123456789", 10.0));
    assert!(plan.done());
    assert_eq!(plan.transferred(), 10);
    assert!(plan.verify_total(10));
}

#[test]
fn offsets_advance_by_what_actually_arrived() {
    let mut plan = ChunkPlan::new(CHUNK_CHARS + 7);
    assert!(accept_len(&mut plan, CHUNK_CHARS));
    assert_eq!(plan.next_offset(), CHUNK_CHARS);
    assert!(!plan.done());
    // A short-but-nonempty slice is legal; the loop simply asks again.
    assert!(accept_len(&mut plan, 3));
    assert_eq!(plan.next_offset(), CHUNK_CHARS + 3);
    assert!(accept_len(&mut plan, 4));
    assert!(plan.done());
    assert!(plan.verify_total(CHUNK_CHARS + 7));
}

#[test]
fn a_missing_slice_is_reported_as_lost() {
    let mut plan = ChunkPlan::new(100);
    // `-1` is how the caller reports "the tab did not answer with a string".
    assert!(!plan.accept("", -1.0));
    assert!(!plan.done());
    assert!(!plan.verify_total(100));
}

#[test]
fn an_empty_slice_is_reported_as_lost() {
    let mut plan = ChunkPlan::new(100);
    assert!(!accept_len(&mut plan, 0));
    assert!(!plan.verify_total(100));
}

#[test]
fn a_nan_length_cannot_stall_the_read_loop() {
    let mut plan = ChunkPlan::new(100);
    assert!(!plan.accept("", f64::NAN));
    assert!(!plan.accept("", f64::INFINITY));
    assert!(!plan.done());
}

#[test]
fn a_fractional_length_cannot_stall_the_read_loop() {
    // A value in (0, 1) used to pass the `> 0` gate and then floor to zero
    // units transferred, so the caller's `while (!plan.done)` loop would ask
    // for the same offset forever with the popup's buttons disabled.
    for length in [0.5, 0.999_999] {
        let mut plan = ChunkPlan::new(100);
        assert!(!plan.accept("x", length), "expected {length} to be refused");
        assert_eq!(plan.transferred(), 0);
        assert!(!plan.done());
    }
    // A whole number that merely arrives as a float is still fine.
    let mut plan = ChunkPlan::new(100);
    assert!(plan.accept("x", 100.0));
    assert!(plan.done());
}

#[test]
fn a_length_with_a_fraction_is_refused_even_when_large() {
    let mut plan = ChunkPlan::new(100);
    assert!(!plan.accept("x", 10.5));
    assert!(!plan.done());
}

#[test]
fn a_slice_longer_than_requested_is_refused() {
    let mut plan = ChunkPlan::new(CHUNK_CHARS * 2);
    assert!(!accept_len(&mut plan, CHUNK_CHARS + 1));
    assert!(!plan.done());
}

#[test]
fn an_overrun_past_the_expected_length_is_refused() {
    let mut plan = ChunkPlan::new(10);
    assert!(!accept_len(&mut plan, 11));
    assert!(!plan.done());
}

#[test]
fn a_failed_plan_stays_failed() {
    let mut plan = ChunkPlan::new(10);
    assert!(!accept_len(&mut plan, 0));
    assert!(!accept_len(&mut plan, 10));
    assert!(!plan.done());
    assert!(!plan.verify_total(10));
}

#[test]
fn the_final_length_must_match_exactly() {
    let mut plan = ChunkPlan::new(10);
    assert!(accept_len(&mut plan, 10));
    assert!(plan.verify_total(10));
    assert!(!plan.verify_total(9));
    assert!(!plan.verify_total(11));
}

#[test]
fn truncation_is_detected_in_both_spacings() {
    for body in [
        r#"{"truncated": true}"#,
        r#"{"truncated":true}"#,
        "{\"truncated\"  :\n\ttrue}",
        r#"{"nodes":[],"truncated" : true}"#,
    ] {
        let mut plan = ChunkPlan::new(body.len() as u32);
        assert!(plan.accept(body, body.len() as f64));
        assert!(plan.truncated(), "expected {body:?} to read as truncated");
    }
}

#[test]
fn a_false_or_absent_flag_does_not_trip_the_notice() {
    for body in [
        r#"{"truncated": false}"#,
        r#"{"truncated":"true"}"#,
        r#"{"nodes":[]}"#,
        r#"{"untruncated": true}"#,
        r#"{"truncatedish": true}"#,
    ] {
        let mut plan = ChunkPlan::new(body.len() as u32);
        assert!(plan.accept(body, body.len() as f64));
        assert!(
            !plan.truncated(),
            "expected {body:?} not to read as truncated"
        );
    }
}

#[test]
fn truncation_is_detected_across_a_slice_boundary() {
    let head = r#"{"nodes":[],"trunc"#;
    let tail = r#"ated": true}"#;
    let total = (head.len() + tail.len()) as u32;
    let mut plan = ChunkPlan::new(total);
    assert!(plan.accept(head, head.len() as f64));
    assert!(!plan.truncated());
    assert!(plan.accept(tail, tail.len() as f64));
    assert!(plan.truncated());
    assert!(plan.done());
}

#[test]
fn the_matcher_re_anchors_on_a_fresh_quote_after_a_mismatch() {
    // The first `"trunc"` fails at the closing quote; matching must restart
    // from that quote rather than skipping past it.
    let body = r#"{"trunc""truncated":true}"#;
    let mut plan = ChunkPlan::new(body.len() as u32);
    assert!(plan.accept(body, body.len() as f64));
    assert!(plan.truncated());
}

#[test]
fn the_matcher_keeps_the_shared_quote_when_a_full_key_is_not_followed_by_a_colon() {
    // The key ends with the quote that also begins it, so two keys can
    // overlap on one character. Falling all the way back to the start after
    // the `PreColon` mismatch consumed that quote and missed the real flag.
    for body in [
        r#"{"truncated"truncated": true}"#,
        r#"{"truncated""truncated":true}"#,
        r#"{"truncated"x"truncated" : true}"#,
    ] {
        let mut plan = ChunkPlan::new(body.len() as u32);
        assert!(plan.accept(body, body.len() as f64));
        assert!(plan.truncated(), "expected {body:?} to read as truncated");
    }
}

#[test]
fn the_overlapping_key_case_also_works_across_a_slice_boundary() {
    let head = r#"{"a":1,"truncated"trunc"#;
    let tail = r#"ated": true}"#;
    let total = (head.len() + tail.len()) as u32;
    let mut plan = ChunkPlan::new(total);
    assert!(plan.accept(head, head.len() as f64));
    assert!(!plan.truncated());
    assert!(plan.accept(tail, tail.len() as f64));
    assert!(plan.truncated());
}

#[test]
fn a_full_key_followed_by_a_non_quote_still_does_not_false_positive() {
    for body in [
        r#"{"truncated"x: true}"#,
        r#"{"truncated"-false,"other":1}"#,
    ] {
        let mut plan = ChunkPlan::new(body.len() as u32);
        assert!(plan.accept(body, body.len() as f64));
        assert!(
            !plan.truncated(),
            "expected {body:?} not to read as truncated"
        );
    }
}

#[test]
fn the_size_precheck_matches_the_servers_body_cap() {
    let cap = f64::from(MAX_SNAPSHOT_MB) * 1024.0 * 1024.0;
    assert!(!snapshot_too_large(0.0));
    assert!(!snapshot_too_large(cap));
    assert!(snapshot_too_large(cap + 1.0));
    assert_eq!(MAX_SNAPSHOT_MB, 48);
}

#[test]
fn the_envelope_precheck_matches_the_endpoints_whole_body_cap() {
    // The /mcp fallback wraps the snapshot in a JSON envelope whose
    // re-escaping can exceed the raw snapshot cap; the outer bound is the
    // endpoint-wide 64 MiB `mcp_serve::MAX_BODY`.
    let cap = f64::from(MCP_FALLBACK_BODY_MB) * 1024.0 * 1024.0;
    assert!(!mcp_envelope_too_large(cap));
    assert!(mcp_envelope_too_large(cap + 1.0));
    assert_eq!(MCP_FALLBACK_BODY_MB, 64);
    // The envelope cap must stay above the raw cap or the fallback could
    // never carry a maximum-size snapshot at all.
    const { assert!(MCP_FALLBACK_BODY_MB > MAX_SNAPSHOT_MB) }
}
