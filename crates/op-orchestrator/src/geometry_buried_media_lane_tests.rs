//! A retained full-width carousel must not paint over its own controls.
use super::*;
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, PenNodeExt};
use serde_json::json;

const RESPONSE: &str = include_str!("../tests/fixtures/carousel-1018/product-hero-attempt2.js");

fn retained() -> EditorState {
    let (nodes, state) = crate::script_gen::parse_script(RESPONSE).unwrap();
    assert!(state.is_empty());
    EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0","children":[{
            "type":"frame","id":"root","name":"商品详情","layout":"vertical",
            "width":375,"height":812,"clipContent":true,"children":nodes
        }]}))
        .unwrap(),
    )
}

fn named<'a>(node: &'a PenNode, name: &str) -> Option<&'a PenNode> {
    if node.base().name.as_deref() == Some(name) {
        return Some(node);
    }
    node.children()?.iter().find_map(|child| named(child, name))
}

fn viewport(state: &EditorState) -> Value {
    serde_json::to_value(named(&state.active_children()[0], "轮播视口").unwrap()).unwrap()
}

fn collect(state: &EditorState) -> Vec<EditorCommand> {
    let mut commands = Vec::new();
    collect_buried_overlay_fixes_in_state(
        state,
        &viewport(state),
        &super::super::resolved_rects(state),
        &mut commands,
    );
    commands
}

fn snapshot_without_order(node: &Value, out: &mut std::collections::BTreeMap<String, Value>) {
    let mut value = node.clone();
    value.as_object_mut().unwrap().remove("children");
    out.insert(node["id"].as_str().unwrap().into(), value);
    for child in children(node) {
        snapshot_without_order(child, out);
    }
}

#[test]
fn retained_carousel_media_lane_moves_behind_controls_without_editing_content() {
    let mut state = retained();
    let before = viewport(&state);
    let mut nodes_before = std::collections::BTreeMap::new();
    snapshot_without_order(&before, &mut nodes_before);
    let commands = collect(&state);
    assert!(
        matches!(&commands[..], [EditorCommand::MoveNode {node_id,index:Some(1),..}]
        if node_id.as_str()==before["children"][0]["id"].as_str().unwrap())
    );
    for command in commands {
        assert!(state.apply(command));
    }
    let after = viewport(&state);
    assert_eq!(after["children"][0], before["children"][1]);
    assert_eq!(after["children"][1], before["children"][0]);
    let mut nodes_after = std::collections::BTreeMap::new();
    snapshot_without_order(&after, &mut nodes_after);
    assert_eq!(nodes_after, nodes_before);
    assert!(collect(&state).is_empty(), "correct order is stable");
}

#[test]
fn canonical_fallback_and_mixed_carousel_pages_keep_the_same_overlay_order() {
    for mixed in [false, true] {
        let mut state = retained();
        let image_ids: Vec<_> = (1..=5)
            .map(|i| {
                NodeId::new(
                    named(&state.active_children()[0], &format!("轮播图-{i}"))
                        .unwrap()
                        .id_str(),
                )
            })
            .collect();
        if mixed {
            for i in [0, 2] {
                assert!(op_image_enrich::apply_result(
                    &mut state,
                    &image_ids[i],
                    "assets/retained-local-image.webp"
                ));
            }
        }
        assert_eq!(
            crate::apply_image_fallback_policy_to_state(&mut state, true),
            if mixed { 3 } else { 5 }
        );
        let before = viewport(&state);
        let commands = collect(&state);
        assert_eq!(commands.len(), 1, "mixed={mixed}: {commands:?}");
        for command in commands {
            assert!(state.apply(command));
        }
        let after = viewport(&state);
        assert_eq!(after["children"][0], before["children"][1]);
        assert_eq!(after["children"][1], before["children"][0]);
        assert!(collect(&state).is_empty());
    }
}

#[test]
fn carousel_overlays_and_page_widths_survive_real_fallback_then_whole_cleanup_twice() {
    for fallback in [false, true] {
        let mut sink = VecDocSink::new();
        sink.state = retained();
        if fallback {
            assert_eq!(
                crate::apply_image_fallback_policy_to_state(&mut sink.state, true),
                5
            );
        }
        let plan = serde_json::from_value(json!({
            "rootFrame":{"id":"root","name":"商品详情","width":375,"height":812},
            "subtasks":[]
        }))
        .unwrap();
        for _ in 0..3 {
            let root_id = sink.state.active_children()[0].id_str().to_string();
            crate::cleanup::run_cleanup_passes(&mut sink, &plan, &[root_id.as_str()]);
            let view = viewport(&sink.state);
            assert_eq!(view["children"][0]["name"], "悬浮控件");
            assert_eq!(view["children"][1]["name"], "轮播条");
            let rects = super::super::resolved_rects(&sink.state);
            let lane = &view["children"][1];
            assert_eq!(lane["width"].as_f64(), Some(1875.0));
            for page in children(lane) {
                let r = rect_of(page, &rects).unwrap();
                assert_eq!((r.w, r.h), (375.0, 340.0));
            }
            assert!(collect(&sink.state).is_empty());
        }
    }
}

fn fixture() -> Value {
    json!({"type":"frame","id":"viewport","width":375,"height":340,
        "layout":"none","clipContent":true,"children":[
        {"type":"frame","id":"lane","x":0,"y":0,"width":750,"height":340,
         "layout":"horizontal","children":[
            {"type":"image","id":"page1","width":375,"height":340,"src":"photo1.jpg"},
            {"type":"image","id":"page2","width":375,"height":340,"src":"photo2.jpg"}]},
        {"type":"frame","id":"dots","x":0,"y":316,"width":375,"height":8,
         "layout":"horizontal","children":[
            {"type":"ellipse","id":"dot","width":6,"height":6,"fill":[{"type":"solid","color":"#FFFFFF"}]}]},
        {"type":"frame","id":"back","x":16,"y":16,"width":40,"height":40,
         "children":[{"type":"icon_font","id":"glyph","width":20,"height":20,"iconFontName":"chevron-left"}]}]})
}

fn rects() -> HashMap<String, Rect> {
    let rect = |x, y, w, h| Rect { x, y, w, h };
    HashMap::from([
        ("viewport".into(), rect(0.0, 0.0, 375.0, 340.0)),
        ("lane".into(), rect(0.0, 0.0, 750.0, 340.0)),
        ("page1".into(), rect(0.0, 0.0, 375.0, 340.0)),
        ("page2".into(), rect(375.0, 0.0, 375.0, 340.0)),
        ("dots".into(), rect(0.0, 316.0, 375.0, 8.0)),
        ("dot".into(), rect(184.0, 317.0, 6.0, 6.0)),
        ("back".into(), rect(16.0, 16.0, 40.0, 40.0)),
        ("glyph".into(), rect(26.0, 26.0, 20.0, 20.0)),
    ])
}

#[test]
fn media_lane_sink_keeps_multiple_overlay_order_and_correct_order_stable() {
    let mut value = fixture();
    assert_eq!(
        clipped_media_lane_sink(None, &value, children(&value), &rects()),
        Some((0, 2))
    );
    let lane = value["children"].as_array_mut().unwrap().remove(0);
    value["children"].as_array_mut().unwrap().push(lane);
    assert_eq!(value["children"][0]["id"], "dots");
    assert_eq!(value["children"][1]["id"], "back");
    assert!(clipped_media_lane_sink(None, &value, children(&value), &rects()).is_none());
}

#[test]
fn media_lane_sink_rejects_dynamic_masked_faded_and_non_media_stacks() {
    for case in [
        "lane-blur",
        "lane-fade",
        "page-fade",
        "page-blend",
        "lane-blend",
        "page-mask",
        "overlay-mask",
        "binding",
        "dynamic-gap",
        "swipe",
        "padding",
        "hidden-page",
        "fit",
        "rounded-page",
        "business-card",
        "opaque-panel",
    ] {
        let mut value = fixture();
        match case {
            "lane-blur" => {
                value["children"][0]["effects"] = json!([{ "type":"blur", "radius":12 }])
            }
            "lane-fade" => value["children"][0]["opacity"] = json!(0.4),
            "page-fade" => value["children"][0]["children"][0]["opacity"] = json!(0.4),
            "page-blend" => value["children"][0]["children"][0]["blendMode"] = json!("multiply"),
            "lane-blend" => value["children"][0]["blendMode"] = json!("multiply"),
            "page-mask" => value["children"][0]["children"][0]["maskType"] = json!("alpha"),
            "overlay-mask" => value["children"][1]["isMask"] = json!(true),
            "binding" => {
                value["children"][0]["children"][0]["bindings"] = json!({"opacity":"$app.fade"})
            }
            "dynamic-gap" => value["children"][0]["gap"] = json!("$app.gap"),
            "swipe" => value["children"][0]["events"] = json!({"onSwipe":[{"toast":"next"}]}),
            "padding" => value["padding"] = json!(16),
            "hidden-page" => value["children"][0]["children"][0]["visible"] = json!(false),
            "fit" => value["children"][0]["children"][0]["objectFit"] = json!("fit"),
            "rounded-page" => value["children"][0]["children"][0]["cornerRadius"] = json!(40),
            "business-card" => {
                value["children"][0]["children"][0] = json!({
                "type":"frame","id":"page1","width":375,"height":340,
                "children":[{"type":"text","id":"copy","content":"Product details"}]})
            }
            "opaque-panel" => {
                value["children"][1]["y"] = json!(0);
                value["children"][1]["height"] = json!(340);
                value["children"][1]["fill"] = json!([{"type":"solid","color":"#FFFFFF"}]);
            }
            _ => unreachable!(),
        }
        let mut bounds = rects();
        if case == "opaque-panel" {
            let panel = bounds.get_mut("dots").unwrap();
            panel.y = 0.0;
            panel.h = 340.0;
        }
        assert!(
            clipped_media_lane_sink(None, &value, children(&value), &bounds).is_none(),
            "{case}"
        );
    }
}

#[test]
fn media_lane_uses_the_actual_visible_page_not_the_wide_lane_for_coverage() {
    let value = fixture();
    for shifted in [false, true] {
        let mut bounds = rects();
        let page = bounds.get_mut("page1").unwrap();
        if shifted {
            page.x = 375.0;
        } else {
            page.w = 180.0;
        }
        assert!(clipped_media_lane_sink(None, &value, children(&value), &bounds).is_none());
    }
}

#[test]
fn edited_fallback_caption_is_not_treated_as_canonical_media() {
    let mut state = retained();
    assert_eq!(
        crate::apply_image_fallback_policy_to_state(&mut state, true),
        5
    );
    let mut value = viewport(&state);
    value["children"][0]["children"][0]["children"][1]["content"] = json!("Business promotion");
    assert!(clipped_media_lane_sink(
        Some(&state),
        &value,
        children(&value),
        &super::super::resolved_rects(&state)
    )
    .is_none());
}

#[test]
fn transparent_canonical_fallback_does_not_claim_an_opaque_media_backdrop() {
    let mut state = retained();
    assert_eq!(
        crate::apply_image_fallback_policy_to_state(&mut state, true),
        5
    );
    let mut value = viewport(&state);
    value["children"][0]["children"][0]["fill"] = json!([{"type":"solid","color":"#FFFFFF66"}]);
    assert!(clipped_media_lane_sink(
        Some(&state),
        &value,
        children(&value),
        &super::super::resolved_rects(&state)
    )
    .is_none());
}

#[test]
fn transparent_overlay_wrapper_cannot_hide_a_full_size_opaque_business_surface() {
    let mut value = fixture();
    let glyph = value["children"][2]["children"][0].clone();
    value["children"][2] = json!({
        "type":"frame", "id":"back", "x":0, "y":0,
        "width":375, "height":340, "layout":"none", "children":[
            {"type":"frame", "id":"panel", "x":0, "y":0,
             "width":375, "height":340,
             "fill":[{"type":"solid","color":"#FFFFFF"}]}, glyph
        ]
    });
    let mut bounds = rects();
    let viewport = *bounds.get("viewport").unwrap();
    bounds.insert("back".into(), viewport);
    bounds.insert("panel".into(), viewport);
    assert!(clipped_media_lane_sink(None, &value, children(&value), &bounds).is_none());
}

#[test]
fn canonical_fallback_requires_a_normal_fully_opaque_solid_fill() {
    let mut state = retained();
    assert_eq!(
        crate::apply_image_fallback_policy_to_state(&mut state, true),
        5
    );
    for fill in [
        json!({"type":"solid","color":"#FFFFFF","opacity":0}),
        json!({"type":"solid","color":"#FFFFFF","opacity":0.99}),
        json!({"type":"solid","color":"#FFFFFF","blendMode":"multiply"}),
        json!({"type":"solid","color":"$unknown-media-tint"}),
        json!({"type":"solid","color":"$app.tint"}),
        json!({"type":"solid","color":"transparent"}),
    ] {
        let mut value = viewport(&state);
        value["children"][0]["children"][0]["fill"] = json!([fill]);
        assert!(crate::clipped_media_strip::media_lane(&value, 375.0).is_some());
        assert!(
            clipped_media_lane_sink(
                Some(&state),
                &value,
                children(&value),
                &super::super::resolved_rects(&state)
            )
            .is_none(),
            "fill: {fill}"
        );
    }
}

#[test]
fn canonical_fallback_resolves_document_and_active_theme_before_claiming_coverage() {
    let mut state = retained();
    assert_eq!(
        crate::apply_image_fallback_policy_to_state(&mut state, true),
        5
    );
    let value = viewport(&state);
    assert_eq!(
        value["children"][0]["children"][0]["fill"][0]["color"],
        "$--muted"
    );
    // Built-in semantic colours are valid renderer defaults, not unknown tokens.
    assert_eq!(collect(&state).len(), 1);
    assert!(clipped_media_lane_sink(
        None,
        &value,
        children(&value),
        &super::super::resolved_rects(&state)
    )
    .is_none());
    state.doc.themes = Some(serde_json::from_value(json!({"Mode":["Light","Dark"]})).unwrap());
    state.doc.variables = Some(
        serde_json::from_value(json!({
            "--muted":{"type":"color","value":[
                {"value":"#FFFFFF","theme":{"Mode":"Light"}},
                {"value":"#FFFFFF00","theme":{"Mode":"Dark"}}
            ]}
        }))
        .unwrap(),
    );
    assert_eq!(collect(&state).len(), 1, "document default is opaque Light");
    state
        .ui
        .variables
        .active_theme
        .insert("Mode".into(), "Dark".into());
    assert!(
        collect(&state).is_empty(),
        "active Dark token is transparent"
    );
    state
        .ui
        .variables
        .active_theme
        .insert("Mode".into(), "Light".into());
    assert_eq!(collect(&state).len(), 1);
}

#[test]
fn media_lane_does_not_reorder_declared_animations_on_pages_lanes_or_overlays() {
    for location in ["viewport", "lane", "page", "overlay"] {
        for keyframes in [
            json!({"from":{"opacity":1},"to":{"opacity":0}}),
            json!({"from":{"translateX":0},"to":{"translateX":375}}),
        ] {
            // Deserialize the real motion schema so this cannot pass merely
            // because a malformed annotation was rejected elsewhere.
            let animation: jian_ops_schema::motion::NodeAnimation = serde_json::from_value(json!({
                "trigger":"mount","durationMs":300,"keyframes":keyframes
            }))
            .unwrap();
            let mut value = fixture();
            let target = match location {
                "viewport" => &mut value,
                "lane" => &mut value["children"][0],
                "page" => &mut value["children"][0]["children"][0],
                "overlay" => &mut value["children"][2],
                _ => unreachable!(),
            };
            target["animations"] = json!([animation]);
            assert!(crate::clipped_media_strip::media_lane(&value, 375.0).is_some());
            assert!(
                clipped_media_lane_sink(None, &value, children(&value), &rects()).is_none(),
                "{location}: {keyframes}"
            );
        }
    }
}
