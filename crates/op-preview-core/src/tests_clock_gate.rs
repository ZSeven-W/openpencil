//! R2B1 clock-gate regressions: out-of-order explicit event timestamps.
//!
//! `dispatch_pointer_phase_at` synchronizes the session clock and the
//! jian runtime clock MONOTONICALLY from the event's own `t_ms` before
//! the transition gate and the dispatch run. Two guarantees hold for an
//! out-of-order event (`t_ms` behind the current clock):
//! 1. neither clock moves backward, and
//! 2. the runtime `PointerEvent.t_ms` stays the factual caller
//!    timestamp — a velocity-sensing recognizer (Swipe) is judged by the
//!    event pair's own delta, never by a re-stamped (stale-ahead) clock.

#![cfg(test)]

use super::{test_measure, PreviewSession};
use jian_core::gesture::pointer::PointerPhase;

/// Self-contained swipe fixture (the same shape as the host-level
/// suite): the frame owns `onSwipe` incrementing `$app.swipes`; the
/// child rectangle is the hit target.
fn swipe_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": { "swipes": { "type": "int", "default": 0 } },
        "children": [
            { "type": "frame", "id": "screen", "width": 400, "height": 400,
              "events": { "onSwipe": [ { "set": { "$app.swipes": "$app.swipes + 1" } } ] },
              "children": [
                  { "type": "rectangle", "id": "btn", "x": 10, "y": 10,
                    "width": 100, "height": 100 }
              ] }
        ]
    }"##;
    jian_ops_schema::load_str(src)
        .expect("parse swipe doc")
        .value
}

fn enter() -> PreviewSession {
    PreviewSession::enter(
        &swipe_doc(),
        (800.0, 600.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter preview")
}

/// Scene == runtime space (root authored at the origin), so the runtime
/// rect doubles as the scene-space dispatch point.
fn center(session: &PreviewSession, id: &str) -> (f32, f32) {
    let (x, y, w, h) = session.node_rect(id).expect("runtime rect");
    (x + w / 2.0, y + h / 2.0)
}

/// The complement of the transition-gate regression: an out-of-order
/// event (`t_ms` behind the current session clock) must neither move the
/// clocks backward nor lose its factual timestamp. The Swipe is judged
/// off the pair's own 100ms delta (950 → 1_050) — if the Down were
/// re-stamped with the synced 2_000 clock the measured delta would
/// collapse to 0 and nothing could claim.
#[test]
fn out_of_order_explicit_timestamp_never_moves_clocks_backward() {
    let mut session = enter();
    let (cx, cy) = center(&session, "btn");

    // The host clock is AHEAD of the gesture's own event timestamps
    // (events dispatched between frame pumps, host push in flight).
    session.set_now_ms(2_000);
    assert_eq!(session.runtime().last_now_ms(), 2_000);

    // Down/Move carry their own 100ms delta: 60px over 100ms = 600 px/s.
    let _ = session.dispatch_pointer_phase_at(cx, cy, PointerPhase::Down, 950);
    let handled = session.dispatch_pointer_phase_at(cx + 60.0, cy, PointerPhase::Move, 1_050);
    assert!(
        handled,
        "the swipe must be judged off the factual event timestamps, not a re-stamped clock"
    );

    // Neither clock moved backward while the out-of-order events ran.
    assert_eq!(session.runtime().last_now_ms(), 2_000);
    assert_eq!(session.last_now_ms, 2_000);
    assert_eq!(
        session
            .runtime()
            .state
            .app_get("swipes")
            .expect("swipes seeded from doc state")
            .as_i64(),
        Some(1),
        "onSwipe must run exactly once from the factual-delta claim"
    );

    // A NEWER event still advances both clocks forward (monotonic).
    let _ = session.dispatch_pointer_phase_at(cx, cy, PointerPhase::Cancel, 3_000);
    assert_eq!(session.runtime().last_now_ms(), 3_000);
    assert_eq!(session.last_now_ms, 3_000);
}
