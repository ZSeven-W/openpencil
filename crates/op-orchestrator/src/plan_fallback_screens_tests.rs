//! Real two-screen briefs whose first screen's sections follow the
//! declaration colon and whose second screen opens its own sentence.

use super::*;
use crate::plan::build_fallback_plan;
use crate::plan_normalize::normalize;
use crate::screen_groups::group_subtasks_by_screen;
use crate::types::DesignRequest;

const BONSAI: &str = include_str!("fixtures/fallback_bonsai_two_screen_prompt_1023.txt");
const STARGAZE: &str = include_str!("fixtures/fallback_stargaze_two_screen_prompt_1022.txt");
const TEANOTES: &str = include_str!("fixtures/fallback_teanotes_two_screen_prompt_1023.txt");

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

fn names(screens: &[DeclaredScreen]) -> Vec<&str> {
    screens.iter().map(|screen| screen.name.as_str()).collect()
}

#[test]
fn sentence_per_screen_briefs_split_into_their_declared_screens() {
    for (brief, expected, home_detail, second_detail) in [
        (
            BONSAI,
            ["首页", "盆景详情页"],
            "「我的盆景」横滑三张卡片",
            "满幅真柏盆景照片做顶部大图",
        ),
        (
            STARGAZE,
            ["首页", "天体详情页"],
            "逐小时透明度条形时间轴",
            "满幅土星照片做顶部大图",
        ),
        (
            TEANOTES,
            ["首页", "冲泡计时详情页"],
            "「风味雷达」小图",
            "顶部茶汤特写照片",
        ),
    ] {
        let screens = declared_screens(brief).expect("two declared screens");
        assert_eq!(names(&screens), expected);
        assert!(screens[0].wording.contains(home_detail), "{screens:?}");
        assert!(screens[0].wording.contains("底部标签栏四项"), "{screens:?}");
        assert!(!screens[0].wording.contains(second_detail), "{screens:?}");
        assert!(screens[1].wording.contains(second_detail), "{screens:?}");
        assert!(!screens[1].wording.contains(home_detail), "{screens:?}");
    }
}

#[test]
fn failed_planning_keeps_each_declared_screen_in_its_own_root() {
    let request = request(BONSAI);
    let mut plan = build_fallback_plan(&request);
    assert_eq!(
        (plan.root_frame.width, plan.root_frame.height),
        (375.0, 812.0)
    );
    assert_eq!(plan.subtasks.len(), 2);
    for (task, name) in plan.subtasks.iter().zip(["首页", "盆景详情页"]) {
        assert_eq!(task.label, name);
        assert_eq!(task.screen.as_deref(), Some(name));
        assert_eq!(task.covers.as_deref(), Some(&[name.to_string()][..]));
        assert_eq!((task.region.width, task.region.height), (375.0, 812.0));
    }
    normalize(&mut plan, &request);
    let groups = group_subtasks_by_screen(&plan.subtasks);
    assert_eq!(
        groups
            .iter()
            .map(|group| group.screen.as_str())
            .collect::<Vec<_>>(),
        ["首页", "盆景详情页"]
    );
    let home = plan
        .subtasks
        .iter()
        .find(|task| task.label == "首页")
        .unwrap();
    let detail = plan
        .subtasks
        .iter()
        .find(|task| task.label == "盆景详情页")
        .unwrap();
    assert_ne!(home.parent_frame_id, detail.parent_frame_id);
}

#[test]
fn notes_after_the_screens_do_not_leak_into_the_last_screen() {
    let brief = "记账 App 两屏（375×812）：首页顶部余额卡，最近账单五条。账单详情页：金额大字、分类与备注。交互：账单行 onTap 进详情。动效：金额数字滚动。";
    let screens = declared_screens(brief).unwrap();
    assert_eq!(names(&screens), ["首页", "账单详情页"]);
    assert_eq!(screens[1].wording, "金额大字、分类与备注");
}

#[test]
fn counts_that_do_not_match_the_described_screens_keep_section_mode() {
    for brief in [
        // Three declared, two described.
        "记账 App 三屏（375×812）：首页顶部余额卡。账单详情页：金额大字。",
        // A single screen whose detail merely mentions a page.
        "记账 App 单屏（375×812）：首页顶部余额卡。账单详情页：金额大字。",
        // A negated multi-screen example.
        "不要做成两屏：首页顶部余额卡。账单详情页：金额大字。",
        // The Home "single page" delivery choice outranks the brief's count.
        "交付 1 页界面：只设计用户最需要的一个页面。\n\n用户需求：记账 App 两屏（375×812）：首页顶部余额卡。账单详情页：金额大字。",
        // Normal-mode's wrapper count alone is not the user's declaration.
        "交付 2 页完整界面：页面之间用 onTap 串起导航。统一的组件。",
    ] {
        assert_eq!(declared_screens(brief), None, "{brief}");
    }
}

#[test]
fn explicit_full_page_lists_still_use_the_existing_path() {
    let brief = "电商 App 两屏（375×812）：商品页(大图轮播+价格)、购物车页(商品两条+结算)。";
    let plan = build_fallback_plan(&request(brief));
    let labels: Vec<_> = plan
        .subtasks
        .iter()
        .map(|task| task.label.as_str())
        .collect();
    assert_eq!(labels, ["商品页", "购物车页"]);
    assert!(plan.subtasks[0]
        .elements
        .as_deref()
        .unwrap()
        .starts_with("商品页(大图轮播+价格)"));
}
