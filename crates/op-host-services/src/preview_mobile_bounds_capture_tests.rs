//! Visual and scroll acceptance of the retained phone bounds repair.

use op_preview_core::PreviewSession;
use serde_json::{json, Value};

fn text_ids(value: &Value, ids: &mut Vec<String>) {
    if value["type"] == "text" {
        if let Some(id) = value["id"].as_str() {
            ids.push(id.to_owned());
        }
    }
    if let Some(kids) = value["children"].as_array() {
        for child in kids {
            text_ids(child, ids);
        }
    }
}

#[test]
#[ignore = "requires OPENPENCIL_QA_MOBILE_BOUNDS_DOCUMENT and capture directory"]
fn captured_mobile_bounds_keep_navigation_inside_and_last_content_reachable() {
    let source =
        std::fs::read_to_string(std::env::var("OPENPENCIL_QA_MOBILE_BOUNDS_DOCUMENT").unwrap())
            .unwrap();
    let doc = op_pen_loader::payload::load_canonical(&source)
        .unwrap()
        .value;
    let original = serde_json::to_value(&doc).unwrap();
    let root = &original["children"][0];
    let kids = root["children"].as_array().unwrap();
    let viewport = kids.iter().find(|v| v["role"] == "scroll-area").unwrap();
    let root_id = root["id"].as_str().unwrap();
    let viewport_id = viewport["id"].as_str().unwrap();
    let nav_id = kids.last().unwrap()["id"].as_str().unwrap();
    let status_id = kids[0]["id"].as_str().unwrap();
    let mut text = vec![];
    text_ids(viewport, &mut text);
    let mut session = PreviewSession::enter(
        &doc,
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
    let page = before.active_page().unwrap();
    let board = page.find(root_id).unwrap().bounds;
    let vp = page.find(viewport_id).unwrap().bounds;
    let nav = page.find(nav_id).unwrap().bounds;
    let status = page.find(status_id).unwrap().bounds;
    assert!(nav.origin.y + nav.size.y <= board.origin.y + board.size.y + 1.0);
    let last_id = text
        .iter()
        .max_by(|a, b| {
            let a = page.find(a).unwrap().bounds;
            let b = page.find(b).unwrap().bounds;
            (a.origin.y + a.size.y).total_cmp(&(b.origin.y + b.size.y))
        })
        .unwrap();
    let output =
        std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_MOBILE_BOUNDS_CAPTURE").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(
        output.join("first-view.png"),
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
        vp.origin.x + vp.size.x - 20.0,
        vp.origin.y + 50.0,
        0.0,
        -10000.0
    ));
    session.pump(3000);
    let _ = session.preview_scene_for_test();
    session.pump(4000);
    let after = session.preview_scene_for_test();
    let page = after.active_page().unwrap();
    let last = page.find(last_id).unwrap().bounds;
    assert_eq!(page.find(nav_id).unwrap().bounds, nav);
    assert_eq!(page.find(status_id).unwrap().bounds, status);
    assert!(
        last.origin.y + last.size.y <= vp.origin.y + vp.size.y + 1.0,
        "{last:?} inside {vp:?}"
    );
    std::fs::write(
        output.join("scrolled-bottom.png"),
        crate::export::render_node_raster_bytes(
            &after,
            root_id,
            crate::export::RasterFormat::Png,
            1.0,
        )
        .unwrap(),
    )
    .unwrap();
    std::fs::write(output.join("receipt.json"),serde_json::to_vec_pretty(&json!({"navigation_inside":true,"status_and_nav_stationary":true,
        "last_text_reachable":true,"last_text_id":last_id,"last_text_bottom":last.origin.y+last.size.y,"viewport_bottom":vp.origin.y+vp.size.y,
        "source_unchanged":serde_json::to_value(&doc).unwrap()==original})).unwrap()).unwrap();
}
