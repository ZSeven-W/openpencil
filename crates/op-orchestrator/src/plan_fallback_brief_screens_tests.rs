//! Retained normal-mode briefs from the 2026-10-18 generation logs.
//! The fixture bytes are the JSON-decoded `[SMOKE] prompt=` value, including
//! the HomeState two-page wrapper, not a simplified replacement brief.

use super::*;
use crate::plan::build_fallback_plan;
use crate::plan_normalize::normalize;
use crate::screen_groups::group_subtasks_by_screen;
use crate::test_support::VecDocSink;
use crate::types::{DesignRequest, DocSink};
use crate::wire_screen_navigation::{ensure_planned_screen_routes, prompt_screen_route_inventory};
use jian_ops_schema::screen_projection::project_screens;
use op_editor_core::{AppPages, EditorState, HomeDevice, HomeFamily, PenNodeExt};

const PRODUCT: &str = include_str!("fixtures/fallback_product_detail_prompt_1018.txt");
const RESTAURANT: &str = include_str!("fixtures/fallback_restaurant_detail_prompt_1018.txt");

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

fn assert_native_screens(
    prompt: &str,
    names: &[&str],
    details: &[&str],
    expected_nav_screen: Option<&str>,
) {
    let request = request(prompt);
    let mut plan = build_fallback_plan(&request);
    assert_eq!(plan.subtasks.len(), names.len());
    assert_eq!(
        (plan.root_frame.width, plan.root_frame.height),
        (375.0, 812.0)
    );
    for ((task, name), detail) in plan.subtasks.iter().zip(names).zip(details) {
        assert_eq!(task.label, *name);
        assert_eq!(task.screen.as_deref(), Some(*name));
        assert_eq!((task.region.width, task.region.height), (375.0, 812.0));
        assert_eq!(task.covers.as_deref(), Some(&[name.to_string()][..]));
        assert!(task.elements.as_deref().unwrap().contains(detail));
    }
    assert!(crate::plan_coverage::missing_sections(&required_sections(prompt), &plan).is_empty());
    let original_pages = plan.subtasks.clone();
    normalize(&mut plan, &request);
    let groups = group_subtasks_by_screen(&plan.subtasks);
    assert_eq!(groups.len(), names.len());
    assert_ne!(
        plan.subtasks[0].parent_frame_id,
        plan.subtasks[1].parent_frame_id
    );
    for original in &original_pages {
        let task = plan
            .subtasks
            .iter()
            .find(|task| task.id == original.id)
            .expect("normalization must retain each original page task");
        assert_eq!(
            (task.region.width, task.region.height),
            (375.0, 812.0),
            "{}",
            task.id
        );
        assert_eq!(task.screen, original.screen);
        assert_eq!(task.elements, original.elements);
        assert_eq!(task.covers, original.covers);
        let parent = format!(
            "{}-{}",
            plan.root_frame.id,
            original.screen.as_deref().unwrap()
        );
        assert_eq!(task.parent_frame_id.as_deref(), Some(parent.as_str()));
    }
    // Home normalization may append a separate bar. It must neither shrink
    // an original full-page task nor create navigation under the detail page.
    let added: Vec<_> = plan
        .subtasks
        .iter()
        .filter(|task| !original_pages.iter().any(|page| page.id == task.id))
        .collect();
    if let Some(screen) = expected_nav_screen {
        assert_eq!(added.len(), 1);
        let nav = added[0];
        assert_eq!(nav.id, "bottom-navigation-0");
        assert_eq!((nav.region.width, nav.region.height), (375.0, 78.0));
        assert_eq!(nav.screen.as_deref(), Some(screen));
        let parent = format!("{}-{screen}", plan.root_frame.id);
        assert_eq!(nav.parent_frame_id.as_deref(), Some(parent.as_str()));
    } else {
        assert!(added.is_empty());
    }

    // Exercise the real multi-root insertion and pre-worker route assignment.
    // Insertion remaps IDs; bind the plan to the inserted roots as the run loop does.
    let (commands, _, baselines) =
        crate::scaffold::build_screen_group_scaffold(&plan, &groups, true, 0.0, 0.0)
            .expect("screen scaffold");
    assert_eq!(baselines, vec![1; names.len()]);
    let mut sink = VecDocSink::new();
    for command in commands {
        assert!(sink.apply(command));
    }
    let roots = sink.state.active_children();
    assert_eq!(roots.len(), names.len());
    for (index, node) in roots.iter().enumerate() {
        let data = serde_json::to_value(node).unwrap();
        assert_eq!(data["name"], names[index]);
        assert_eq!(data["width"], 375.0);
        assert_eq!(data["height"], 812.0);
        assert_eq!(data["children"][0]["role"], "status-bar");
        for &task_index in &groups[index].indices {
            plan.subtasks[task_index].parent_frame_id = Some(node.id_str().to_string());
        }
    }
    if let Some(screen) = expected_nav_screen {
        let nav = plan
            .subtasks
            .iter()
            .find(|task| task.id == "bottom-navigation-0")
            .unwrap();
        let home_index = names.iter().position(|name| *name == screen).unwrap();
        assert_eq!(
            nav.parent_frame_id.as_deref(),
            Some(roots[home_index].id_str())
        );
        assert_ne!(nav.parent_frame_id.as_deref(), Some(roots[1].id_str()));
    }
    let inventory = prompt_screen_route_inventory(&plan, &sink.state);
    assert_eq!(
        inventory,
        vec![
            (names[0].to_string(), "/".into()),
            (names[1].to_string(), "/screen-1".into())
        ]
    );
    ensure_planned_screen_routes(&mut sink, &plan);
    assert_eq!(prompt_screen_route_inventory(&plan, &sink.state), inventory);
    for (node, (_, path)) in sink.state.active_children().iter().zip(&inventory) {
        assert_eq!(serde_json::to_value(node).unwrap()["screen"], path.as_str());
    }

    // Native preview consumes these exact top-level markers, not plan labels.
    let (projected, warnings) = project_screens(&sink.state.doc);
    assert!(warnings.is_empty(), "{warnings:?}");
    let (projected, _) = projected.expect("native screen projection");
    let pages = projected.pages.as_ref().unwrap();
    let routes = projected.routes.as_ref().unwrap();
    assert_eq!(pages.len(), names.len());
    assert_eq!(routes.entry, "/");
    assert_eq!(routes.routes.len(), names.len());
    for (index, (_, path)) in inventory.iter().enumerate() {
        assert_eq!(pages[index].name, names[index]);
        assert_eq!(routes.routes[path].page_id, pages[index].id);
    }
    let saved = serde_json::to_string(&sink.state.doc).unwrap();
    ensure_planned_screen_routes(&mut sink, &plan);
    assert_eq!(serde_json::to_string(&sink.state.doc).unwrap(), saved);
}

#[test]
fn retained_wrapped_product_fallback_preserves_two_native_screens_and_routes() {
    assert!(PRODUCT.starts_with("请设计一套可编辑的高保真手机 App 界面"));
    assert!(PRODUCT.contains("\n\n用户需求："));
    assert_native_screens(
        PRODUCT,
        &["商品页", "购物车页"],
        &[
            "商品页(大图轮播占位+价格+规格选择+评价三条+底部加购条)",
            "购物车页(商品两条+金额汇总+结算)",
        ],
        None,
    );
}

#[test]
fn retained_wrapped_restaurant_fallback_preserves_two_native_screens_and_routes() {
    assert_native_screens(
        RESTAURANT,
        &["首页", "商家详情"],
        &[
            "首页(地址栏+分类九宫格+限时横幅+商家列表五条)",
            "商家详情(头图+菜品分类横滑+菜品列表六条+购物车条)",
        ],
        Some("首页"),
    );
}

#[test]
fn bare_original_briefs_keep_the_same_screen_contract() {
    for prompt in [PRODUCT, RESTAURANT] {
        let original = prompt.split_once("\n\n用户需求：").unwrap().1;
        let wrapped = build_fallback_plan(&request(prompt));
        let bare = build_fallback_plan(&request(original));
        assert_eq!(bare.subtasks.len(), 2);
        for (actual, expected) in bare.subtasks.iter().zip(&wrapped.subtasks) {
            assert_eq!(actual.screen, expected.screen);
            assert_eq!(actual.region.height, 812.0);
            assert_eq!(actual.elements, expected.elements);
        }
    }
}

#[test]
fn declared_pages_use_the_requested_viewport_without_summing_or_splitting_it() {
    let prompt = "桌面 App 2 页（1440×900）：商品页（商品列表）、购物车页（金额汇总）。";
    let (tasks, height) = brief_section_subtasks(prompt, 1440.0, Some(900.0), true, None).unwrap();
    assert_eq!(height, 900.0);
    assert_eq!(tasks.len(), 2);
    assert!(tasks.iter().all(|task| task.screen.is_some()
        && task.region.width == 1440.0
        && task.region.height == 900.0));
}

#[test]
fn single_screen_sections_keep_one_root_and_shared_height() {
    let prompt = "手机 App 一屏（375×812）：商品信息（大图+价格）、规格选择（颜色+尺码）。";
    let request = request(prompt);
    let mut plan = build_fallback_plan(&request);
    assert_eq!(plan.subtasks.len(), 2);
    assert!(plan.subtasks.iter().all(|task| task.screen.is_none()));
    assert!(plan.subtasks.iter().all(|task| task.region.height == 406.0));
    normalize(&mut plan, &request);
    assert!(group_subtasks_by_screen(&plan.subtasks).is_empty());
    assert!(plan
        .subtasks
        .iter()
        .all(|task| task.parent_frame_id.as_deref() == Some("page")));
}

#[test]
fn home_single_choice_overrides_the_retained_two_screen_brief() {
    let mut state = EditorState::new();
    state.editor_ui.home.task = HomeFamily::AppUi;
    state.editor_ui.home.set_device(HomeDevice::Mobile);
    state
        .editor_ui
        .home
        .set_draft(PRODUCT.split_once("\n\n用户需求：").unwrap().1);
    state.editor_ui.home.set_app_pages(AppPages::Single);
    let prompt = state.editor_ui.home.generation_prompt().unwrap();
    assert!(prompt.contains("交付 1 页界面：只设计用户最需要的一个页面"));
    assert!(prompt.contains("App 两屏可交互"));
    let request = request(&prompt);
    let mut plan = build_fallback_plan(&request);
    normalize(&mut plan, &request);
    assert!(group_subtasks_by_screen(&plan.subtasks).is_empty());
    assert!(plan
        .subtasks
        .iter()
        .all(|task| task.parent_frame_id.as_deref() == Some("page")));
}

#[test]
fn screen_details_come_from_the_matched_list_not_an_earlier_title_mention() {
    let prompt = "商品页和购物车 App 两屏（375×812）：商品页（大图+价格+规格+评价三条+底部加购条）、购物车页（商品两条+金额汇总+结算）。";
    let plan = build_fallback_plan(&request(prompt));
    assert_eq!(plan.subtasks.len(), 2);
    assert_eq!(plan.subtasks[0].screen.as_deref(), Some("商品页"));
    assert!(plan.subtasks[0]
        .elements
        .as_deref()
        .unwrap()
        .starts_with("商品页（大图+价格+规格+评价三条+底部加购条）"));
}

#[test]
fn negated_multiple_screen_request_keeps_normalized_sections_on_one_root() {
    for prompt in [
        "手机 App 单页，不要做两屏：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App 不需要两屏：商品信息（大图+价格）、规格选择（颜色+尺码）。",
    ] {
        let request = request(prompt);
        let mut plan = build_fallback_plan(&request);
        assert_eq!(plan.subtasks.len(), 2);
        normalize(&mut plan, &request);
        assert!(
            group_subtasks_by_screen(&plan.subtasks).is_empty(),
            "{prompt}"
        );
        assert!(plan
            .subtasks
            .iter()
            .all(|task| task.parent_frame_id.as_deref() == Some("page")));
    }
}

#[test]
fn english_single_page_and_negated_counts_keep_real_sections_on_one_root() {
    for header in [
        "Single-page mobile app, 2 screens",
        "Single screen mobile app, 2 pages",
        "Mobile app, do NOT create 2 screens",
        "Mobile app, don't create 2 pages",
        "Mobile app, don’t create 2 screens",
        "Mobile app, no need for 2 screens",
    ] {
        // Use the real coverage extractor: ASCII comma-only lists are not
        // enumerated by that extractor, which would make a rejection vacuous.
        let prompt = format!("{header}（375×812）：商品信息（大图+价格）、规格选择（颜色+尺码）。");
        assert_eq!(required_sections(&prompt), vec!["商品信息", "规格选择"]);
        let request = request(&prompt);
        let mut plan = build_fallback_plan(&request);
        assert_eq!(plan.subtasks.len(), 2);
        assert!(
            plan.subtasks.iter().all(|task| task.screen.is_none()),
            "{header}"
        );
        normalize(&mut plan, &request);
        assert!(
            group_subtasks_by_screen(&plan.subtasks).is_empty(),
            "{header}"
        );
        assert!(plan
            .subtasks
            .iter()
            .all(|task| task.parent_frame_id.as_deref() == Some("page")));
    }
}

#[test]
fn english_product_name_is_not_a_negation() {
    let prompt =
        "Notion mobile App 2 screens（375×812）：商品页（大图+价格）、购物车页（金额汇总+结算）。";
    assert_eq!(required_sections(prompt), vec!["商品页", "购物车页"]);
    let plan = build_fallback_plan(&request(prompt));
    assert_eq!(plan.subtasks.len(), 2);
    assert_eq!(plan.subtasks[0].screen.as_deref(), Some("商品页"));
    assert_eq!(plan.subtasks[1].screen.as_deref(), Some("购物车页"));
    assert!(plan.subtasks.iter().all(|task| task.region.height == 812.0));
}

#[test]
fn ordinary_counted_lists_copy_and_unrelated_page_mentions_are_not_screens() {
    let heads = vec!["商品信息".into(), "规格选择".into()];
    for prompt in [
        "手机 App：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App 两个区块：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App 两条列表：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App 一屏：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App 三屏：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App 第2页：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App 第 2 页：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App（示例两屏）：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App 标题“两屏”：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App 标题文案：两屏：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "Caption: 2 screens: 商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "例如手机 App 两屏：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "另有两屏。当前屏：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "交付2页完整界面。当前页区块：商品信息（大图+价格）、规格选择（颜色+尺码）。",
        "手机 App 两屏：商品信息、规格选择。",
        "手机 App 两屏：商品信息（）、规格选择（颜色+尺码）。",
    ] {
        assert!(explicit_screen_items(prompt, &heads).is_none(), "{prompt}");
    }
}

#[test]
fn nested_detail_counts_and_separators_do_not_change_page_ownership() {
    let prompt = "App 2 screens（375×812）：商品页（轮播两屏、规格(颜色、尺码)+评价三条）、购物车页（商品两条+结算）。";
    let heads = vec!["商品页".into(), "购物车页".into()];
    assert!(explicit_screen_items(prompt, &heads).is_some());
    assert!(explicit_screen_items(prompt, &["商品页".into(), "评价".into()]).is_none());
}
