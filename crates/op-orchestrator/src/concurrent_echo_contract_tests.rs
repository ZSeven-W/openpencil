//! A geometry rewrite must preserve the content gates already passed by the original.

use super::*;
use crate::plan::{Region, RootFrameSpec};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use op_editor_core::{NodeId, PenNodeExt};
use serde_json::json;

fn fixture(
    cjk: bool,
) -> (
    VecDocSink,
    Subtask,
    OrchestratorPlan,
    DesignRequest,
    SubtaskOutcome,
) {
    let rows=(0..5).map(|index|json!({"type":"frame","id":format!("row-{index}"),"name":format!("Item {index}"),"layout":"horizontal","width":"fill_container","height":"fit_content","gap":8,
        "children":[{"type":"frame","id":format!("name-block-{index}"),"layout":"vertical","width":"fill_container","height":"fit_content","children":[{"type":"text","id":format!("name-{index}"),"name":"Name","content":if cjk {"这是一条必须完整保留的中文操作说明，不要因为修排版丢掉它。".to_string()}else{format!("Alexander Wellington Montgomery item {index}")},"fontSize":15}]},
        {"type":"frame","id":format!("time-block-{index}"),"width":"fit_content","height":"fit_content","children":[{"type":"text","id":format!("time-{index}"),"content":"9:00 AM","fontSize":13}]}]})).collect::<Vec<_>>();
    let doc=jian_ops_schema::load_str(&json!({"version":"1.0.0","children":[
        {"type":"frame","id":"original","name":"Original list","layout":"vertical","width":220,"height":"fit_content","children":rows},
        {"type":"frame","id":"unrelated","name":"User work","width":600,"height":100,"children":[{"type":"text","id":"keep","content":"Keep this work"}]}
    ]}).to_string()).unwrap().value;
    let mut sink = VecDocSink::new();
    sink.state = op_editor_core::EditorState::from_document(doc);
    let task = Subtask {
        id: "list".into(),
        label: if cjk {
            "中文列表".into()
        } else {
            "list of 5 items".into()
        },
        region: Region {
            width: 220.0,
            height: 500.0,
        },
        bleed_hero: false,
        id_prefix: "list".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    };
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "page".into(),
            name: "Page".into(),
            width: 220.0,
            height: 812.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![task.clone()],
        style_guide_name: None,
    };
    let req = DesignRequest {
        prompt: if cjk {
            "做一个中文列表页面，保持完整的中文说明".into()
        } else {
            "Create a list of 5 items with English copy".into()
        },
        model: Some("glm-5.3-flash".into()),
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
    let outcome = SubtaskOutcome {
        id: "list".into(),
        node_count: 1,
        error: None,
        inserted_root_ids: vec!["original".into()],
        headline: None,
        subtask: None,
    };
    (sink, task, plan, req, outcome)
}

fn replacement(count: usize) -> String {
    let mut script=r#"const root=I(null,{type:"frame",name:"Recovered list",layout:"vertical",width:220,height:"fit_content"});"#.to_string();
    for i in 0..count {
        script.push_str(&format!(r#"I(root,{{type:"frame",name:"Item {i}",layout:"vertical",width:"fill_container",height:"fit_content",children:[{{type:"text",content:"Recovered item {i}",fontSize:14}}]}});"#));
    }
    script
}

fn echo(count: usize, cjk: bool) -> (VecDocSink, SubtaskOutcome, Vec<Progress>) {
    let (mut sink, task, plan, req, outcome) = fixture(cjk);
    assert!(
        !crate::geometry_validation::geometry_diagnostics_for_roots(
            &sink.state,
            &outcome.inserted_root_ids
        )
        .is_empty(),
        "the original must actually need a layout echo"
    );
    let unchanged = sink.state.doc.children[1].clone();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(replacement(count))]);
    let mut events = Vec::new();
    let result = futures::executor::block_on(maybe_geometry_echo_with_outcomes(
        &task,
        &plan,
        &req,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
        None,
        &GeometryEchoBudget::new(1),
        &mut |e| events.push(e),
        &[],
        outcome,
    ));
    assert_eq!(
        op_editor_core::walkers::find_node(sink.state.active_children(), &NodeId::new("unrelated"))
            .unwrap(),
        &unchanged
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Progress::GeometryEcho { .. })));
    (sink, result, events)
}

#[test]
fn incomplete_geometry_rewrite_keeps_all_original_items() {
    let (sink, result, _) = echo(2, false);
    assert_eq!(result.inserted_root_ids, vec!["original"]);
    assert_eq!(
        crate::subtask_completeness::delivered_item_count(&sink, &result.inserted_root_ids),
        5
    );
    assert_eq!(
        sink.state.active_children().len(),
        2,
        "rejected replacement must be removed"
    );
}

#[test]
fn geometry_rewrite_in_the_wrong_language_keeps_original_copy() {
    let (_, result, _) = echo(5, true);
    assert_eq!(result.inserted_root_ids, vec!["original"]);
}

#[test]
fn complete_geometry_rewrite_replaces_only_the_original_subtree() {
    let (sink, result, _) = echo(5, false);
    assert!(op_editor_core::walkers::find_node(
        sink.state.active_children(),
        &NodeId::new("original")
    )
    .is_none());
    assert_eq!(
        crate::subtask_completeness::delivered_item_count(&sink, &result.inserted_root_ids),
        5
    );
    assert!(sink
        .state
        .active_children()
        .iter()
        .any(|n| n.base().name.as_deref() == Some("Recovered list")));
}
