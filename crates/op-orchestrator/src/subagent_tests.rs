//! Sub-agent tests — shared fixtures live here; the per-cluster cases are
//! mounted as child modules below.

use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::LlmError;
use futures::executor::block_on;
use jian_ops_schema::node::PenNode;

fn req() -> DesignRequest {
    DesignRequest {
        prompt: "a page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn subtask() -> Subtask {
    Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: Region {
            width: 1200.0,
            height: 400.0,
        },
        bleed_hero: false,
        id_prefix: "hero".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    }
}

// A single I(null, {...}) call whose node object nests its children inline
// (batch_design's insert accepts a whole subtree per call). Authored ids
// are dropped: the batch_design executor reassigns fresh ids to every
// inserted node regardless, so tests that use this constant must not assert
// on literal id strings.
const NODE_SCRIPT: &str = r#"I(null, {"type":"frame","name":"Card","x":0,"y":0,"width":1200,"height":200,"children":[{"type":"text","content":"Hero","fontSize":18}]});"#;

// ── rejection-log subtree digest ─────────────────────────────────────────────

#[test]
fn rejected_subtree_summary_covers_the_node_and_its_direct_children() {
    let node = serde_json::json!({
        "type":"frame","id":"n9","name":"breath-ring",
        "width":220,"height":220,"layout":"none",
        "children":[
            {"type":"text","id":"timer","name":"计时","x":70,"y":80,"width":80,"height":40},
            {"type":"ellipse","id":"progress","name":"Ring Progress",
             "x":10,"y":0,"width":220,"height":220,"layout":null},
            {"type":"frame","id":"halo","name":"Halo","children":[
                {"type":"ellipse","id":"grandchild","width":10,"height":10}
            ]}
        ]
    });

    let summary = rejected_subtree_summary(&node);

    assert!(
        summary.contains("\"id\":\"n9\""),
        "rejected node: {summary}"
    );
    assert!(
        summary.contains("\"id\":\"timer\""),
        "direct child: {summary}"
    );
    assert!(
        summary.contains("\"id\":\"progress\""),
        "direct child: {summary}"
    );
    assert!(summary.contains("\"x\":10"), "child geometry: {summary}");
    assert!(
        !summary.contains("\"id\":\"grandchild\""),
        "depth stops at direct children"
    );
    assert!(!summary.contains('\n'), "must stay one line");
}

#[test]
fn rejected_subtree_summary_truncates_beyond_twelve_children() {
    let mut node = serde_json::json!({
        "type":"frame","id":"big","width":300,"height":300,
        "children":[]
    });
    let kids = node["children"].as_array_mut().expect("children");
    for index in 0..14 {
        kids.push(serde_json::json!({
            "type":"text","id":format!("c{index}"),"content":"x"
        }));
    }

    let summary = rejected_subtree_summary(&node);

    assert!(
        summary.contains("\"id\":\"c11\""),
        "12th child kept: {summary}"
    );
    assert!(
        !summary.contains("\"id\":\"c12\""),
        "13th child dropped: {summary}"
    );
    assert!(
        summary.contains("\"+2 more\""),
        "truncation marker: {summary}"
    );
}

#[path = "subagent_coalesce_tests.rs"]
mod coalesce_tests;
#[path = "subagent_run_subtask_tests.rs"]
mod run_subtask_tests;
