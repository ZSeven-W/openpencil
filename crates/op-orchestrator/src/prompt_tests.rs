use super::*;
use crate::plan::{Region, RootFrameSpec};
use op_editor_core::ComponentLibrary;

/// Test shim: the production `build_subagent_prompt` gained a `components`
/// param (the AVAILABLE COMPONENTS manifest source). The vast majority of
/// these tests predate it and exercise the no-component path, so this forwards
/// an empty registry — behaviour identical to before that param existed.
/// Component-aware behaviour is covered by the dedicated `available_components_*`
/// tests, which call `build_subagent_prompt` directly with a populated library.
fn bsp(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    abort: AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
) -> (CallRequest, SkillLoadReport) {
    build_subagent_prompt(
        subtask,
        plan,
        req,
        abort,
        reduced_complexity,
        minimal_skills,
        &ComponentLibrary::default(),
    )
}

fn req() -> DesignRequest {
    DesignRequest {
        prompt: "a pricing page".into(),
        model: Some("claude".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
        pinned_style_guide: None,
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

fn subtask() -> crate::plan::Subtask {
    crate::plan::Subtask {
        id: "s".into(),
        label: "Section".into(),
        region: crate::plan::Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "s".into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

// Cluster test modules — this file keeps the shared fixtures; each child
// mounts with `use super::*` so it sees both them and `crate::prompt`.
#[path = "prompt_components_tests.rs"]
mod components_tests;
#[path = "prompt_deck_skill_tests.rs"]
mod deck_skill_tests;
#[path = "prompt_overlay_skill_tests.rs"]
mod overlay_skill_tests;
#[path = "prompt_planning_tests.rs"]
mod planning_tests;
#[path = "prompt_skill_budget_tests.rs"]
mod skill_budget_tests;
#[path = "prompt_subagent_content_tests.rs"]
mod subagent_content_tests;
#[path = "prompt_timeout_feedback_tests.rs"]
mod timeout_feedback_tests;
