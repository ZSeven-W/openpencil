//! multiscreen-fanout-break fix (item A) tests for `plan_normalize::normalize`'s
//! screen-grouping. Split into its own sibling file (rather than growing the
//! inline `mod tests { ... }` in `plan_normalize.rs`, already near the
//! 800-line cap) — self-contained fixture helpers mirror that module's
//! `req` / `subtask` / `plan` exactly.

use super::*;
use crate::plan::PlanFill;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
use crate::types::{ContinuationContext, DesignRequest};

fn req() -> DesignRequest {
    DesignRequest {
        prompt: "x".into(),
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

fn subtask_with_screen(id: &str, label: &str, screen: Option<&str>) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region {
            width: 100.0,
            height: 100.0,
        },
        id_prefix: String::new(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: screen.map(str::to_string),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn plan(width: f64, subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks,
        style_guide_name: None,
    }
}

fn continuation_req() -> DesignRequest {
    DesignRequest {
        prompt: "Continue with the star map, observation plan, and profile screens".into(),
        continuation_context: Some(ContinuationContext {
            screen_width: 390.0,
            screen_height: 844.0,
            background_color: Some("#050508".into()),
            screen_names: vec!["星图".into(), "观测计划".into(), "我的".into()],
        }),
        ..Default::default()
    }
}

/// multiscreen-fanout-break regression lock: ≥2 distinct `screen` labels
/// must NOT collapse onto the shared `root_id` — each group gets its own
/// placeholder id, and every group's subtasks share it.
#[test]
fn normalize_groups_subtasks_by_screen_when_multiple_screens_present() {
    let mut p = plan(
        1200.0,
        vec![
            subtask_with_screen("home-hero", "Home Hero", Some("Home")),
            subtask_with_screen("profile-hero", "Profile Hero", Some("Profile")),
            subtask_with_screen("home-feat", "Home Features", Some("Home")),
        ],
    );
    normalize(&mut p, &req());

    let home_parent = p.subtasks[0].parent_frame_id.clone();
    let profile_parent = p.subtasks[1].parent_frame_id.clone();
    assert_ne!(
        home_parent, profile_parent,
        "distinct screens must get distinct placeholder roots"
    );
    assert_ne!(
        home_parent.as_deref(),
        Some("root"),
        "must NOT be the shared root_id"
    );
    assert_eq!(
        p.subtasks[2].parent_frame_id, home_parent,
        "same-screen subtasks share their group's root"
    );
}

/// Regression lock (spec point 2, zero-tags case): no subtask carries a
/// `screen` label → single-root behavior stays byte-identical to today.
#[test]
fn normalize_zero_screen_labels_keeps_single_root() {
    let mut p = plan(
        1200.0,
        vec![
            subtask_with_screen("hero", "Hero", None),
            subtask_with_screen("feat", "Features", None),
        ],
    );
    normalize(&mut p, &req());
    for st in &p.subtasks {
        assert_eq!(st.parent_frame_id.as_deref(), Some("root"));
    }
}

/// Regression lock (spec point 2, all-same-tag case): every subtask tagged
/// with the SAME screen must also stay single-root — grouping only fans out
/// on ≥2 DISTINCT screen values.
#[test]
fn normalize_all_same_screen_label_keeps_single_root() {
    let mut p = plan(
        1200.0,
        vec![
            subtask_with_screen("hero", "Hero", Some("Home")),
            subtask_with_screen("feat", "Features", Some("Home")),
        ],
    );
    normalize(&mut p, &req());
    for st in &p.subtasks {
        assert_eq!(st.parent_frame_id.as_deref(), Some("root"));
    }
}

#[test]
fn continuation_contract_overrides_valid_but_wrong_screen_plan() {
    let names = ["星图", "观测计划", "我的"];
    let mut tasks = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let mut task = subtask_with_screen(
                &format!("screen-{}", index + 1),
                &format!("{name} detail"),
                Some(name),
            );
            task.region = Region {
                width: 1512.0 - index as f64 * 100.0,
                height: 982.0 - index as f64 * 50.0,
            };
            task.parent_frame_id = Some("giant-generic-root".into());
            task
        })
        .collect::<Vec<_>>();
    let mut p = plan(1512.0, std::mem::take(&mut tasks));
    p.root_frame.height = 982.0;
    p.root_frame.fill = Some(vec![PlanFill {
        kind: "solid".into(),
        color: "#16002E".into(),
    }]);

    normalize(&mut p, &continuation_req());

    assert_eq!((p.root_frame.width, p.root_frame.height), (390.0, 844.0));
    assert_eq!(p.root_frame.first_solid_hex().as_deref(), Some("#050508"));
    assert_eq!(
        p.subtasks
            .iter()
            .map(|task| task.screen.as_deref().unwrap_or_default())
            .collect::<Vec<_>>(),
        names
    );
    let parents = p
        .subtasks
        .iter()
        .map(|task| {
            assert_eq!((task.region.width, task.region.height), (390.0, 844.0));
            let parent = task.parent_frame_id.as_deref().expect("group parent");
            assert_ne!(parent, "giant-generic-root");
            parent
        })
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(parents.len(), 3, "each exact screen gets its own root");
}

#[test]
fn continuation_contract_fans_out_a_valid_generic_plan_to_exact_screens() {
    let mut generic = subtask_with_screen("section-1", "Section 1", None);
    generic.region = Region {
        width: 1512.0,
        height: 982.0,
    };
    generic.parent_frame_id = Some("giant-generic-root".into());
    let mut p = plan(1512.0, vec![generic]);
    p.root_frame.height = 982.0;

    normalize(&mut p, &continuation_req());

    assert_eq!(p.subtasks.len(), 3);
    assert_eq!(
        p.subtasks
            .iter()
            .map(|task| task.screen.as_deref().unwrap_or_default())
            .collect::<Vec<_>>(),
        ["星图", "观测计划", "我的"]
    );
    assert!(p.subtasks.iter().all(|task| {
        (task.region.width, task.region.height) == (390.0, 844.0)
            && task.label != "Section 1"
            && task.parent_frame_id.as_deref() != Some("giant-generic-root")
    }));
}

#[test]
fn continuation_artboards_are_not_shrunk_by_dashboard_section_heuristics() {
    let mut generic = subtask_with_screen("section-1", "Dashboard Section", None);
    generic.region = Region {
        width: 1512.0,
        height: 982.0,
    };
    let mut p = plan(1512.0, vec![generic]);
    p.root_frame.height = 982.0;
    let mut request = continuation_req();
    request.prompt = "Continue the mobile observatory dashboard screens".into();

    normalize(&mut p, &request);

    assert_eq!(p.subtasks.len(), 3);
    assert!(p
        .subtasks
        .iter()
        .all(|task| (task.region.width, task.region.height) == (390.0, 844.0)));
}
