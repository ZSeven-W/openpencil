use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress};
use op_editor_core::EditorCommand;
use serde_json::json;

const RAW: &str =
    include_str!("../tests/fixtures/subtask_completeness/product-info-statistics-task-1018.json");
const NORMALIZED: &str = include_str!(
    "../tests/fixtures/subtask_completeness/product-info-statistics-normalized-task-1018.json"
);
const REVIEWS: &str = include_str!(
    "../tests/fixtures/subtask_completeness/product-reviews-statistics-task-1018.json"
);
const RESPONSE: &str =
    include_str!("../tests/fixtures/subtask_completeness/product-info-statistics-attempt1-1018.js");

fn task(elements: &str) -> Subtask {
    let mut task: Subtask = serde_json::from_str(RAW).unwrap();
    task.elements = Some(elements.into());
    task
}

fn inserted(script: &str) -> (VecDocSink, SubtaskOutcome) {
    let (nodes, _) = crate::script_gen::parse_script(script).unwrap();
    let mut sink = VecDocSink::new();
    let ids = sink
        .insert_subtree_returning_root_ids(nodes, &NodeId::NONE)
        .unwrap();
    let outcome = SubtaskOutcome {
        id: "info".into(),
        node_count: ids.len(),
        inserted_root_ids: ids,
        error: None,
        headline: None,
        subtask: None,
    };
    (sink, outcome)
}

#[test]
fn retained_info_statistics_are_display_copy_before_and_after_normalization() {
    let (sink, outcome) = inserted(RESPONSE);
    assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 0);
    let before = serde_json::to_value(&sink.state.doc).unwrap();
    assert!(before.to_string().contains("1,236 条评价"));
    assert!(before.to_string().contains("已售 3,200+"));
    for fixture in [RAW, NORMALIZED] {
        let task: Subtask = serde_json::from_str(fixture).unwrap();
        assert_eq!(expected_item_count(&task), None);
        assert!(incomplete_attempt(&sink, &task, &outcome).is_none());
    }
    assert_eq!(serde_json::to_value(&sink.state.doc).unwrap(), before);
}

#[test]
fn statistics_fields_accept_small_large_grouped_and_cjk_values_without_count_caps() {
    for number in ["3", "三", "1236", "1,236", "1, 236", "1,\t236"] {
        for field in [
            "销量与评分行",
            "销量评分行",
            "评分行",
            "评价统计行",
            "元数据行",
        ] {
            let elements = format!("{field}：4.8 分 · {number} 条评价 · 已售 3,200+");
            assert_eq!(expected_item_count(&task(&elements)), None, "{elements}");
        }
        let elements = format!("metadata row: {number} items total");
        assert_eq!(expected_item_count(&task(&elements)), None, "{elements}");
    }
}

#[test]
fn statistics_masks_preserve_real_review_product_and_large_list_promises() {
    for (elements, expected) in [
        ("展示3条评价", 3),
        ("评价3条", 3),
        ("列出12个商品", 12),
        ("商品2条", 2),
        ("销量与评分行：展示3条评价", 3),
        ("销量与评分行：列出12个商品", 12),
        ("评价统计行：236条评价列表，每条头像+昵称", 236),
        (
            "销量与评分行：1,236条评价；显示236条评价列表（每条头像+昵称）",
            236,
        ),
        ("metadata row: show 12 products", 12),
        ("metadata row: list of 12 items total", 12),
        ("metadata row：12 products sold cards", 12),
        ("评价统计行：3条评价卡", 3),
        ("评价统计行：3条评价项", 3),
        ("销量与评分行：3条评价，每条头像+昵称", 3),
        ("元数据行：3条评价，列表含头像和昵称", 3),
        ("metadata row：3 items total, each with a thumbnail", 3),
        ("元数据行：3条评价，列表含12px头像和昵称", 3),
    ] {
        assert_eq!(
            expected_item_count(&task(elements)),
            Some(expected),
            "{elements}"
        );
    }
}

#[test]
fn statistics_field_scope_does_not_erase_neighboring_lists_or_borrow_a_label() {
    for separator in ["；", ";", "\n", "\r\n", "。", "，", ", "] {
        let elements =
            format!("销量与评分行：4.8分 · 1,236条评价{separator}同时展示3条评价，每条头像+昵称");
        assert_eq!(expected_item_count(&task(&elements)), Some(3), "{elements}");
    }
    for elements in [
        "展示3条评价，每条包含评分行：4.8分 · 1,236条评价",
        "销量与评分行：1,236条评价；评价列表三条，每条头像昵称",
        "metadata row: 1,236 items total; list of 3 products",
        "metadata row: 1, 236 items total, show 3 products",
        "销量与评分行：1,236条评价，商品评价区：3条评价",
        "销量与评分行：1,236条评价，列表包含3条评价，每条头像昵称",
        "元数据行：1,236条评价，列表包含三条评价，每条头像昵称",
        "metadata row: 1,236 items total, list contains 3 items, each with a thumbnail",
    ] {
        assert_eq!(expected_item_count(&task(elements)), Some(3), "{elements}");
    }
    let mut labeled = task("展示3条评价，每条头像昵称");
    labeled.label = "销量与评分行".into();
    assert_eq!(expected_item_count(&labeled), Some(3));
    labeled.label = "meta".into();
    labeled.elements = Some("row：3条评价".into());
    assert_eq!(expected_item_count(&labeled), Some(3));
    for elements in [
        "销量与评分行\n3条评价",
        "meta row\n3 products",
        "信息行：3个商品",
    ] {
        assert_eq!(expected_item_count(&task(elements)), Some(3), "{elements}");
    }
}

#[test]
fn statistics_mask_changes_only_value_number_bytes_and_keeps_literal_copy_untouched() {
    let text = "价格¥129；销量与评分行：4.8分·1, 236条评价·已售3,200+；评价列表三条";
    let masked = statistics::mask_counts(text);
    assert_eq!(masked.len(), text.len());
    assert_eq!(masked, text.replace("1, 236", "      "));
    assert_eq!(statistics::mask_counts(&masked), masked);
    // This helper is read-only: it returns parsing text, never rewrites plan copy.
    assert!(text.contains("1, 236条评价"));
}

#[test]
fn retained_reviews_and_mixed_statistics_still_reject_two_of_three_delivered_rows() {
    let script = r#"const sec=I(null,{type:"frame",name:"用户评价",layout:"vertical"});
        ["评价A","评价B"].forEach(text=>I(sec,{type:"frame",name:"评价行",children:[
            {type:"text",content:text}]}));"#;
    let (sink, outcome) = inserted(script);
    assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 2);
    let reviews: Subtask = serde_json::from_str(REVIEWS).unwrap();
    for task in [
        reviews,
        task("销量与评分行：4.8分 · 1,236条评价；评价列表三条，每条头像昵称"),
    ] {
        assert_eq!(expected_item_count(&task), Some(3));
        let failure = incomplete_attempt(&sink, &task, &outcome).unwrap();
        assert_eq!((failure.expected, failure.delivered), (3, 2));
    }
}

#[test]
fn retained_first_info_response_finishes_without_statistics_quota_retry_or_rollback() {
    let task: Subtask = serde_json::from_str(NORMALIZED).unwrap();
    let plan = serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"商品详情","width":375,"height":812},
        "subtasks":[task]
    }))
    .unwrap();
    let request = DesignRequest {
        prompt: "设计商品价格与信息区：商品标题、价格、销量与评分行和优惠券提示条。".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(RESPONSE.into())]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let outcome =
        futures::executor::block_on(crate::concurrent::run_subtask_retry_ladder_with_outcomes(
            &task,
            &plan,
            &request,
            &llm,
            &mut sink,
            &AbortFlag::new(),
            crate::model_profile::ModelTier::Full,
            None,
            &GeometryEchoBudget::new(0),
            &mut |event| events.push(event),
            &[],
        ));
    assert!(outcome.error.is_none(), "{outcome:?}");
    assert!(outcome.node_count > 0);
    assert_eq!(llm.user_prompts().len(), 1);
    assert!(!events.iter().any(|event| matches!(
        event,
        Progress::SubtaskRetry { .. } | Progress::SubtaskIncomplete { .. }
    )));
    assert!(!sink
        .applied
        .iter()
        .any(|command| matches!(command, EditorCommand::DeleteNode { .. })));
    let document = serde_json::to_value(&sink.state.doc).unwrap().to_string();
    assert!(document.contains("1,236 条评价"));
    assert!(document.contains("已售 3,200+"));
    assert!(document.contains("云感纯棉圆领 T 恤"));
}
