use super::*;
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

// ── existing single-root tests (must stay green) ───────────────────────────

#[test]
fn build_scaffold_desktop_one_root_no_children() {
    let cmds = build_scaffold(&plan(), false).expect("scaffold");
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        EditorCommand::InsertSubtree {
            nodes, parent_id, ..
        } => {
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

// ── Section-stack gap default (breathing room between sections) ─────────────

#[test]
fn resolve_section_gap_defaults_to_20_when_unset_or_zero() {
    // The LLM frequently omits the page gap (None) or emits 0 → sections touch.
    // Both fall back to the canonical 20 px so the page breathes like the TS refs.
    assert_eq!(resolve_section_gap(None), 20.0);
    assert_eq!(resolve_section_gap(Some(0.0)), 20.0);
    // An explicit positive gap is the design's intent — honor it.
    assert_eq!(resolve_section_gap(Some(12.0)), 12.0);
    assert_eq!(resolve_section_gap(Some(32.0)), 32.0);
}

#[test]
fn build_scaffold_root_gap_breathes_when_plan_gap_is_zero() {
    // plan() authors gap: Some(0.0); the scaffold root must still carry 20.
    let cmds = build_scaffold(&plan(), true).expect("scaffold");
    match &cmds[0] {
        EditorCommand::InsertSubtree { nodes, .. } => {
            let json = serde_json::to_value(&nodes[0]).expect("root json");
            assert_eq!(json["gap"], 20.0, "zero plan gap must breathe at 20px");
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}

// ── TS `replaceEmptyFrame` parity: reuse the starter in place ───────────────

#[test]
fn build_scaffold_reusing_emits_replace_subtree_with_reused_id() {
    // The fresh-canvas starter has a different id ("n10") than the planned
    // root ("root"); reuse must stamp the built root with the STARTER's id
    // (so the slot is preserved in place) and replace its subtree — never
    // insert a brand-new root beside it.
    let cmds = build_scaffold_reusing(&plan(), false, "n10").expect("reuse scaffold");
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        EditorCommand::ReplaceSubtree {
            node_id,
            node,
            drop_children,
            ..
        } => {
            assert_eq!(node_id.as_str(), "n10");
            assert!(
                *drop_children,
                "must drop the empty starter's (absent) children"
            );
            // The built root carries the reused id, not the plan's "root".
            assert_eq!(node.id_str(), "n10");
            assert!(node.children().map(|c| c.is_empty()).unwrap_or(true));
        }
        other => panic!("expected ReplaceSubtree, got {other:?}"),
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

// ── two-column dashboard pre-build (sidebar app-shell from the first stroke) ──

fn st(id: &str, label: &str) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region {
            width: 1200.0,
            height: 300.0,
        },
        id_prefix: id.into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn sidebar_dashboard_plan() -> OrchestratorPlan {
    let mut p = plan();
    p.root_frame.width = 1200.0;
    p.subtasks = vec![
        st("sidebar", "Sidebar Navigation"),
        st("metrics", "Key Metrics"),
        st("table", "Client Table"),
        st("appts", "Upcoming Appointments"),
    ];
    p
}

#[test]
fn plan_is_sidebar_dashboard_fires_for_desktop_sidebar_plus_content() {
    assert!(plan_is_sidebar_dashboard(&sidebar_dashboard_plan(), false));
}

#[test]
fn plan_is_sidebar_dashboard_false_when_mobile() {
    assert!(!plan_is_sidebar_dashboard(&sidebar_dashboard_plan(), true));
}

#[test]
fn plan_is_sidebar_dashboard_false_when_no_sidebar() {
    let mut p = sidebar_dashboard_plan();
    p.subtasks[0] = st("hero", "Hero Banner");
    assert!(!plan_is_sidebar_dashboard(&p, false));
}

#[test]
fn plan_is_sidebar_dashboard_false_for_landing_nav_without_data() {
    // A landing page can have a "Navigation" subtask but no data sections —
    // it must NOT be mistaken for a sidebar dashboard.
    let mut p = plan();
    p.root_frame.width = 1440.0;
    p.subtasks = vec![
        st("nav", "Navigation"),
        st("hero", "Hero"),
        st("features", "Features"),
        st("pricing", "Pricing"),
    ];
    assert!(!plan_is_sidebar_dashboard(&p, false));
}

#[test]
fn plan_is_sidebar_dashboard_false_for_landing_nav_with_graph_and_metrics() {
    let mut p = plan();
    p.root_frame.width = 1440.0;
    p.subtasks = vec![
        st("nav", "Navigation"),
        st("hero", "Hero Section"),
        st("capabilities", "Capability Graph"),
        st("proof", "Customer Metrics"),
        st("pricing", "Pricing"),
        st("cta", "Final CTA & Footer Navigation"),
    ];
    assert!(!plan_is_sidebar_dashboard(&p, false));
}

#[test]
fn plan_is_sidebar_dashboard_false_when_narrow() {
    let mut p = sidebar_dashboard_plan();
    p.root_frame.width = 600.0;
    assert!(!plan_is_sidebar_dashboard(&p, false));
}

#[test]
fn build_scaffold_prebuilds_two_columns_for_sidebar_dashboard() {
    let cmds = build_scaffold(&sidebar_dashboard_plan(), false).expect("scaffold");
    match &cmds[0] {
        EditorCommand::InsertSubtree { nodes, .. } => {
            let rv = serde_json::to_value(&nodes[0]).expect("root json");
            assert_eq!(rv["layout"], "horizontal", "two-column root is horizontal");
            let kids = rv["children"].as_array().expect("children");
            assert_eq!(kids.len(), 2, "[Sidebar | Main Content]");
            assert_eq!(kids[0]["name"], "Sidebar");
            assert_eq!(kids[0]["width"], 260.0);
            assert_eq!(kids[0]["height"], "fill_container");
            assert_eq!(kids[0]["clipContent"], true);
            assert_eq!(kids[1]["name"], "Main Content");
            assert_eq!(kids[1]["width"], "fill_container");
            assert_eq!(kids[1]["layout"], "vertical");
            assert!(kids[0]["children"].as_array().unwrap().is_empty());
            assert!(kids[1]["children"].as_array().unwrap().is_empty());
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}

#[test]
fn build_scaffold_single_root_for_non_dashboard() {
    let mut p = plan();
    p.subtasks = vec![st("hero", "Hero"), st("features", "Features")];
    let cmds = build_scaffold(&p, false).expect("scaffold");
    match &cmds[0] {
        EditorCommand::InsertSubtree { nodes, .. } => {
            let rv = serde_json::to_value(&nodes[0]).expect("root json");
            assert_ne!(
                rv["layout"], "horizontal",
                "non-dashboard stays single root"
            );
            assert!(rv["children"].as_array().unwrap().is_empty());
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}

#[test]
fn plan_is_sidebar_dashboard_strong_sidebar_fires_without_data_keywords() {
    // The gap: a strong "Sidebar" subtask whose content sections lack
    // table/metric/chart keywords (Client Directory / Schedule) used to miss the
    // content gate → single-root → sidebar filled full-width during streaming.
    // A strong sidebar signal must now pre-build the two columns regardless.
    let mut p = plan();
    p.root_frame.width = 1280.0;
    p.subtasks = vec![
        st("sidebar", "Sidebar Navigation"),
        st("dir", "Client Directory"),
        st("sched", "Schedule"),
    ];
    assert!(plan_is_sidebar_dashboard(&p, false));
}

#[test]
fn plan_is_sidebar_dashboard_weak_nav_still_needs_data_sections() {
    // A weak "Navigation" signal (no strong sidebar/rail token) without data
    // sections stays single-root (landing-page guard intact).
    let mut p = plan();
    p.root_frame.width = 1440.0;
    p.subtasks = vec![
        st("nav", "Navigation"),
        st("hero", "Hero"),
        st("features", "Features"),
    ];
    assert!(!plan_is_sidebar_dashboard(&p, false));
}

/// A deck plans one subtask per slide, each with its own `screen`. The
/// scaffold must emit one root per group — a single command here is what
/// made a six-slide deck arrive as one board (measured 2026-08-01).
#[test]
fn a_screen_group_per_slide_emits_a_root_per_slide() {
    use crate::screen_groups::ScreenGroup;

    let mut plan = crate::plan::OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: "page".into(),
            name: "Deck".into(),
            width: 1920.0,
            height: 1080.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: Vec::new(),
        style_guide_name: None,
    };
    let names = [
        "01 Cover",
        "02 Agenda",
        "03 Growth",
        "04 Numbers",
        "05 Plan",
        "06 Close",
    ];
    let groups: Vec<ScreenGroup> = names
        .iter()
        .enumerate()
        .map(|(i, screen)| ScreenGroup {
            screen: (*screen).to_string(),
            indices: vec![i],
        })
        .collect();
    plan.subtasks = names
        .iter()
        .map(|screen| crate::plan::Subtask {
            id: (*screen).to_string(),
            label: (*screen).to_string(),
            region: crate::plan::Region {
                width: 1920.0,
                height: 1080.0,
            },
            id_prefix: (*screen).to_string(),
            parent_frame_id: None,
            elements: None,
            screen: Some((*screen).to_string()),
            generated_root_id: None,
            existing_section_labels: None,
            retry_feedback: None,
        })
        .collect();

    let (cmds, placeholder_ids, _) =
        build_screen_group_scaffold(&plan, &groups, false, 0.0, 0.0).expect("scaffold builds");
    assert_eq!(cmds.len(), groups.len(), "one insert per slide");

    // Placeholder ids are the join key back to each group; a collision would
    // let one slide's root overwrite another's.
    let unique: std::collections::BTreeSet<&String> = placeholder_ids.iter().collect();
    assert_eq!(unique.len(), placeholder_ids.len(), "{placeholder_ids:?}");
}
