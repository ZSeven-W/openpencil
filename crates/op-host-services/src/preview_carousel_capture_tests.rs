//! Retained-document visual proof through the shared preview and PNG renderer.

use op_preview_core::PreviewSession;
use serde_json::{json, Value};

fn carousel(value: &Value) -> Option<(&Value, &Value)> {
    let kids = value.get("children").and_then(Value::as_array)?;
    if op_editor_core::scroll_viewport::is_horizontal_scroll_viewport(value) {
        let lane = if kids.len() == 1 { &kids[0] } else { value };
        if lane["children"]
            .as_array()
            .is_some_and(|items| items.len() >= 6)
        {
            return Some((value, lane));
        }
    }
    kids.iter().find_map(carousel)
}

#[test]
#[ignore = "requires OPENPENCIL_QA_MOBILE_DOCUMENT and OPENPENCIL_QA_CAPTURE_DIR"]
fn captured_carousel_pngs_show_first_and_last_course() {
    let path = std::env::var("OPENPENCIL_QA_MOBILE_DOCUMENT").unwrap();
    let source = std::fs::read_to_string(&path).unwrap();
    let document = op_pen_loader::payload::load_canonical(&source)
        .unwrap()
        .value;
    let original = serde_json::to_value(&document).unwrap();
    let (viewport, lane) = carousel(&original).expect("six-card carousel");
    let viewport_id = viewport["id"].as_str().unwrap();
    let last_id = lane["children"].as_array().unwrap().last().unwrap()["id"]
        .as_str()
        .unwrap();
    let root_id = original["children"][0]["id"].as_str().unwrap();
    let output = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_CAPTURE_DIR").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = PreviewSession::enter(
        &document,
        (800.0, 1000.0),
        &Default::default(),
        0,
        false,
        false,
        std::rc::Rc::new(jian_skia::SkiaMeasure::new()),
        0,
    )
    .unwrap();
    session.begin_lifecycle(0);
    session.pump(1000);
    let _ = session.preview_scene_for_test();
    session.pump(2000);
    let before = session.preview_scene_for_test();
    let vp = before
        .active_page()
        .unwrap()
        .find(viewport_id)
        .unwrap()
        .bounds;
    std::fs::write(
        output.join("before.png"),
        crate::export::render_node_raster_bytes(
            &before,
            root_id,
            crate::export::RasterFormat::Png,
            1.0,
        )
        .unwrap(),
    )
    .unwrap();
    assert!(session.dispatch_wheel(
        vp.origin.x + 20.0,
        vp.origin.y + vp.size.y / 2.0,
        -10000.0,
        0.0
    ));
    session.pump(3000);
    let _ = session.preview_scene_for_test();
    session.pump(4000);
    let after = session.preview_scene_for_test();
    let last = after.active_page().unwrap().find(last_id).unwrap().bounds;
    assert!(last.origin.x + last.size.x <= vp.origin.x + vp.size.x + 1.0);
    std::fs::write(
        output.join("after.png"),
        crate::export::render_node_raster_bytes(
            &after,
            root_id,
            crate::export::RasterFormat::Png,
            1.0,
        )
        .unwrap(),
    )
    .unwrap();
    let mut touch = PreviewSession::enter(
        &document,
        (800.0, 1000.0),
        &Default::default(),
        0,
        false,
        false,
        std::rc::Rc::new(jian_skia::SkiaMeasure::new()),
        0,
    )
    .unwrap();
    touch.begin_lifecycle(0);
    touch.pump(1000);
    let _ = touch.preview_scene_for_test();
    touch.pump(2000);
    let (x, y) = (
        vp.origin.x + vp.size.x - 20.0,
        vp.origin.y + vp.size.y / 2.0,
    );
    use jian_core::gesture::pointer::{PointerKind, PointerPhase};
    touch.dispatch_pointer_for_id_at(42, PointerKind::Touch, x, y, PointerPhase::Down, 2100);
    let touch_consumed = touch.dispatch_pointer_for_id_at(
        42,
        PointerKind::Touch,
        x - 10000.0,
        y,
        PointerPhase::Move,
        2200,
    );
    touch.dispatch_pointer_for_id_at(
        42,
        PointerKind::Touch,
        x - 10000.0,
        y,
        PointerPhase::Up,
        2250,
    );
    touch.pump(3000);
    let _ = touch.preview_scene_for_test();
    touch.pump(4000);
    let touch_scene = touch.preview_scene_for_test();
    let touch_last = touch_scene
        .active_page()
        .unwrap()
        .find(last_id)
        .unwrap()
        .bounds;
    assert!(
        touch_consumed && touch_last.origin.x + touch_last.size.x <= vp.origin.x + vp.size.x + 1.0
    );
    std::fs::write(output.join("receipt.json"),serde_json::to_vec_pretty(&json!({"source":path,"wheel_consumed":true,"sixth_card_reachable":true,
        "touch_drag_consumed":touch_consumed,"touch_sixth_card_reachable":true,
        "last_after_x":last.origin.x,"viewport_right":vp.origin.x+vp.size.x,"source_unchanged":serde_json::to_value(&document).unwrap()==original,
        "method":"same captured document -> actual PreviewSession wheel -> shared production PNG renderer"})).unwrap()).unwrap();
}
