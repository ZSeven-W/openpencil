//! Reviewed P1 regression: the mobile editor pointer clock.
//!
//! `op_editor_press_at` / `op_editor_move_at` / `op_editor_release_at` /
//! `op_editor_cancel_gesture_at` are the dedicated production entry points
//! the iOS / Android / Harmony shells call with each event's factual
//! monotonic timestamp. Two facts must hold end to end:
//!
//! 1. The GLOBAL clocks (Session `now_ms`, `WidgetHostNative::now_ms`, and
//!    through it the live `PreviewSession` + jian runtime) advance
//!    monotonically — `max(current, candidate)` — from the event time
//!    BEFORE every early return (safe-area miss, pointer-capture miss,
//!    collaboration suppression, Cancel).
//! 2. The event's FACTUAL timestamp travels independently into the
//!    runtime: a frame pump at 2000 followed by Down(t=950) + Move(t=1050)
//!    keeps every global clock at 2000 while the Swipe recognizer measures
//!    the 100 ms delta and the `onSwipe` ActionList runs exactly once.
//!
//! So the global clock restore push (`host.set_now_ms`) is intentionally
//! NOT the carrier of the raw event timestamp — the scoped host
//! `apply_press_at` / `apply_cursor_move_at` context carries it.
//!
//! These tests drive the product input path end to end: an editor session
//! in Preview mode (device-frame presentation) receives Down/Move/Up with
//! NO intervening `op_frame`. The host clock is read back through
//! [`op_host_native::WidgetHostNative::next_animation_deadline_ms`] — its
//! single public clock readout, which preview mode pins to `now_ms + 33 ms`
//! (the fixture has no focused text input / tooltip / toast, so nothing else
//! can supply an earlier deadline).
//!
//! The swipe geometry itself (60 px in 100 ms = 600 px/s on the judged
//! axis) is exactly the shape the host-level preview swipe suite proves
//! reaches an `onSwipe` ActionList via the same
//! `PreviewSession::dispatch_pointer_phase_at` path; what is asserted here
//! is the FFI→host half of that chain plus the global-clock guarantee.

#![cfg(all(test, not(target_os = "windows")))]

use crate::desc::{Callbacks, CreateOptions};
use crate::lifecycle::{OpEngine, Session};
use crate::{op_pointer, OpStatus};
use op_editor_core::PreviewDeviceKind;
use op_editor_ui::widgets::host_canvas_geometry::canvas_region;
use op_editor_ui::{Point2D, Rect};
use op_host_native::preview::device_frame::compute_frame_geometry;

const VIEWPORT: (f32, f32) = (800.0, 600.0);

/// A 400 px screen frame owning `onSwipe` (increments `$app.swipes` and
/// records the judged direction) with a child rectangle as the hit target
/// — the same fixture shape the host-level preview swipe tests use; the
/// 400 px root infers Phone presentation.
const SWIPE_DOC_JSON: &str = r##"{
    "version": "1.1",
    "formatVersion": "1.1",
    "id": "x",
    "app": { "name": "x", "version": "1", "id": "x" },
    "state": {
        "swipes": { "type": "int", "default": 0 },
        "dir": { "type": "string", "default": "" }
    },
    "children": [
        { "type": "frame", "id": "screen", "x": 0, "y": 0, "width": 400, "height": 400,
          "events": { "onSwipe": [
              { "set": { "$app.swipes": "$app.swipes + 1" } },
              { "set": { "$app.dir": "$event.direction" } }
          ] },
          "children": [
              { "type": "rectangle", "id": "btn", "x": 10, "y": 10,
                "width": 100, "height": 100 }
          ] }
    ]
}"##;

/// Editor-mode engine holding a live preview of the swipe fixture. No
/// frame is ever pumped: the pointer tests must prove the event itself
/// carries the clock.
fn swipe_engine() -> OpEngine {
    let mut engine = OpEngine::new(
        Session::new(CreateOptions {
            document: SWIPE_DOC_JSON.to_owned(),
            width: VIEWPORT.0,
            height: VIEWPORT.1,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
            documents_root: None,
        })
        .expect("editor session"),
    );
    let session = engine.session_mut_for_test();
    let viewport = session.editor_viewport();
    let host = session.editor_mut().expect("editor host");
    assert!(
        host.enter_preview(viewport),
        "preview starts with the swipe fixture"
    );
    // The enter-time device frame is computed against the host's cached
    // viewport (zero on a fresh host) — same as the host-level preview
    // tests, recompute against the real viewport.
    host.preview_resize(viewport.0, viewport.1);
    engine
}

/// The host's single public clock readout. In preview mode the deadline
/// list pins to `now_ms + 33 ms`.
fn host_deadline_ms(engine: &mut OpEngine) -> Option<u64> {
    engine
        .session_mut_for_test()
        .editor()
        .expect("editor host")
        .next_animation_deadline_ms()
}

/// The session's global clock (the value every frame pump / background
/// tick / pointer event advances monotonically).
fn session_now_ms(engine: &mut OpEngine) -> u64 {
    engine.session_mut_for_test().now_ms
}

/// Screen-space point for a doc-space point through the device frame's
/// fit/centre transform — the same `compute_frame_geometry` math the
/// host's `recompute_device_frame` runs, solved against the same
/// canvas region, so the swipe travels the real
/// screen → device-frame → doc mapping.
fn screen_at(engine: &mut OpEngine, doc_x: f32, doc_y: f32) -> (f32, f32) {
    let session = engine.session_mut_for_test();
    let host = session.editor().expect("editor host");
    let (cx, cy, cw, ch) = canvas_region(host.editor_state(), VIEWPORT.0, VIEWPORT.1);
    let frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        Rect {
            origin: Point2D::new(cx, cy),
            size: Point2D::new(cw, ch),
        },
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(400.0, 400.0),
        },
        None,
        None,
    );
    (
        frame.content_origin.x + doc_x * frame.fit,
        frame.content_origin.y + doc_y * frame.fit,
    )
}

/// Cloned `$app` snapshot through the `testing` seam — the narrow value
/// copy that replaces any reference into the interior-mutable runtime.
fn app_state_i64(engine: &mut OpEngine, key: &str) -> Option<i64> {
    engine
        .session_mut_for_test()
        .editor()
        .expect("editor host")
        .preview_app_state_value_for_test(key)
        .and_then(|value| value.as_i64())
}

fn app_state_string(engine: &mut OpEngine, key: &str) -> Option<String> {
    engine
        .session_mut_for_test()
        .editor()
        .expect("editor host")
        .preview_app_state_value_for_test(key)
        .and_then(|value| value.as_str().map(str::to_owned))
}

/// Dedicated `_at` route, no intervening frame: Down(t=0) + Move(t=100)
/// through the SAME production entry the clients call. The runtime gets
/// the factual 100 ms delta, the `onSwipe` ActionList runs exactly once
/// with the judged direction, and the global clocks start at the event
/// times (fresh host clock is 0).
#[test]
fn dedicated_time_stamped_pointer_entries_swipe_without_an_intervening_frame() {
    let mut engine = swipe_engine();
    let pointer = &mut engine as *mut OpEngine;
    assert_eq!(
        host_deadline_ms(&mut engine),
        Some(33),
        "fresh host clock starts at 0 (preview pins the deadline to now + 33)"
    );

    // Down at t=0 on the frame body below the floating device-switcher
    // pill (the swipe start must not land on the pill, which owns any
    // press inside it before the preview tier sees it), through the
    // real screen → device-frame → doc mapping the shell uses.
    let (px, py) = screen_at(&mut engine, 60.0, 250.0);
    assert_eq!(
        unsafe { crate::op_editor_press_at(pointer, px, py, 0) },
        OpStatus::Ok
    );
    assert_eq!(session_now_ms(&mut engine), 0);
    assert_eq!(app_state_i64(&mut engine, "swipes"), Some(0));

    // The swipe-claim Move at t=100: 60 px over 100 ms = 600 px/s on
    // the judged axis. No op_frame between Down and Move: the clock
    // must come from the event itself.
    let (mx, my) = screen_at(&mut engine, 120.0, 250.0);
    assert_eq!(
        unsafe { crate::op_editor_move_at(pointer, mx, my, 100) },
        OpStatus::Ok
    );
    assert_eq!(
        host_deadline_ms(&mut engine),
        Some(133),
        "the host clock must jump to the event timestamp, not stay at the last frame pump"
    );
    assert_eq!(session_now_ms(&mut engine), 100);
    assert_eq!(
        app_state_i64(&mut engine, "swipes"),
        Some(1),
        "onSwipe must run exactly once from the factual 100 ms delta"
    );
    assert_eq!(
        app_state_string(&mut engine, "dir"),
        Some("right".to_owned()),
        "the judged direction must come from the gesture payload"
    );

    // Release endpoint at t=200: one-shot swipe does not repeat.
    assert_eq!(
        unsafe { crate::op_editor_release_at(pointer, mx, my, 200) },
        OpStatus::Ok
    );
    assert_eq!(app_state_i64(&mut engine, "swipes"), Some(1));
}

/// The critical global-clock regression: pump/set the global time to
/// 2000, then deliver a dedicated Down at 950 and Move at 1050. Every
/// global clock (Session + WidgetHost + live preview runtime) must stay
/// at 2000 — the clock is NOT overwritten with the raw event timestamp —
/// while the Swipe recognizer measures the factual 100 ms delta and
/// `onSwipe` runs exactly once.
#[test]
fn out_of_order_dedicated_events_keep_global_clocks_and_swipe_uses_factual_delta() {
    let mut engine = swipe_engine();
    let pointer = &mut engine as *mut OpEngine;

    // Pump the global clock to 2000 the way a frame pump / background
    // tick feeds it (no GPU surface is attached in this suite, so the
    // render-free clock feed stands in for op_frame — it advances the
    // exact same Session + WidgetHost global clock).
    let mut active = true;
    assert_eq!(
        unsafe { crate::op_background_tick(pointer, 2000, &mut active) },
        OpStatus::Ok
    );
    assert_eq!(session_now_ms(&mut engine), 2000);
    assert_eq!(host_deadline_ms(&mut engine), Some(2033));

    // The gesture's own timestamps sit BEHIND the pumped clock. The
    // swipe start stays below the floating device-switcher pill so the
    // press reaches the preview tier (the pill owns presses over it).
    let (px, py) = screen_at(&mut engine, 60.0, 250.0);
    assert_eq!(
        unsafe { crate::op_editor_press_at(pointer, px, py, 950) },
        OpStatus::Ok
    );
    let (mx, my) = screen_at(&mut engine, 120.0, 250.0);
    assert_eq!(
        unsafe { crate::op_editor_move_at(pointer, mx, my, 1050) },
        OpStatus::Ok
    );

    // Global clocks stay at the pumped 2000; the raw event timestamp
    // must NOT have been pushed into WidgetHostNative::now_ms.
    assert_eq!(
        session_now_ms(&mut engine),
        2000,
        "the session clock must not regress to an out-of-order event"
    );
    assert_eq!(
        host_deadline_ms(&mut engine),
        Some(2033),
        "the host global clock must stay at the frame pump time (2000 + 33)"
    );
    // Swipe measured the factual 100 ms pair delta (950 → 1050) even
    // though the global clock reads 2000.
    assert_eq!(
        app_state_i64(&mut engine, "swipes"),
        Some(1),
        "onSwipe must run exactly once from the factual 100 ms delta despite the ahead clock"
    );

    // The release endpoint carries the same factual discipline.
    assert_eq!(
        unsafe { crate::op_editor_release_at(pointer, mx, my, 1100) },
        OpStatus::Ok
    );
    assert_eq!(session_now_ms(&mut engine), 2000);
    assert_eq!(app_state_i64(&mut engine, "swipes"), Some(1));
}

/// Cancel and every early return must advance the global clocks first
/// (monotonically) and never move them backward.
#[test]
fn cancel_and_early_returns_advance_global_clocks_monotonically() {
    let mut engine = swipe_engine();
    let pointer = &mut engine as *mut OpEngine;

    // Cancel at 300 jumps the clocks to 300 (start 0).
    assert_eq!(
        unsafe { crate::op_editor_cancel_gesture_at(pointer, 300) },
        OpStatus::Ok
    );
    assert_eq!(session_now_ms(&mut engine), 300);
    assert_eq!(host_deadline_ms(&mut engine), Some(333));

    // An out-of-order Cancel (behind the current clock) leaves it alone.
    assert_eq!(
        unsafe { crate::op_editor_cancel_gesture_at(pointer, 100) },
        OpStatus::Ok
    );
    assert_eq!(session_now_ms(&mut engine), 300);
    assert_eq!(host_deadline_ms(&mut engine), Some(333));

    // Safe-area miss: the early return must still have advanced the
    // global clock to the event's own time first (400).
    assert_eq!(
        unsafe { crate::op_editor_press_at(pointer, -1.0, -1.0, 400) },
        OpStatus::Ok
    );
    assert_eq!(session_now_ms(&mut engine), 400);
    assert_eq!(host_deadline_ms(&mut engine), Some(433));

    // Pointer-capture miss (press lands in the top safe-area band): the
    // capture gate returns before any host routing, but the clock has
    // already advanced.
    assert_eq!(
        unsafe { crate::op_editor_press_at(pointer, -10.0, -10.0, 500) },
        OpStatus::Ok
    );
    assert_eq!(session_now_ms(&mut engine), 500);

    // Collaboration-suppressed stream through the generic editor pointer
    // route: the early return still receives the event clock (600).
    engine.session_mut_for_test().suppress_collab_pointer(7);
    assert_eq!(
        unsafe { op_pointer(pointer, 7, 1, 10.0, 10.0, 600) },
        OpStatus::Ok
    );
    assert_eq!(session_now_ms(&mut engine), 600);
    assert_eq!(host_deadline_ms(&mut engine), Some(633));

    // Cancel through the generic route carries its factual time too.
    assert_eq!(
        unsafe { op_pointer(pointer, 2, 3, 10.0, 10.0, 700) },
        OpStatus::Ok
    );
    assert_eq!(session_now_ms(&mut engine), 700);
    assert_eq!(host_deadline_ms(&mut engine), Some(733));
}
