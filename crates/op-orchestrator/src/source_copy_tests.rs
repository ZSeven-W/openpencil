use super::*;
use serde_json::json;

const BRIEF: &str = "咖啡小聚。\n把以下资料排成一张1080×1440活动卡，仅1页，文字可编辑，不用图片。保留全部原文、价格和时间，不添加事实；正文至少36px，三款饮品分别列出。\n晴日咖啡 · 静安店\n2026年10月17日\n14:30–16:00\n拿铁 28元\n美式 22元\n燕麦拿铁 32元\n到店自取，不含配送。";

fn texts(lines: &[&str]) -> Vec<PenNode> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_value(json!({
                "type":"text", "id":format!("t{i}"), "content":line,
            }))
            .unwrap()
        })
        .collect()
}

#[test]
fn header_only_output_is_six_missing_lines_even_with_a_large_decorative_tree() {
    let state = EditorState::new();
    let mut nodes = texts(&["咖啡小聚", "晴日咖啡", "· 静安店"]);
    for i in 0..42 {
        nodes.push(
            serde_json::from_value(
                json!({"type":"rectangle", "id":format!("dot{i}"), "name":"到店自取，不含配送。"}),
            )
            .unwrap(),
        );
    }
    let missing = SourceCopy::from_brief(BRIEF).missing(&state, &nodes);
    assert_eq!(
        missing,
        [
            "2026年10月17日",
            "14:30–16:00",
            "拿铁 28元",
            "美式 22元",
            "燕麦拿铁 32元",
            "到店自取，不含配送。"
        ]
    );
}

#[test]
fn editable_split_and_styled_copy_passes_but_changed_price_does_not() {
    let state = EditorState::new();
    let mut nodes = texts(&[
        "晴日咖啡",
        "· 静安店",
        "2026年10月17日",
        "14:30–16:00",
        "拿铁",
        "28元",
        "美式 22元",
        "燕麦拿铁 32元",
    ]);
    nodes.push(serde_json::from_value(json!({"type":"text","id":"footer","content":[{"text":"到店自取，"},{"text":"不含配送。"}]})).unwrap());
    let contract = SourceCopy::from_brief(BRIEF);
    assert!(contract.missing(&state, &nodes).is_empty());
    nodes[5] = texts(&["29元"]).remove(0);
    assert_eq!(contract.missing(&state, &nodes), ["拿铁 28元"]);
}

#[test]
fn hidden_copy_does_not_satisfy_a_delivery_contract() {
    let state = EditorState::new();
    let nodes = vec![serde_json::from_value(json!({
        "type":"frame", "id":"hidden", "opacity":0,
        "children":texts(&SourceCopy::from_brief(BRIEF).lines.iter().map(String::as_str).collect::<Vec<_>>()),
    })).unwrap()];
    assert_eq!(
        SourceCopy::from_brief(BRIEF).missing(&state, &nodes).len(),
        7
    );
}

#[test]
fn repeated_lines_need_repeated_occurrences_and_long_product_names_do_not_pay_twice() {
    let state = EditorState::new();
    let brief = "Keep all following text verbatim:\nLatte 28\nOat Latte 28\nLatte 28";
    assert_eq!(
        SourceCopy::from_brief(brief).missing(&state, &texts(&["Oat Latte 28", "Latte 28"])),
        ["Latte 28"]
    );
}

#[test]
fn freeform_and_paraphrase_briefs_do_not_create_literal_contracts() {
    for brief in [
        "给咖啡店设计天马行空的活动卡，突出时间和三款饮品",
        "将以下资料整理为一句广告语：\n咖啡店价格28元",
        "请保留全部原文，调整字号与颜色",
        "以下资料不要保留全部原文，提炼成短文案。\n咖啡小聚",
        "Do not preserve all following text; summarize it.\nCoffee",
    ] {
        assert!(SourceCopy::from_brief(brief).lines.is_empty(), "{brief}");
    }
    let wrapped = format!("请做一套图文卡片（card）。\n\n用户需求：{BRIEF}");
    assert_eq!(SourceCopy::from_brief(&wrapped).lines.len(), 7);
}

#[test]
fn fenced_materials_stop_at_the_closing_fence_and_oversized_contracts_are_not_truncated() {
    let brief = "Keep all following text verbatim:\n```text\nCoffee\n14:30\n```\nUse large type.";
    assert_eq!(SourceCopy::from_brief(brief).lines, ["Coffee", "14:30"]);
    let large = format!(
        "Keep all following text verbatim:\n{}",
        "Coffee\n".repeat(65)
    );
    assert!(SourceCopy::from_brief(&large).lines.is_empty());
}

#[test]
#[ignore = "set OPENPENCIL_SOURCE_COPY_REPLAY to the retained desktop document"]
fn replay_saved_desktop_missing_copy() {
    let path = std::env::var("OPENPENCIL_SOURCE_COPY_REPLAY").expect("retained document path");
    let document = jian_ops_schema::load_str(&std::fs::read_to_string(path).unwrap())
        .unwrap()
        .value;
    let state = EditorState::from_document(document);
    let missing = SourceCopy::from_brief(BRIEF).missing(&state, state.active_children());
    // The saved document also sets the store-name suffix to opacity zero:
    // six absent source lines plus one incompletely visible brand line.
    assert_eq!(missing.len(), 7, "{missing:?}");
    let boards = state
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect::<Vec<_>>();
    let audit = crate::quality_audit::audit_final_quality_with_brief(&state, &boards, BRIEF);
    assert_eq!(
        audit
            .remaining
            .iter()
            .filter(|i| i.source == "source-copy-missing")
            .count(),
        7
    );
    if let Ok(output) = std::env::var("OPENPENCIL_SOURCE_COPY_AUDIT_OUTPUT") {
        std::fs::write(
            output,
            serde_json::to_vec_pretty(
                &serde_json::json!({"missing":missing,"remaining":audit.remaining}),
            )
            .unwrap(),
        )
        .unwrap();
    }
}
