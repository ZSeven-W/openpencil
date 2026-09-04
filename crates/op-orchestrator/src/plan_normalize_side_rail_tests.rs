use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

fn subtask(id: &str, label: &str, elements: &str) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region {
            width: 1200.0,
            height: 100.0,
        },
        id_prefix: String::new(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: Some(elements.into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn plan(subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Landing Page".into(),
            width: 1200.0,
            height: 0.0,
            layout: Some("vertical".into()),
            gap: Some(0.0),
            padding: None,
            fill: None,
        },
        subtasks,
        style_guide_name: None,
    }
}

#[test]
fn folds_chinese_side_progress_subtask_into_navigation() {
    let mut plan = plan(vec![
        subtask("nav", "Navigation Bar", "logo, links"),
        subtask("hero", "Hero", "headline, CTA"),
        subtask("features", "Features", "three cards"),
        subtask("progress", "右侧阅读进度条", "竖向进度指示器，跟随滚动"),
        subtask("testimonials", "Testimonials", "quotes"),
        subtask("cta", "CTA", "button"),
        subtask("footer", "Footer", "links"),
    ]);

    let removed = fold_side_progress_rail(&mut plan);

    assert_eq!(removed, 1);
    assert_eq!(plan.subtasks.len(), 6);
    let nav = &plan.subtasks[0];
    assert!(nav
        .elements
        .as_deref()
        .unwrap()
        .contains(PROGRESS_BAR_ELEMENTS));
    assert!(!plan.subtasks.iter().any(|st| st.id == "progress"));
}

#[test]
fn folds_english_side_progress_rail() {
    let mut plan = plan(vec![
        subtask("nav", "Navigation Bar", "logo, links"),
        subtask("hero", "Hero", "headline"),
        subtask(
            "progress",
            "Side Progress Rail",
            "vertical scroll indicator",
        ),
    ]);

    assert_eq!(fold_side_progress_rail(&mut plan), 1);
    assert_eq!(plan.subtasks.len(), 2);
}

#[test]
fn leaves_legitimate_pricing_and_navigation_untouched() {
    let mut plan = plan(vec![
        subtask("nav", "Navigation Bar", "logo, links, progress bar"),
        subtask("hero", "Hero", "headline"),
        subtask("pricing", "Pricing", "3 tiers"),
    ]);
    let before = plan.clone();

    assert_eq!(fold_side_progress_rail(&mut plan), 0);
    assert_eq!(plan, before);
}

#[test]
fn leaves_deck_progress_slide_untouched() {
    let mut plan = plan(vec![
        subtask("cover", "Cover", "title"),
        subtask("progress", "进度", "slide content"),
        subtask("close", "Closing", "takeaway"),
    ]);
    plan.root_frame.width = 1920.0;
    plan.root_frame.height = 1080.0;
    for (st, screen) in plan.subtasks.iter_mut().zip(["Cover", "进度", "Closing"]) {
        st.screen = Some(screen.into());
    }
    let before = plan.clone();

    assert_eq!(fold_side_progress_rail(&mut plan), 0);
    assert_eq!(plan, before);
}

#[test]
fn folding_is_idempotent_and_appends_the_progress_bar_once() {
    let mut plan = plan(vec![
        subtask("nav", "Navigation Bar", "logo, links"),
        subtask("hero", "Hero", "headline"),
        subtask(
            "progress",
            "Side Progress Rail",
            "vertical scroll indicator",
        ),
    ]);

    assert_eq!(fold_side_progress_rail(&mut plan), 1);
    assert_eq!(fold_side_progress_rail(&mut plan), 0);
    let elements = plan.subtasks[0].elements.as_deref().unwrap();
    assert_eq!(elements.matches(PROGRESS_BAR_ELEMENTS).count(), 1);
}

#[test]
fn a_rail_in_a_multi_screen_plan_is_left_alone() {
    // Folding here would move screen 2's rail into screen 1's nav.
    let mut plan = plan(vec![
        subtask("nav", "Navigation Bar", "logo, links"),
        subtask("hero", "Hero", "headline, CTA"),
        subtask("features", "Features", "three cards"),
        subtask("progress", "右侧阅读进度条", "竖向进度指示器，跟随滚动"),
        subtask("footer", "Footer", "links"),
    ]);
    for (index, subtask) in plan.subtasks.iter_mut().enumerate() {
        subtask.screen = Some(if index < 3 { "首页" } else { "详情" }.to_string());
    }
    let before = plan.subtasks.len();
    assert_eq!(fold_side_progress_rail(&mut plan), 0);
    assert_eq!(plan.subtasks.len(), before);
}
