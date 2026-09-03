//! Explicit root-dimension regressions for `plan_normalize`.

use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

fn request(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
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

fn subtask(id: &str, label: &str, width: f64, height: f64) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region { width, height },
        id_prefix: String::new(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn plan(name: &str, subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: name.into(),
            width: 1200.0,
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

#[test]
fn normalize_applies_explicit_root_dimensions_before_dashboard_sizing() {
    let mut plan = plan(
        "Dashboard",
        vec![
            subtask("sidebar", "Sidebar Navigation", 100.0, 500.0),
            subtask("kpi-metrics", "KPI Metrics", 800.0, 0.0),
        ],
    );
    let norm = normalize(&mut plan, &request("Design a 1440×900 analytics dashboard"));

    assert_eq!(plan.root_frame.width, 1440.0);
    assert_eq!(plan.root_frame.height, 900.0);
    assert!(
        norm.preserve_requested_root_height,
        "an explicit width-height pair must become a cleanup contract"
    );
    let metric = plan
        .subtasks
        .iter()
        .find(|subtask| subtask.id == "kpi-metrics")
        .unwrap();
    assert_eq!(
        metric.region.width, 1180.0,
        "dashboard main width must derive from the requested 1440px root"
    );
}

#[test]
fn normalize_updates_full_width_landing_regions_for_explicit_width() {
    let mut plan = plan(
        "Landing Page",
        vec![
            subtask("nav", "Navigation Bar", 1200.0, 100.0),
            subtask("hero", "Hero Section", 640.0, 100.0),
        ],
    );
    let norm = normalize(
        &mut plan,
        &request("Design a desktop landing page. Make the root exactly 1440px wide."),
    );

    assert_eq!(plan.root_frame.width, 1440.0);
    assert_eq!(plan.root_frame.height, 800.0);
    assert!(
        !norm.preserve_requested_root_height,
        "width-only requests must still let cleanup grow the root height"
    );
    assert_eq!(plan.subtasks[0].region.width, 1440.0);
    assert_eq!(
        plan.subtasks[1].region.width, 640.0,
        "an intentionally partial-width region must stay partial"
    );
}
