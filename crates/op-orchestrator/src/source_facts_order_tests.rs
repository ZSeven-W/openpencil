use super::*;
use serde_json::json;

const BRIEF: &str = "配送详情，骑手李师傅，预计12分钟送达，订单为双人晚餐，送到滨江路88号。";

fn request(brief: &str) -> DesignRequest {
    DesignRequest {
        prompt: brief.into(),
        ..Default::default()
    }
}

fn text(name: &str, content: &str) -> PenNode {
    serde_json::from_value(json!({"type":"text","id":"t","name":name,"content":content})).unwrap()
}

#[test]
fn source_facts_order_rejects_actual_header_and_card_identifiers_before_mutation() {
    for node in [
        text("Order Number", "订单号 WDS-20240612"),
        text("订单号", "WD20240612-088"),
        text("orderId", "OP-1234"),
        text("order_number", "OP-1234"),
        text("order-number", "OP-1234"),
        text("detail", "Order ID: OP-1234"),
        text("detail", "Order No. #123456"),
        text("detail", "Order #123456"),
    ] {
        let before = serde_json::to_value(&node).unwrap();
        let reason = rejection(&request(BRIEF), std::slice::from_ref(&node)).unwrap();
        assert!(reason.starts_with("self-check failed: unsupported-order-identifier"));
        assert_eq!(serde_json::to_value(&node).unwrap(), before);
    }
}

#[test]
fn source_facts_order_pairs_local_labels_with_styled_values() {
    let row: PenNode = serde_json::from_value(json!({"type":"frame","id":"row","children":[
        {"type":"text","id":"label","content":"订单编号"},
        {"type":"text","id":"value","content":[{"text":"WD20240612"},{"text":"-088"}]}
    ]}))
    .unwrap();
    let reason = rejection(&request(BRIEF), &[row]).unwrap();
    assert!(reason.contains("WD20240612-088"));
    let component: PenNode = serde_json::from_value(
        json!({"type":"frame","id":"number","name":"order-id","children":[
            {"type":"text","id":"value","content":"OP-1234"}
        ]}),
    )
    .unwrap();
    assert!(rejection(&request(BRIEF), &[component]).is_some());
}

#[test]
fn source_facts_order_preserves_requested_fields_sources_and_concept_collections() {
    for brief in [
        "配送详情，订单号 OP-1234，在页头和卡片展示。",
        "配送详情保留订单编号字段，值待填写。",
        "Order details with an order ID field.",
        "Show delivery progress for OP-1234.",
        "订单表格10行，展示示例订单。",
        "Admin console with an orders table of 10 rows.",
        "订单详情使用虚构数据。",
        "Order detail using sample data.",
        "复刻参考图中的订单信息。",
    ] {
        assert!(
            rejection(&request(brief), &[text("order-id", "OP-1234")]).is_none(),
            "{brief}"
        );
    }
    // A prefix of a supplied identifier is not the same identifier.
    assert!(rejection(
        &request("跟踪订单OP-12345的配送情况"),
        &[text("order-id", "OP-1234")]
    )
    .is_some());
    for brief in [
        "配送订单详情，不要示例数据。",
        "Order detail, no sample data.",
    ] {
        assert!(rejection(&request(brief), &[text("order-id", "OP-1234")]).is_some());
    }
}

#[test]
fn source_facts_order_ignores_unrelated_values_placeholders_and_hidden_nodes() {
    let mut nodes = vec![
        text("eta", "12"),
        text("order-count", "128"),
        text("order-id-count", "128"),
        text("border-id", "123456"),
        text("订单号", "—"),
        text("订单号", "待填写"),
        text("订单号", "$order.id"),
        text("price", "123456"),
        text("detail", "Order identity 123456"),
        text("detail", "滨江路88号"),
    ];
    nodes.push(
        serde_json::from_value(json!({"type":"frame","id":"hidden","enabled":false,
        "children":[{"type":"text","id":"v","content":"订单号 WDS-20240612"}]}))
        .unwrap(),
    );
    assert!(rejection(&request(BRIEF), &nodes).is_none());
    // Do not join a label in one card with arbitrary data in another.
    let page: PenNode = serde_json::from_value(json!({"type":"frame","id":"page","children":[
        {"type":"text","id":"label","content":"订单编号"},
        {"type":"frame","id":"stats","children":[
            {"type":"text","id":"a","content":"12"},{"type":"text","id":"b","content":"128"},
            {"type":"text","id":"c","content":"100"},{"type":"text","id":"d","content":"200"}
        ]}
    ]}))
    .unwrap();
    assert!(rejection(&request(BRIEF), &[page]).is_none());
}

#[test]
fn source_facts_order_check_is_independent_of_requested_ratings() {
    let req = request("配送详情，骑手评分4.8，联系骑手。");
    assert!(super::super::rejection(&req, &[text("order-id", "WDS-20240612")]).is_some());
}

#[test]
fn source_facts_order_hint_overrules_planner_without_rewriting_brief() {
    let task: Subtask = serde_json::from_value(json!({"id":"header","label":"顶部导航",
        "region":{"width":375,"height":70},"elements":"返回、订单标题、订单编号"}))
    .unwrap();
    let mut prompt = BRIEF.to_string();
    append_section_instruction(&request(BRIEF), &task, &mut prompt);
    assert!(prompt.starts_with(BRIEF));
    assert!(prompt.contains("THIS SECTION — UNSUPPORTED ORDER IDENTIFIER"));
    for brief in ["订单号 OP-1234", "订单表格，填充10条示例数据", "复刻参考图"]
    {
        let mut prompt = String::new();
        append_section_instruction(&request(brief), &task, &mut prompt);
        assert!(prompt.is_empty());
    }
}
