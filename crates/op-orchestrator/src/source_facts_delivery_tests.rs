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

#[test]
fn normal_mode_canvas_dimensions_never_authorize_item_quantities() {
    for size in ["375×812", "375 × 812", "375 x 812", "390✕844"] {
        let req = request(&format!(
            "请设计手机 App（{size}）。交付1页。用户需求：{BRIEF}"
        ));
        for value in ["双人晚餐 ×1", "×1份", "共1件", "2份", "x 1"] {
            assert!(
                rejection(&req, &[text("meal", value)]).is_some(),
                "{size}: {value}"
            );
        }
        assert!(rejection(&req, &[text("canvas", size)]).is_none());
        let subtask: Subtask = serde_json::from_value(json!({"id":"order","label":"订单", "region":{"width":375,"height":100},"elements":"双人晚餐 ×1"})).unwrap();
        let mut prompt = String::new();
        append_section_instruction(&req, &subtask, &mut prompt);
        assert!(prompt.contains("item quantity"));
    }
    for supplied in ["双人晚餐×1", "共1件", "2份晚餐"] {
        let req = request(&format!("375×812 配送详情，{supplied}"));
        assert!(rejection(&req, &[text("meal", supplied)]).is_none());
    }
}

#[test]
fn active_delivery_omits_unsupported_distance_and_completion_actions() {
    for value in [
        "距离您约1.5公里",
        "1.5km",
        "200 m",
        "确认收货",
        "Confirm receipt",
    ] {
        assert!(
            rejection(&request(BRIEF), &[text("line", value)]).is_some(),
            "{value}"
        );
    }
    for value in [
        "双人晚餐",
        "滨江路88号",
        "12分钟",
        "联系骑手",
        "返回",
        "更新位置",
    ] {
        assert!(
            rejection(&request(BRIEF), &[text("line", value)]).is_none(),
            "{value}"
        );
    }
    for brief in [
        "配送详情，预计12分钟送达，提供确认收货按钮",
        "配送详情，已送达",
        "Delivery tracking, out for delivery, include Confirm receipt",
    ] {
        assert!(
            rejection(&request(brief), &[text("cta", "确认收货")]).is_none(),
            "{brief}"
        );
    }
    for brief in [
        "配送追踪，距离1.5km",
        "配送追踪，展示骑手距离",
        "Delivery details, show remaining distance",
    ] {
        assert!(
            rejection(&request(brief), &[text("distance", "1.5km")]).is_none(),
            "{brief}"
        );
    }
    // A supplied distance does not authorize a new action, or vice versa.
    assert!(rejection(
        &request(&format!("{BRIEF} 距离1.5km")),
        &[text("cta", "确认收货")]
    )
    .is_some());
    assert!(rejection(
        &request(&format!("{BRIEF} 提供确认收货按钮")),
        &[text("distance", "1.5km")]
    )
    .is_some());
}
