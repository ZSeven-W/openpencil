//! A captured mobile carousel must be reachable through the real preview input.

use super::tests_binding_overlay::{enter, find};
use serde_json::{json, Value};

fn carousel(value: &Value) -> Option<(&Value, &Value)> {
    let kids = value.get("children").and_then(Value::as_array)?;
    if value["clipContent"] == true
        && kids.len() == 1
        && kids[0]["layout"] == "horizontal"
        && kids[0]["children"]
            .as_array()
            .is_some_and(|items| items.len() >= 6)
    {
        return Some((value, &kids[0]));
    }
    kids.iter().find_map(carousel)
}

#[test]
#[ignore = "requires OPENPENCIL_QA_MOBILE_DOCUMENT and receipt output path"]
fn retained_mobile_wheel_reaches_the_sixth_course() {
    let source =
        std::fs::read_to_string(std::env::var("OPENPENCIL_QA_MOBILE_DOCUMENT").unwrap()).unwrap();
    let document = op_pen_loader::payload::load_canonical(&source)
        .unwrap()
        .value;
    let value = serde_json::to_value(&document).unwrap();
    let (viewport, lane) = carousel(&value).expect("captured six-card carousel");
    let viewport_id = viewport["id"].as_str().unwrap();
    let last_id = lane["children"].as_array().unwrap().last().unwrap()["id"]
        .as_str()
        .unwrap();
    let mut session = enter(&document);
    session.begin_lifecycle(0);
    session.pump(1000);
    let before = session.preview_scene_for_test();
    let vp = find(&before, viewport_id).bounds;
    let first = find(&before, last_id).bounds;
    let consumed = session.dispatch_wheel(
        vp.origin.x + 20.0,
        vp.origin.y + vp.size.y / 2.0,
        -10000.0,
        0.0,
    );
    session.pump(2000);
    let after = session.preview_scene_for_test();
    let last = find(&after, last_id).bounds;
    let reachable = last.origin.x + last.size.x <= vp.origin.x + vp.size.x + 1.0;
    let receipt = json!({"wheel_consumed":consumed,"sixth_card_reachable":reachable,
        "last_before_x":first.origin.x,"last_after_x":last.origin.x,"viewport_right":vp.origin.x+vp.size.x});
    std::fs::write(
        std::env::var("OPENPENCIL_QA_MOBILE_RECEIPT").unwrap(),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(consumed && reachable, "{receipt}");
}
