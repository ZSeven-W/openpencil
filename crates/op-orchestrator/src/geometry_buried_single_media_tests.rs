use super::*;
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use serde_json::json;

const RESPONSE: &str = include_str!("../tests/fixtures/carousel-1018/product-hero-attempt1.js");
const BEFORE: &str =
    include_str!("../tests/fixtures/carousel-1018/product-hero-attempt1-before-overlay.op");

fn before() -> EditorState {
    EditorState::from_document(serde_json::from_str(BEFORE).unwrap())
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

fn rects(state: &EditorState) -> HashMap<String, Rect> {
    super::super::super::resolved_rects(state)
}

fn collect(state: &EditorState) -> Vec<EditorCommand> {
    let mut commands = Vec::new();
    collect_buried_overlay_fixes_in_state(state, &viewport(state), &rects(state), &mut commands);
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
fn retained_single_media_dots_need_one_move_and_preserve_front_button_shadows() {
    for fallback in [false, true] {
        let mut state = before();
        if fallback {
            assert_eq!(
                crate::apply_image_fallback_policy_to_state(&mut state, true),
                1
            );
        }
        let original = viewport(&state);
        assert_eq!(original["children"][0]["name"], "返回按钮");
        assert_eq!(original["children"][1]["name"], "收藏按钮");
        assert_eq!(original["children"][3]["name"], "指示器");
        assert!(!original["children"][0]["effects"]
            .as_array()
            .unwrap()
            .is_empty());
        let mut before_nodes = std::collections::BTreeMap::new();
        snapshot_without_order(&original, &mut before_nodes);
        let commands = collect(&state);
        assert!(
            matches!(&commands[..], [EditorCommand::MoveNode {node_id,index:Some(3),..}]
            if node_id.as_str()==original["children"][2]["id"].as_str().unwrap()),
            "{commands:?}"
        );
        for command in commands {
            assert!(state.apply(command));
        }
        let after = viewport(&state);
        assert_eq!(after["children"][0], original["children"][0]);
        assert_eq!(after["children"][1], original["children"][1]);
        assert_eq!(after["children"][2], original["children"][3]);
        assert_eq!(after["children"][3], original["children"][2]);
        let mut after_nodes = std::collections::BTreeMap::new();
        snapshot_without_order(&after, &mut after_nodes);
        assert_eq!(before_nodes, after_nodes);
        assert!(collect(&state).is_empty());
    }
}

#[test]
fn raw_single_media_response_and_real_fallback_keep_dots_after_whole_cleanup_replay() {
    for fallback in [false, true] {
        let (nodes, app_state) = crate::script_gen::parse_script(RESPONSE).unwrap();
        assert!(app_state.is_empty());
        let mut doc_sink = VecDocSink::new();
        doc_sink.state = EditorState::from_document(serde_json::from_value(json!({
            "version":"1.0","children":[{"type":"frame","id":"root","name":"商品详情",
                "layout":"vertical","width":375,"height":812,"clipContent":true,"children":nodes}]
        })).unwrap());
        if fallback {
            assert_eq!(
                crate::apply_image_fallback_policy_to_state(&mut doc_sink.state, true),
                1
            );
        }
        let plan = serde_json::from_value(json!({
            "rootFrame":{"id":"root","name":"商品详情","width":375,"height":812},"subtasks":[]
        }))
        .unwrap();
        let mut settled = None;
        for _ in 0..3 {
            let root_id = doc_sink.state.active_children()[0].id_str().to_string();
            crate::cleanup::run_cleanup_passes(&mut doc_sink, &plan, &[root_id.as_str()]);
            let view = viewport(&doc_sink.state);
            assert_eq!(view["children"][0]["name"], "返回按钮");
            assert_eq!(view["children"][1]["name"], "收藏按钮");
            assert_eq!(view["children"][2]["name"], "指示器");
            let bounds = rects(&doc_sink.state);
            let photo = rect_of(&view["children"][3], &bounds).unwrap();
            assert_eq!((photo.w, photo.h), (375.0, 340.0));
            assert_eq!(children(&view["children"][2]).len(), 5);
            assert!(collect(&doc_sink.state).is_empty());
            if let Some(ref expected) = settled {
                assert_eq!(&view, expected);
            }
            settled = Some(view);
        }
    }
}

#[test]
fn single_media_shape_matches_typed_existing_rail_contract_without_extending_lane() {
    let state = before();
    let base = viewport(&state);
    for (pointer, replacement) in [
        ("", Value::Null),
        ("/clipContent", json!(false)),
        ("/padding", json!(24)),
        ("/x", json!(24)),
        ("/children/2/width", json!(327)),
        ("/children/2/minWidth", json!(375)),
        ("/children/2/bindings", json!({"width":"$app.width"})),
    ] {
        let mut value = base.clone();
        if !pointer.is_empty() {
            let (parent, field) = pointer.rsplit_once('/').unwrap();
            value.pointer_mut(parent).unwrap()[field] = replacement;
        }
        let typed: PenNode = serde_json::from_value(value.clone()).unwrap();
        let expected = pointer.is_empty();
        assert_eq!(
            crate::clipped_media_strip::single_media_page(&value, 375.0).is_some(),
            expected,
            "{pointer}"
        );
        assert_eq!(
            crate::clipped_media_strip::full_width_media_section(&typed, 375.0),
            expected,
            "{pointer}"
        );
        assert!(crate::clipped_media_strip::media_lane(&value, 375.0).is_none());
        let serialized = serde_json::to_value(typed).unwrap();
        assert_eq!(
            crate::clipped_media_strip::single_media_page(&serialized, 375.0).is_some(),
            expected
        );
    }
}

#[test]
fn single_media_crossed_effects_and_all_dynamic_compositing_are_not_reordered() {
    let state = before();
    let bounds = rects(&state);
    let base = viewport(&state);
    for (pointer, field, replacement) in [
        ("", "effects", json!([{"type":"blur","radius":4}])),
        (
            "/children/2",
            "effects",
            json!([{"type":"blur","radius":4}]),
        ),
        (
            "/children/3",
            "effects",
            json!([{"type":"shadow","offsetX":0,"offsetY":2,"blur":8,"spread":0,"color":"#0000001A"}]),
        ),
        (
            "/children/0",
            "effects",
            json!([{"type":"background_blur","radius":8}]),
        ),
        (
            "/children/0",
            "effects",
            json!([{"type":"shadow","offsetX":0,"offsetY":2,"blur":"$app.blur","spread":0,"color":"#0000001A"}]),
        ),
        (
            "/children/0",
            "effects",
            json!([{"type":"shadow","offsetX":0,"offsetY":2,"blur":8,"spread":0,"color":"$app.shadow"}]),
        ),
        ("/children/2", "opacity", json!(0.5)),
        ("/children/3", "opacity", json!(0.5)),
        ("/children/2", "blendMode", json!("multiply")),
        ("/children/3", "isMask", json!(true)),
        (
            "/children/0",
            "animations",
            json!([{"trigger":"mount","durationMs":300,"keyframes":{"from":{"opacity":1},"to":{"opacity":0}}}]),
        ),
        ("/children/2", "bindings", json!({"x":"$app.offset"})),
        (
            "/children/3",
            "bindings",
            json!({"visible":"$app.showDots"}),
        ),
    ] {
        let mut value = base.clone();
        value.pointer_mut(pointer).unwrap()[field] = replacement;
        assert!(
            sink(Some(&state), &value, children(&value), &bounds).is_none(),
            "{pointer}/{field}"
        );
    }
}

#[test]
fn single_media_rejects_large_nested_surfaces_and_nonmedia_business_cards() {
    let state = before();
    let base = viewport(&state);
    for nested in [false, true] {
        let mut value = base.clone();
        let mut bounds = rects(&state);
        let view_rect = *rect_of(&value, &bounds).unwrap();
        let dots = &mut value["children"][3];
        dots["height"] = json!(340);
        dots["y"] = json!(0);
        let id = dots["id"].as_str().unwrap().to_string();
        bounds.insert(id, view_rect);
        if nested {
            dots["children"].as_array_mut().unwrap().push(json!({
                "type":"frame","id":"business-plate","x":0,"y":0,"width":375,"height":340,
                "fill":[{"type":"solid","color":"#FFFFFF"}]
            }));
            bounds.insert("business-plate".into(), view_rect);
        } else {
            dots["fill"] = json!([{"type":"solid","color":"#FFFFFF"}]);
        }
        assert!(sink(Some(&state), &value, children(&value), &bounds).is_none());
    }
    let mut value = base;
    let id = value["children"][2]["id"].clone();
    value["children"][2] = json!({"type":"frame","id":id,"x":0,"y":0,"width":375,"height":340,
        "fill":[{"type":"solid","color":"#FFFFFF"}],"imageSearchQuery":"product",
        "children":[{"type":"text","id":"copy","content":"Product promotion"}]});
    assert!(sink(Some(&state), &value, children(&value), &rects(&state)).is_none());
}

#[test]
fn single_canonical_fallback_keeps_transparency_and_actual_partial_coverage() {
    let mut state = before();
    assert_eq!(
        crate::apply_image_fallback_policy_to_state(&mut state, true),
        1
    );
    let base = viewport(&state);
    for fill in [
        json!({"type":"solid","color":"#FFFFFF00"}),
        json!({"type":"solid","color":"#FFFFFF","opacity":0}),
        json!({"type":"solid","color":"#FFFFFF","blendMode":"multiply"}),
        json!({"type":"solid","color":"$app.tint"}),
    ] {
        let mut value = base.clone();
        value["children"][2]["fill"] = json!([fill]);
        assert!(crate::clipped_media_strip::single_media_page(&value, 375.0).is_some());
        assert!(sink(Some(&state), &value, children(&value), &rects(&state)).is_none());
    }
    let mut bounds = rects(&state);
    let id = base["children"][2]["id"].as_str().unwrap();
    bounds.get_mut(id).unwrap().w = 180.0;
    assert!(sink(Some(&state), &base, children(&base), &bounds).is_none());
}
