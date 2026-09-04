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
        id_prefix: "hero".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

// A single I(null, {...}) call whose node object nests its children inline
// (batch_design's insert accepts a whole subtree per call). Authored ids
// are dropped: the batch_design executor reassigns fresh ids to every
// inserted node regardless, so tests that use this constant must not assert
// on literal id strings.
const NODE_SCRIPT: &str = r#"I(null, {"type":"frame","name":"Card","x":0,"y":0,"width":1200,"height":200,"children":[{"type":"text","content":"Hero","fontSize":18}]});"#;

#[path = "subagent_coalesce_tests.rs"]
mod coalesce_tests;
#[path = "subagent_run_subtask_tests.rs"]
mod run_subtask_tests;
