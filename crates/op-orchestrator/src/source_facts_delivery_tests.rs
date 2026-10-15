use super::*;
use serde_json::json;

const BRIEF: &str = "配送追踪详情，骑手李师傅，预计12分钟送达，订单为双人晚餐，送到滨江路88号。";
fn request(brief: &str) -> DesignRequest {
    DesignRequest {
        prompt: brief.into(),
        ..Default::default()
    }
}
fn text(name: &str, value: &str) -> PenNode {
    serde_json::from_value(json!({"type":"text","id":"t","name":name,"content":value})).unwrap()
}

#[test]
fn delivery_rejects_price_clock_and_quantity_without_changing_nodes() {
    let nodes = vec![
        text("meal", "双人晚餐 ×1"),
        text("price", "¥88.00"),
        text("eta", "预计19:32送达"),
    ];
    let before = serde_json::to_value(&nodes).unwrap();
    let reason = rejection(&request(BRIEF), &nodes).unwrap();
    for evidence in ["unsupported-delivery-facts", "¥88.00", "19:32", "×1"] {
        assert!(reason.contains(evidence));
    }
    assert_eq!(serde_json::to_value(&nodes).unwrap(), before);
    for node in [
        text("price", "88.00"),
        text("line", "$25"),
        text("line", "RMB 88"),
        text("line", "88元"),
        text("quantity", "1"),
    ] {
        assert!(rejection(&request(BRIEF), &[node]).is_some());
    }
}

#[test]
fn delivery_pairs_local_styled_arrival_label_but_not_unrelated_status_clock() {
    let row: PenNode = serde_json::from_value(json!({"type":"frame","id":"eta","children":[
        {"type":"text","id":"l","content":"预计送达"},
        {"type":"text","id":"v","content":[{"text":"19:"},{"text":"32"}]}
    ]}))
    .unwrap();
    assert!(rejection(&request(BRIEF), &[row])
        .unwrap()
        .contains("19:32"));
    let page: PenNode = serde_json::from_value(json!({"type":"frame","id":"page","children":[
        {"type":"text","id":"status","content":"9:41"},
        {"type":"text","id":"title","content":"订单"},
        {"type":"text","id":"eta","content":"预计12分钟送达"},
        {"type":"text","id":"meal","content":"双人晚餐"}
    ]}))
    .unwrap();
    assert!(rejection(&request(BRIEF), &[page]).is_none());
    let small_page: PenNode =
        serde_json::from_value(json!({"type":"frame","id":"page","children":[
            {"type":"text","id":"status","content":"9:41"},
            {"type":"text","id":"eta","content":"预计12分钟送达"}
        ]}))
        .unwrap();
    assert!(rejection(&request(BRIEF), &[small_page]).is_none());
}

#[test]
fn delivery_preserves_authorized_fields_and_other_design_domains() {
    let nodes = vec![
        text("price", "¥88.00"),
        text("arrival", "19:32"),
        text("quantity", "×1"),
    ];
    for brief in [
        "配送详情，金额¥88.00，19:32送达，双人晚餐×1。",
        "配送追踪，展示价格、送达时刻和数量字段。",
        "Delivery tracking with price, arrival time and quantity fields.",
        "配送追踪详情，允许虚构数据。",
        "Delivery details using sample data.",
        "配送订单列表，展示详情。",
        "复刻参考图中的配送详情。",
        "设计餐厅菜单",
        "Design a product pricing page",
        "跑步详情页",
    ] {
        assert!(rejection(&request(brief), &nodes).is_none(), "{brief}");
    }
    for brief in [
        "配送详情，不要虚构数据。",
        "Delivery details, no sample data.",
    ] {
        assert!(rejection(&request(brief), &nodes).is_some(), "{brief}");
    }
    // Allowing one field must not authorize all optional transaction metadata.
    assert!(rejection(
        &request(&format!("{BRIEF}金额88元。")),
        &[text("eta", "19:32")]
    )
    .is_some());
    let hidden: PenNode =
        serde_json::from_value(json!({"type":"frame","id":"h","enabled":false,"children":[
            {"type":"text","id":"v","content":"¥88.00"}
        ]}))
        .unwrap();
    assert!(rejection(
        &request(BRIEF),
        &[hidden, text("eta", "12分钟"), text("price", "$order.price")]
    )
    .is_none());
}

#[test]
fn delivery_section_hint_uses_original_brief_authority() {
    let subtask: Subtask = serde_json::from_value(json!({"id":"order","label":"订单", "region":{"width":375,"height":100},"elements":"订单金额88元、双人晚餐×1、预计19:32送达"})).unwrap();
    let mut prompt = String::new();
    append_section_instruction(&request(BRIEF), &subtask, &mut prompt);
    assert!(prompt.contains("price/total"));
    assert!(prompt.contains("absolute arrival clock"));
    assert!(prompt.contains("item quantity"));
    let mut prompt = String::new();
    append_section_instruction(
        &request("配送详情，金额88元，19:32送达，双人晚餐×1。"),
        &subtask,
        &mut prompt,
    );
    assert!(prompt.is_empty());
}
