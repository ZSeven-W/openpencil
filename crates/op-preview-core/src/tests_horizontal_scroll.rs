//! Carousel scrolling through both desktop and mobile product input adapters.

use super::tests_binding_overlay::{enter, find};
use crate::{PreviewInput, PreviewInputEnvelope};
use jian_core::geometry::point;
use jian_core::gesture::pointer::{PointerEvent, PointerKind, PointerPhase};
use serde_json::json;

fn document() -> jian_ops_schema::PenDocument {
    let cards: Vec<_> = (0..6).map(|i| json!({"type":"frame","id":format!("card{i}"),
        "width":160,"height":100,"layout":"vertical",
        "events":{"onTap":[{"set":{"$app.taps":"$app.taps + 1"}},{"set":{"$app.last":i.to_string()}}]},
        "children":[{"type":"text","id":format!("label{i}"),"content":format!("Course {i}"),"fontSize":16}]
    })).collect();
    jian_ops_schema::load_str(&json!({"version":"1.1","formatVersion":"1.1","id":"carousel",
        "app":{"name":"Carousel","version":"1","id":"app"},"state":{"taps":{"type":"int","default":0},"last":{"type":"int","default":-1}},
        "children":[{"type":"frame","id":"screen","x":80,"y":40,"width":375,"height":812,"layout":"vertical","children":[
            {"type":"frame","id":"header","height":60,"width":"fill_container"},
            {"type":"frame","id":"viewport","name":"课程横向轨道","width":"fill_container","height":100,"layout":"vertical","clipContent":true,
                "children":[{"type":"frame","id":"lane","layout":"horizontal","width":"fit_content","height":100,"gap":12,"children":cards}]},
            {"type":"frame","id":"footer","height":60,"width":"fill_container"}
        ]}]
    }).to_string()).unwrap().value
}

fn pointer(session: &mut crate::PreviewSession, phase: PointerPhase, x: f32, y: f32, t: u64) {
    let mut event = PointerEvent::simple_at(7, phase, point(x, y), t);
    event.kind = PointerKind::Touch;
    session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(event)));
}

#[test]
fn horizontal_wheel_reaches_last_card_and_preserves_other_chrome_and_source() {
    let doc = document();
    let original = serde_json::to_value(&doc).unwrap();
    let mut session = enter(&doc);
    let before = session.preview_scene_for_test();
    let vp = find(&before, "viewport").bounds;
    let header = find(&before, "header").bounds;
    let footer = find(&before, "footer").bounds;
    assert!(session.dispatch_wheel(vp.origin.x + 20.0, vp.origin.y + 50.0, -10000.0, 0.0));
    let after = session.preview_scene_for_test();
    let last = find(&after, "card5").bounds;
    assert!((last.origin.x + last.size.x - vp.origin.x - vp.size.x).abs() < 1.0);
    assert_eq!(find(&after, "header").bounds, header);
    assert_eq!(find(&after, "footer").bounds, footer);
    assert!(!session.dispatch_wheel(vp.origin.x + 20.0, vp.origin.y + 50.0, 0.0, -100.0));
    assert_eq!(
        find(&session.preview_scene_for_test(), "card5").bounds,
        last
    );
    assert!(!session.dispatch_wheel(10.0, 10.0, -100.0, 0.0));
    assert_eq!(serde_json::to_value(&doc).unwrap(), original);
}

#[test]
fn touch_drag_scrolls_without_firing_a_card_tap() {
    let mut session = enter(&document());
    let vp = find(&session.preview_scene_for_test(), "viewport").bounds;
    let (x, y) = (vp.origin.x + 240.0, vp.origin.y + 50.0);
    pointer(&mut session, PointerPhase::Down, x, y, 0);
    pointer(&mut session, PointerPhase::Move, x - 100.0, y + 2.0, 100);
    pointer(&mut session, PointerPhase::Up, x - 100.0, y + 2.0, 120);
    session.pump(1000);
    assert_eq!(
        session.runtime().state.app_get("taps").unwrap().as_i64(),
        Some(0)
    );
    assert!(
        find(&session.preview_scene_for_test(), "card0")
            .bounds
            .origin
            .x
            < vp.origin.x - 80.0
    );
}

#[test]
fn short_touch_still_clicks_and_vertical_intent_does_not_shift_the_lane() {
    let mut session = enter(&document());
    let vp = find(&session.preview_scene_for_test(), "viewport").bounds;
    let (x, y) = (vp.origin.x + 60.0, vp.origin.y + 50.0);
    pointer(&mut session, PointerPhase::Down, x, y, 0);
    pointer(&mut session, PointerPhase::Up, x + 2.0, y, 50);
    session.pump(1000);
    assert_eq!(
        session.runtime().state.app_get("taps").unwrap().as_i64(),
        Some(1)
    );
    let before = find(&session.preview_scene_for_test(), "card0").bounds;
    pointer(&mut session, PointerPhase::Down, x, y, 1100);
    pointer(&mut session, PointerPhase::Move, x + 2.0, y - 60.0, 1200);
    pointer(&mut session, PointerPhase::Up, x + 2.0, y - 60.0, 1250);
    session.pump(2000);
    assert_eq!(
        find(&session.preview_scene_for_test(), "card0").bounds,
        before
    );
}

#[test]
fn legacy_touch_adapter_scrolls_and_stops_at_the_far_edge() {
    let mut session = enter(&document());
    let vp = find(&session.preview_scene_for_test(), "viewport").bounds;
    let (x, y) = (vp.origin.x + 250.0, vp.origin.y + 50.0);
    session.dispatch_pointer_for_id_at(12, PointerKind::Touch, x, y, PointerPhase::Down, 0);
    assert!(session.dispatch_pointer_for_id_at(
        12,
        PointerKind::Touch,
        x - 2000.0,
        y,
        PointerPhase::Move,
        100
    ));
    session.dispatch_pointer_for_id_at(
        12,
        PointerKind::Touch,
        x - 2000.0,
        y,
        PointerPhase::Up,
        120,
    );
    let last = find(&session.preview_scene_for_test(), "card5").bounds;
    assert!((last.origin.x + last.size.x - vp.origin.x - vp.size.x).abs() < 1.0);
}

#[test]
fn image_crops_and_authored_swipe_actions_do_not_become_implicit_carousels() {
    let mut value = serde_json::to_value(document()).unwrap();
    let viewport = &mut value["children"][0]["children"][1];
    viewport["name"] = json!("Clipped media card");
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(value.clone()).unwrap();
    let mut session = enter(&doc);
    let vp = find(&session.preview_scene_for_test(), "viewport").bounds;
    assert!(!session.dispatch_wheel(vp.origin.x + 20.0, vp.origin.y + 50.0, -100.0, 0.0));
    value["children"][0]["children"][1]["name"] = json!("Horizontal carousel");
    value["children"][0]["children"][1]["events"] =
        json!({"onSwipe":[{"set":{"$app.taps":"$app.taps + 1"}}]});
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(value).unwrap();
    let mut session = enter(&doc);
    assert!(!session.dispatch_wheel(vp.origin.x + 20.0, vp.origin.y + 50.0, -100.0, 0.0));
}

#[test]
fn clicking_after_scroll_hits_the_visible_sixth_card_and_background_cancels_a_drag() {
    let mut session = enter(&document());
    let vp = find(&session.preview_scene_for_test(), "viewport").bounds;
    session.dispatch_wheel(vp.origin.x + 20.0, vp.origin.y + 50.0, -10000.0, 0.0);
    let last = find(&session.preview_scene_for_test(), "card5").bounds;
    let (x, y) = (last.origin.x + last.size.x / 2.0, last.origin.y + 50.0);
    pointer(&mut session, PointerPhase::Down, x, y, 0);
    pointer(&mut session, PointerPhase::Up, x, y, 50);
    session.pump(1000);
    assert_eq!(
        session.runtime().state.app_get("last").unwrap().as_i64(),
        Some(5)
    );
    pointer(&mut session, PointerPhase::Down, x, y, 1100);
    pointer(&mut session, PointerPhase::Move, x + 80.0, y, 1200);
    session.cancel_input_ownership("background");
    let before = find(&session.preview_scene_for_test(), "card5").bounds;
    pointer(&mut session, PointerPhase::Move, x + 180.0, y, 1300);
    pointer(&mut session, PointerPhase::Up, x + 180.0, y, 1350);
    session.pump(2000);
    assert_eq!(
        find(&session.preview_scene_for_test(), "card5").bounds,
        before
    );
    assert_eq!(
        session.runtime().state.app_get("taps").unwrap().as_i64(),
        Some(1)
    );
}
