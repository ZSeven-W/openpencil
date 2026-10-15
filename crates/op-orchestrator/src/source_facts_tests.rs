use super::*;
use serde_json::json;

const BRIEF: &str = "用户需求：配送追踪详情，预计12分钟送达，骑手李师傅和联系按钮。";

fn request(brief: &str) -> DesignRequest {
    DesignRequest {
        prompt: brief.into(),
        ..Default::default()
    }
}

fn text(name: &str, content: &str) -> PenNode {
    serde_json::from_value(json!({"type":"text","id":"t","name":name,"content":content})).unwrap()
}

fn rating() -> Vec<PenNode> {
    vec![serde_json::from_value(json!({
        "type":"frame","id":"tag","name":"rating-tag","children":[
            {"type":"icon_font","id":"star","iconFontName":"star"},
            {"type":"text","id":"value","name":"rating-value","content":"4.9"}
        ]
    }))
    .unwrap()]
}

fn proposed_rating() -> Subtask {
    serde_json::from_value(json!({
        "id":"rider","label":"骑手卡片","region":{"width":375,"height":96},
        "elements":"骑手姓名与评价标签，联系按钮"
    }))
    .unwrap()
}

#[test]
fn source_facts_rejects_explicit_numeric_rating_components_without_mutating_them() {
    let nodes = rating();
    let before = serde_json::to_value(&nodes).unwrap();
    let reason = rejection(&request(BRIEF), &nodes).unwrap();
    assert!(reason.starts_with("self-check failed: unsupported-optional-rating"));
    assert!(reason.contains("4.9") && reason.contains("rating-tag"));
    assert_eq!(serde_json::to_value(&nodes).unwrap(), before);
    for name in [
        "riderRating",
        "rider_rating",
        "RATING",
        "RiderRATING",
        "评分",
        "评价标签",
    ] {
        assert!(
            rejection(&request(BRIEF), &[text(name, "4.9")]).is_some(),
            "{name}"
        );
    }
}

#[test]
fn source_facts_checks_visible_labels_and_styled_copy_in_a_small_component() {
    for content in ["评分：4.9", "Rating: 4.9/5", "星级 4.9分"] {
        assert!(rejection(&request(BRIEF), &[text("detail", content)]).is_some());
    }
    let nodes = vec![serde_json::from_value(json!({
        "type":"frame","id":"tag","children":[
            {"type":"text","id":"label","content":"评分"},
            {"type":"text","id":"value","content":[{"text":"4"},{"text":".9"}]}
        ]
    }))
    .unwrap()];
    assert!(rejection(&request(BRIEF), &nodes).is_some());
}

#[test]
fn source_facts_leaves_unrelated_numbers_favorites_placeholders_and_hidden_nodes_alone() {
    // The price is requested, so this fixture isolates rating false positives.
    let req = request(&format!("{BRIEF}保留价格字段。"));
    let mut nodes = vec![
        text("saturation", "4.9"),
        text("operating-cost", "4.9"),
        text("eta", "12"),
        text("price", "4.9"),
        text("version", "4.9"),
        text("rating-placeholder", "—"),
        text("rating-placeholder", "待填写"),
        text("rating-binding", "$app.rating"),
        text("rating-count", "4"),
        serde_json::from_value(
            json!({"type":"icon_font","id":"favorite","name":"Favorite","iconFontName":"star"}),
        )
        .unwrap(),
    ];
    nodes.push(
        serde_json::from_value(
            json!({"type":"frame","id":"hidden","opacity":0,"children":rating()}),
        )
        .unwrap(),
    );
    assert!(rejection(&req, &nodes).is_none());
    // A favorite star next to a price is not a rating component.
    let favorites = vec![serde_json::from_value(
        json!({"type":"frame","id":"favorites","children":nodes[..4]}),
    )
    .unwrap()];
    assert!(rejection(&req, &favorites).is_none());
    let count_only = vec![serde_json::from_value(json!({
        "type":"frame", "id":"rating-summary", "name":"rating-summary", "children":[{
            "type":"frame", "id":"rating-count", "name":"rating-count", "children":[{
                "type":"text", "id":"number", "content":"4"
            }]
        }]
    }))
    .unwrap()];
    assert!(rejection(&request(BRIEF), &count_only).is_none());
}

#[test]
fn source_facts_respects_explicit_ratings_fields_and_reviews_without_broad_demo_permission() {
    for brief in [
        "配送页，骑手评分4.8",
        "保留评分字段，值待填写",
        "商家详情有评价列表",
        "Show a rating field",
        "Show customer reviews",
        "星级选择器",
        "Create a fictional rider rating as demo data",
        "骑手李师傅 ★ 4.9",
        "A 5-star hotel detail page",
        "A customer satisfaction score field",
    ] {
        // This guard checks only unrequested additions, not exact-value fidelity.
        assert!(rejection(&request(brief), &rating()).is_none(), "{brief}");
    }
    for brief in [
        BRIEF,
        "Admin console: a users table of 10 rows × 6 columns (status pills, row actions).",
        "设计配送追踪页，订单表格填充示例数据。",
    ] {
        assert!(rejection(&request(brief), &rating()).is_some(), "{brief}");
    }
}

#[test]
fn source_facts_is_conservative_about_actual_reference_and_existing_document_content() {
    for brief in [
        "复刻这张截图的全部内容",
        "根据参考图制作骑手卡片",
        "Match the attached image",
        "Reproduce this screenshot",
        "参考 https://example.com/delivery 的内容",
        "照参考稿复刻全部内容",
        "Follow the reference design",
    ] {
        assert!(rejection(&request(brief), &rating()).is_none(), "{brief}");
    }
    let mut req = request(BRIEF);
    req.append_context = Some(crate::types::AppendContext {
        target_parent_id: "existing".into(),
        target_width: 375.0,
        existing_section_labels: vec!["骑手卡片".into()],
        is_mobile: true,
    });
    assert!(rejection(&req, &rating()).is_none());
    req.append_context = None;
    req.continuation_context = Some(crate::types::ContinuationContext {
        screen_width: 375.0,
        screen_height: 812.0,
        background_color: None,
        screen_names: vec!["配送追踪".into()],
    });
    assert!(rejection(&req, &rating()).is_none());
    req.continuation_context = None;
    req.reference_skeleton = Some(
        serde_json::from_value(json!({
            "source":"imported", "width":375, "sections":[],
            "navKind":"none", "heroKind":"none", "columnRhythm":[]
        }))
        .unwrap(),
    );
    assert!(
        rejection(&req, &rating()).is_some(),
        "a structure-only skeleton contains no source values"
    );
}

#[test]
fn source_facts_section_hint_is_specific_and_does_not_rewrite_the_brief_or_plan() {
    let req = request(BRIEF);
    let subtask = proposed_rating();
    let mut prompt = format!(
        "ORIGINAL USER BRIEF:\n{}\nEND ORIGINAL USER BRIEF",
        req.prompt
    );
    append_section_instruction(&req, &subtask, &mut prompt);
    assert!(prompt.contains(BRIEF));
    assert!(prompt.contains("THIS SECTION — UNSUPPORTED RATING"));
    assert_eq!(
        subtask.elements.as_deref(),
        Some("骑手姓名与评价标签，联系按钮")
    );
    for brief in ["骑手评分4.8", "参考图里的骑手卡片"] {
        let mut prompt = String::new();
        append_section_instruction(&request(brief), &subtask, &mut prompt);
        assert!(prompt.is_empty());
    }
    let mut no_rating = subtask;
    no_rating.elements = Some("骑手姓名、联系按钮".into());
    let mut prompt = String::new();
    append_section_instruction(&req, &no_rating, &mut prompt);
    assert!(prompt.is_empty());
}
