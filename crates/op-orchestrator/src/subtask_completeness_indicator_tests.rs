use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress};
use serde_json::{json, Value};

// Exact image-led hero promise from the 1018 product-detail planner response.
const HERO_ELEMENTS: &str = "ARCHETYPE: image-led — 轮播图为主角占首屏 42%，覆盖右上角返回/收藏圆形玻璃按钮，底部 5 个圆点指示器（第 1 个高亮），纯图片占位不叠加文案";
const RETAINED: [&str; 2] = [
    include_str!("../tests/fixtures/carousel-1018/product-hero-attempt1.js"),
    include_str!("../tests/fixtures/carousel-1018/product-hero-attempt2.js"),
];

fn task(elements: &str) -> Subtask {
    serde_json::from_value(json!({
        "id":"hero", "label":"商品首屏", "elements":elements,
        "region":{"width":375,"height":340}
    }))
    .unwrap()
}

fn hero_sink() -> (VecDocSink, SubtaskOutcome) {
    let dots: Vec<Value> = (0..5)
        .map(|i| json!({"type":"ellipse","id":format!("dot-{i}"),"width":6,"height":6}))
        .collect();
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0","children":[{
            "type":"frame","id":"hero","layout":"vertical","width":375,"height":340,
            "children":[
                {"type":"image","id":"photo","src":"product.png","width":375,"height":300},
                {"type":"frame","id":"dots","layout":"horizontal","children":dots}
            ]
        }]}))
        .unwrap(),
    );
    let outcome = SubtaskOutcome {
        id: "hero".into(),
        node_count: 1,
        inserted_root_ids: vec!["hero".into()],
        error: None,
        headline: None,
        subtask: None,
    };
    (sink, outcome)
}

#[test]
fn image_led_hero_indicator_count_does_not_require_five_content_items() {
    let (sink, outcome) = hero_sink();
    assert_eq!(expected_item_count(&task(HERO_ELEMENTS)), None);
    assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 1);
    assert!(incomplete_attempt(&sink, &task(HERO_ELEMENTS), &outcome).is_none());
}

#[test]
fn explicit_indicator_counts_support_local_single_range_and_number_word_forms() {
    for elements in [
        "底部5个圆点指示器",
        "3–5 个圆点指示器",
        "三至五个小圆点指示器",
        "2到4个轮播指示点",
        "五个分页圆点",
        "分页指示器共5个",
        "轮播图指示器数量为三到五个",
        "圆点指示器5个（第1个高亮）",
        "5 pagination dots",
        "3–5 carousel indicators",
        "list of 5 pagination dots",
        "list of 3 to 5 dot indicators",
        "list of five page indicators",
        "pagination dots: list of five",
        "carousel indicators list of 3–5",
    ] {
        assert_eq!(expected_item_count(&task(elements)), None, "{elements}");
    }
}

#[test]
fn indicator_exclusion_preserves_same_clause_images_products_and_lists() {
    for (elements, expected) in [
        ("5个圆点指示器，展示5张商品图", 5),
        ("展示5张商品图，5个圆点指示器", 5),
        ("3–5个分页圆点与2张图片", 2),
        ("五个轮播指示点，三至五张商品图", 3),
        ("分页指示器共5个，商品卡3个", 3),
        ("5个圆点指示器和5个商品卡", 5),
        ("list of five page indicators; 3 products", 3),
        ("3 products with a list of 5 pagination dots", 3),
        ("carousel indicators list of 5; list of 3 cards", 3),
        ("list of 5 pagination dots with 3–5 product cards", 3),
    ] {
        assert_eq!(
            expected_item_count(&task(elements)),
            Some(expected),
            "{elements}"
        );
    }
    let (sink, outcome) = hero_sink();
    for elements in ["5张商品图，底部5个圆点指示器", "5个商品卡，5个圆点指示器"]
    {
        let failure = incomplete_attempt(&sink, &task(elements), &outcome).unwrap();
        assert_eq!((failure.expected, failure.delivered), (5, 1), "{elements}");
    }
}

#[test]
fn ordinary_dots_and_content_about_indicators_keep_their_item_contracts() {
    for elements in [
        "5个圆点商品",
        "5条圆点列表",
        "5个圆点",
        "5个圆点指示器商品卡",
        "5个分页指示器示例",
        "分页圆点5个商品",
        "5 products with pagination dots",
        "5 items as carousel indicators",
        "list of five dotted products",
        "list of 5 pagination dot cards",
        "pagination indicators list of 5 products",
    ] {
        assert_eq!(expected_item_count(&task(elements)), Some(5), "{elements}");
    }
    let mut named_hero = task("5张商品图");
    named_hero.label = "轮播图与5个圆点指示器".into();
    assert_eq!(expected_item_count(&named_hero), Some(5));
}

#[test]
fn indicator_mask_changes_only_its_count_span_and_keeps_utf8_offsets() {
    let text = "商品五张，三至五个圆点指示器，第一个高亮";
    let masked = indicator::mask_counts(text);
    assert_eq!(masked.len(), text.len());
    assert_eq!(masked, "商品五张，         个圆点指示器，第一个高亮");
    assert!(local_item_text(&task(HERO_ELEMENTS)).contains("5 个圆点指示器"));
}

#[test]
fn indicator_count_never_borrows_a_noun_across_fields_or_lines() {
    for (label, elements) in [
        ("商品轮播与分页指示器", "5个热门商品卡，保留商品名称与价格"),
        ("Carousel indicators", "list of five featured products"),
        ("5个", "圆点指示器"),
    ] {
        let mut task = task(elements);
        task.label = label.into();
        assert_eq!(expected_item_count(&task), Some(5));
        let text = local_item_text(&task);
        assert_eq!(indicator::mask_counts(&text), text);
    }
    for newline in ["\n", "\r\n", "\r"] {
        let text = format!("底部圆点指示器{newline}5个热门商品卡");
        assert_eq!(indicator::mask_counts(&text), text);
        assert_eq!(expected_item_count(&task(&text)), Some(5));
    }
}

#[test]
fn english_indicator_word_ranges_are_masked_and_suffix_checked_as_one_span() {
    for range in ["three to five", "three–five", "3 to 5", "3–5"] {
        let elements = format!("list of {range} carousel indicators; list of two products");
        assert_eq!(expected_item_count(&task(&elements)), Some(2), "{elements}");
        let elements = format!("carousel indicators: list of {range} cards");
        assert_eq!(expected_item_count(&task(&elements)), Some(3), "{elements}");
        assert_eq!(indicator::mask_counts(&elements), elements);
    }
}

#[test]
fn parenthesized_indicator_counts_keep_separate_content_promises() {
    for count in ["（5个）", "(5个)", "（三至五个）"] {
        let elements = format!("底部圆点指示器{count}，展示2张商品图");
        assert_eq!(expected_item_count(&task(&elements)), Some(2), "{elements}");
    }
    for elements in ["圆点指示器（5个商品卡）", "圆点指示器（5个）商品卡"] {
        assert_eq!(expected_item_count(&task(elements)), Some(5), "{elements}");
    }
}

#[test]
fn retained_hero_responses_do_not_turn_five_dots_into_a_five_image_promise() {
    for (script, old_delivered) in RETAINED.into_iter().zip([2, 5]) {
        let (nodes, _) = crate::script_gen::parse_script(script).unwrap();
        let mut sink = VecDocSink::new();
        let ids = sink
            .insert_subtree_returning_root_ids(nodes, &NodeId::NONE)
            .unwrap();
        let outcome = SubtaskOutcome {
            id: "hero".into(),
            node_count: ids.len(),
            inserted_root_ids: ids,
            error: None,
            headline: None,
            subtask: None,
        };
        // Keep the original delivery algorithm unchanged: the first response
        // has one image, five ellipses and a repeated pair of utility buttons.
        assert_eq!(
            delivered_item_count(&sink, &outcome.inserted_root_ids),
            old_delivered
        );
        assert!(incomplete_attempt(&sink, &task(HERO_ELEMENTS), &outcome).is_none());
    }
}

#[test]
fn retained_first_hero_finishes_without_a_quota_retry() {
    let task = task(HERO_ELEMENTS);
    let plan = serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"商品详情","width":375,"height":812},
        "subtasks":[task]
    }))
    .unwrap();
    let request = DesignRequest {
        prompt: HERO_ELEMENTS.into(),
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
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(RETAINED[0].into())]);
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
    assert!(incomplete_attempt(&sink, &task, &outcome).is_none());
    assert!(!events.iter().any(|event| matches!(
        event,
        Progress::SubtaskRetry { .. } | Progress::SubtaskIncomplete { .. }
    )));
}
