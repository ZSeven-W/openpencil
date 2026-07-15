//! Device-frame host lifecycle, scroll, and capture integration tests.

#![cfg(all(test, not(target_os = "windows")))]

use super::WidgetHostNative;
use op_editor_core::{EditorState, PreviewDeviceKind};
use std::sync::{LazyLock, Mutex, MutexGuard};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

fn load(src: &str) -> jian_ops_schema::PenDocument {
    jian_ops_schema::load_str(src)
        .expect("parse device-frame fixture")
        .value
}

fn phone_doc(height: u32) -> jian_ops_schema::PenDocument {
    load(&format!(
        r##"{{
            "version": "1.0.0",
            "children": [{{
                "type": "frame", "id": "screen", "x": 0, "y": 0,
                "width": 390, "height": {height},
                "fill": [{{"type":"solid","color":"#ffffff"}}],
                "children": []
            }}]
        }}"##
    ))
}

fn narrow_semantic_nav_doc() -> jian_ops_schema::PenDocument {
    load(
        r##"{
            "version": "1.0.0",
            "children": [{
                "type": "frame", "id": "screen", "x": 0, "y": 0,
                "width": 390, "height": 2000,
                "fill": [{"type":"solid","color":"#ffffff"}],
                "children": [{
                    "type": "frame", "id": "nav", "x": 95, "y": 1940,
                    "width": 200, "height": 60,
                    "semantics": { "role": "nav" },
                    "fill": [{"type":"solid","color":"#222222"}],
                    "children": []
                }]
            }]
        }"##,
    )
}

fn host_with_doc(doc: jian_ops_schema::PenDocument) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let imported = EditorState::from_document(doc);
    host.install_imported_state(imported);
    host
}

#[test]
fn enter_preview_infers_and_writes_back_kind() {
    let _guard = test_lock();
    let mut host = host_with_doc(phone_doc(800));
    host.enter_preview((800.0, 600.0));
    assert_eq!(
        host.editor_state.editor_ui.preview_device,
        Some(PreviewDeviceKind::Phone)
    );
    host.exit_preview();
    assert_eq!(host.editor_state.editor_ui.preview_device, None);
}

#[test]
fn manual_pick_wins_until_exit_then_reinfers() {
    let _guard = test_lock();
    let mut host = host_with_doc(phone_doc(800));
    host.enter_preview((800.0, 600.0));
    host.set_preview_device(PreviewDeviceKind::Desktop, 800.0, 600.0);
    assert_eq!(host.infer_device_kind(), PreviewDeviceKind::Desktop);
    host.exit_preview();
    host.enter_preview((800.0, 600.0));
    assert_eq!(
        host.editor_state.editor_ui.preview_device,
        Some(PreviewDeviceKind::Phone),
        "re-enter re-infers"
    );
}

#[test]
fn device_scroll_divides_by_fit_and_clamps() {
    let _guard = test_lock();
    let mut host = host_with_doc(phone_doc(2000));
    host.last_viewport_w = 300.0;
    host.last_viewport_h = 400.0;
    host.enter_preview((300.0, 400.0));
    host.recompute_device_frame(300.0, 400.0);
    let fit = host.preview_device_frame.as_ref().unwrap().fit;
    host.apply_device_scroll(-100.0);
    assert!((host.preview_scroll_y - 100.0 / fit).abs() < 0.5);
    host.apply_device_scroll(1e6);
    assert_eq!(host.preview_scroll_y, 0.0);
}

#[test]
fn dead_zone_press_leaves_no_stale_capture() {
    let _guard = test_lock();
    let mut host = host_with_doc(narrow_semantic_nav_doc());
    host.last_viewport_w = 800.0;
    host.last_viewport_h = 900.0;
    host.enter_preview((800.0, 900.0));
    host.recompute_device_frame(800.0, 900.0);
    let strip = host
        .preview_device_frame
        .as_ref()
        .unwrap()
        .pinned
        .as_ref()
        .unwrap()
        .strip;
    let consumed =
        host.preview_dispatch_press(strip.origin.x + 2.0, strip.origin.y + 10.0, 800.0, 900.0);
    assert!(!consumed, "dead-zone press must not dispatch");
    assert!(host.preview_surface_capture.is_none());
    assert!(!host.preview_dispatch_release());
    assert!(host.preview_surface_capture.is_none());
}

#[test]
fn screen_switch_resets_scroll_and_reinfers() {
    let _guard = test_lock();
    let mut host = host_with_doc(phone_doc(2000));
    host.last_viewport_w = 800.0;
    host.last_viewport_h = 900.0;
    host.enter_preview((800.0, 900.0));
    host.recompute_device_frame(800.0, 900.0);
    host.preview_scroll_y = 250.0;
    host.on_preview_screen_switched(800.0, 900.0);
    assert_eq!(host.preview_scroll_y, 0.0);
    assert!(host.preview_surface_capture.is_none());
    assert!(host.preview_device_frame.is_some());
}
