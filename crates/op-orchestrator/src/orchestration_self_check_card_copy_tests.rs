use super::*;

const PROMPT: &str =
    "把文章做成3张1080×1440中文图文卡片，每张讲一个步骤；第二步先做一份，先查信息，再调样式。";

fn failed_card() -> Vec<PenNode> {
    let doc =
        jian_ops_schema::load_str(include_str!("test_fixtures/header-only-generated-card.op"))
            .unwrap()
            .value;
    doc.children
}

#[test]
fn real_header_only_card_is_rejected_on_every_retry_rung() {
    for mode in [IntentCheckMode::Reject, IntentCheckMode::Advisory] {
        let mut nodes = failed_card();
        let reason = crate::subagent_self_check::gate_generated_nodes(
            &mut nodes,
            1080.0,
            PROMPT,
            "card2-step2",
            mode,
        )
        .unwrap_err();
        assert!(reason.contains("missing-card-reading-content"));
    }
}

#[test]
fn actual_title_or_body_content_allows_the_same_card() {
    let mut nodes = failed_card();
    let text:PenNode=serde_json::from_value(serde_json::json!({"type":"text","id":"copy","name":"page-title","content":"第二步 · 先做一份"})).unwrap();
    let jian_ops_schema::node::PenNode::Frame(frame) = &mut nodes[0] else {
        panic!("frame")
    };
    frame.children.as_mut().unwrap().push(text);
    let mut report = SelfCheckReport::default();
    check_reading_content(&nodes, PROMPT, &mut report);
    assert!(report.issues.is_empty());
}

#[test]
fn intentional_header_skeletons_and_components_keep_their_contract() {
    for prompt in [
        "做3张卡片，只做页眉",
        "做一张骨架屏卡片",
        "设计手机 App 界面的空白画布",
    ] {
        let mut report = SelfCheckReport::default();
        check_reading_content(&failed_card(), prompt, &mut report);
        assert!(report.issues.is_empty());
    }
    let nodes = vec![serde_json::from_value(
        serde_json::json!({"type":"ref","id":"instance","ref":"master"}),
    )
    .unwrap()];
    let mut report = SelfCheckReport::default();
    check_reading_content(&nodes, PROMPT, &mut report);
    assert!(report.issues.is_empty());
}
