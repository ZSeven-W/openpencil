use super::*;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use serde_json::{json, Value};

fn effect() -> Value {
    json!({"type":"frame","id":"effect","layout":"none","width":600,"height":200,"children":[
        {"type":"text","id":"back","x":3,"y":3,"fontSize":100,"fontFamily":"Noto Serif SC","fontWeight":700,"content":"咖啡小聚","width":600,"height":"fit_content","fill":[{"type":"solid","color":"#B74A37"}]},
        {"type":"text","id":"front","x":0,"y":0,"fontSize":100,"fontFamily":"Noto Serif SC","fontWeight":700,"content":"咖啡小聚","width":600,"height":"fit_content","fill":[{"type":"solid","color":"#1E1A16"}]}
    ]})
}

#[test]
fn generation_orders_the_reading_layer_on_top_without_changing_node_fields() {
    let original = effect();
    let mut nodes = vec![serde_json::from_value(original.clone()).unwrap()];
    normalize_generated_order(&mut nodes);
    let fixed = serde_json::to_value(&nodes[0]).unwrap();
    let mut expected: PenNode = serde_json::from_value(original).unwrap();
    expected.children_mut().unwrap().swap(0, 1);
    assert_eq!(nodes[0], expected);
    assert_eq!(fixed["children"][0]["id"], "front");
    let before = nodes.clone();
    normalize_generated_order(&mut nodes);
    assert_eq!(nodes, before);
}

#[test]
fn ambiguous_duplicate_or_different_reading_blocks_do_not_claim_an_effect() {
    for (path, value) in [
        ("content", json!("另一段完全不同的文字")),
        ("fontSize", json!(80)),
        ("fontFamily", json!("Archivo")),
        ("lineHeight", json!(2.0)),
        ("width", json!(500)),
        ("opacity", json!(0.5)),
        ("visible", json!(false)),
        ("rotation", json!(15)),
        ("events", json!({"onTap":[]})),
        ("constraints", json!({"horizontal":"right"})),
        ("x", json!(40)),
    ] {
        let mut v = effect();
        v["children"][0][path] = value;
        assert!(pair(&v).is_none(), "{path}: {v}");
    }
    let mut v = effect();
    v["children"][0]["x"] = json!(0);
    v["children"][0]["y"] = json!(0);
    assert!(pair(&v).is_none());
    for (key, value) in [
        ("clipContent", json!(true)),
        ("fill", json!([{"type":"solid","color":"#FFFFFF"}])),
        ("opacity", json!(0.5)),
        ("layout", json!("vertical")),
    ] {
        let mut v = effect();
        v[key] = value;
        assert!(pair(&v).is_none(), "{key}");
    }
    let mut v = effect();
    v["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"text","id":"third","content":"额外文案"}));
    assert!(pair(&v).is_none());
}

#[test]
fn styled_layers_match_only_when_reading_and_typography_match() {
    let mut v = effect();
    v["children"][0]["content"] =
        json!([{"text":"咖啡","fontWeight":700,"fill":"#B74A37"},{"text":"小聚"}]);
    v["children"][1]["content"] =
        json!([{"text":"咖啡","fontWeight":700,"fill":"#1E1A16"},{"text":"小聚"}]);
    assert!(pair(&v).is_some());
    v["children"][1]["content"][0]["fontWeight"] = json!(400);
    assert!(pair(&v).is_none());
}

#[test]
fn different_styled_reading_copy_is_still_a_real_collision() {
    let mut v = effect();
    v["children"][0]["content"] = json!([{"text":"另一段正文"}]);
    v["children"][1]["content"] = json!([{"text":"咖啡小聚"}]);
    let doc = jian_ops_schema::load_str(&json!({"version":"1.0.0","children":[v]}).to_string())
        .unwrap()
        .value;
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = crate::geometry_validation::geometry_diagnostics(&state);
    assert!(
        issues.iter().any(|i| i.contains("TEXT leaves")),
        "{issues:?}"
    );
}

#[test]
fn a_matching_offset_copy_keeps_its_authored_coordinates_under_real_layout() {
    let doc = jian_ops_schema::load_str(&json!({"version":"1.0.0","children":[
        {"type":"frame","id":"board","width":800,"height":600,"padding":80,"layout":"vertical","children":[effect()]}
    ]}).to_string()).unwrap().value;
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    let original = sink.state().doc.clone();
    assert_eq!(
        crate::geometry_validation::clamp_absolute_children_into_parent(&mut sink, "board"),
        0
    );
    assert_eq!(sink.state().doc, original);
}

#[path = "text_effect_overlay_replay_tests.rs"]
mod replay;
