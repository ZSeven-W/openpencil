use super::*;
use serde_json::json;

fn course_viewport() -> Value {
    json!({
        "type": "frame", "id": "viewport", "name": "课程轨道 Wrapper",
        "width": "fill_container", "layout": "vertical", "clipContent": true,
        "children": [{
            "type": "frame", "id": "lane", "name": "课程轨道 Row",
            "width": "fit_content", "layout": "horizontal", "gap": 12,
            "padding": [0, 0, 4, 24],
            "children": (0..6).map(|index| json!({
                "type": "frame", "id": format!("card-{index}"),
                "width": if index == 0 { 210 } else { 132 },
                "height": "fit_content", "layout": "vertical", "clipContent": true,
                "children": [{
                    "type": "text", "id": format!("title-{index}"), "content": "课程"
                }]
            })).collect::<Vec<_>>()
        }]
    })
}

fn assert_both(value: Value, expected: bool) {
    assert_eq!(
        is_clipped_horizontal_lane_viewport(&value),
        expected,
        "{value}"
    );
    let node: PenNode = serde_json::from_value(value).expect("valid node fixture");
    assert_eq!(is_clipped_horizontal_lane_node(&node), expected);
}

#[test]
fn mixed_width_course_lane_accepts_chinese_and_unnamed_viewports() {
    let viewport = course_viewport();
    assert_both(viewport.clone(), true);
    for layout in [Some("vertical"), Some("horizontal"), None] {
        let mut unnamed = viewport.clone();
        unnamed.as_object_mut().unwrap().remove("name");
        unnamed["children"][0]
            .as_object_mut()
            .unwrap()
            .remove("name");
        if let Some(layout) = layout {
            unnamed["layout"] = json!(layout);
        } else {
            unnamed.as_object_mut().unwrap().remove("layout");
        }
        assert_both(unnamed, true);
    }
}

#[test]
fn ordinary_rows_and_noncanonical_viewports_are_not_scroll_lanes() {
    let viewport = course_viewport();
    let mut cases = Vec::new();
    let mut vertical = viewport.clone();
    vertical["children"][0]["layout"] = json!("vertical");
    cases.push(vertical);
    let mut absolute = viewport.clone();
    absolute["layout"] = json!("none");
    cases.push(absolute);
    let mut unclipped = viewport.clone();
    unclipped.as_object_mut().unwrap().remove("clipContent");
    cases.push(unclipped);
    for width in [json!("fill_container"), json!(375), Value::Null] {
        let mut non_hug = viewport.clone();
        non_hug["children"][0]["width"] = width;
        cases.push(non_hug);
    }
    let mut multiple = viewport.clone();
    multiple["children"].as_array_mut().unwrap().push(json!({
        "type": "text", "id": "heading", "content": "推荐课程"
    }));
    cases.push(multiple);
    let mut direct = viewport["children"][0].clone();
    direct["clipContent"] = json!(true);
    cases.push(direct);
    for case in cases {
        assert_both(case, false);
    }
}

#[test]
fn table_roles_do_not_gain_a_scroll_exemption() {
    for depth in 0..3 {
        let mut viewport = course_viewport();
        match depth {
            0 => viewport["role"] = json!("table"),
            1 => viewport["children"][0]["role"] = json!("table-row"),
            _ => viewport["children"][0]["children"][0]["role"] = json!("table-cell"),
        }
        assert_both(viewport, false);
    }
}

#[test]
fn lane_requires_multiple_numeric_container_cards() {
    for width in [
        json!(0),
        json!(-1),
        json!("132"),
        json!("fill_container"),
        Value::Null,
    ] {
        let mut viewport = course_viewport();
        viewport["children"][0]["children"][1]["width"] = width;
        assert_both(viewport, false);
    }
    let mut single = course_viewport();
    single["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    assert_both(single, false);
    let mut text = course_viewport();
    text["children"][0]["children"][0] = json!({
        "type": "text", "id": "plain-text", "width": 210, "content": "课程"
    });
    assert_both(text, false);
}

#[test]
fn typed_nonfinite_card_widths_are_not_scroll_lanes() {
    for width in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let mut node: PenNode = serde_json::from_value(course_viewport()).unwrap();
        let PenNode::Frame(viewport) = &mut node else {
            unreachable!()
        };
        let PenNode::Frame(lane) = &mut viewport.children.as_mut().unwrap()[0] else {
            unreachable!()
        };
        let PenNode::Frame(card) = &mut lane.children.as_mut().unwrap()[0] else {
            unreachable!()
        };
        card.container.width = Some(SizingBehavior::Number(width));
        assert!(!is_clipped_horizontal_lane_node(&node));
    }
}

#[test]
fn custom_horizontal_gestures_reserve_viewport_or_lane() {
    for on_lane in [false, true] {
        for hook in ["onDrag", "onSwipe"] {
            let mut viewport = course_viewport();
            let target = if on_lane {
                &mut viewport["children"][0]
            } else {
                &mut viewport
            };
            target["events"] = json!({hook: [{"set": {"$app.direction": "left"}}]});
            assert_both(viewport, false);
        }
    }
    let mut empty = course_viewport();
    empty["events"] = json!({"onDrag": [], "onSwipe": []});
    assert_both(empty, true);
}
