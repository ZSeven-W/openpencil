//! Unsupported business claims must retry before any generated nodes land.

use super::*;
use jian_ops_schema::node::{PenNode, TextContent};
use serde_json::json;

const BRIEF: &str = "设计一个375×812配送追踪App页面，骑手阿明，提供联系骑手按钮。";
const REJECTION: &str = "unsupported-optional-rating";

#[path = "source_facts_order_retry_tests.rs"]
mod order_tests;

fn request() -> DesignRequest {
    DesignRequest {
        prompt: BRIEF.into(),
        model: Some("glm-5.3-flash".into()),
        validation_enabled: false,
        ..Default::default()
    }
}

fn plan() -> String {
    json!({
        "rootFrame": {
            "id": "root", "name": "配送追踪", "width": 375, "height": 812,
            "layout": "vertical", "gap": 0,
            "fill": [{"type": "solid", "color": "#F8FAFC"}]
        },
        "subtasks": [{
            "id": "courier", "label": "骑手信息",
            // A planner-added rating must not become evidence of a real rating.
            "elements": "骑手姓名、联系骑手按钮、姓名与评价标签",
            "region": {"width": 375, "height": 200}
        }]
    })
    .to_string()
}

fn script(with_rating: bool) -> String {
    let mut children = vec![json!({
        "type": "text", "name": "courier-name", "content": "骑手阿明",
        "fontFamily": "Noto Sans SC", "fontSize": 20, "fontWeight": 600,
        "width": "fill_container", "height": "fit_content"
    })];
    if with_rating {
        children.push(json!({
            "type": "frame", "name": "rating-tag", "layout": "horizontal",
            "width": "fit_content", "height": "fit_content", "gap": 4,
            "children": [{
                "type": "text", "name": "rating-value", "content": "4.9",
                "fontSize": 14, "width": "fit_content", "height": "fit_content"
            }]
        }));
    }
    children.push(json!({
        "type": "frame", "name": "contact-button", "role": "button",
        "layout": "horizontal", "width": "fill_container", "height": 44,
        "justifyContent": "center", "alignItems": "center", "cornerRadius": 10,
        "fill": [{"type": "solid", "color": "#0F172A"}],
        "children": [{
            "type": "text", "name": "contact-label", "content": "联系骑手",
            "fontFamily": "Noto Sans SC", "fontSize": 16,
            "width": "fit_content", "height": "fit_content",
            "fill": [{"type": "solid", "color": "#FFFFFF"}]
        }]
    }));
    format!(
        "I(null, {});",
        json!({
            "type": "frame", "name": "骑手信息", "layout": "vertical",
            "width": "fill_container", "height": "fit_content", "padding": 24,
            "gap": 16, "children": children
        })
    )
}

fn texts(nodes: &[PenNode]) -> Vec<String> {
    let mut result = Vec::new();
    for node in nodes {
        if let PenNode::Text(text) = node {
            result.push(match &text.content {
                TextContent::Plain(text) => text.clone(),
                TextContent::Styled(runs) => runs.iter().map(|run| run.text.as_str()).collect(),
            });
        }
        if let Some(children) = node.children() {
            result.extend(texts(children));
        }
    }
    result
}

fn inserted_rating(command: &EditorCommand) -> bool {
    match command {
        EditorCommand::InsertSubtree { nodes, .. }
        | EditorCommand::InsertAuthoredSubtree { nodes, .. } => {
            texts(nodes).iter().any(|text| text == "4.9")
        }
        EditorCommand::Batch { commands } => commands.iter().any(inserted_rating),
        _ => false,
    }
}

#[test]
fn source_facts_rating_retries_only_the_rejected_section_before_insertion() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(plan()),
        ScriptResponse::Text(script(true)),
        ScriptResponse::Text(script(false)),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let summary = futures::executor::block_on(Orchestrator::new().run(
        request(),
        &mut sink,
        &llm,
        &mut |event| events.push(event),
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("a corrected section should recover from the unsupported rating");

    assert_eq!(summary.subtasks.len(), 1);
    assert!(summary.subtasks[0].error.is_none());
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 3, "one plan and two section attempts");
    assert!(prompts[1].contains("姓名与评价标签"));
    assert!(prompts[2].contains(REJECTION));
    assert!(events.iter().any(|event| matches!(
        event,
        Progress::SubtaskRetry { id, attempt: 2, reason }
            if id == "courier" && reason.contains(REJECTION)
    )));
    let systems = llm.system_prompts();
    assert_eq!(
        systems[1], systems[2],
        "a content rejection must preserve the second attempt's full skills"
    );
    let delivered = texts(sink.state.active_children());
    for required in ["骑手阿明", "联系骑手"] {
        assert_eq!(
            delivered
                .iter()
                .filter(|text| text.as_str() == required)
                .count(),
            1,
            "the complete corrected section must retain {required} exactly once"
        );
    }
    assert!(!delivered.iter().any(|text| text == "4.9"));
    assert!(
        !sink.applied.iter().any(inserted_rating),
        "the unsupported rating must never land, even temporarily"
    );
    assert_eq!(sink.batch_depth, 0);
}

#[test]
fn source_facts_rating_rejects_every_attempt_including_final_salvage() {
    let llm = ScriptedLlm::new(
        std::iter::once(ScriptResponse::Text(plan()))
            .chain((0..4).map(|_| ScriptResponse::Text(script(true))))
            .collect(),
    );
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let result = futures::executor::block_on(Orchestrator::new().run(
        request(),
        &mut sink,
        &llm,
        &mut |event| events.push(event),
        &AbortFlag::new(),
        &stub_providers(),
    ));

    assert!(
        matches!(result, Err(OrchestratorError::AllFailed(ref reason)) if reason.contains(REJECTION)),
        "{result:?}"
    );
    let prompts = llm.user_prompts();
    assert_eq!(
        prompts.len(),
        5,
        "one plan, three attempts and final salvage"
    );
    assert!(prompts[2..].iter().all(|prompt| prompt.contains(REJECTION)));
    for expected_attempt in [2, 3, 4] {
        assert!(events.iter().any(|event| matches!(
            event,
            Progress::SubtaskRetry { id, attempt, .. }
                if id == "courier" && *attempt == expected_attempt
        )));
    }
    assert!(
        sink.state.active_children().is_empty(),
        "a factually rejected section must not survive as a completed work"
    );
    assert!(
        !sink.applied.iter().any(inserted_rating),
        "even final advisory-mode attempts must reject unsupported facts before insertion"
    );
    assert_eq!(sink.batch_depth, 0);
}
