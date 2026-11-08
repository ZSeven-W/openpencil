//! In-view motion triggers observe the device frame's visible window, not
//! the page root that grows to its full content height.

#![cfg(test)]

use super::device_frame::{compute_frame_geometry, DeviceFrame};
use super::{test_measure, PreviewSession};
use op_editor_core::PreviewDeviceKind;
use op_editor_ui::{Point2D, Rect};
use serde_json::{json, Value};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, h),
    }
}

/// A 390-wide phone page, taller than the 844-tall phone frame.
fn page(children: Value) -> PreviewSession {
    let source = json!({
        "version":"1.1","formatVersion":"1.1",
        "children":[{"type":"frame","id":"screen","width":390,"height":1600,
            "clipContent":false,"children":children}]
    });
    let doc = jian_ops_schema::load_str(&source.to_string())
        .expect("in-view fixture")
        .value;
    let mut session = PreviewSession::enter(
        &doc,
        (1000.0, 1000.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
        0,
    )
    .expect("preview session");
    session.begin_lifecycle(0);
    session
}

fn fade_in(id: &str, y: f32, height: f32, once: bool) -> Value {
    json!({"type":"rectangle","id":id,"x":0,"y":y,"width":390,"height":height,
    "opacity":1,"fill":[{"type":"solid","color":"#ffffff"}],
    "animations":[{"trigger":"inView","durationMs":300,"easing":"linear",
        "once":once,"fillMode":"none","keyframes":[
            {"offset":0,"values":{"opacity":0}},
            {"offset":1,"values":{"opacity":1}}
        ]}]})
}

fn phone_frame(session: &PreviewSession, nav: Option<(&str, Rect)>) -> DeviceFrame {
    let (_, root) = session.framed_root().expect("framed root");
    let mut frame = compute_frame_geometry(
        PreviewDeviceKind::Phone,
        rect(0.0, 0.0, 1000.0, 1000.0),
        root,
        nav.map(|(_, rect)| rect),
        None,
    );
    if let (Some(pinned), Some((id, _))) = (frame.pinned.as_mut(), nav) {
        pinned.node_id = id.to_owned();
    }
    frame
}

fn opacity(session: &PreviewSession, id: &str) -> f32 {
    session
        .preview_scene_for_test()
        .active_page()
        .and_then(|page| page.find(id))
        .expect("scene node")
        .opacity
}

fn assert_mid(value: f32, what: &str) {
    assert!(
        value > 0.0 && value < 1.0,
        "{what} should be mid-tween, got {value}"
    );
}

#[test]
fn in_view_below_the_fold_waits_until_the_page_scrolls_to_it() {
    let mut session = page(json!([
        fade_in("top", 100.0, 80.0, true),
        fade_in("below", 1200.0, 80.0, true),
    ]));
    let frame = phone_frame(&session, None);
    assert!(session.sync_in_view_viewport(Some(&frame), 0.0));
    assert!(
        !session.sync_in_view_viewport(Some(&frame), 0.0),
        "an unchanged window must not ask the host to repaint"
    );
    let _ = session.preview_scene_for_test();
    session.pump(150);
    assert_mid(opacity(&session, "top"), "the row inside the first screen");
    session.pump(400);
    assert!(
        opacity(&session, "below") < 0.001,
        "a row below the fold must hold its first keyframe until scrolled to"
    );

    assert!(session.sync_in_view_viewport(Some(&frame), 900.0));
    let _ = session.preview_scene_for_test();
    session.pump(550);
    assert_mid(
        opacity(&session, "below"),
        "the row once scrolled into view",
    );
}

#[test]
fn pinned_nav_counts_as_in_view_wherever_it_sits_in_the_page() {
    let mut session = page(json!([
        fade_in("note", 1480.0, 40.0, true),
        fade_in("nav", 1540.0, 60.0, true),
    ]));
    let frame = phone_frame(&session, Some(("nav", rect(0.0, 1540.0, 390.0, 60.0))));
    assert!(session.sync_in_view_viewport(Some(&frame), 0.0));
    let _ = session.preview_scene_for_test();
    session.pump(150);
    assert_mid(opacity(&session, "nav"), "the pinned nav");
    assert!(
        opacity(&session, "note") < 0.001,
        "an unpinned node at the same depth is still below the fold"
    );
}

#[test]
fn clearing_the_window_falls_back_to_the_whole_root() {
    let mut session = page(json!([fade_in("below", 1200.0, 80.0, true)]));
    let frame = phone_frame(&session, None);
    session.sync_in_view_viewport(Some(&frame), 0.0);
    let _ = session.preview_scene_for_test();
    session.pump(100);
    assert!(opacity(&session, "below") < 0.001);

    assert!(session.sync_in_view_viewport(None, 0.0));
    let _ = session.preview_scene_for_test();
    session.pump(250);
    assert_mid(
        opacity(&session, "below"),
        "the row once the window is cleared",
    );
}

#[test]
fn repeating_in_view_fires_again_after_scrolling_away_and_back() {
    let mut session = page(json!([fade_in("again", 1200.0, 80.0, false)]));
    let frame = phone_frame(&session, None);
    session.sync_in_view_viewport(Some(&frame), 900.0);
    let _ = session.preview_scene_for_test();
    session.pump(150);
    assert_mid(opacity(&session, "again"), "the first reveal");
    session.pump(500);
    assert!((opacity(&session, "again") - 1.0).abs() < 0.001);

    session.sync_in_view_viewport(Some(&frame), 0.0);
    let _ = session.preview_scene_for_test();
    session.pump(600);
    session.sync_in_view_viewport(Some(&frame), 900.0);
    let _ = session.preview_scene_for_test();
    session.pump(750);
    assert_mid(opacity(&session, "again"), "the second reveal");
}
