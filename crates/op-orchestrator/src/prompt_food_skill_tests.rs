//! Food-delivery corpus delivery guards — "the corpus was edited" vs "the
//! model saw it", read off the FINAL assembled `system_prompt`.
//!
//! Measured before these guards (2026-09-24): the delivery-app brief also asks
//! for motion and interaction, so every mobile subtask resolved
//! `scroll-orchestration` + `interactivity` + `kinetic-typography` and dropped
//! `mobile-app` for budget — the screen's own domain rules never reached GLM or
//! Opus while the motion rules did. A new `food-delivery` skill would have
//! been dropped the same way; the phase default moved 17150 → 20300 so both fit.

use super::*;
use crate::plan::{Region, RootFrameSpec};

/// The measured delivery brief, verbatim — its motion / interaction phrases
/// are what pull the competing skills in.
const DELIVERY_BRIEF: &str = "外卖 App 两屏可交互（375×812）：首页(地址栏+分类九宫格+限时横幅+商家列表五条)、\
商家详情(头图+菜品分类横滑+菜品列表六条+购物车条)。交互：商家行 onTap 进详情。动效：限时横幅倒计时数字滚动，\
商家卡 inView 交错 fade-up，加入购物车按钮 pressed 态缩放。 配色与质感由你根据产品类型和使用场景决定；渐变只在合理处使用。";

fn phone_plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "detail".into(),
            name: "商家详情".into(),
            width: 375.0,
            height: 812.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn dish_list_subtask() -> crate::plan::Subtask {
    crate::plan::Subtask {
        id: "dish-list".into(),
        label: "菜品列表".into(),
        region: Region {
            width: 375.0,
            height: 640.0,
        },
        bleed_hero: false,
        id_prefix: "dish-list".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: Some(
            "6 dish rows each with 80px food photo, dish name, 月售, price, add button".into(),
        ),
        screen: Some("商家详情".into()),
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    }
}

fn request(model: &str) -> DesignRequest {
    DesignRequest {
        prompt: DELIVERY_BRIEF.into(),
        model: Some(model.into()),
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

fn skill_tail(name: &str) -> &'static str {
    op_ai_skills::get_skill_by_name(name)
        .unwrap_or_else(|| panic!("{name} must be registered"))
        .content
        .trim_end()
        .lines()
        .last()
        .expect("skill body is non-empty")
}

/// GLM-5.3-Flash and Opus 5.5 (both Full tier) — the two models the delivery
/// runs used — receive the food and mobile domain skills whole.
#[test]
fn a_motion_bearing_delivery_brief_keeps_its_domain_skills_whole() {
    for model in ["claude-opus-5-5", "glm-5.3-flash"] {
        let (call, report) = build_subagent_prompt(
            &dish_list_subtask(),
            &phone_plan(),
            &request(model),
            AbortFlag::new(),
            false,
            false,
            &op_editor_core::ComponentLibrary::default(),
        );
        let loaded: Vec<&str> = report.included.iter().map(|e| e.name.as_str()).collect();
        for name in ["food-delivery", "mobile-app"] {
            assert!(
                call.system_prompt.contains(skill_tail(name)),
                "model {model:?}: {name} was dropped or tail-truncated \
                 ({}/{} tokens; loaded {loaded:?}; dropped {:?})",
                report.budget_used,
                report.budget_max,
                report.dropped,
            );
        }
        assert!(
            call.system_prompt.contains("LEFT CATEGORY RAIL"),
            "model {model:?}: the store-page menu rule is missing"
        );
    }
}

/// The skill is keyword-gated: a non-food mobile brief does not pay for it.
#[test]
fn a_non_food_mobile_brief_does_not_load_the_food_skill() {
    let mut req = request("claude-opus-5-5");
    req.prompt = "健身打卡 App 首页（375×812）：今日训练、连续打卡天数、本周目标进度".into();
    let (call, _report) = build_subagent_prompt(
        &dish_list_subtask_renamed(),
        &phone_plan(),
        &req,
        AbortFlag::new(),
        false,
        false,
        &op_editor_core::ComponentLibrary::default(),
    );
    assert!(!call.system_prompt.contains("LEFT CATEGORY RAIL"));
}

fn dish_list_subtask_renamed() -> crate::plan::Subtask {
    let mut s = dish_list_subtask();
    s.label = "今日训练".into();
    s.elements = Some("workout cards with duration and progress".into());
    s.screen = Some("首页".into());
    s
}
