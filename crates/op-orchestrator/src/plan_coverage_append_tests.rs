use super::*;
use crate::plan::RootFrameSpec;
use crate::plan_coverage::required_sections;

const ARENA_M02_BRIEF: &str = "健身 App 首页（375×812）：顶部问候与头像、今日目标环形进度、**横向滚动的课程卡片轨道**（六张，超出屏幕裁剪）、本周活动条形图七天、底部导航四标签。";

const ENGLISH_FITNESS_BRIEF: &str = "A fitness app home screen (375x812) with a greeting header, a daily goal ring, a horizontally scrolling course carousel, a weekly activity bar chart, and a bottom tab bar.";

fn subtask(id: &str, label: &str, elements: &str) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region {
            width: 375.0,
            height: 200.0,
        },
        bleed_hero: false,
        id_prefix: id.into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: Some(elements.into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    }
}

fn plan(subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "page".into(),
            name: "Page".into(),
            width: 375.0,
            height: 812.0,
            layout: Some("vertical".into()),
            gap: Some(0.0),
            padding: Some(0.0),
            fill: None,
        },
        subtasks,
        style_guide_name: None,
    }
}

/// The exact plan the arena-m02 run shipped (0925 stderr.log): three
/// subtasks for a five-section brief.
fn arena_m02_shipped_plan() -> OrchestratorPlan {
    plan(vec![
        subtask(
            "greeting",
            "顶部问候与头像",
            "quiet section: tonal surfaces, no accent fills, one hairline max; greeting, supporting line, circular profile avatar, notification control",
        ),
        subtask(
            "weekly-activity",
            "本周活动条形图",
            "quiet section: tonal surfaces, no accent fills, one hairline max; seven-day weekly activity bar chart with one vertical bar for each day, highlighted current day",
        ),
        subtask(
            "bottom-navigation",
            "底部导航四标签",
            "quiet section: tonal surfaces, no accent fills, one hairline max; four icon-and-label bottom navigation tabs: 训练, 课程, 社区, 我的, with 训练 selected",
        ),
    ])
}

fn ids(plan: &OrchestratorPlan) -> Vec<&str> {
    plan.subtasks.iter().map(|st| st.id.as_str()).collect()
}

fn labels(plan: &OrchestratorPlan) -> Vec<&str> {
    plan.subtasks.iter().map(|st| st.label.as_str()).collect()
}

#[test]
fn arena_m02_dropped_sections_are_appended_in_brief_order() {
    let required = required_sections(ARENA_M02_BRIEF);
    let mut plan = arena_m02_shipped_plan();

    let outcome = append_missing_sections(&mut plan, &required);

    assert_eq!(
        outcome.appended,
        vec!["今日目标环形进度", "横向滚动的课程卡片轨道"]
    );
    assert!(outcome.skipped.is_empty(), "{:?}", outcome.skipped);
    assert_eq!(
        labels(&plan),
        vec![
            "顶部问候与头像",
            "今日目标环形进度",
            "横向滚动的课程卡片轨道",
            "本周活动条形图",
            "底部导航四标签",
        ]
    );
    let appended = &plan.subtasks[2];
    assert_eq!(appended.id, "brief-section-2");
    assert_eq!(appended.region.width, 375.0);
    assert_eq!(
        appended.covers.as_deref(),
        Some(&["横向滚动的课程卡片轨道".to_string()][..])
    );
    assert!(appended
        .elements
        .as_deref()
        .is_some_and(|e| e.contains("横向滚动的课程卡片轨道")));
    // The gate is now satisfied, and a second pass is a no-op.
    assert!(check_coverage(&required, &plan).missing.is_empty());
    let again = append_missing_sections(&mut plan, &required);
    assert!(again.appended.is_empty());
    assert_eq!(plan.subtasks.len(), 5);
}

#[test]
fn english_brief_dropped_sections_are_appended_before_the_tab_bar() {
    let required = required_sections(ENGLISH_FITNESS_BRIEF);
    let mut plan = plan(vec![
        subtask("greeting", "Greeting Header", "hello line, avatar"),
        subtask("weekly", "Weekly Activity", "bar chart of seven days"),
        subtask("tabs", "Bottom Tab Bar", "four tabs"),
    ]);

    let outcome = append_missing_sections(&mut plan, &required);

    assert_eq!(
        outcome.appended,
        vec!["daily goal ring", "horizontally scrolling course carousel"]
    );
    assert_eq!(
        ids(&plan),
        vec![
            "greeting",
            "brief-section-1",
            "brief-section-2",
            "weekly",
            "tabs"
        ]
    );
}

#[test]
fn a_fully_covering_plan_is_left_unchanged() {
    let required = required_sections(ARENA_M02_BRIEF);
    let mut covering = plan(vec![
        subtask("greeting", "顶部问候与头像", "greeting, avatar"),
        subtask("goal", "今日目标环形进度", "goal ring"),
        subtask(
            "rail",
            "课程卡片轨道",
            "six course cards, horizontal scroll",
        ),
        subtask("weekly", "本周活动条形图", "seven bars"),
        subtask("nav", "底部导航", "four tabs"),
    ]);
    let before = covering.clone();

    let outcome = append_missing_sections(&mut covering, &required);

    assert_eq!(outcome, AppendOutcome::default());
    assert_eq!(covering, before);
    assert_eq!(outcome_log_line(&outcome), None);
}

/// English labels for a Chinese brief: the matcher sees almost everything as
/// missing. That is language drift, not dropped sections — never append.
#[test]
fn translated_labels_are_not_mistaken_for_dropped_sections() {
    let required = required_sections(ARENA_M02_BRIEF);
    let mut english = plan(vec![
        subtask("greeting", "Greeting", "avatar"),
        subtask("goal", "Goal Ring", "ring"),
        subtask("rail", "Course Rail", "six cards"),
        subtask("weekly", "Weekly Chart", "bars"),
        subtask("nav", "Bottom Navigation", "tabs"),
    ]);
    let before = english.clone();

    let outcome = append_missing_sections(&mut english, &required);

    assert!(outcome.appended.is_empty());
    assert!(outcome
        .skipped
        .iter()
        .all(|(_, reason)| *reason == SkipReason::MatcherUnreliable));
    assert_eq!(english, before);
}

/// A single CJK section missing from an otherwise English-labelled plan is a
/// script mismatch, not a drop.
#[test]
fn a_cjk_section_against_ascii_only_labels_is_skipped() {
    let required = vec![
        "hero".to_string(),
        "pricing".to_string(),
        "页脚".to_string(),
    ];
    let mut english = plan(vec![
        subtask("hero", "Hero", "headline"),
        subtask("pricing", "Pricing", "three tiers"),
        subtask("footer", "Site Bottom", "links"),
    ]);

    let outcome = append_missing_sections(&mut english, &required);

    assert!(outcome.appended.is_empty());
    assert_eq!(
        outcome.skipped,
        vec![("页脚".to_string(), SkipReason::ScriptMismatch)]
    );
    assert_eq!(english.subtasks.len(), 3);
}

#[test]
fn status_bars_and_multi_screen_plans_are_never_appended_to() {
    let required = vec![
        "状态栏".to_string(),
        "顶部问候".to_string(),
        "本周图表".to_string(),
    ];
    let mut single = plan(vec![
        subtask("greeting", "顶部问候", "greeting"),
        subtask("weekly", "本周图表", "bars"),
    ]);
    let outcome = append_missing_sections(&mut single, &required);
    assert_eq!(
        outcome.skipped,
        vec![("状态栏".to_string(), SkipReason::StatusBar)]
    );
    assert_eq!(single.subtasks.len(), 2);

    let required = vec![
        "顶部问候".to_string(),
        "本周图表".to_string(),
        "排行榜".to_string(),
    ];
    let mut screens = plan(vec![
        subtask("greeting", "顶部问候", "greeting"),
        subtask("weekly", "本周图表", "bars"),
    ]);
    for st in &mut screens.subtasks {
        st.screen = Some("首页".into());
    }
    let outcome = append_missing_sections(&mut screens, &required);
    assert_eq!(
        outcome.skipped,
        vec![("排行榜".to_string(), SkipReason::MultiScreen)]
    );
    assert_eq!(screens.subtasks.len(), 2);
}

#[test]
fn normalize_drops_are_reported_but_status_bars_are_not() {
    let required = vec!["状态栏".to_string(), "今日目标环形进度".to_string()];
    let before_plan = plan(vec![
        subtask("status", "状态栏", "time, battery"),
        subtask("goal", "今日目标环形进度", "ring"),
    ]);
    let before = check_coverage(&required, &before_plan);
    let after_plan = plan(vec![]);
    assert_eq!(
        dropped_by_normalize(&before, &required, &after_plan),
        vec!["今日目标环形进度"]
    );
}
