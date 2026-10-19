use super::*;
use serde_json::json;

#[test]
fn retained_product_controls_with_icons_and_separators_are_rejected_without_rewriting() {
    let fixture: Value = serde_json::from_str(include_str!(
        "test_fixtures/inert-product-steppers-1019.json"
    ))
    .unwrap();
    assert_eq!(
        fixture["source_sha256"],
        "848f250dbadee8d9616779cac1b35fcde4d21265e937b54814646512e3179dad"
    );
    for value in fixture["controls"].as_array().unwrap() {
        let nodes: Vec<PenNode> = vec![serde_json::from_value(value.clone()).unwrap()];
        let before = nodes.clone();
        assert!(rejection("电商商品详情 App 两屏可交互", &nodes).is_some());
        assert_eq!(nodes, before);
    }
}

fn stepper() -> Value {
    json!({"type":"frame","id":"counter","layout":"horizontal","children":[
        {"type":"frame","id":"minus","children":[{"type":"icon_font","id":"minus-icon","iconFontName":"lucide:minus"}]},
        {"type":"text","id":"qty","content":"1"},
        {"type":"frame","id":"plus","children":[{"type":"text","id":"plus-icon","content":"+"}]}
    ]})
}
fn check(value: Value, prompt: &str) -> Option<String> {
    rejection(prompt, &[serde_json::from_value(value).unwrap()])
}

#[test]
fn inert_stepper_requires_both_actions_and_the_same_bound_quantity() {
    let mut value = stepper();
    assert!(check(value.clone(), "两屏可交互")
        .unwrap()
        .contains("interaction-inert-stepper"));
    value["children"][1]["bindings"] = json!({"content":"to_str($app.qtyA)"});
    value["children"][0]["events"] =
        json!({"onTap":[{"set":{"$state.qtyA":"$app.qtyA > 1 ? $app.qtyA - 1 : 1"}}]});
    value["children"][2]["events"] = json!({"onTap":[{"set":{"$app.qtyB":"$app.qtyB + 1"}}]});
    assert!(check(value.clone(), "Two interactive screens").is_some());
    value["children"][2]["events"] = json!({"onTap":[{"set":{"$app.qtyA":"$app.qtyA + 1"}}]});
    assert!(check(value, "Two interactive screens").is_none());
}

#[test]
fn static_work_and_unrelated_symbols_are_not_reinterpreted() {
    for prompt in ["static-only mockup", "仅静态，不需要交互", "Product poster"] {
        assert!(check(stepper(), prompt).is_none());
    }
    let mut value = stepper();
    value["children"][1]["content"] = json!("Basic plan");
    assert!(check(value, "interactive").is_none());
}
