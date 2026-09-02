//! The scroll-orchestration budget arm: a weak-model tier asked for a
//! scrolling landing page must receive the `scroll-orchestration` skill
//! whole. Under the plain Basic 5200 / Standard 6500 arms it was dropped
//! for `BudgetExhausted` on every scroll prompt, silently — the file on
//! disk stayed correct while no weak model ever saw it.

use super::*;
use crate::plan::{Region, RootFrameSpec};

fn page_plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "page".into(),
            name: "Page".into(),
            width: 1440.0,
            height: 0.0,
            layout: Some("vertical".into()),
            gap: Some(0.0),
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn hero_subtask() -> crate::plan::Subtask {
    crate::plan::Subtask {
        id: "hero".into(),
        label: "Hero Section with Parallax".into(),
        region: Region {
            width: 1440.0,
            height: 720.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn request(prompt: &str, model: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        model: Some(model.into()),
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

const SCROLL_PROMPT: &str =
    "做一个桌面端 landing 页，要有滚动视差效果，顶部导航 sticky，卡片交错入场";

fn loaded_untruncated(report: &SkillLoadReport, name: &str) -> bool {
    report
        .included
        .iter()
        .any(|entry| entry.name == name && !entry.truncated)
}

/// Basic (glm-4.6) and Standard (gpt-4o) tiers both get the whole skill.
#[test]
fn weak_tiers_keep_the_scroll_skill_whole_on_a_scroll_prompt() {
    for model in ["glm-4.6", "gpt-4o"] {
        let (call, report) = build_subagent_prompt(
            &hero_subtask(),
            &page_plan(),
            &request(SCROLL_PROMPT, model),
            AbortFlag::new(),
            false,
            false,
            &op_editor_core::ComponentLibrary::default(),
        );
        assert!(
            loaded_untruncated(&report, "scroll-orchestration"),
            "model {model:?}: scroll-orchestration missing or truncated; loaded {:?} ({}/{})",
            report
                .included
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>(),
            report.budget_used,
            report.budget_max,
        );
        assert!(
            call.system_prompt.contains("END SCROLL ORCHESTRATION"),
            "model {model:?}: the corpus tail must reach the prompt"
        );
    }
}

/// An ordinary page keeps its plain tier arm: the skill is keyword-gated,
/// so it never appears, and the budget stays at the TS-parity number.
#[test]
fn a_plain_page_prompt_is_unchanged_by_the_scroll_arm() {
    let (_call, report) = build_subagent_prompt(
        &hero_subtask(),
        &page_plan(),
        &request("做一个 SaaS 产品定价页，三档套餐卡片", "glm-4.6"),
        AbortFlag::new(),
        false,
        false,
        &op_editor_core::ComponentLibrary::default(),
    );
    assert!(!report
        .included
        .iter()
        .any(|entry| entry.name == "scroll-orchestration"));
    assert_eq!(report.budget_max, 5200, "the plain Basic arm is untouched");
}
