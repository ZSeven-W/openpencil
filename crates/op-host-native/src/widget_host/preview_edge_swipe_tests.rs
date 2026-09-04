//! Track C-4 edge-swipe-to-pop tests.

#![cfg(test)]

use super::WidgetHostNative;
use jian_core::action::services::Router;
use op_editor_core::EditorState;
use std::sync::{LazyLock, Mutex, MutexGuard};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

const TWO_SCREEN_PHONE_DOC_JSON: &str = r##"{
    "version": "1.1",
    "formatVersion": "1.1",
    "id": "x",
    "app": { "name": "x", "version": "1", "id": "x" },
    "pages": [
        { "id": "canvas", "name": "Canvas", "children": [
            { "type": "frame", "id": "home", "name": "Home", "screen": "/",
              "x": 0, "y": 0, "width": 390, "height": 844, "children": [] },
            { "type": "frame", "id": "detail", "name": "Detail", "screen": "/detail",
              "x": 500, "y": 0, "width": 390, "height": 844, "children": [] }
        ] }
    ]
}"##;

const VIEWPORT: (f32, f32) = (1200.0, 800.0);

fn app_mode_host() -> WidgetHostNative {
    let doc: jian_ops_schema::PenDocument =
        serde_json::from_str(TWO_SCREEN_PHONE_DOC_JSON).expect("valid doc");
    let mut host = WidgetHostNative::new();
    host.install_imported_state(EditorState::from_document(doc));
    assert!(host.enter_preview(VIEWPORT));
    assert!(host.preview.as_ref().unwrap().is_app_mode());
    host.recompute_device_frame(VIEWPORT.0, VIEWPORT.1);
    assert!(
        host.preview_device_frame.is_some(),
        "a 390-wide root must infer Phone + compute a device frame"
    );
    host
}

/// Push to "/detail" and settle the reconcile, so `can_pop()` is true —
/// the edge-swipe gate every positive-case test needs armed.
fn push_to_detail(host: &mut WidgetHostNative) {
    host.preview
        .as_mut()
        .unwrap()
        .router_for_test()
        .push("/detail");
    assert!(host.preview.as_mut().unwrap().reconcile(1_000).switched);
    host.on_preview_screen_switched(VIEWPORT.0, VIEWPORT.1);
    assert!(host.preview.as_ref().unwrap().can_pop());
}

fn left_edge_x(host: &WidgetHostNative) -> f32 {
    host.preview_device_frame.as_ref().unwrap().content_span_x.0
}

#[test]
fn press_within_edge_zone_arms_only_when_there_is_somewhere_to_pop_to() {
    let _guard = test_lock();
    let mut host = app_mode_host();
    let edge = left_edge_x(&host);

    // At the entry screen ("/"), there's nothing to pop to — must not arm
    // even though the press lands exactly in the edge zone.
    host.preview_dispatch_press(edge + 5.0, 400.0, VIEWPORT.0, VIEWPORT.1);
    assert!(
        host.preview_edge_swipe_start_x.is_none(),
        "entry screen has no back target — must never arm"
    );
    host.preview_dispatch_release();

    push_to_detail(&mut host);
    let edge = left_edge_x(&host);
    host.preview_dispatch_press(edge + 5.0, 400.0, VIEWPORT.0, VIEWPORT.1);
    assert!(
        host.preview_edge_swipe_start_x.is_some(),
        "press inside the edge zone with somewhere to pop to must arm"
    );
}

#[test]
fn press_outside_edge_zone_never_arms() {
    let _guard = test_lock();
    let mut host = app_mode_host();
    push_to_detail(&mut host);
    let edge = left_edge_x(&host);

    host.preview_dispatch_press(edge + 100.0, 400.0, VIEWPORT.0, VIEWPORT.1);
    assert!(
        host.preview_edge_swipe_start_x.is_none(),
        "a press well inside the content must not arm the edge-swipe candidate"
    );
}

#[test]
fn drag_below_threshold_does_not_pop() {
    let _guard = test_lock();
    let mut host = app_mode_host();
    push_to_detail(&mut host);
    let edge = left_edge_x(&host);

    host.preview_dispatch_press(edge + 5.0, 400.0, VIEWPORT.0, VIEWPORT.1);
    assert!(host.preview_edge_swipe_start_x.is_some());
    // 40px < the 60px threshold.
    host.preview_dispatch_move(edge + 45.0, 400.0);
    assert!(
        host.preview_edge_swipe_start_x.is_some(),
        "a drag under the threshold must stay armed, not fire"
    );
    assert_eq!(
        host.preview.as_ref().unwrap().current_path_for_test(),
        "/detail"
    );
}

#[test]
fn drag_past_threshold_fires_pop_cancels_the_gesture_and_disarms() {
    let _guard = test_lock();
    let mut host = app_mode_host();
    push_to_detail(&mut host);
    let edge = left_edge_x(&host);

    host.preview_dispatch_press(edge + 5.0, 400.0, VIEWPORT.0, VIEWPORT.1);
    assert!(host.preview_any_pointer_held_for_test());
    // 65px > the 60px threshold.
    let handled = host.preview_dispatch_move(edge + 70.0, 400.0);
    assert!(handled, "the firing move is itself consumed");
    assert!(
        host.preview_edge_swipe_start_x.is_none(),
        "a fired candidate must disarm — at most once per gesture"
    );
    assert!(
        !host.preview_any_pointer_held_for_test(),
        "the underlying pointer gesture is cancelled, not left held"
    );

    // `pop_screen` only mutates the router; the mounted screen swaps on
    // the next reconcile — same as every other navigation trigger.
    assert!(host.preview.as_mut().unwrap().reconcile(2_000).switched);
    assert_eq!(host.preview.as_ref().unwrap().current_path_for_test(), "/");
}

#[test]
fn release_without_crossing_the_threshold_disarms_silently() {
    let _guard = test_lock();
    let mut host = app_mode_host();
    push_to_detail(&mut host);
    let edge = left_edge_x(&host);

    host.preview_dispatch_press(edge + 5.0, 400.0, VIEWPORT.0, VIEWPORT.1);
    assert!(host.preview_edge_swipe_start_x.is_some());
    host.preview_dispatch_release();
    assert!(host.preview_edge_swipe_start_x.is_none());
    assert_eq!(
        host.preview.as_ref().unwrap().current_path_for_test(),
        "/detail"
    );
}
