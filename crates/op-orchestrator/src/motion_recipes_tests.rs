use super::motion_recipes::apply;
use crate::test_support::VecDocSink;
use jian_ops_schema::motion::MotionPreference;
use serde_json::{json, Value};

fn sink_with(root: Value) -> VecDocSink {
    let mut sink = VecDocSink::new();
    let root = serde_json::from_value(root).expect("motion fixture root");
    sink.state.doc.children = vec![root];
    sink
}

fn text(id: &str, font_size: f64) -> Value {
    json!({
        "type": "text", "id": id, "content": id, "fontSize": font_size,
        "width": 200, "height": 40
    })
}

fn card(id: &str) -> Value {
    json!({
        "type": "frame", "id": id, "name": format!("Merchant {id}"),
        "width": 320, "height": 96, "children": [text(&format!("{id}-title"), 16.0)]
    })
}

fn category_tile(id: &str) -> Value {
    let mut label = text(&format!("{id}-label"), 12.0);
    label["content"] = json!("餐馆");
    json!({
        "type": "frame", "id": id, "layout": "vertical", "children": [
            {"type": "frame", "id": format!("{id}-holder"), "children": [{
                "type": "icon_font", "id": format!("{id}-icon"), "iconFontName": "star",
                "width": 20, "height": 20
            }]},
            label
        ]
    })
}

fn mobile_root(children: Vec<Value>) -> Value {
    json!({
        "type": "frame", "id": "root", "name": "App Home", "width": 390,
        "height": 844, "layout": "vertical", "children": children
    })
}

fn node_json(sink: &VecDocSink, id: &str) -> Value {
    fn find(node: &Value, id: &str) -> Option<Value> {
        if node["id"].as_str() == Some(id) {
            return Some(node.clone());
        }
        node["children"]
            .as_array()?
            .iter()
            .find_map(|child| find(child, id))
    }
    let root = serde_json::to_value(&sink.state.active_children()[0]).expect("root json");
    find(&root, id).expect("motion node exists")
}

fn motion_count(sink: &VecDocSink) -> usize {
    fn walk(node: &Value) -> usize {
        usize::from(node.get("animations").is_some())
            + node["children"]
                .as_array()
                .map(|children| children.iter().map(walk).sum())
                .unwrap_or(0)
    }
    walk(&serde_json::to_value(&sink.state.active_children()[0]).expect("root json"))
}

#[test]
fn app_shape_gets_hero_display_cards_and_cta_motion() {
    let mut children = vec![
        json!({
            "type": "frame", "id": "hero", "name": "Hero (bleed)",
            "width": 390, "height": 240, "children": [{
                "type": "rectangle", "id": "hero-media", "width": 390, "height": 240,
                "fill": [{"type": "solid", "color": "#224466"}]
            }]
        }),
        text("display", 40.0),
    ];
    children.extend((0..5).map(|index| card(&format!("card-{index}"))));
    children.push(json!({
        "type": "frame", "id": "cta", "role": "cta", "width": 320, "height": 48
    }));
    children.push(json!({
        "type": "frame", "id": "bottom-nav", "role": "bottom-tab-bar", "width": 390,
        "height": 72, "children": (0..4).map(|i| card(&format!("tab-{i}"))).collect::<Vec<_>>()
    }));
    let mut sink = sink_with(mobile_root(children));

    assert_eq!(apply(&mut sink, "root"), 8);
    assert_eq!(
        node_json(&sink, "hero")["animations"][0]["trigger"],
        "mount"
    );
    assert_eq!(node_json(&sink, "display")["animations"][0]["delayMs"], 120);
    for (index, delay) in [0, 60, 120, 180, 240].into_iter().enumerate() {
        let animation = &node_json(&sink, &format!("card-{index}"))["animations"][0];
        assert_eq!(animation["trigger"], "inView");
        assert_eq!(animation["delayMs"].as_u64().unwrap_or(0), delay);
        assert!(animation["once"].as_bool().unwrap_or(true));
    }
    assert_eq!(node_json(&sink, "cta")["transition"]["durationMs"], 150);
    assert_eq!(motion_count(&sink), 7);
    assert!(node_json(&sink, "tab-0")["animations"].is_null());
}

#[test]
fn deck_root_is_untouched() {
    let mut sink = sink_with(json!({
        "type": "frame", "id": "deck", "name": "Pitch Deck", "width": 1920,
        "height": 1080, "children": [card("card-0"), card("card-1"), card("card-2")]
    }));
    assert_eq!(apply(&mut sink, "deck"), 0);
    assert_eq!(motion_count(&sink), 0);
}

#[test]
fn authored_animation_skips_the_whole_root() {
    let mut root = mobile_root(vec![card("card-0"), card("card-1"), card("card-2")]);
    root["children"][0]["animations"] = json!([{
        "trigger": "mount", "keyframes": [{"offset": 0, "values": {"opacity": 0}},
        {"offset": 1, "values": {"opacity": 1}}], "durationMs": 200
    }]);
    let mut sink = sink_with(root);
    assert_eq!(apply(&mut sink, "root"), 0);
    assert_eq!(motion_count(&sink), 1);
}

#[test]
fn reduced_motion_skips_the_whole_root() {
    let mut sink = sink_with(mobile_root(vec![
        card("card-0"),
        card("card-1"),
        card("card-2"),
    ]));
    sink.state.doc.motion = Some(MotionPreference::Reduced);
    assert_eq!(apply(&mut sink, "root"), 0);
    assert_eq!(motion_count(&sink), 0);
}

#[test]
fn animation_budget_stops_at_twenty_four_nodes() {
    let cards = (0..30)
        .map(|index| card(&format!("card-{index}")))
        .collect();
    let mut sink = sink_with(mobile_root(cards));
    apply(&mut sink, "root");
    assert_eq!(motion_count(&sink), 24);
}

#[test]
fn category_grid_and_bottom_navigation_are_excluded() {
    let rows = (0..3)
        .map(|row| json!({
            "type": "frame", "id": format!("row-{row}"), "layout": "horizontal",
            "children": (0..3).map(|col| category_tile(&format!("tile-{row}-{col}"))).collect::<Vec<_>>()
        }))
        .collect::<Vec<_>>();
    let mut sink = sink_with(mobile_root(vec![
        json!({"type": "frame", "id": "grid", "layout": "vertical", "children": rows}),
        json!({
            "type": "frame", "id": "nav", "role": "bottom-tab-bar", "children":
                (0..4).map(|i| card(&format!("nav-{i}"))).collect::<Vec<_>>()
        }),
    ]));
    apply(&mut sink, "root");
    assert_eq!(motion_count(&sink), 0);
}

#[test]
fn landing_page_gets_feature_family_reveal() {
    let mut sink = sink_with(json!({
        "type": "frame", "id": "landing", "name": "Landing", "width": 1200,
        "height": 800, "children": [card("feature-0"), card("feature-1"), card("feature-2")]
    }));
    apply(&mut sink, "landing");
    for (index, delay) in [0, 60, 120].into_iter().enumerate() {
        assert_eq!(
            node_json(&sink, &format!("feature-{index}"))["animations"][0]["delayMs"]
                .as_u64()
                .unwrap_or(0),
            delay
        );
    }
}
