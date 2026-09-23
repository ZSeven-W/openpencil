use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

fn req() -> DesignRequest {
    req_with_prompt("x")
}

fn req_with_prompt(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        ..Default::default()
    }
}

fn subtask(id: &str, label: &str) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region {
            width: 100.0,
            height: 100.0,
        },
        bleed_hero: false,
        id_prefix: String::new(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
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

#[test]
fn app_home_plan_without_navbar_gets_one_appended() {
    // glm planned "Food App Home" with 4 content sections and no navbar —
    // an app home's bottom tab bar is anatomy, not an option.
    let mut p = plan(
        390.0,
        vec![
            subtask("header", "Header & Search"),
            subtask("cats", "Category Rail"),
            subtask("feat", "Featured Restaurant Banner"),
            subtask("popular", "Popular Dishes"),
        ],
    );
    p.root_frame.name = "Food App Home".into();
    normalize(&mut p, &req_with_prompt("well-designed food app"));
    let labels: Vec<String> = p.subtasks.iter().map(|s| s.label.to_lowercase()).collect();
    assert!(
        labels.iter().any(|l| l.contains("navigation")),
        "navbar appended: {labels:?}"
    );
    assert!(
        labels.last().unwrap().contains("navigation"),
        "navbar is LAST: {labels:?}"
    );
}

#[test]
fn single_task_mobile_flow_gets_no_navbar() {
    let mut p = plan(
        390.0,
        vec![subtask("form", "Login Form"), subtask("cta", "Actions")],
    );
    p.root_frame.name = "Login Screen".into();
    normalize(&mut p, &req_with_prompt("mobile login screen"));
    assert!(
        !p.subtasks
            .iter()
            .any(|s| s.label.to_lowercase().contains("navigation")),
        "{:?}",
        p.subtasks.iter().map(|s| &s.label).collect::<Vec<_>>()
    );
}

#[test]
fn normalize_assigns_id_prefix_and_parent() {
    let mut p = plan(
        1200.0,
        vec![subtask("hero", "Hero"), subtask("feat", "Features")],
    );
    let info = normalize(&mut p, &req());
    assert!(!info.is_mobile);
    for st in &p.subtasks {
        assert_eq!(st.id_prefix, st.id);
        assert_eq!(st.parent_frame_id.as_deref(), Some("root"));
    }
}

#[test]
fn normalize_flags_mobile_by_width() {
    let mut p = plan(390.0, vec![subtask("hero", "Hero")]);
    let info = normalize(&mut p, &req());
    assert!(info.is_mobile);
}

#[test]
fn a_deck_board_keeps_its_projector_aspect_whatever_the_model_planned() {
    // A cover slide shipped at 1920x2277 because the root height was
    // resized to its content; the deck contract says the board is fixed.
    for planned in [(1920.0, 0.0), (1200.0, 675.0), (1920.0, 2277.0)] {
        let mut p = plan(planned.0, vec![subtask("s1", "Slide")]);
        p.root_frame.height = planned.1;
        let info = normalize(&mut p, &req_with_prompt("做一个 6 页的季度汇报 PPT"));
        assert_eq!(
            (p.root_frame.width, p.root_frame.height),
            (1920.0, 1080.0),
            "planned {planned:?} must be pinned to the projector board"
        );
        assert!(
            info.preserve_requested_root_height,
            "content-fitting must not be allowed to grow a slide"
        );
    }
}

#[test]
fn a_non_deck_plan_is_not_pinned() {
    let mut p = plan(1200.0, vec![subtask("s1", "Hero")]);
    p.root_frame.height = 0.0;
    let info = normalize(&mut p, &req_with_prompt("a marketing landing page"));
    assert_eq!(p.root_frame.width, 1200.0);
    assert!(
        !info.preserve_requested_root_height,
        "a scrolling page still sizes to its content"
    );
}

#[test]
fn normalize_mobile_forces_vertical_root_layout_and_keeps_positive_gap() {
    let mut p = plan(390.0, vec![subtask("hero", "Hero")]);
    p.root_frame.layout = Some("none".into());
    p.root_frame.gap = Some(12.0);
    p.root_frame.padding = Some(16.0);

    normalize(&mut p, &req());

    assert_eq!(p.root_frame.layout.as_deref(), Some("vertical"));
    assert_eq!(p.root_frame.gap, Some(12.0));
    assert_eq!(p.root_frame.padding, Some(0.0));
}
#[test]
fn normalize_mobile_zero_gap_uses_section_spacing() {
    let mut p = plan(390.0, vec![subtask("hero", "Hero")]);
    p.root_frame.gap = Some(0.0);

    normalize(&mut p, &req());

    assert_eq!(p.root_frame.gap, Some(16.0));
}
#[test]
fn normalize_mobile_zero_height_uses_default_viewport() {
    let mut p = plan(390.0, vec![subtask("hero", "Hero")]);
    p.root_frame.height = 0.0;
    normalize(&mut p, &req());
    assert_eq!(p.root_frame.height, 812.0);
}
#[test]
fn normalize_mobile_preserves_positive_height() {
    let mut p = plan(390.0, vec![subtask("hero", "Hero")]);
    p.root_frame.height = 844.0;
    normalize(&mut p, &req());
    assert_eq!(p.root_frame.height, 844.0);
}
#[test]
fn normalize_strips_status_bar_subtask_on_mobile() {
    let mut p = plan(
        390.0,
        vec![subtask("status-bar", "Status Bar"), subtask("hero", "Hero")],
    );
    normalize(&mut p, &req());
    assert_eq!(p.subtasks.len(), 1);
    assert_eq!(p.subtasks[0].id, "hero");
}
#[test]
fn normalize_strips_status_bar_mentions_from_mobile_subtask_elements() {
    let mut st = subtask("delivery-header", "Delivery Header");
    st.region.width = 390.0;
    st.region.height = 118.0;
    st.elements =
        Some("built-in mobile status bar, Brooklyn delivery location row, dropdown chevron".into());
    let mut p = plan(390.0, vec![st]);
    p.root_frame.height = 844.0;

    normalize(&mut p, &req_with_prompt("mobile food app"));

    let elements = p.subtasks[0]
        .elements
        .as_deref()
        .expect("elements should remain");
    assert!(
        !elements.to_lowercase().contains("status bar"),
        "mobile subtask elements must not contradict the built-in status bar rule"
    );
    assert!(
        elements.contains("Brooklyn delivery location row"),
        "non-status-bar element fragments should be preserved"
    );
}
#[test]
fn normalize_keeps_status_bar_subtask_on_desktop() {
    // 桌面端不剔除(只有移动端 scaffold 注入固定状态栏)。
    let mut p = plan(
        1200.0,
        vec![subtask("status-bar", "Status Bar"), subtask("hero", "Hero")],
    );
    normalize(&mut p, &req());
    assert_eq!(p.subtasks.len(), 2);
}
#[test]
fn normalize_adds_requested_bottom_nav_subtask_on_mobile() {
    let mut p = plan(
        390.0,
        vec![
            subtask("header", "Delivery Header"),
            subtask("popular", "Popular Restaurants"),
        ],
    );
    normalize(
        &mut p,
        &req_with_prompt("mobile food delivery screen with bottom navigation bar"),
    );

    let nav = p
        .subtasks
        .iter()
        .find(|st| st.id == "bottom-navigation")
        .expect("normalization should append missing bottom nav");
    assert_eq!(nav.parent_frame_id.as_deref(), Some("root"));
    assert_eq!(nav.id_prefix, "bottom-navigation");
    assert_eq!(nav.region.width, 390.0);
    assert!(nav
        .elements
        .as_deref()
        .unwrap_or_default()
        .contains("bottom-tab-bar"));
}
#[test]
fn normalize_does_not_duplicate_existing_bottom_nav_subtask() {
    let mut p = plan(
        390.0,
        vec![
            subtask("header", "Delivery Header"),
            subtask("bottom-tabs", "Bottom Tab Bar"),
        ],
    );
    normalize(
        &mut p,
        &req_with_prompt("mobile food delivery screen with bottom navigation bar"),
    );

    let count = p
        .subtasks
        .iter()
        .filter(|st| st.label.contains("Bottom"))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn normalize_mobile_bottom_nav_region_to_bar_size() {
    let mut st = subtask("bottom-nav", "Bottom Navigation");
    st.region.width = 260.0;
    st.region.height = 456.0;
    st.elements = Some("bottom navigation tabs".into());
    let mut p = plan(390.0, vec![st]);

    normalize(
        &mut p,
        &req_with_prompt("mobile food app with bottom navigation"),
    );

    let nav = &p.subtasks[0];
    assert_eq!(nav.region.width, 390.0);
    assert_eq!(nav.region.height, 78.0);
}

#[test]
fn normalize_does_not_add_bottom_nav_on_desktop() {
    let mut p = plan(1200.0, vec![subtask("content", "Content")]);
    normalize(
        &mut p,
        &req_with_prompt("desktop dashboard with bottom navigation bar"),
    );

    assert!(p.subtasks.iter().all(|st| st.id != "bottom-navigation"));
}

// -----------------------------------------------------------------------
// Task C1 — dashboard branch
// -----------------------------------------------------------------------
fn dash_req(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        ..Default::default()
    }
}
/// Build a dashboard-like plan with a sidebar + two data subtasks.
fn dashboard_plan(root_width: f64) -> OrchestratorPlan {
    let st_sidebar = Subtask {
        id: "sidebar".into(),
        label: "Sidebar Navigation".into(),
        region: Region {
            width: 100.0,
            height: 500.0,
        },
        bleed_hero: false,
        id_prefix: String::new(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    };
    // chart subtask — LLM-provided height 300, within [inferred*0.6, inferred*1.6]
    // inferred for "chart" = 320  →  min=192, max=512  →  300 in range → keep 300
    let st_chart = Subtask {
        id: "revenue-chart".into(),
        label: "Revenue Chart".into(),
        region: Region {
            width: 800.0,
            height: 300.0,
        },
        bleed_hero: false,
        id_prefix: String::new(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    };
    // metric subtask — LLM-provided height 0 (invalid) → use inferred = 160
    let st_metric = Subtask {
        id: "kpi-metrics".into(),
        label: "KPI Metrics".into(),
        region: Region {
            width: 800.0,
            height: 0.0,
        },
        bleed_hero: false,
        id_prefix: String::new(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    };
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Dashboard".into(),
            width: root_width,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![st_sidebar, st_chart, st_metric],
        style_guide_name: None,
    }
}

#[test]
fn normalize_dashboard_rewrites_width_per_infer() {
    // root_width = 1200  →  main_width = max(320, 1200-260) = 940
    // sidebar → 260
    // chart/revenue → round(940 * 0.62) = round(582.8) = 583
    // metric (default) → 940
    let mut p = dashboard_plan(1200.0);
    let req = dash_req("an analytics admin dashboard");
    normalize(&mut p, &req);

    let sidebar = p.subtasks.iter().find(|s| s.id == "sidebar").unwrap();
    assert_eq!(sidebar.region.width, 260.0, "sidebar width must be 260");

    let chart = p.subtasks.iter().find(|s| s.id == "revenue-chart").unwrap();
    let expected_chart_w = (940.0_f64 * 0.62).round();
    assert_eq!(
        chart.region.width, expected_chart_w,
        "chart width = main*0.62"
    );

    let metric = p.subtasks.iter().find(|s| s.id == "kpi-metrics").unwrap();
    assert_eq!(metric.region.width, 940.0, "metric width = full main");
}

#[test]
fn normalize_dashboard_height_kept_when_in_range() {
    // chart: inferred=320, LLM=300
    // min=round(320*0.6)=192, max=round(320*1.6)=512
    // 300 ∈ [192,512] → keep 300
    let mut p = dashboard_plan(1200.0);
    let req = dash_req("analytics admin dashboard");
    normalize(&mut p, &req);

    let chart = p.subtasks.iter().find(|s| s.id == "revenue-chart").unwrap();
    assert_eq!(
        chart.region.height, 300.0,
        "LLM height kept when in clamp range"
    );
}

#[test]
fn normalize_dashboard_height_replaced_when_zero() {
    // metric: LLM height=0 (invalid) → use inferred=160
    let mut p = dashboard_plan(1200.0);
    let req = dash_req("data analytics admin dashboard");
    normalize(&mut p, &req);

    let metric = p.subtasks.iter().find(|s| s.id == "kpi-metrics").unwrap();
    assert_eq!(
        metric.region.height, 160.0,
        "zero LLM height replaced by inferred"
    );
}

#[test]
fn normalize_dashboard_height_clamped_to_max() {
    // "customer-table" subtask: id+label match "table" (no "transaction"),
    // so inferred = 340  →  max=round(340*1.6)=544
    // max(round(340*0.6), min(2000, 544)) = max(204, 544) = 544
    let mut p = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Dashboard".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![Subtask {
            id: "customer-table".into(),
            label: "Customer Table".into(),
            region: Region {
                width: 800.0,
                height: 2000.0,
            },
            bleed_hero: false,
            id_prefix: String::new(),
            parent_frame_id: None,
            insert_after_sibling_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
            covers: None,
            retry_feedback: None,
        }],
        style_guide_name: None,
    };
    normalize(&mut p, &dash_req("data analytics admin dashboard"));
    assert_eq!(
        p.subtasks[0].region.height,
        (340.0_f64 * 1.6).round(),
        "over-tall LLM height clamped to max"
    );
}

#[test]
fn normalize_dashboard_height_clamped_to_min() {
    // chart subtask with too-short LLM height (50)
    // inferred=320  →  min=round(320*0.6)=192, max=round(320*1.6)=512
    // max(192, min(50, 512)) = max(192, 50) = 192
    let mut p = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Dashboard".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![Subtask {
            id: "rev-chart".into(),
            label: "Revenue Chart".into(),
            region: Region {
                width: 800.0,
                height: 50.0,
            },
            bleed_hero: false,
            id_prefix: String::new(),
            parent_frame_id: None,
            insert_after_sibling_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
            existing_section_labels: None,
            covers: None,
            retry_feedback: None,
        }],
        style_guide_name: None,
    };
    normalize(&mut p, &dash_req("data analytics admin dashboard"));
    assert_eq!(
        p.subtasks[0].region.height,
        (320.0_f64 * 0.6).round(),
        "too-short LLM height clamped to min"
    );
}

#[test]
fn normalize_non_dashboard_plan_is_unaffected() {
    // A landing page prompt — NOT dashboard-like — must not rewrite regions.
    let mut p = plan(
        1200.0,
        vec![
            subtask("hero", "Hero Section"),
            subtask("features", "Feature Cards"),
        ],
    );
    // Give them non-zero regions so default-fill doesn't fire.
    p.subtasks[0].region = Region {
        width: 1200.0,
        height: 560.0,
    };
    p.subtasks[1].region = Region {
        width: 1200.0,
        height: 400.0,
    };

    normalize(
        &mut p,
        &dash_req("a beautiful landing page for a SaaS product"),
    );

    assert_eq!(p.subtasks[0].region.width, 1200.0);
    assert_eq!(p.subtasks[0].region.height, 560.0);
    assert_eq!(p.subtasks[1].region.width, 1200.0);
    assert_eq!(p.subtasks[1].region.height, 400.0);
}
