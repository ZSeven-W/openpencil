use super::*;
use crate::plan::RootFrameSpec;

fn subtask(id: &str, label: &str, width: f64) -> Subtask {
    serde_json::from_value(serde_json::json!({
        "id": id, "label": label, "region": {"width": width, "height": 900}
    }))
    .expect("subtask")
}

fn plan(width: f64, subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Workspace".into(),
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

/// arena-w01's plan, verbatim labels.
fn w01_subtasks(drawer_width: f64) -> Vec<Subtask> {
    vec![
        subtask("sidebar", "项目侧栏", 260.0),
        subtask("top-tabs", "顶部标签切换", 1180.0),
        subtask("kanban-columns", "看板三列各四张任务卡", 1180.0),
        subtask("task-detail-drawer", "右侧任务详情抽屉", drawer_width),
    ]
}

#[test]
fn right_side_cues_are_recognized() {
    for (id, label) in [
        ("task-detail-drawer", "右侧任务详情抽屉"),
        ("inspector", "Properties Inspector"),
        ("panel", "Right Panel"),
        ("x", "Order details panel"),
    ] {
        assert!(is_right_rail_subtask(&subtask(id, label, 400.0)), "{label}");
    }
    for (id, label) in [
        ("kanban-columns", "看板三列各四张任务卡"),
        ("hero", "Bright hero banner"),
        ("copyright", "Copyright footer"),
        ("sidebar", "Left sidebar navigation"),
    ] {
        assert!(
            !is_right_rail_subtask(&subtask(id, label, 400.0)),
            "{label}"
        );
    }
}

#[test]
fn w01_plan_gets_a_rail_with_a_sane_width() {
    assert_eq!(
        plan_right_rail_width(&plan(1440.0, w01_subtasks(420.0))),
        Some(420.0)
    );
    // A full-width region from the planner is not a rail width.
    assert_eq!(
        plan_right_rail_width(&plan(1440.0, w01_subtasks(1180.0))),
        Some(400.0)
    );
}

#[test]
fn narrow_artboard_or_rail_only_plans_keep_two_columns() {
    assert_eq!(
        plan_right_rail_width(&plan(1024.0, w01_subtasks(420.0))),
        None
    );
    let only_rail = vec![
        subtask("sidebar", "项目侧栏", 260.0),
        subtask("task-detail-drawer", "右侧任务详情抽屉", 420.0),
    ];
    assert_eq!(plan_right_rail_width(&plan(1440.0, only_rail)), None);
    let no_rail = vec![
        subtask("sidebar", "项目侧栏", 260.0),
        subtask("kanban-columns", "看板三列各四张任务卡", 1180.0),
    ];
    assert_eq!(plan_right_rail_width(&plan(1440.0, no_rail)), None);
}
