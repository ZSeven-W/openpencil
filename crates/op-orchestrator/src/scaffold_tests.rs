use super::*;
use crate::concurrent::ScreenGroup;
use crate::plan::{OrchestratorPlan, PlanFill, Region, RootFrameSpec, Subtask};
use op_editor_core::PenNodeExt;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Design".into(),
            width: 1200.0,
            height: 800.0,
            layout: Some("vertical".into()),
            gap: Some(0.0),
            padding: Some(0.0),
            fill: Some(vec![PlanFill {
                kind: "solid".into(),
                color: "#FFFFFF".into(),
            }]),
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn plan_with_subtasks(subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan { subtasks, ..plan() }
}

fn subtask(id: &str, screen: Option<&str>, height: f64) -> Subtask {
    Subtask {
        id: id.into(),
        label: id.into(),
        id_prefix: id.into(),
        region: Region {
            width: 1200.0,
            height,
        },
        parent_frame_id: None,
        elements: None,
        screen: screen.map(|s| s.to_string()),
        generated_root_id: None,
        existing_section_labels: None,
    }
}

// ── existing single-root tests (must stay green) ───────────────────────────

#[test]
fn build_scaffold_desktop_one_root_no_children() {
    let cmds = build_scaffold(&plan(), false).expect("scaffold");
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        EditorCommand::InsertSubtree { nodes, parent_id } => {
            assert_eq!(nodes.len(), 1);
            assert!(!parent_id.is_real()); // NONE → page root
            assert_eq!(nodes[0].id_str(), "root");
            assert!(nodes[0].children().map(|c| c.is_empty()).unwrap_or(true));
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}

#[test]
fn build_scaffold_mobile_injects_status_bar() {
    let cmds = build_scaffold(&plan(), true).expect("scaffold");
    match &cmds[0] {
        EditorCommand::InsertSubtree { nodes, .. } => {
            let children = nodes[0].children().expect("frame children");
            assert_eq!(children.len(), 1);
            let status_json = serde_json::to_value(&children[0]).expect("status json");
            assert_eq!(status_json["role"], "status-bar");
            assert_eq!(status_json["height"], 62.0);

            let status_children = status_json["children"]
                .as_array()
                .expect("status bar children");
            assert_eq!(status_children.len(), 2);
            assert_eq!(status_children[0]["name"], "Time");
            assert_eq!(status_children[0]["children"][0]["content"], "9:41");
            assert_eq!(status_children[1]["name"], "Levels");
            assert_eq!(status_children[1]["children"].as_array().unwrap().len(), 3);
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}

#[test]
fn build_scaffold_mobile_status_bar_uses_fixed_icon_positions() {
    let cmds = build_scaffold(&plan(), true).expect("scaffold");
    match &cmds[0] {
        EditorCommand::InsertSubtree { nodes, .. } => {
            let status_json = serde_json::to_value(&nodes[0].children().expect("children")[0])
                .expect("status json");
            assert_eq!(
                status_json["layout"], "none",
                "status-bar chrome must not depend on auto-layout; path icons overlap in the native renderer when it does"
            );

            let levels = &status_json["children"][1];
            assert_eq!(levels["name"], "Levels");
            assert_eq!(levels["layout"], "none");
            assert!(levels["x"].as_f64().unwrap_or(0.0) > 280.0);

            let icons = levels["children"].as_array().expect("levels children");
            let xs = icons
                .iter()
                .map(|icon| icon["x"].as_f64().expect("icon x"))
                .collect::<Vec<_>>();
            assert!(
                xs.windows(2).all(|pair| pair[1] > pair[0]),
                "status-bar icons must have increasing explicit x positions, got {xs:?}"
            );
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}

#[test]
fn build_scaffold_mobile_status_bar_clamps_levels_to_explicit_narrow_width() {
    // iPhone SE: 320 wide. The pre-fix scaffold hardcoded levels.x=286
    // which put the right-aligned chrome (cellular/wifi/battery, 78 wide)
    // at x=286..364 — 44 px off-screen on a 320-wide root.
    let mut narrow = plan();
    narrow.root_frame.width = 320.0;
    let cmds = build_scaffold(&narrow, true).expect("scaffold");
    match &cmds[0] {
        EditorCommand::InsertSubtree { nodes, .. } => {
            let status_json = serde_json::to_value(&nodes[0].children().expect("children")[0])
                .expect("status json");
            let levels = &status_json["children"][1];
            let levels_x = levels["x"].as_f64().expect("levels x");
            let levels_w = levels["width"].as_f64().expect("levels width");
            assert!(
                levels_x + levels_w <= 320.0,
                "levels right edge {} overflows root width 320",
                levels_x + levels_w
            );
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}

#[test]
fn build_scaffold_single_root_uses_safe_canvas_offset() {
    let cmds = build_scaffold(&plan(), true).expect("scaffold");
    match &cmds[0] {
        EditorCommand::InsertSubtree { nodes, .. } => {
            assert_eq!(nodes[0].base().x, Some(80.0));
            assert_eq!(nodes[0].base().y, Some(40.0));
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}

// ── Task A2: concurrent scaffold tests ────────────────────────────────────

/// 3 screen groups → 3 root frames.
#[test]
fn concurrent_scaffold_three_groups_gives_three_roots() {
    let subtasks = vec![
        subtask("a", Some("login"), 400.0),
        subtask("b", Some("home"), 600.0),
        subtask("c", Some("profile"), 500.0),
    ];
    let plan = plan_with_subtasks(subtasks);
    let groups = vec![
        ScreenGroup {
            screen: "login".into(),
            indices: vec![0],
        },
        ScreenGroup {
            screen: "home".into(),
            indices: vec![1],
        },
        ScreenGroup {
            screen: "profile".into(),
            indices: vec![2],
        },
    ];
    let (cmds, root_ids, baselines) =
        build_scaffold_concurrent_mobile(&plan, &groups, false).expect("concurrent scaffold");
    assert_eq!(cmds.len(), 3, "expected 3 InsertSubtree commands");
    assert_eq!(root_ids.len(), 3);
    assert_eq!(baselines.len(), 3);
    // All 3 are InsertSubtree commands.
    for cmd in &cmds {
        assert!(matches!(cmd, EditorCommand::InsertSubtree { .. }));
    }
}

/// Frame 2's x = width + 100, frame 3's x = 2*(width+100).
#[test]
fn concurrent_scaffold_x_layout_left_to_right_gap_100() {
    let subtasks = vec![
        subtask("a", Some("s1"), 400.0),
        subtask("b", Some("s2"), 400.0),
        subtask("c", Some("s3"), 400.0),
    ];
    let plan = plan_with_subtasks(subtasks);
    let groups = vec![
        ScreenGroup {
            screen: "s1".into(),
            indices: vec![0],
        },
        ScreenGroup {
            screen: "s2".into(),
            indices: vec![1],
        },
        ScreenGroup {
            screen: "s3".into(),
            indices: vec![2],
        },
    ];
    let (cmds, _, _) = build_scaffold_concurrent_mobile(&plan, &groups, false).expect("scaffold");
    let xs: Vec<f64> = cmds
        .iter()
        .map(|cmd| match cmd {
            EditorCommand::InsertSubtree { nodes, .. } => nodes[0].base().x.unwrap_or(0.0),
            _ => panic!("expected InsertSubtree"),
        })
        .collect();
    assert_eq!(xs[0], 0.0, "frame 0 x should be 0");
    assert_eq!(
        xs[1], 1300.0,
        "frame 1 x should be width(1200)+gap(100)=1300"
    );
    assert_eq!(xs[2], 2600.0, "frame 2 x should be 2*(1200+100)=2600");
}

/// Each group's root ID is assigned; root_ids are distinct.
#[test]
fn concurrent_scaffold_root_ids_are_distinct() {
    let subtasks = vec![
        subtask("a", Some("login"), 400.0),
        subtask("b", Some("home"), 600.0),
    ];
    let plan = plan_with_subtasks(subtasks);
    let groups = vec![
        ScreenGroup {
            screen: "login".into(),
            indices: vec![0],
        },
        ScreenGroup {
            screen: "home".into(),
            indices: vec![1],
        },
    ];
    let (_, root_ids, _) =
        build_scaffold_concurrent_mobile(&plan, &groups, false).expect("scaffold");
    assert_eq!(root_ids.len(), 2);
    assert_ne!(root_ids[0], root_ids[1], "root IDs must be distinct");
}

/// Desktop height = max(320, Σ region heights) per group.
#[test]
fn concurrent_scaffold_desktop_height_is_max_320_sum_region_heights() {
    // Group 0: subtask heights 200 + 300 = 500 → height 500 (≥320)
    // Group 1: subtask height 100 → height 320 (clamped to min 320)
    let subtasks = vec![
        subtask("a", Some("s1"), 200.0),
        subtask("b", Some("s1"), 300.0),
        subtask("c", Some("s2"), 100.0),
    ];
    let plan = plan_with_subtasks(subtasks);
    let groups = vec![
        ScreenGroup {
            screen: "s1".into(),
            indices: vec![0, 1],
        },
        ScreenGroup {
            screen: "s2".into(),
            indices: vec![2],
        },
    ];
    let (cmds, _, _) = build_scaffold_concurrent_mobile(&plan, &groups, false).expect("scaffold");
    let heights: Vec<f64> = cmds
        .iter()
        .map(|cmd| match cmd {
            EditorCommand::InsertSubtree { nodes, .. } => nodes[0].height_px().unwrap_or(0.0),
            _ => panic!("expected InsertSubtree"),
        })
        .collect();
    assert_eq!(heights[0], 500.0, "group 0 height should be sum=500");
    assert_eq!(heights[1], 320.0, "group 1 height should be min=320");
}

/// Mobile height = plan.root_frame.height (or 812 if 0).
#[test]
fn concurrent_scaffold_mobile_height_uses_root_frame_height() {
    let subtasks = vec![
        subtask("a", Some("s1"), 200.0),
        subtask("b", Some("s2"), 300.0),
    ];
    let plan = plan_with_subtasks(subtasks);
    let groups = vec![
        ScreenGroup {
            screen: "s1".into(),
            indices: vec![0],
        },
        ScreenGroup {
            screen: "s2".into(),
            indices: vec![1],
        },
    ];
    let (cmds, _, _) = build_scaffold_concurrent_mobile(&plan, &groups, true).expect("scaffold");
    // plan.root_frame.height = 800.0, so both roots should have height 800
    for cmd in &cmds {
        match cmd {
            EditorCommand::InsertSubtree { nodes, .. } => {
                assert_eq!(
                    nodes[0].height_px().unwrap_or(0.0),
                    800.0,
                    "mobile height should equal root_frame.height"
                );
            }
            _ => panic!("expected InsertSubtree"),
        }
    }
}

/// Mobile scaffold baseline = 1 per root (status bar injected).
#[test]
fn concurrent_scaffold_mobile_baseline_is_one_per_root() {
    let subtasks = vec![
        subtask("a", Some("s1"), 200.0),
        subtask("b", Some("s2"), 300.0),
    ];
    let plan = plan_with_subtasks(subtasks);
    let groups = vec![
        ScreenGroup {
            screen: "s1".into(),
            indices: vec![0],
        },
        ScreenGroup {
            screen: "s2".into(),
            indices: vec![1],
        },
    ];
    let (cmds, _, baselines) =
        build_scaffold_concurrent_mobile(&plan, &groups, true).expect("scaffold");
    assert_eq!(baselines, vec![1, 1], "mobile baselines should be 1 each");
    // Each root frame should have 1 child (status bar).
    for cmd in &cmds {
        match cmd {
            EditorCommand::InsertSubtree { nodes, .. } => {
                let children = nodes[0].children().expect("frame has children");
                assert_eq!(
                    children.len(),
                    1,
                    "mobile root should have status bar child"
                );
            }
            _ => panic!("expected InsertSubtree"),
        }
    }
}

/// Desktop scaffold baseline = 0 per root.
#[test]
fn concurrent_scaffold_desktop_baseline_is_zero_per_root() {
    let subtasks = vec![
        subtask("a", Some("s1"), 400.0),
        subtask("b", Some("s2"), 400.0),
        subtask("c", Some("s3"), 400.0),
    ];
    let plan = plan_with_subtasks(subtasks);
    let groups = vec![
        ScreenGroup {
            screen: "s1".into(),
            indices: vec![0],
        },
        ScreenGroup {
            screen: "s2".into(),
            indices: vec![1],
        },
        ScreenGroup {
            screen: "s3".into(),
            indices: vec![2],
        },
    ];
    let (_, _, baselines) =
        build_scaffold_concurrent_mobile(&plan, &groups, false).expect("scaffold");
    assert_eq!(
        baselines,
        vec![0, 0, 0],
        "desktop baselines should be 0 each"
    );
}

/// Frame name = group's screen string (when first subtask has a screen).
#[test]
fn concurrent_scaffold_frame_name_uses_screen_string() {
    let subtasks = vec![
        subtask("a", Some("Login Screen"), 400.0),
        subtask("b", Some("Home Screen"), 400.0),
    ];
    let plan = plan_with_subtasks(subtasks);
    let groups = vec![
        ScreenGroup {
            screen: "Login Screen".into(),
            indices: vec![0],
        },
        ScreenGroup {
            screen: "Home Screen".into(),
            indices: vec![1],
        },
    ];
    let (cmds, _, _) = build_scaffold_concurrent_mobile(&plan, &groups, false).expect("scaffold");
    let names: Vec<String> = cmds
        .iter()
        .map(|cmd| match cmd {
            EditorCommand::InsertSubtree { nodes, .. } => {
                nodes[0].base().name.clone().unwrap_or_default()
            }
            _ => panic!("expected InsertSubtree"),
        })
        .collect();
    assert_eq!(names[0], "Login Screen");
    assert_eq!(names[1], "Home Screen");
}

/// All InsertSubtree commands use NodeId::NONE as parent_id (page root insert).
#[test]
fn concurrent_scaffold_all_inserts_use_none_parent() {
    let subtasks = vec![
        subtask("a", Some("s1"), 400.0),
        subtask("b", Some("s2"), 400.0),
    ];
    let plan = plan_with_subtasks(subtasks);
    let groups = vec![
        ScreenGroup {
            screen: "s1".into(),
            indices: vec![0],
        },
        ScreenGroup {
            screen: "s2".into(),
            indices: vec![1],
        },
    ];
    let (cmds, _, _) = build_scaffold_concurrent_mobile(&plan, &groups, false).expect("scaffold");
    for cmd in &cmds {
        match cmd {
            EditorCommand::InsertSubtree { parent_id, .. } => {
                assert!(
                    !parent_id.is_real(),
                    "all concurrent roots insert at page level"
                );
            }
            _ => panic!("expected InsertSubtree"),
        }
    }
}

/// Empty groups slice → empty results (no panic).
#[test]
fn concurrent_scaffold_empty_groups_returns_empty() {
    let (cmds, root_ids, baselines) =
        build_scaffold_concurrent_mobile(&plan(), &[], false).expect("scaffold");
    assert!(cmds.is_empty());
    assert!(root_ids.is_empty());
    assert!(baselines.is_empty());
}
