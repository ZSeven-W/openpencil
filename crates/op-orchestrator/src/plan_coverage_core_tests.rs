//! One test per normalisation, each from a real `[PLAN] coverage: missing`
//! line of the GLM-5.3-Flash design-arena runs (0926*, 0927a).

use super::*;
use crate::plan::{Region, RootFrameSpec};
use crate::plan_coverage::{check_coverage, required_section_details, required_sections};
use crate::plan_coverage_append::{append_missing_sections, SkipReason};

fn subtask(id: &str, label: &str, elements: Option<&str>) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region {
            width: 1440.0,
            height: 300.0,
        },
        bleed_hero: false,
        id_prefix: id.into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: elements.map(str::to_owned),
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
            width: 1440.0,
            height: 900.0,
            layout: Some("vertical".into()),
            gap: Some(0.0),
            padding: Some(0.0),
            fill: None,
        },
        subtasks,
        style_guide_name: None,
    }
}

fn labelled(labels: &[(&str, &str)]) -> OrchestratorPlan {
    plan(
        labels
            .iter()
            .map(|(id, label)| subtask(id, label, None))
            .collect(),
    )
}

fn missing(section: &str, plan: &OrchestratorPlan) -> bool {
    check_coverage(&[section.to_string()], plan)
        .missing
        .contains(&section.to_string())
}

const D01: &str = "运营数据看板（1440×900，桌面）：左侧栏（品牌+六个导航项+底部用户）、顶部工具栏、四个 KPI 卡、折线图+柱图并排、下方数据表 8 行 6 列带分页。";
const D03: &str = "IoT 设备监控台（1440×900）：侧栏、顶部状态摘要三卡、设备网格 12 个（图标+名称+在线状态+数值）、右侧告警列表五条带时间与等级色标。";
const W01: &str = "项目管理 Web 应用主界面（1440×900）：侧栏项目列表、顶部标签切换（看板/列表/日历）、看板三列各四张任务卡（标题+标签+头像+截止日）、右侧任务详情抽屉打开状态。";

#[test]
fn a_count_and_classifier_is_not_part_of_the_section() {
    // 0927a arena-d03 run-2: `alerts(告警列表面板)` planned, `右侧告警列表五条`
    // reported missing, re-planned, then appended as a duplicate.
    assert_eq!(cjk_core("右侧告警列表五条").as_deref(), Some("告警列表"));
    assert!(!missing(
        "右侧告警列表五条",
        &labelled(&[("sidebar", "侧栏"), ("alerts", "告警列表面板")])
    ));
    // 0927a arena-d01 run-2: `table(数据表)` planned, `下方数据表 8 行 6 列`
    // reported missing twice; 0926b arena-d01 appended a second table.
    assert_eq!(cjk_core("下方数据表 8 行 6 列").as_deref(), Some("数据表"));
    assert!(!missing(
        "下方数据表 8 行 6 列",
        &labelled(&[("table", "数据表")])
    ));
    assert!(!missing(
        "下方数据表 8 行 6 列",
        &labelled(&[("table", "数据明细表")])
    ));
    // 0926c arena-d01 run-2: `kpis(KPI 卡片行)`.
    assert_eq!(cjk_core("四个 KPI 卡").as_deref(), Some("KPI 卡"));
    assert!(!missing(
        "四个 KPI 卡",
        &labelled(&[("kpis", "KPI 卡片行")])
    ));
}

#[test]
fn a_count_that_splits_a_section_leaves_its_head() {
    // 0927a arena-w01 run-1: `kanban(看板三列任务区)` planned.
    assert_eq!(cjk_core("看板三列各四张任务卡").as_deref(), Some("看板"));
    assert!(!missing(
        "看板三列各四张任务卡",
        &labelled(&[("kanban", "看板三列任务区")])
    ));
    // A two-character head is only trusted in a label: a view switcher lists
    // `看板` as a tab caption in plans without any board.
    let tabs_only = plan(vec![
        subtask("sidebar", "项目侧栏", None),
        subtask(
            "topbar",
            "顶部工具栏",
            Some("分段标签（看板 / 列表 / 日历）"),
        ),
    ]);
    assert!(missing("看板三列各四张任务卡", &tabs_only));
}

#[test]
fn position_and_state_qualifiers_are_set_aside() {
    // 0927a arena-w01 run-1: `drawer(任务详情抽屉)` planned, yet
    // `右侧任务详情抽屉打开状态` was appended as a second drawer.
    assert_eq!(
        cjk_core("右侧任务详情抽屉打开状态").as_deref(),
        Some("任务详情抽屉")
    );
    assert!(!missing(
        "右侧任务详情抽屉打开状态",
        &labelled(&[("drawer", "任务详情抽屉")])
    ));
    // 0927a arena-w01 run-1: `topbar(顶部标签与工具栏)` — `切换` names the
    // interaction of the tab row, not the row.
    assert_eq!(cjk_core("顶部标签切换").as_deref(), Some("顶部标签"));
    assert!(!missing(
        "顶部标签切换",
        &labelled(&[("topbar", "顶部标签与工具栏")])
    ));
}

#[test]
fn a_short_form_matches_the_long_form_and_the_english_name() {
    // 0927a arena-d03 run-1: `sidebar(侧边导航栏)`; 0926b arena-d03: `侧边栏`;
    // 0926 arena-d01: `Left Sidebar`.
    for label in ["侧边导航栏", "侧边栏", "侧栏导航", "Sidebar"] {
        assert!(
            !missing("侧栏", &labelled(&[("sidebar", label)])),
            "{label}"
        );
    }
    assert!(!missing(
        "左侧栏",
        &labelled(&[("sidebar", "Left Sidebar")])
    ));
    // 0926b arena-w01 run-2: `topbar(Top Bar with View Tabs)`.
    assert!(!missing(
        "顶部标签切换",
        &labelled(&[("topbar", "Top Bar with View Tabs")])
    ));
}

#[test]
fn true_misses_stay_missing() {
    // 0926c arena-w01 run-1 planned only a sidebar and a main area.
    let lumped = labelled(&[("sidebar", "项目侧栏"), ("main", "主内容区")]);
    for section in [
        "顶部标签切换",
        "看板三列各四张任务卡",
        "右侧任务详情抽屉打开状态",
    ] {
        assert!(missing(section, &lumped), "{section}");
    }
    // 0926b arena-d03 run-1: no alert list anywhere.
    let no_alerts = labelled(&[("sidebar", "侧栏导航"), ("content", "内容区")]);
    assert!(missing("右侧告警列表五条", &no_alerts));
    // A top bar never vouches for a bottom bar.
    assert!(missing("底部导航栏", &labelled(&[("top", "顶部导航栏")])));
}

#[test]
fn english_counts_and_trailing_descriptors_are_dropped() {
    assert_eq!(english_core("FAQ six items").as_deref(), Some("faq"));
    assert_eq!(
        english_core("footer with four columns").as_deref(),
        Some("footer")
    );
    assert_eq!(
        english_core("three feature sections alternating image/text").as_deref(),
        Some("feature sections")
    );
    // 0927a arena-l02 run-1: `faq(FAQ Section)`, `footer(Footer)`.
    let l02 = labelled(&[
        ("faq", "FAQ Section"),
        ("footer", "Footer"),
        ("filters", "Filter Bar"),
    ]);
    assert!(!missing("FAQ six items", &l02));
    assert!(!missing("footer with four columns", &l02));
    assert!(!missing("filter bar with three selects", &l02));
}

#[test]
fn a_plural_section_is_covered_by_numbered_subtasks() {
    // 0927a arena-l02 run-2: `feature-1(Feature Section — Analytics)` …
    let numbered = labelled(&[
        ("feature-1", "Analytics"),
        ("feature-2", "Automation"),
        ("feature-3", "Collaboration"),
    ]);
    assert!(!missing(
        "three feature sections alternating image/text",
        &numbered
    ));
    assert!(missing(
        "three feature sections alternating image/text",
        &labelled(&[("feature-1", "Analytics")])
    ));
}

#[test]
fn a_chart_compound_is_covered_by_the_chart_row() {
    // 0926c arena-d01: `charts(图表区)`.
    assert!(!missing(
        "折线图+柱图并排",
        &labelled(&[("charts", "图表区")])
    ));
    assert!(!missing(
        "折线图+柱图并排",
        &labelled(&[("charts", "Charts Row")])
    ));
    assert!(missing(
        "折线图+柱图并排",
        &labelled(&[("main", "主内容区")])
    ));
    // Elements that name only one of the charts still leave it missing.
    let one_chart = plan(vec![subtask("charts", "图表区", Some("左侧折线图"))]);
    assert!(missing("折线图+柱图并排", &one_chart));
}

#[test]
fn a_descriptor_detail_is_required_but_never_appended() {
    // arena-d03: `等级色标` is what the alert list carries, split off on `与`.
    let required = required_sections(D03);
    assert!(required.contains(&"等级色标".to_string()));
    let details = required_section_details(D03);
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].name, "等级色标");
    assert_eq!(details[0].detail_of.as_deref(), Some("右侧告警列表五条"));
    // 0927a arena-d03 run-2 retry: appended [右侧告警列表五条, 等级色标].
    let mut shipped = plan(vec![
        subtask("sidebar", "侧栏", Some("brand block, stacked nav items")),
        subtask("summary", "顶部状态摘要三卡", Some("3 stat cards")),
        subtask("devicegrid", "设备网格", Some("grid of 12 device cards")),
        subtask(
            "alerts",
            "告警列表面板",
            Some("5 alert rows each with severity color bar + timestamp"),
        ),
    ]);
    let outcome = append_missing_sections(&mut shipped, &required, &details);
    assert!(outcome.appended.is_empty(), "{outcome:?}");
    assert_eq!(
        outcome.skipped,
        vec![("等级色标".to_string(), SkipReason::Detail)]
    );
}

#[test]
fn an_appended_section_carries_its_details() {
    let required = required_sections(D03);
    let details = required_section_details(D03);
    let mut no_alerts = plan(vec![
        subtask("sidebar", "侧栏", None),
        subtask("summary", "顶部状态摘要三卡", None),
        subtask("devicegrid", "设备网格", None),
    ]);
    let outcome = append_missing_sections(&mut no_alerts, &required, &details);
    assert_eq!(outcome.appended, vec!["右侧告警列表五条".to_string()]);
    let appended = no_alerts
        .subtasks
        .iter()
        .find(|st| st.label == "右侧告警列表五条")
        .expect("appended alert list");
    assert!(appended
        .elements
        .as_deref()
        .is_some_and(|el| el.contains("including 等级色标")));
}

#[test]
fn briefs_without_descriptor_clauses_have_no_details() {
    assert!(required_section_details(D01).is_empty());
    assert!(required_section_details(W01).is_empty());
    // `带头像` has nothing after a conjunction: no detail, and the section
    // list is unchanged.
    let app16 = "App 设置页（375×812）：账户区带头像、四组设置分组共十二行（图标+标题+右侧值或开关）、退出登录按钮。不要底部导航。";
    assert!(required_section_details(app16).is_empty());
}

#[test]
fn an_english_and_after_a_with_clause_is_a_detail() {
    let d02 = "Admin console (1440×900): sidebar with grouped nav, header with search and avatar, filter bar with three selects, a users table of 10 rows × 6 columns (status pills, row actions), pagination footer.";
    let details = required_section_details(d02);
    assert_eq!(details.len(), 1, "{details:?}");
    assert_eq!(details[0].name, "avatar");
    assert_eq!(details[0].detail_of.as_deref(), Some("header with search"));
}
