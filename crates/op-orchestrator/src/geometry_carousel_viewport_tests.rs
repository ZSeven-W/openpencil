use super::*;
use serde_json::json;

fn viewport() -> Value {
    json!({"type":"frame","id":"viewport","layout":"vertical",
    "clipContent":true,"width":375,"children":[{
        "type":"frame","id":"lane","layout":"horizontal","width":"fit_content",
        "children":[
            {"type":"frame","id":"hero","width":210},
            {"type":"frame","id":"small","width":132}
        ]
    }]})
}

fn rects() -> HashMap<String, Rect> {
    HashMap::from([
        (
            "viewport".into(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 375.0,
                h: 200.0,
            },
        ),
        (
            "hero".into(),
            Rect {
                x: 24.0,
                y: 0.0,
                w: 210.0,
                h: 180.0,
            },
        ),
        (
            "small".into(),
            Rect {
                x: 246.0,
                y: 0.0,
                w: 132.0,
                h: 140.0,
            },
        ),
    ])
}

#[test]
fn vertical_carousel_requires_both_lane_structure_and_resolved_overflow() {
    let original = viewport();
    let mut bounds = rects();
    assert!(is_intentional_horizontal_scroller(&original, &bounds));

    bounds.get_mut("small").unwrap().x = 242.0;
    assert!(!is_intentional_horizontal_scroller(&original, &bounds));
    assert!(!is_intentional_horizontal_scroller(
        &original,
        &HashMap::new()
    ));

    let mut ordinary = original.clone();
    ordinary["name"] = json!("carousel scroll rail");
    ordinary["children"][0]["width"] = json!("fill_container");
    assert!(!is_intentional_horizontal_scroller(&ordinary, &rects()));
    ordinary = original;
    ordinary["clipContent"] = json!(false);
    assert!(!is_intentional_horizontal_scroller(&ordinary, &rects()));
}
