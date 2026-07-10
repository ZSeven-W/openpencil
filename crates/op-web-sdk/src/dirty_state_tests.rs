// Unit tests for the dirty/schedule state machine backing the rAF pump.
// Plain-Rust, no wasm/canvaskit dependency — see `dirty_state.rs`.
//
// In these tests, "the arm succeeded" is simulated by calling `commit_arm()`
// after `mark_dirty()` returned true, and "the arm failed" by NOT calling
// it — exactly mirroring `render.rs::arm_pump`, which commits only when
// `requestAnimationFrame` accepted the callback.
use super::dirty_state::{DirtyState, FailureAction, MAX_CONSECUTIVE_FAILURES};

/// Drive one successful arm: mark, assert an arm was needed, commit it.
fn mark_and_commit(s: &DirtyState) {
    assert!(s.mark_dirty(), "expected the machine to need arming");
    s.commit_arm();
}

/// Drive one full successful frame for whatever is currently pending.
fn pump_successful_frame(s: &DirtyState) {
    let gen = s.begin_paint().expect("expected a generation to paint");
    let needs_rearm = s.finish_paint_success(gen);
    assert!(!needs_rearm, "no concurrent mark in this helper");
}

#[test]
fn initial_state_is_dirty_and_unscheduled() {
    let s = DirtyState::new_dirty();
    assert!(s.is_dirty());
    // The first frame is armed by start_raf_pump through the same
    // request-then-commit path as every later one — never pre-committed.
    assert!(!s.is_scheduled());
}

#[test]
fn mark_dirty_while_idle_requests_a_frame() {
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);
    pump_successful_frame(&s);
    assert!(!s.is_dirty());
    assert!(!s.is_scheduled());

    let must_arm = s.mark_dirty();
    assert!(must_arm, "an idle pump must be told to arm a new frame");
    s.commit_arm();
    assert!(s.is_dirty());
    assert!(s.is_scheduled());
}

#[test]
fn mark_dirty_while_a_frame_is_already_pending_does_not_double_schedule() {
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);
    // A frame is now pending. A second mark before it fires must not ask
    // for a second rAF request.
    let must_arm = s.mark_dirty();
    assert!(!must_arm, "a frame is already in flight");
    assert!(s.is_dirty());
    assert!(s.is_scheduled());
}

#[test]
fn successful_frame_clears_dirty_and_schedule() {
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);
    let gen = s.begin_paint().expect("initial content must be painted");
    assert!(!s.is_scheduled(), "begin_paint consumes the schedule");
    assert!(!s.finish_paint_success(gen));
    assert!(!s.is_dirty());
    assert!(!s.is_scheduled());
}

#[test]
fn begin_paint_reports_no_work_when_nothing_is_dirty() {
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);
    pump_successful_frame(&s);
    // Pump fires again (e.g. a stray queued frame) with nothing changed.
    assert_eq!(s.begin_paint(), None);
}

#[test]
fn failed_arm_leaves_the_machine_re_armable() {
    // F2 regression: requestAnimationFrame rejected the request. The old
    // code set scheduled=true BEFORE requesting, so a failure left
    // dirty=true + scheduled=true with no frame — every later mark saw
    // "already scheduled" and never retried: a permanent freeze.
    let s = DirtyState::new_dirty();

    // Arm attempt fails: mark says "arm needed" but commit_arm is never
    // called (mirrors arm_pump on a failed request).
    assert!(s.mark_dirty());
    assert!(s.is_dirty());
    assert!(!s.is_scheduled(), "a failed request must not look pending");

    // The next mark must retry the arm — this is the fix.
    assert!(
        s.mark_dirty(),
        "after a failed arm the machine must ask to arm again"
    );
    s.commit_arm();
    pump_successful_frame(&s);
    assert!(!s.is_dirty());
}

#[test]
fn mark_during_paint_survives_the_post_paint_consume() {
    // F3 regression: a mark landing between begin_paint and
    // finish_paint_success must not be consumed by this frame — only the
    // pre-paint generation is.
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);

    let gen = s.begin_paint().expect("first frame has work");

    // Mark arrives DURING the paint. begin_paint already consumed the
    // schedule, so the mark arms its own frame; simulate a successful arm.
    assert!(s.mark_dirty(), "schedule was consumed, so arming is needed");
    s.commit_arm();

    // Post-paint consume of the pre-paint generation only.
    let needs_rearm = s.finish_paint_success(gen);
    assert!(
        !needs_rearm,
        "the during-paint mark already armed its own frame"
    );
    assert!(s.is_dirty(), "the newer generation must remain unpainted");
    assert!(s.is_scheduled());

    // The armed frame paints the newer generation.
    pump_successful_frame(&s);
    assert!(!s.is_dirty());
}

#[test]
fn mark_during_paint_with_failed_arm_is_rescued_by_finish_paint() {
    // Same as above, but the during-paint mark's own rAF request failed:
    // finish_paint_success must report that the caller has to re-arm.
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);

    let gen = s.begin_paint().expect("first frame has work");
    // During-paint mark whose arm failed: no commit_arm.
    assert!(s.mark_dirty());

    let needs_rearm = s.finish_paint_success(gen);
    assert!(
        needs_rearm,
        "newer unpainted content with no pending frame must be re-armed"
    );
    s.commit_arm(); // the pump's rescue arm succeeds
    pump_successful_frame(&s);
    assert!(!s.is_dirty());
}

#[test]
fn paint_failure_re_arms_then_stops_at_the_bound() {
    // Bounded-failure path: a failing paint keeps the generation dirty
    // and retries, but the same content may only fail
    // MAX_CONSECUTIVE_FAILURES times before the breaker stops the
    // pump (the caller logs once).
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);

    for attempt in 1..=MAX_CONSECUTIVE_FAILURES {
        let gen = s.begin_paint().expect("failed content stays paintable");
        assert!(gen > 0);
        let action = s.record_failure();
        if attempt < MAX_CONSECUTIVE_FAILURES {
            assert_eq!(action, FailureAction::Rearm);
            s.commit_arm(); // the retry frame was requested successfully
            assert!(s.is_dirty(), "a failed paint must not consume dirty");
        } else {
            assert_eq!(action, FailureAction::Stop);
        }
    }

    // Breaker tripped: still dirty, but deliberately stopped — no work is
    // handed out, so the pump cannot spin.
    assert!(s.is_stopped());
    assert!(s.is_dirty());
    assert_eq!(s.begin_paint(), None);
}

#[test]
fn arm_failures_share_the_same_breaker_as_paint_failures() {
    // F1 regression: a rejected requestAnimationFrame used to leave the
    // machine dirty-but-unscheduled with no failure recorded at all — if
    // no further external mark ever arrived, the viewer stayed frozen
    // forever with nothing logged. `render.rs::arm_pump` now routes a
    // failed request through this same `record_failure()` breaker used by
    // paint failures, so it is exercised (and bounded) without needing an
    // external mark: a mix of arm and paint failures on the same content
    // generation still trips the breaker at MAX_CONSECUTIVE_FAILURES and
    // a subsequent mark still resets it.
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);

    // First attempt: the paint itself fails.
    let gen = s.begin_paint().expect("initial content is paintable");
    assert_eq!(s.record_failure(), FailureAction::Rearm);
    assert!(gen > 0);
    assert!(s.is_dirty(), "a failed attempt must not consume dirty");

    // Remaining attempts: requestAnimationFrame itself keeps failing (no
    // begin_paint/finish_paint call at all — arm_pump never got a frame to
    // paint with). This mirrors the retry loop in `render.rs::arm_pump`.
    for attempt in 2..MAX_CONSECUTIVE_FAILURES {
        let action = s.record_failure();
        assert_eq!(action, FailureAction::Rearm, "attempt {attempt}");
    }
    // The final failure (whichever kind) trips the breaker.
    assert_eq!(s.record_failure(), FailureAction::Stop);
    assert!(s.is_stopped());
    assert!(s.is_dirty(), "content is still unpainted, not lost");
    assert_eq!(
        s.begin_paint(),
        None,
        "a tripped breaker must hand out no work"
    );

    // No external mark needed to observe the trip — but a new mark still
    // recovers it, exactly like the pure-paint-failure case.
    assert!(s.mark_dirty(), "post-stop mark must ask to arm");
    assert!(!s.is_stopped());
    s.commit_arm();
    pump_successful_frame(&s);
    assert!(!s.is_dirty());
}

#[test]
fn new_mark_resets_the_failure_breaker_and_retries() {
    // After a bounded stop, new content (a fresh mark) must reset the
    // breaker so the viewer recovers instead of staying dead.
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);
    for _ in 1..=MAX_CONSECUTIVE_FAILURES {
        s.begin_paint().expect("still paintable until the stop");
        if s.record_failure() == FailureAction::Rearm {
            s.commit_arm();
        }
    }
    assert!(s.is_stopped());

    assert!(s.mark_dirty(), "post-stop mark must ask to arm");
    assert!(!s.is_stopped(), "new content resets the breaker");
    s.commit_arm();
    pump_successful_frame(&s);
    assert!(!s.is_dirty());
}

#[test]
fn success_resets_the_consecutive_failure_count() {
    // One failure, then a success, then failures again: the budget must
    // restart after the success (the bound is on *consecutive* failures).
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);

    s.begin_paint().expect("work pending");
    assert_eq!(s.record_failure(), FailureAction::Rearm);
    s.commit_arm();

    pump_successful_frame(&s); // success resets the counter

    mark_and_commit(&s);
    for attempt in 1..=MAX_CONSECUTIVE_FAILURES {
        s.begin_paint().expect("work pending");
        let action = s.record_failure();
        if attempt < MAX_CONSECUTIVE_FAILURES {
            assert_eq!(action, FailureAction::Rearm, "budget restarted");
            s.commit_arm();
        } else {
            assert_eq!(action, FailureAction::Stop);
        }
    }
}

#[test]
fn detach_cancel_clears_the_pending_schedule() {
    // F1 state-machine side: detach cancels the armed animation frame, so
    // the machine must not keep claiming a frame is pending.
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);
    assert!(s.is_scheduled());

    s.cancel_schedule();
    assert!(!s.is_scheduled());
    // A later mark (e.g. after re-attach against a state something still
    // holds) must know it has to arm again.
    assert!(s.mark_dirty());
}

#[test]
fn idle_pump_never_silently_loses_a_dirty_write() {
    // Regression test for the original frozen-viewer failure mode: content
    // newer than the last successful paint with no frame scheduled and no
    // retry path. Asserts the invariant holds across two full
    // idle -> mark -> paint -> idle cycles (the funnel is reusable, not a
    // one-shot).
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);
    pump_successful_frame(&s);
    assert!(!s.is_dirty() && !s.is_scheduled());

    assert!(s.mark_dirty());
    s.commit_arm();
    assert!(s.is_dirty() && s.is_scheduled());
    pump_successful_frame(&s);
    assert!(!s.is_dirty() && !s.is_scheduled());

    assert!(s.mark_dirty());
    s.commit_arm();
    assert!(s.is_dirty() && s.is_scheduled());
}

#[test]
fn generation_counters_wrap_safely_past_u64_max() {
    // F2 regression: `mark_dirty` used an unchecked `+ 1` and every
    // needs-paint comparison used `>`. Once `content_gen` wraps past
    // `u64::MAX` back to a small value, `>` aliases — a chronologically
    // newer generation can compare numerically *less than* `painted_gen`,
    // which would read as "clean" even though the content was never
    // painted. `wrapping_add` + `!=` fixes both halves. Start right at the
    // wrap boundary and drive several mark/paint cycles across it.
    let s = DirtyState::new_dirty_with_gens(u64::MAX - 1, u64::MAX - 2);
    assert!(s.is_dirty(), "content_gen(MAX-1) != painted_gen(MAX-2)");

    // Paint the pre-wrap generation.
    let gen = s.begin_paint().expect("pre-wrap generation is dirty");
    assert_eq!(gen, u64::MAX - 1);
    assert!(!s.finish_paint_success(gen));
    assert!(!s.is_dirty());

    // One more mark before the wrap: content_gen becomes u64::MAX.
    assert!(s.mark_dirty());
    s.commit_arm();
    let gen = s.begin_paint().expect("MAX is newer than MAX-1");
    assert_eq!(gen, u64::MAX);
    assert!(!s.finish_paint_success(gen));
    assert!(!s.is_dirty());

    // The next mark wraps: content_gen goes u64::MAX -> 0. Under the old
    // `>` comparison this would read as NOT dirty (0 > u64::MAX is
    // false), silently losing the update. `!=` still reports it correctly.
    assert!(
        s.mark_dirty(),
        "wrapped generation must still request an arm"
    );
    s.commit_arm();
    let gen = s
        .begin_paint()
        .expect("wrapped generation 0 differs from MAX");
    assert_eq!(gen, 0);
    assert!(!s.finish_paint_success(gen));
    assert!(!s.is_dirty());

    // Continue past the wrap to confirm ordinary operation resumes.
    assert!(s.mark_dirty());
    s.commit_arm();
    let gen = s.begin_paint().expect("post-wrap content is dirty");
    assert_eq!(gen, 1);
    assert!(!s.finish_paint_success(gen));
    assert!(!s.is_dirty());
}

#[test]
fn old_generation_paint_success_does_not_reopen_a_newer_tripped_breaker() {
    // Round-3 regression: a re-entrant mark_dirty() during an in-flight
    // paint bumps content_gen and resets the breaker for the NEW
    // generation; if that new generation's own arm attempts then trip the
    // breaker, the OLD (already snapshotted) paint can still succeed
    // afterwards. Its finish_paint_success must NOT reset the breaker
    // (consecutive_failures / stopped) that now belongs to the newer,
    // still-unpainted generation, and must NOT ask the caller to re-arm —
    // doing either would hand the newer generation a second failure
    // budget and risk a second breaker-trip console.error.
    let s = DirtyState::new_dirty();
    mark_and_commit(&s);

    // The pump snapshots the current (old) generation and starts painting.
    let old_gen = s.begin_paint().expect("old generation has work");

    // A push/mark lands re-entrantly while that paint is still running:
    // new content, new generation. mark_dirty resets the breaker for it.
    assert!(
        s.mark_dirty(),
        "schedule was consumed by begin_paint, so arming is needed"
    );
    assert!(!s.is_stopped(), "a fresh mark always clears the breaker");

    // The new generation's own arm attempts (requestAnimationFrame
    // rejections, simulated the same way arm_failures_share_the_same_
    // breaker_as_paint_failures does — no begin_paint/finish_paint pair,
    // exactly like render.rs::arm_pump's retry loop) fail repeatedly and
    // trip the breaker for THIS (newer) generation.
    for attempt in 1..MAX_CONSECUTIVE_FAILURES {
        assert_eq!(
            s.record_failure(),
            FailureAction::Rearm,
            "attempt {attempt}"
        );
    }
    assert_eq!(s.record_failure(), FailureAction::Stop);
    assert!(
        s.is_stopped(),
        "the newer generation's arm breaker is tripped"
    );
    assert!(s.is_dirty(), "the newer generation is still unpainted");

    // Now the OLD generation's paint (begun before the re-entrant mark)
    // finishes successfully.
    let needs_rearm = s.finish_paint_success(old_gen);

    assert!(
        !needs_rearm,
        "an old generation's success must not ask to re-arm a breaker \
         tripped by a newer generation"
    );
    assert!(
        s.is_stopped(),
        "the newer generation's tripped breaker must survive an older \
         generation's paint success"
    );
    assert!(
        s.is_dirty(),
        "the newer, still-unpainted generation must remain dirty"
    );
    assert_eq!(
        s.begin_paint(),
        None,
        "breaker still tripped: no work handed out, no second budget"
    );

    // Only a fresh mark (new content) resets and recovers the machine.
    assert!(s.mark_dirty(), "fresh mark must ask to arm again");
    assert!(!s.is_stopped(), "new content resets the breaker");
    s.commit_arm();
    pump_successful_frame(&s);
    assert!(!s.is_dirty());
}

#[test]
fn is_dirty_does_not_alias_across_the_wrap_boundary() {
    // Narrower regression than the cycle test above: directly exhibits the
    // exact numeric aliasing a `>` comparison would suffer. `content_gen`
    // wrapped to 0 is chronologically newer than `painted_gen` sitting at
    // `u64::MAX`, so `is_dirty()` must report `true` — `0 > u64::MAX` is
    // `false`, which is precisely the bug `!=` fixes.
    let s = DirtyState::new_dirty_with_gens(0, u64::MAX);
    assert!(
        s.is_dirty(),
        "wrapped content_gen(0) must not alias with painted_gen(MAX)"
    );
}
