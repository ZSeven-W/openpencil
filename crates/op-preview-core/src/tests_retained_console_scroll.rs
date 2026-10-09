//! Retained desktop artifacts exercise the same wheel adapter as the native host.

use super::tests_binding_overlay::{enter, find};
use serde_json::{json, Value};

fn table(v: &Value) -> Option<&Value> {
    let kids = v.get("children").and_then(Value::as_array)?;
    if v.get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase()
        .contains("table")
        && kids
            .iter()
            .filter(|n| n.get("layout").and_then(Value::as_str) == Some("horizontal"))
            .count()
            >= 2
    {
        return Some(v);
    }
    kids.iter().find_map(table)
}

#[test]
#[ignore = "requires OPENPENCIL_QA_SCROLL_DOCUMENT and receipt output path"]
fn retained_console_wheel_reaches_last_row_without_moving_header_or_pagination() {
    let path = std::env::var("OPENPENCIL_QA_SCROLL_DOCUMENT").unwrap();
    let source = std::fs::read_to_string(path).unwrap();
    let document = op_pen_loader::payload::load_canonical(&source)
        .unwrap()
        .value;
    let value = serde_json::to_value(&document).unwrap();
    let main = &value["children"][0]["children"][1];
    let kids = main["children"].as_array().unwrap();
    let viewport = kids.iter().find(|v| v["role"] == "scroll-area").unwrap();
    let viewport_id = viewport["id"].as_str().unwrap();
    let footer_id = kids.last().unwrap()["id"].as_str().unwrap();
    let header_id = kids[0]["id"].as_str().unwrap();
    let last_row_id = table(viewport).unwrap()["children"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let mut session = enter(&document);
    // Match the desktop's steady preview surface, rather than measuring the
    // pre-presentation first keyframe where lifecycle animations are paused.
    session.begin_lifecycle(0);
    session.pump(1000);
    let before = session.preview_scene_for_test();
    let vp = find(&before, viewport_id).bounds;
    let footer_before = find(&before, footer_id).bounds;
    let header_before = find(&before, header_id).bounds;
    let last_before = find(&before, last_row_id).bounds;
    assert!(session.dispatch_wheel(
        vp.origin.x + vp.size.x - 20.0,
        vp.origin.y + 30.0,
        0.0,
        -10000.0
    ));
    // Generated rows can carry 320ms entrance translations. Settle them with
    // the same animation clock the desktop host pumps before measuring reach.
    session.pump(1000);
    session.pump(2000);
    let after = session.preview_scene_for_test();
    let last_after = find(&after, last_row_id).bounds;
    assert_eq!(find(&after, footer_id).bounds, footer_before);
    assert_eq!(find(&after, header_id).bounds, header_before);
    assert!(last_after.origin.y + last_after.size.y <= vp.origin.y + vp.size.y + 1.0);
    if last_before.origin.y + last_before.size.y > vp.origin.y + vp.size.y + 1.0 {
        assert!(last_after.origin.y < last_before.origin.y);
    }
    std::fs::write(
        std::env::var("OPENPENCIL_QA_SCROLL_RECEIPT").unwrap(),
        serde_json::to_vec_pretty(&json!({
            "native_wheel_adapter_consumed":true,"header_and_footer_stationary":true,
            "last_row_id":last_row_id,"last_row_before_y":last_before.origin.y,
            "last_row_after_y":last_after.origin.y,"last_row_reachable":true,
            "viewport_bottom":vp.origin.y+vp.size.y
        }))
        .unwrap(),
    )
    .unwrap();
}
