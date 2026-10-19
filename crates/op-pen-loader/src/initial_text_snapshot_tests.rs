use super::*;
use serde_json::json;

#[test]
fn initial_money_matches_preview_defaults_without_running_actions() {
    let value = json!({"version":"1.0.0",
        "state":{"qtyA":{"type":"number","default":1},"qtyB":{"type":"number","default":1}},
        "children":[
            {"type":"text","id":"subtotal","content":"wrong","bindings":{"content":"'¥' + to_str(199*$app.qtyA+329*$app.qtyB)"}},
            {"type":"text","id":"freight","content":"¥528","bindings":{"content":"'免运费'"}},
            {"type":"text","id":"discount","content":"¥528","bindings":{"content":"'-¥30'"}},
            {"type":"text","id":"total","content":"wrong","bindings":{"content":"'¥' + to_str(199 * $app.qtyA + 329 * $app.qtyB - 30)"},
                "events":{"onTap":[{"set":{"$app.qtyA":"99"}}]}}]});
    let doc = jian_ops_schema::load_str(&value.to_string()).unwrap().value;
    let before = doc.clone();
    let mut roots = doc.children.clone();
    apply(&mut roots, &doc, None);
    let copies: Vec<_> = roots
        .iter()
        .map(|node| {
            let PenNode::Text(text) = node else {
                panic!("text")
            };
            text.content.clone()
        })
        .collect();
    assert_eq!(
        copies,
        ["¥528", "免运费", "-¥30", "¥498"].map(|s| TextContent::Plain(s.into()))
    );
    assert_eq!(doc, before);
}

#[test]
fn page_and_self_defaults_resolve_but_dynamic_or_invalid_values_keep_fallbacks() {
    let value = json!({"version":"1.0.0","pages":[
    {"id":"first","name":"First","children":[],"state":{"fee":{"type":"number","default":1}}},
    {"id":"second","name":"Second","state":{"fee":{"type":"number","default":8}},"children":[
        {"type":"text","id":"local","content":"old","state":{"price":{"type":"number","default":2}},
            "bindings":{"content":"to_str($page.fee + $self.price)"}},
        {"type":"text","id":"event","content":"event fallback","bindings":{"content":"to_str($event.value)"}},
        {"type":"text","id":"time","content":"time fallback","bindings":{"content":"to_str(now())"}},
        {"type":"text","id":"invalid","content":"invalid fallback","bindings":{"content":"1 +"}},
        {"type":"text","id":"null","content":"null fallback","bindings":{"content":"null"}}
    ]}]});
    let doc = jian_ops_schema::load_str(&value.to_string()).unwrap().value;
    let mut roots = doc.pages.as_ref().unwrap()[1].children.clone();
    let original = roots.clone();
    apply(&mut roots, &doc, Some("second"));
    let PenNode::Text(text) = &roots[0] else {
        panic!("text")
    };
    assert_eq!(text.content, TextContent::Plain("10".into()));
    assert_eq!(&roots[1..], &original[1..]);
}
