//! `build_fallback_card_plan` — the branch that keeps a card series from
//! degenerating into a 1200-wide scrolling page when planning fails.

use super::*;
use crate::design_type::detect_design_type;

#[test]
fn an_explicit_single_card_is_a_complete_deliverable_not_a_two_card_series() {
    let plan = plan_for("做1张图文卡片，包含标题、说明和行动提示。");
    assert_eq!(plan.subtasks.len(), 1);
    assert_eq!(plan.subtasks[0].label, "单张作品");
    assert!(plan.subtasks[0]
        .elements
        .as_ref()
        .unwrap()
        .contains("complete standalone deliverable"));
}

#[test]
fn home_poster_default_is_single_but_user_quantity_still_wins() {
    assert_eq!(
        plan_for("请做活动海报（card，竖版图文）。用户需求：介绍内部体验会。")
            .subtasks
            .len(),
        1
    );
    assert_eq!(
        plan_for("请做活动海报（card，竖版图文）。用户需求：做3张活动图。")
            .subtasks
            .len(),
        3
    );
}

fn req(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.to_string(),
        ..Default::default()
    }
}

fn plan_for(prompt: &str) -> OrchestratorPlan {
    build_fallback_card_plan(&req(prompt), detect_design_type(prompt))
}

#[test]
fn every_card_is_its_own_screen_at_the_card_spec_size() {
    let plan = plan_for("帮我做一套小红书卡片：如何早起");
    assert_eq!(plan.root_frame.width, 1080.0);
    assert_eq!(plan.root_frame.height, 1440.0);
    assert!(!plan.subtasks.is_empty());
    for subtask in &plan.subtasks {
        assert!(
            subtask.screen.is_some(),
            "a card without a screen tag collapses into the shared root: {subtask:?}"
        );
        assert_eq!(subtask.region.width, 1080.0);
        assert_eq!(subtask.region.height, 1440.0);
    }
}

#[test]
fn the_run_opens_on_a_cover_and_closes_on_an_action() {
    let plan = plan_for("做一套小红书图文讲复利，内容比较长需要展开说明每一个要点和常见误区");
    let labels: Vec<&str> = plan.subtasks.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels.first(), Some(&"封面"));
    assert_eq!(labels.last(), Some(&"收尾"));
}

#[test]
fn an_explicit_count_is_honoured_in_card_units() {
    // 张 / 图 / cards — never 页, which is the deck's unit.
    assert_eq!(plan_for("小红书卡片 7 张，主题早起").subtasks.len(), 7);
    assert_eq!(plan_for("做一套小红书图文 4 图").subtasks.len(), 4);
    assert_eq!(plan_for("an xhs carousel, 6 cards").subtasks.len(), 6);
}

#[test]
fn an_unstated_count_scales_with_the_brief() {
    let short = plan_for("小红书卡片：早起").subtasks.len();
    let long = plan_for(&format!("小红书卡片：早起。{}", "详细说明".repeat(60)))
        .subtasks
        .len();
    assert!(short >= 2, "{short}");
    assert!(
        long > short,
        "a longer brief earns more cards: {short} vs {long}"
    );
}
