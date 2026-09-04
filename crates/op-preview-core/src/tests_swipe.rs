//! R2B Swipe through the product preview path.
//!
//! `PreviewSession` is the REAL timestamp path product clients use: the
//! scene→runtime mapping + pointer capture live here, and the runtime
//! `PointerEvent.t_ms` must come from the HOST clock so velocity-sensing
//! recognizers (Swipe) and timer deadlines (LongPress) work. These tests
//! drive a real `onSwipe` ActionList through `dispatch_pointer_phase_at`
//! and through the source-compatible legacy wrapper with `set_now_ms`.

#![cfg(test)]

use super::{test_measure, PreviewSession};
use jian_core::gesture::pointer::PointerPhase;
use jian_core::widget_state::WidgetState;

/// A single screen frame carrying an `onSwipe` ActionList that counts
/// swipes and records the judged direction. Child rectangle is the hit
/// target; the frame is the nearest enabled owner (bubbling).
fn swipe_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": {
            "swipes": { "type": "int", "default": 0 },
            "dir": { "type": "string", "default": "" }
        },
        "children": [
            { "type": "frame", "id": "screen", "width": 400, "height": 400,
              "events": {
                "onSwipe": [
                    { "set": { "$app.swipes": "$app.swipes + 1" } },
                    { "set": { "$app.dir": "$event.direction" } }
                ]
              },
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

fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

fn enter() -> PreviewSession {
    PreviewSession::enter(
        &swipe_doc(),
        (800.0, 600.0),
        &default_theme(),
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

/// Centre of the entry screen's "go" button (the two-screen app-mode
/// fixture), in scene space.
fn go_button_center(session: &PreviewSession) -> (f32, f32) {
    let (x, y, w, h) = session.node_rect("go").expect("go button laid out");
    (x + w / 2.0, y + h / 2.0)
}

/// Timestamped Down→Move through `dispatch_pointer_phase_at`: the swipe
/// claim needs a real 60px/100ms velocity, which the runtime ONLY sees if
/// the events carry t_ms from the host clock. The ActionList increments
/// state and reads `$event.direction` from the payload.
#[test]
fn timestamped_swipe_through_preview_session_runs_action_list() {
    let mut session = enter();
    let (cx, cy) = center(&session, "btn");

    let down = session.dispatch_pointer_phase_at(cx, cy, PointerPhase::Down, 0);
    assert!(!down, "Down alone emits nothing");
    // 60px over 100ms = 600 px/s on the judged (horizontal) axis.
    let move_handled = session.dispatch_pointer_phase_at(cx + 60.0, cy, PointerPhase::Move, 100);
    assert!(move_handled, "the claiming Move emits the Swipe semantic");

    let swipes = session
        .runtime()
        .state
        .app_get("swipes")
        .expect("swipes seeded from doc state")
        .as_i64();
    assert_eq!(swipes, Some(1), "onSwipe ActionList must run exactly once");
    assert_eq!(
        session
            .runtime()
            .state
            .app_get("dir")
            .and_then(|v| v.as_str().map(str::to_owned)),
        Some("right".to_owned()),
        "payload direction must come from the judged axis"
    );
}

/// Source-compatible legacy wrapper: `dispatch_pointer_phase` reads the
/// session's `last_now_ms`, so a host that pushes its clock across the
/// gesture phases gets the same real timestamps without changing call
/// sites.
#[test]
fn legacy_wrapper_uses_last_pushed_clock_across_phases() {
    let mut session = enter();
    let (cx, cy) = center(&session, "btn");

    session.set_now_ms(0);
    let _ = session.dispatch_pointer_phase(cx, cy, PointerPhase::Down);
    session.set_now_ms(100);
    let handled = session.dispatch_pointer_phase(cx + 60.0, cy, PointerPhase::Move);
    assert!(handled, "clock-pushing host gets a claimable timestamp");
    assert_eq!(
        session
            .runtime()
            .state
            .app_get("swipes")
            .expect("swipes")
            .as_i64(),
        Some(1)
    );

    // A fresh gesture with NO clock push stays at last_now_ms... advance
    // first so this assertion is about the fallback, not stale state:
    // timestamps that never move cannot fabricate a velocity.
    let _ = session.dispatch_pointer_phase(cx + 60.0, cy, PointerPhase::Up);
    session.set_now_ms(10_000);
    let _ = session.dispatch_pointer_phase(cx, cy, PointerPhase::Down);
    let (x2, y2) = center(&session, "btn");
    let _ = session.dispatch_pointer_phase(x2 + 60.0, y2, PointerPhase::Move);
    assert_eq!(
        session
            .runtime()
            .state
            .app_get("swipes")
            .expect("swipes")
            .as_i64(),
        Some(1),
        "a zero-delta timestamp pair must not invent a velocity"
    );
}

/// The exact old-footgun regression: without a pushed clock the legacy
/// wrapper stamps t_ms = 0 on every phase, so a fast-looking drag has no
/// measurable velocity fact and must never claim a Swipe (timestamps are
/// never invented).
#[test]
fn legacy_wrapper_without_clock_never_claims_swipe() {
    let mut session = enter();
    let (cx, cy) = center(&session, "btn");

    let _ = session.dispatch_pointer_phase(cx, cy, PointerPhase::Down);
    let handled = session.dispatch_pointer_phase(cx + 60.0, cy, PointerPhase::Move);
    assert!(
        !handled,
        "t_ms=0 both phases -> no velocity fact -> no Swipe claim"
    );
    assert_eq!(
        session
            .runtime()
            .state
            .app_get("swipes")
            .expect("swipes")
            .as_i64(),
        Some(0)
    );
}

/// Pointer capture survives the timestamped path: the Down anchors the
/// scene→runtime mapping and the Move reuses it (same behaviour as the
/// legacy wrapper, only the timestamp differs).
#[test]
fn timestamped_phases_keep_capture_and_release_anchor() {
    let mut session = enter();
    let (cx, cy) = center(&session, "btn");

    let _ = session.dispatch_pointer_phase_at(cx, cy, PointerPhase::Down, 0);
    let _ = session.dispatch_pointer_phase_at(cx + 60.0, cy, PointerPhase::Move, 100);
    // Up reuses the captured mapping (same as a drag end) — no remap, and
    // no second Swipe (one-shot) nor a stray Tap.
    let up = session.dispatch_pointer_phase_at(cx + 60.0, cy, PointerPhase::Up, 200);
    assert!(!up);
    assert_eq!(
        session
            .runtime()
            .state
            .app_get("swipes")
            .expect("swipes")
            .as_i64(),
        Some(1)
    );
    // A Cancel on an idle session is inert (no anchor to release).
    assert!(!session.dispatch_pointer_phase_at(cx, cy, PointerPhase::Cancel, 300));
}

/// Reviewed P1 regression: the explicit event clock must reach the
/// session/runtime BEFORE the transition gate runs. The host's last
/// `set_now_ms` push (1_100) is INSIDE the 240 ms screen slide started
/// at 1_000 by the push onto `/detail`, while the event itself carries
/// t_ms 1_500 — past the end (1_240). A gate that reads only the stale
/// `last_now_ms` discards the tap outright; the fix syncs the clock
/// from `t_ms` first, so the pointer dispatches and the /detail switch
/// toggles.
#[test]
fn explicit_event_time_past_transition_end_is_not_discarded() {
    let doc: jian_ops_schema::PenDocument =
        serde_json::from_str(super::tests_app_mode::TWO_SCREEN_DOC_JSON).expect("two-screen doc");
    let mut session = PreviewSession::enter(
        &doc,
        (1200.0, 800.0),
        &default_theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter app-mode preview");

    // Push onto /detail: reconcile starts the 240 ms push at 1_000.
    let (bx, by) = go_button_center(&session);
    session.dispatch_tap(bx, by);
    assert!(session.reconcile(1_000).switched);
    assert!(
        session.transition_active_for_test(1_100),
        "sanity: 1_100 sits inside the push window"
    );

    // The host clock push is mid-transition...
    session.set_now_ms(1_100);
    assert!(
        session.transition_active(),
        "sanity: stale clock is mid-transition"
    );

    // ...while the event itself is past the end. It must dispatch: the
    // session clock advances to the EVENT time before the gate runs.
    let (x, y, w, h) = session
        .node_rect("sw-detail")
        .expect("detail switch laid out");
    let (sx, sy) = (x + w / 2.0, y + h / 2.0);
    let _ = session.dispatch_pointer_phase_at(sx, sy, PointerPhase::Down, 1_500);
    assert_eq!(
        session.last_now_ms, 1_500,
        "the session clock must sync forward to the event time"
    );
    assert_eq!(
        session.runtime().last_now_ms(),
        1_500,
        "the runtime clock must advance with the event"
    );
    assert!(
        !session.transition_active(),
        "the event time is past the window"
    );
    let _ = session.dispatch_pointer_phase_at(sx, sy, PointerPhase::Up, 1_600);
    match session.runtime().widget_states.get("sw-detail") {
        Some(WidgetState::Toggle { on }) => assert!(
            *on,
            "the tap past the transition end must reach the /detail switch"
        ),
        other => panic!("expected Toggle state for sw-detail, got {other:?}"),
    }
}
