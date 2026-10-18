use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress};
use op_editor_core::EditorCommand;
use serde_json::{json, Value};

const SCRIPTS: [&str; 2] = [
    include_str!("../tests/fixtures/subtask_completeness/product-reviews-attempt2-1018.js"),
    include_str!("../tests/fixtures/subtask_completeness/product-reviews-attempt3-1018.js"),
];

fn task() -> Subtask {
    serde_json::from_str(include_str!(
        "../tests/fixtures/subtask_completeness/product-reviews-statistics-task-1018.json"
    ))
    .unwrap()
}

fn outcome(ids: Vec<String>) -> SubtaskOutcome {
    SubtaskOutcome {
        id: "reviews".into(),
        node_count: ids.len(),
        inserted_root_ids: ids,
        error: None,
        headline: None,
        subtask: None,
    }
}

fn inserted(script: &str) -> (VecDocSink, SubtaskOutcome) {
    let (nodes, _) = crate::script_gen::parse_script(script).unwrap();
    let mut sink = VecDocSink::new();
    let ids = sink
        .insert_subtree_returning_root_ids(nodes, &NodeId::NONE)
        .unwrap();
    (sink, outcome(ids))
}

fn proof_count(sink: &VecDocSink, outcome: &SubtaskOutcome) -> usize {
    reviews::delivered_reviews(sink, &outcome.inserted_root_ids).map_or(0, |p| p.card_ids.len())
}

#[test]
fn retained_reviews_with_nested_or_direct_photos_prove_three_without_mutating_the_tree() {
    for script in SCRIPTS {
        let (sink, result) = inserted(script);
        assert_eq!(delivered_item_count(&sink, &result.inserted_root_ids), 2);
        let before = serde_json::to_value(&sink.state.doc).unwrap();
        let proof = reviews::delivered_reviews(&sink, &result.inserted_root_ids).unwrap();
        assert_eq!(proof.card_ids.len(), 3);
        assert_eq!(proof.attachment_ids.len(), 1);
        assert!(incomplete_attempt(&sink, &task(), &result).is_none());
        assert_eq!(serde_json::to_value(&sink.state.doc).unwrap(), before);
        for expected in [2, 3, 4] {
            let mut contract = task();
            contract.elements = Some(format!("评价列表{expected}条，每条头像昵称、星级和正文"));
            assert_eq!(proof_count(&sink, &result), 3);
            assert_eq!(
                incomplete_attempt(&sink, &contract, &result).is_some(),
                expected > 3
            );
        }
    }
}

#[test]
fn removing_any_real_review_still_rejects_two_of_three() {
    for script in SCRIPTS {
        let (sink, result) = inserted(script);
        let ids = reviews::delivered_reviews(&sink, &result.inserted_root_ids)
            .unwrap()
            .card_ids;
        for id in ids {
            let mut missing = VecDocSink::new();
            missing.state = EditorState::from_document(sink.state.doc.clone());
            assert!(missing.apply(EditorCommand::DeleteNode {
                node_id: NodeId::new(id),
                page_id: None
            }));
            assert_eq!(proof_count(&missing, &result), 2);
            let failure = incomplete_attempt(&missing, &task(), &result).unwrap();
            assert_eq!((failure.expected, failure.delivered), (3, 2));
        }
    }
}

fn text(id: &str, content: &str) -> Value {
    json!({"type":"text","id":id,"content":content})
}

fn image(id: &str) -> Value {
    json!({"type":"image","id":id,"src":"photo.png","width":56,"height":56})
}

fn card(id: &str, photo: bool) -> Value {
    let mut children = vec![
        json!({"type":"frame","id":format!("head-{id}"),"layout":"horizontal","children":[
            {"type":"ellipse","id":format!("avatar-{id}"),"width":32,"height":32},
            {"type":"frame","id":format!("identity-{id}"),"layout":"vertical","children":[
                text(&format!("name-{id}"),id),
                {"type":"frame","id":format!("stars-{id}"),"layout":"horizontal","children":
                    (0..5).map(|i|json!({"type":"icon_font","id":format!("star-{id}-{i}"),"iconFontName":"star"})).collect::<Vec<_>>()}
            ]}
        ]}),
        text(&format!("body-{id}"), "Comfortable fabric and a good fit."),
    ];
    if photo {
        children.push(image(&format!("photo-{id}")));
    }
    json!({"type":"frame","id":id,"name":"untrusted arbitrary name","layout":"vertical","children":children})
}

fn roots(children: Vec<Value>) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0","children":children})).unwrap(),
    );
    sink
}

fn section(children: Vec<Value>) -> (VecDocSink, SubtaskOutcome) {
    (
        roots(vec![
            json!({"type":"frame","id":"section","layout":"vertical","children":children}),
        ]),
        outcome(vec!["section".into()]),
    )
}

#[test]
fn local_contracts_do_not_borrow_statistics_foreign_sections_or_named_shells() {
    for elements in [
        "三条评价列表项",
        "评价三条",
        "评价列表：3条",
        "3 customer reviews",
    ] {
        assert!(
            reviews::has_local_review_contract(elements, 3),
            "{elements}"
        );
        assert!(
            !reviews::has_local_review_contract(elements, 4),
            "{elements}"
        );
    }
    for elements in [
        "商品列表3条",
        "3 products",
        "评分3颗星",
        "review-card",
        "评价区域",
    ] {
        assert!(
            !reviews::has_local_review_contract(elements, 3),
            "{elements}"
        );
    }
    let mut contract = task();
    contract.label = "商品信息".into();
    contract.elements = Some("销量与评分行：4.8分 · 1,236条评价".into());
    assert!(!reviews::has_local_review_contract(
        &local_item_text(&contract),
        1236
    ));
    let (sink, result) = section(vec![card("a", true), card("b", false), card("c", false)]);
    contract.elements = Some("商品列表3条".into());
    assert!(incomplete_attempt(&sink, &contract, &result).is_some());
}

#[test]
fn images_headers_empty_shells_and_extra_content_cannot_pad_two_reviews() {
    for extra in [
        image("extra"),
        text("extra", "用户评价 (1,236)"),
        json!({"type":"frame","id":"extra","name":"review-card","children":[]}),
        json!({"type":"frame","id":"extra","name":"review-card","children":[{"type":"ellipse","id":"avatar"}]}),
        card("bad-body", false),
        card("extra-text", false),
        card("extra-button", false),
    ] {
        let mut extra = extra;
        let id = extra["id"].as_str().unwrap().to_owned();
        if id == "bad-body" {
            extra["children"][1]["content"] = json!("");
        }
        if id == "extra-text" {
            extra["children"]
                .as_array_mut()
                .unwrap()
                .push(text("third", "another paragraph"));
        }
        if id == "extra-button" {
            extra["children"]
                .as_array_mut()
                .unwrap()
                .push(json!({"type":"frame","id":"button","children":[text("label","Helpful")]}));
        }
        let (sink, result) = section(vec![card("a", true), card("b", false), extra]);
        assert_eq!(proof_count(&sink, &result), 2, "{id}");
        assert!(
            incomplete_attempt(&sink, &task(), &result).is_some(),
            "{id}"
        );
    }
}

#[test]
fn photos_paragraphs_and_stars_inside_one_review_are_never_other_reviews() {
    let mut single = card("a", true);
    single["children"]
        .as_array_mut()
        .unwrap()
        .extend([image("photo2"), image("photo3")]);
    let (sink, result) = section(vec![single]);
    assert_eq!(proof_count(&sink, &result), 0);
    let (sink, result) = section(vec![card("a", true)]);
    assert_eq!(proof_count(&sink, &result), 1);
}

#[test]
fn actual_parent_ownership_visibility_and_header_structure_bound_the_family() {
    let sink = roots(vec![
        json!({"type":"frame","id":"one","children":[card("a",true),card("b",false)]}),
        json!({"type":"frame","id":"two","children":[card("c",false)]}),
    ]);
    assert_eq!(
        proof_count(&sink, &outcome(vec!["one".into(), "two".into()])),
        2
    );
    assert_eq!(
        proof_count(&sink, &outcome(vec!["a".into(), "b".into(), "c".into()])),
        2
    );
    assert_eq!(proof_count(&sink, &outcome(vec!["two".into()])), 1);
    for (key, value) in [
        ("enabled", json!(false)),
        ("opacity", json!(0)),
        ("opacity", json!("$app.alpha")),
        ("bindings", json!({"visible":"$app.show"})),
    ] {
        let mut hidden = card("hidden", false);
        hidden[key] = value;
        let (sink, result) = section(vec![card("a", true), card("b", false), hidden]);
        assert_eq!(proof_count(&sink, &result), 2);
    }
    let mut hidden_parent =
        json!({"type":"frame","id":"hidden-parent","enabled":false,"children":[card("c",false)]});
    let (sink, result) = section(vec![
        card("a", true),
        card("b", false),
        hidden_parent.take(),
    ]);
    assert_eq!(proof_count(&sink, &result), 2);
    let mut changed = card("c", false);
    changed["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(text("metadata", "Verified"));
    let (sink, result) = section(vec![card("a", true), card("b", false), changed]);
    assert_eq!(proof_count(&sink, &result), 2);
}

#[test]
fn optional_photo_wrappers_keep_events_and_nonimage_content_outside_the_exception() {
    for extra in [
        json!({"type":"frame","id":"wrapper","children":[image("wrapped")]}),
        json!({"type":"frame","id":"wrapper","children":[image("wrapped"),text("caption","caption")]}),
        json!({"type":"image","id":"interactive","src":"photo.png","events":{"onTap":[{"toast":"'preview'"}]}}),
    ] {
        let valid = extra["id"] == "wrapper" && extra["children"].as_array().unwrap().len() == 1;
        let mut third = card("c", false);
        third["children"].as_array_mut().unwrap().push(extra);
        let (sink, result) = section(vec![card("a", true), card("b", false), third]);
        assert_eq!(proof_count(&sink, &result), if valid { 3 } else { 2 });
    }
}

#[test]
fn retained_second_response_finishes_without_a_false_quota_retry() {
    let task = task();
    let plan=serde_json::from_value(json!({"rootFrame":{"id":"root","name":"商品详情","width":375,"height":812},"subtasks":[task]})).unwrap();
    let request = DesignRequest {
        prompt: "评价三条，首条含照片".into(),
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
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(SCRIPTS[0].into())]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let result =
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
    assert!(result.error.is_none(), "{result:?}");
    assert_eq!(llm.user_prompts().len(), 1);
    assert_eq!(proof_count(&sink, &result), 3);
    assert!(!events.iter().any(|e| matches!(
        e,
        Progress::SubtaskRetry { .. } | Progress::SubtaskIncomplete { .. }
    )));
}
