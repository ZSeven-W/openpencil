use serde_json::json;

use super::*;

#[test]
fn product_rail_inside_clipping_viewport_is_not_auto_fixed() {
    let mut nodes: Vec<PenNode> = serde_json::from_value(json!([{
        "type": "frame",
        "id": "section",
        "name": "Popular Destinations Rail",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "children": [{
            "type": "frame",
            "id": "rail-viewport",
            "name": "Rail Viewport",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "clipContent": true,
            "children": [{
                "type": "frame",
                "id": "rail",
                "name": "Rail",
                "width": "fit_content",
                "height": "fit_content",
                "layout": "horizontal",
                "gap": 16,
                "children": [
                    destination_card("kyoto", "Kyoto"),
                    destination_card("santorini", "Santorini"),
                    destination_card("lisbon", "Lisbon")
                ]
            }]
        }]
    }]))
    .expect("parse nodes");
    let before = serde_json::to_value(&nodes).expect("serialize before");

    let report = check_generated_nodes(&nodes, 375.0);
    assert!(
        !report.has_fatal(),
        "inner rail of a clipped viewport is an intentional scroller: {report:?}"
    );
    assert!(
        !auto_fix_fixable_issues(&mut nodes, 375.0),
        "self-check auto-fix must not flatten intentional scrollers"
    );
    assert_eq!(serde_json::to_value(&nodes).unwrap(), before);
}

fn destination_card(id: &str, city: &str) -> serde_json::Value {
    json!({
        "type": "frame",
        "id": format!("{id}-card"),
        "name": format!("{city} Card"),
        "width": 176,
        "height": 260,
        "layout": "vertical",
        "clipContent": true,
        "children": [
            {"type": "image", "id": format!("{id}-img"), "src": "", "width": 176, "height": 150},
            {"type": "text", "id": format!("{id}-title"), "content": city}
        ]
    })
}

fn fitness_viewport() -> Value {
    let cards: Vec<_> = [210, 132, 132, 132, 132, 132]
        .into_iter()
        .enumerate()
        .map(|(index, width)| {
            let mut card = destination_card(&format!("course-{index}"), "课程");
            card["width"] = json!(width);
            card["children"][0]["width"] = json!(width);
            card
        })
        .collect();
    json!({"type":"frame","id":"viewport","name":"课程轨道 Wrapper",
    "layout":"vertical","width":"fill_container","height":"fit_content",
    "clipContent":true,"children":[{
        "type":"frame","id":"lane","name":"课程轨道 Row",
        "layout":"horizontal","width":"fit_content","height":"fit_content",
        "gap":12,"padding":[0,0,4,24],"children":cards
    }]})
}

#[test]
fn vertical_carousel_viewport_preserves_hero_and_small_card_widths() {
    let original = fitness_viewport();
    let mut nodes: Vec<PenNode> = serde_json::from_value(json!([original])).unwrap();
    let before = serde_json::to_value(&nodes).unwrap();
    assert!(!check_generated_nodes(&nodes, 375.0).has_fatal());
    assert!(!auto_fix_fixable_issues(&mut nodes, 375.0));
    assert_eq!(serde_json::to_value(nodes).unwrap(), before);
}

#[test]
fn clipping_a_normal_product_row_does_not_make_it_a_carousel() {
    for clipped in [false, true] {
        let mut original = fitness_viewport();
        original["clipContent"] = json!(clipped);
        if clipped {
            original["children"][0]["width"] = json!("fill_container");
        }
        let mut nodes: Vec<PenNode> = serde_json::from_value(json!([original])).unwrap();
        assert!(check_generated_nodes(&nodes, 375.0).has_fatal());
        assert!(auto_fix_fixable_issues(&mut nodes, 375.0));
        let after = serde_json::to_value(nodes).unwrap();
        assert!(after[0]["children"][0]["children"]
            .as_array()
            .unwrap()
            .iter()
            .all(|card| card["width"] == "fill_container"));
    }
}
