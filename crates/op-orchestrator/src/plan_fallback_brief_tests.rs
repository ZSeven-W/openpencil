//! Brief-driven fallback plan regressions (arena d01, run 0927a/run-3).

use super::*;
use crate::plan::build_fallback_plan;
use crate::plan_coverage::missing_sections;
use crate::plan_normalize::normalize;
use crate::types::DesignRequest;

const D01_PROMPT: &str = "运营数据看板（1440×900，桌面）：左侧栏（品牌+六个导航项+底部用户）、顶部工具栏、四个 KPI 卡、折线图+柱图并排、下方数据表 8 行 6 列带分页。";
const MOBILE_PROMPT: &str = "健身打卡 App 首页（375×812）：顶部问候、今日目标环形进度、三个快捷入口、本周打卡日历、底部导航四个标签。";

fn req(prompt: &str) -> DesignRequest {
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

fn elements(st: &Subtask) -> &str {
    st.elements.as_deref().unwrap_or_default()
}

#[test]
fn d01_fallback_builds_one_subtask_per_brief_section() {
    let request = req(D01_PROMPT);
    let required = required_sections(D01_PROMPT);
    assert_eq!(required.len(), 5, "fixture drift: {required:?}");

    let mut plan = build_fallback_plan(&request);
    let labels: Vec<&str> = plan.subtasks.iter().map(|st| st.label.as_str()).collect();
    assert_eq!(
        labels,
        required.iter().map(String::as_str).collect::<Vec<_>>()
    );
    for (st, head) in plan.subtasks.iter().zip(&required) {
        assert_eq!(st.covers.as_deref(), Some(std::slice::from_ref(head)));
    }
    assert!(
        missing_sections(&required, &plan).is_empty(),
        "the coverage gate must accept the fallback plan"
    );

    // Full wording, parenthetical detail and counts included.
    assert!(elements(&plan.subtasks[0]).contains("左侧栏（品牌+六个导航项+底部用户）"));
    assert!(elements(&plan.subtasks[2]).contains("四个 KPI 卡"));
    assert!(elements(&plan.subtasks[4]).contains("下方数据表 8 行 6 列带分页"));
    assert!(plan
        .subtasks
        .iter()
        .all(|st| elements(st).contains("every count it states")));

    // Unique ASCII-safe ids; the CJK left rail gets the canonical sidebar id.
    let mut ids: Vec<&str> = plan.subtasks.iter().map(|st| st.id.as_str()).collect();
    assert_eq!(ids[0], "sidebar");
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 5, "ids must be unique");
    assert!(plan.subtasks.iter().all(|st| st
        .id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')));

    // Rail is narrow and full height; the rest share the main column.
    assert_eq!(plan.subtasks[0].region.width, 260.0);
    assert_eq!(plan.subtasks[0].region.height, 900.0);
    for st in &plan.subtasks[1..] {
        assert_eq!(st.region.width, 1180.0, "{}", st.label);
        assert_eq!(st.region.height, 225.0, "{}", st.label);
    }

    normalize(&mut plan, &request);
    assert_eq!(plan.root_frame.width, 1440.0);
    assert_eq!(plan.root_frame.height, 900.0);
    assert_eq!(plan.subtasks.len(), 5);
    assert!(
        crate::scaffold::plan_is_sidebar_dashboard(&plan, false),
        "the rail must pre-build the two-column app shell"
    );
}

#[test]
fn brief_without_sections_keeps_the_length_sliced_skeleton() {
    let prompt = "Design a landing page for a coffee shop";
    assert!(required_sections(prompt).is_empty());
    let plan = build_fallback_plan(&req(prompt));
    assert_eq!(plan.subtasks.len(), 1);
    assert_eq!(plan.subtasks[0].id, "section-1");
    assert_eq!(plan.subtasks[0].label, "Section 1");
    assert_eq!(plan.subtasks[0].elements, None);
    assert_eq!(plan.root_frame.width, 1200.0);
    assert_eq!(plan.root_frame.height, 360.0);
}

#[test]
fn desktop_brief_without_requested_size_sums_default_heights() {
    let prompt = "项目管理后台：侧边栏、任务看板、成员列表";
    let required = required_sections(prompt);
    assert!(!required.is_empty(), "fixture drift: {required:?}");
    let plan = build_fallback_plan(&req(prompt));
    assert_eq!(plan.subtasks.len(), required.len());
    assert_eq!(plan.root_frame.width, 1200.0);
    let stacked = plan
        .subtasks
        .iter()
        .filter(|st| st.region.width != 260.0)
        .count() as f64;
    assert_eq!(plan.root_frame.height, stacked * DESKTOP_SECTION_HEIGHT);
}

#[test]
fn mobile_brief_gets_brief_sections_at_phone_size() {
    let request = req(MOBILE_PROMPT);
    let required = required_sections(MOBILE_PROMPT);
    assert!(required.len() >= 3, "fixture drift: {required:?}");

    let plan = build_fallback_plan(&request);
    assert_eq!(plan.root_frame.id, "page");
    assert_eq!(plan.root_frame.width, 375.0);
    assert_eq!(plan.root_frame.height, 812.0);
    let labels: Vec<&str> = plan.subtasks.iter().map(|st| st.label.as_str()).collect();
    assert_eq!(
        labels,
        required.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert!(missing_sections(&required, &plan).is_empty());
    for st in &plan.subtasks {
        assert_eq!(st.region.width, 375.0, "no rail narrowing on a phone");
        assert_eq!(st.parent_frame_id.as_deref(), Some("page"));
    }
    let quick = plan
        .subtasks
        .iter()
        .find(|st| st.label == "快捷入口")
        .expect("quick-entry section");
    assert!(elements(quick).contains("三个快捷入口"));
}

#[test]
fn mobile_brief_without_sections_keeps_the_generic_pair() {
    let plan = build_fallback_plan(&req("Design a 390x844 mobile food delivery home screen"));
    let ids: Vec<&str> = plan.subtasks.iter().map(|st| st.id.as_str()).collect();
    assert_eq!(ids, vec!["top-summary", "main-content"]);
}

#[test]
fn section_wording_extends_to_the_whole_list_item() {
    assert_eq!(
        section_wording(D01_PROMPT, "左侧栏"),
        "左侧栏（品牌+六个导航项+底部用户）"
    );
    assert_eq!(
        section_wording(D01_PROMPT, "下方数据表 8 行 6 列"),
        "下方数据表 8 行 6 列带分页"
    );
    assert_eq!(section_wording(D01_PROMPT, "顶部工具栏"), "顶部工具栏");
    assert_eq!(
        section_wording(MOBILE_PROMPT, "底部导航"),
        "底部导航四个标签"
    );
    // Case-folded lookup for ASCII heads keeps the brief's own casing.
    assert_eq!(
        section_wording("Sections: Hero, Pricing plans, FAQ", "pricing plans"),
        "Pricing plans"
    );
    assert_eq!(section_wording("nothing here", "侧边栏"), "侧边栏");
}
