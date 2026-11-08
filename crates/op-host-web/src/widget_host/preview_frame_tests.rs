//! Device-frame page-scroll coverage for the web host.
//!
//! Mirrors native `preview_frame_tests.rs::device_scroll_feeds_the_session_page_scroll`:
//! the web host used to scroll the device frame without ever publishing it
//! to the session, so `$scroll` bindings under the framed root never moved.

#![cfg(all(test, feature = "canvaskit"))]

use super::WidgetHost;
use op_editor_core::{EditorState, PreviewDeviceKind};
use op_preview_core::device_frame::scroll_max;
use std::collections::BTreeMap;

const VIEWPORT_W: f32 = 800.0;
const VIEWPORT_H: f32 = 400.0;

fn phone_doc(height: u32) -> jian_ops_schema::PenDocument {
    jian_ops_schema::load_str(&format!(
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
    .expect("parse device-frame fixture")
    .value
}

/// Enter device-frame preview the way `enter_preview_from_browser` does
/// once the session is built (that entry needs a live CanvasKit bridge).
fn host_in_device_preview(doc: jian_ops_schema::PenDocument) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::from_document(doc);
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    let (_, _, canvas_w, canvas_h) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let session = op_preview_core::PreviewSession::enter(
        &host.editor_state.doc,
        (canvas_w, canvas_h),
        &BTreeMap::new(),
        0,
        false,
        false,
        jian_core::layout::measure::default_backend(),
        0,
    )
    .expect("web preview session");
    host.editor_state.editor_ui.enter_preview();
    host.preview = Some(session);
    host.initialize_device_preview(VIEWPORT_W, VIEWPORT_H);
    host
}

#[test]
fn device_scroll_feeds_the_session_page_scroll() {
    let mut host = host_in_device_preview(phone_doc(2000));
    assert_eq!(
        host.editor_state.editor_ui.preview.device,
        Some(PreviewDeviceKind::Phone)
    );
    let frame = host.preview_device_frame.as_ref().expect("device frame");
    let (fit, max) = (frame.fit, scroll_max(frame));
    assert!(max > 0.0, "a 2000 px screen overflows the frame");
    assert_eq!(
        host.preview.as_ref().unwrap().page_scroll(),
        Some((0.0, max)),
        "entering preview publishes max_offset before any scroll"
    );

    host.editor_state_dirty = false;
    assert!(host.apply_device_scroll(-100.0), "the wheel moved the page");
    assert!(host.editor_state_dirty);
    let (offset, published_max) = host.preview.as_ref().unwrap().page_scroll().unwrap();
    assert!(
        (offset - 100.0 / fit).abs() < 0.5,
        "offset mirrors preview_scroll_y"
    );
    assert_eq!(published_max, max);
    assert_eq!(offset, host.preview_scroll_y);
}

#[test]
fn device_switch_republishes_the_page_scroll() {
    let mut host = host_in_device_preview(phone_doc(2000));
    host.apply_device_scroll(-1e6);
    let phone_max = scroll_max(host.preview_device_frame.as_ref().unwrap());
    assert_eq!(
        host.preview.as_ref().unwrap().page_scroll(),
        Some((phone_max, phone_max)),
        "scrolling past the end clamps to max_offset"
    );

    // The Desktop frame shows a different logical height, so the rebuild
    // must publish its own extent (and the reset offset), or
    // `$scroll.progress` keeps reading the phone's.
    host.set_preview_device(PreviewDeviceKind::Desktop, VIEWPORT_W, VIEWPORT_H);
    let desktop_max = scroll_max(host.preview_device_frame.as_ref().unwrap());
    assert_ne!(desktop_max, phone_max, "the two frames scroll differently");
    assert_eq!(host.preview_scroll_y, 0.0);
    assert_eq!(
        host.preview.as_ref().unwrap().page_scroll(),
        Some((0.0, desktop_max))
    );
}
