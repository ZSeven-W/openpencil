use super::*;
use crate::plan::{RootFrameSpec, Subtask};

fn subtask(id: &str, label: &str, elements: &str) -> Subtask {
    serde_json::from_value(serde_json::json!({
        "id": id, "label": label, "elements": elements,
        "region": {"width": 1180, "height": 300}
    }))
    .expect("subtask")
}

fn plan(width: f64, subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Console".into(),
            width,
            height: 900.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks,
        style_guide_name: None,
    }
}

const D03_BRIEF: &str = "IoT 设备监控台（1440×900）：侧栏、顶部状态摘要三卡、设备网格 12 个\
（图标+名称+在线状态+数值）、右侧告警列表五条带时间与等级色标。";

/// arena-d03's plan as Opus 5.5 wrote it (2026-10-01): the brief's 右侧 is gone.
fn d03_subtasks() -> Vec<Subtask> {
    vec![
        subtask("sidebar", "Sidebar", "vertical rail: brand logo, nav items"),
        subtask(
            "main",
            "Status Summary & Device Grid",
            "3 status summary cards (在线设备, 离线设备, 今日告警); device grid 12",
        ),
        subtask(
            "alerts",
            "Alert List",
            "panel title 告警列表 + view-all link; 5 alert items with severity tags",
        ),
    ]
}

#[test]
fn the_region_the_brief_puts_on_the_right_becomes_the_rail() {
    let mut plan = plan(1440.0, d03_subtasks());

    assert_eq!(mark_brief_right_side_subtasks(&mut plan, D03_BRIEF), 1);

    assert_eq!(plan.subtasks[2].label, "Alert List (right panel)");
    assert!(is_right_rail_subtask(&plan.subtasks[2]));
    assert!(!is_right_rail_subtask(&plan.subtasks[1]));
    assert!(crate::scaffold_right_rail::plan_right_rail_width(&plan).is_some());
}

#[test]
fn the_region_noun_stops_at_its_count() {
    assert_eq!(
        brief_right_side_regions(D03_BRIEF),
        vec!["告警列表".to_string()]
    );
    assert_eq!(
        brief_right_side_regions("Dashboard with a right-side activity feed, and charts"),
        vec!["activity feed".to_string()]
    );
}

#[test]
fn a_phone_brief_keeps_its_single_column() {
    let mut plan = plan(390.0, d03_subtasks());
    assert_eq!(mark_brief_right_side_subtasks(&mut plan, D03_BRIEF), 0);
    assert_eq!(plan.subtasks[2].label, "Alert List");
}

#[test]
fn a_plan_that_already_names_its_rail_is_left_alone() {
    let mut subtasks = d03_subtasks();
    subtasks[2].label = "右侧告警面板".into();
    let mut plan = plan(1440.0, subtasks);
    assert_eq!(mark_brief_right_side_subtasks(&mut plan, D03_BRIEF), 0);
    assert_eq!(plan.subtasks[2].label, "右侧告警面板");
}

#[test]
fn an_ambiguous_match_moves_nothing() {
    let mut subtasks = d03_subtasks();
    subtasks[1].elements = Some("device grid; 告警列表 summary chip".into());
    let mut plan = plan(1440.0, subtasks);
    assert_eq!(mark_brief_right_side_subtasks(&mut plan, D03_BRIEF), 0);
}

#[test]
fn a_brief_without_a_right_side_region_changes_nothing() {
    let mut plan = plan(1440.0, d03_subtasks());
    let brief = "IoT 设备监控台：侧栏、状态摘要三卡、设备网格、告警列表五条。";
    assert_eq!(mark_brief_right_side_subtasks(&mut plan, brief), 0);
}
