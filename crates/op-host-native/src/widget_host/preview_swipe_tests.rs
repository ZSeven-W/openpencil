//! R2B: the native host's live-preview pointer path must forward the
//! HOST CLOCK into the runtime event timestamps.
//!
//! `preview_dispatch_press` / `preview_dispatch_move` /
//! `preview_dispatch_release` (and the edge-swipe Cancel) must stamp
//! `PointerEvent.t_ms` with `WidgetHostNative::now_ms` via
//! `PreviewSession::dispatch_pointer_phase_at`, or velocity-sensing
//! gestures (Swipe) can never claim through the product client.
//!
//! The timestamped entry variants (`apply_press_at` /
//! `apply_cursor_move_at` / `apply_release_with_viewport_at`) carry a
//! FACTUAL event timestamp through the same ladder while the global
//! clock advances independently (monotonically): the out-of-order
//! regression below proves a frame-pumped clock at 2000 with Down 950 /
//! Move 1050 keeps the global clock at 2000 yet still measures the
//! factual 100 ms swipe delta.
//!
//! Every assertion reads the live runtime's state through the NARROW
//! `testing` seam — a cloned `$app` value — never a reference into the
//! interior-mutable runtime state.

#![cfg(all(test, not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::EditorState;
use std::sync::{LazyLock, Mutex, MutexGuard};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

const VIEWPORT: (f32, f32) = (1200.0, 800.0);

/// A 400px-wide screen frame owning `onSwipe` (increments `$app.swipes`),
/// with a child rectangle as the hit target. The 400px root infers Phone
/// presentation, so the host routes preview input through the device
/// frame — the same fixture shape the edge-swipe tests already use.
const SWIPE_DOC_JSON: &str = r##"{
    "version": "1.1",
    "formatVersion": "1.1",
    "id": "x",
    "app": { "name": "x", "version": "1", "id": "x" },
    "state": { "swipes": { "type": "int", "default": 0 } },
    "children": [
        { "type": "frame", "id": "screen", "x": 0, "y": 0, "width": 400, "height": 400,
          "events": { "onSwipe": [ { "set": { "$app.swipes": "$app.swipes + 1" } } ] },
          "children": [
              { "type": "rectangle", "id": "btn", "x": 10, "y": 10,
                "width": 100, "height": 100 }
          ] }
    ]
}"##;

fn swipe_host() -> WidgetHostNative {
    let doc: jian_ops_schema::PenDocument =
        serde_json::from_str(SWIPE_DOC_JSON).expect("valid swipe doc");
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(doc));
    assert!(
        host.enter_preview(VIEWPORT),
        "preview must start with the swipe fixture"
    );
    // Compile the device-frame geometry against the real viewport (the
    // runner does this per frame; the edge-swipe tests do it explicitly).
    host.recompute_device_frame(VIEWPORT.0, VIEWPORT.1);
    assert!(
        host.preview_device_frame.is_some(),
        "a 400px root must infer Phone + compute a device frame"
    );
    host
}

/// Screen-space point for a scene/doc-space point through the device
/// frame's scrolled-content transform (scroll 0, no pinned strips).
fn screen_at(host: &WidgetHostNative, doc_x: f32, doc_y: f32) -> (f32, f32) {
    let frame = host.preview_device_frame.as_ref().expect("device frame");
    (
        frame.content_origin.x + doc_x * frame.fit,
        frame.content_origin.y + doc_y * frame.fit,
    )
}

/// Cloned `$app` snapshot through the `testing` seam — the narrow value
/// copy that replaces any reference into the interior-mutable runtime.
fn swipes(host: &WidgetHostNative) -> Option<i64> {
    host.preview
        .as_ref()
        .expect("preview session")
        .app_state_value_for_test("swipes")
        .and_then(|value| value.as_i64())
}

#[test]
fn preview_pointer_path_forwards_host_clock_to_swipe() {
    let _guard = test_lock();
    let mut host = swipe_host();
    host.set_now_ms(0);

    // Press at doc (60,60) — the child's centre — as the host event loop
    // does: the runtime gets Down(t_ms=0).
    let (px, py) = screen_at(&host, 60.0, 60.0);
    let _ = host.preview_dispatch_press(px, py, VIEWPORT.0, VIEWPORT.1);
    assert_eq!(swipes(&host), Some(0), "Down alone must not swipe");

    // Advance the host clock, then move 60px right: 600 px/s on the
    // judged axis — a claim only possible with the timestamped path.
    host.set_now_ms(100);
    let (mx, my) = screen_at(&host, 120.0, 60.0);
    let _ = host.preview_dispatch_move(mx, my);
    assert_eq!(
        swipes(&host),
        Some(1),
        "the host clock must reach the Swipe recognizer via t_ms"
    );

    // Release completes the gesture; the one-shot Swipe does not repeat.
    host.set_now_ms(200);
    let _ = host.preview_dispatch_release();
    assert_eq!(swipes(&host), Some(1));
}

#[test]
fn preview_pointer_path_without_clock_advance_never_swipes() {
    let _guard = test_lock();
    let mut host = swipe_host();
    // The host clock stays at 0 for the whole gesture: both phases stamp
    // t_ms = 0, so there is no measurable velocity fact and the Swipe
    // recognizer must never claim (timestamps are never invented).
    let (px, py) = screen_at(&host, 60.0, 60.0);
    let _ = host.preview_dispatch_press(px, py, VIEWPORT.0, VIEWPORT.1);
    let (mx, my) = screen_at(&host, 120.0, 60.0);
    let _ = host.preview_dispatch_move(mx, my);
    assert_eq!(
        swipes(&host),
        Some(0),
        "zero-delta timestamps must not fabricate a velocity"
    );
    let _ = host.preview_dispatch_release();
    assert_eq!(swipes(&host), Some(0));
}

/// Critical regression: the timestamped host entries carry the FACTUAL
/// event time while the global clock advances independently. Pump the
/// global clock to 2000 (the frame pump's value), then deliver a press at
/// 950 and a move at 1050 through the same timestamped entry variants the
/// FFI editor `_at` functions use. The global clock stays 2000, but the
/// Swipe recognizer measures the factual 100 ms pair delta and `onSwipe`
/// runs exactly once — a clock-overwrite design (pushing the raw event
/// time into `now_ms`) would collapse the delta to 0 and never claim.
#[test]
fn out_of_order_scoped_event_times_keep_global_clock_but_swipe_measures_factual_delta() {
    let _guard = test_lock();
    let mut host = swipe_host();
    host.last_viewport_w = VIEWPORT.0;
    host.last_viewport_h = VIEWPORT.1;
    host.set_now_ms(2000);

    let (px, py) = screen_at(&host, 60.0, 60.0);
    let _ = host.apply_press_at(px, py, VIEWPORT.0, VIEWPORT.1, 950);
    let (mx, my) = screen_at(&host, 120.0, 60.0);
    let _ = host.apply_cursor_move_at(mx, my, 1050);

    // Global clocks stayed at the pumped 2000 — the raw event time must
    // not have been written into `now_ms` / the preview session clock.
    assert_eq!(
        host.now_ms, 2000,
        "the host global clock must stay at the frame pump time"
    );
    assert_eq!(
        host.preview
            .as_ref()
            .expect("preview session")
            .now_ms_for_test(),
        2000,
        "the live preview session clock must stay at the frame pump time"
    );
    // The swipe measured the factual 100 ms delta (950 → 1050).
    assert_eq!(
        swipes(&host),
        Some(1),
        "onSwipe must run exactly once from the factual 100 ms delta despite the ahead global clock"
    );

    // The release endpoint carries the same factual discipline; a backward
    // clock candidate never regresses the global clock.
    let _ = host.apply_release_with_viewport_at(VIEWPORT.0, VIEWPORT.1, 1100);
    assert_eq!(host.now_ms, 2000);
    assert_eq!(swipes(&host), Some(1));
    assert_eq!(
        host.preview
            .as_ref()
            .expect("preview session")
            .now_ms_for_test(),
        2000
    );
}
