use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

fn subtask(id: &str, label: &str, elements: Option<&str>) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region {
            width: 1440.0,
            height: 200.0,
        },
        bleed_hero: false,
        id_prefix: String::new(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: elements.map(Into::into),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: None,
        retry_feedback: None,
    }
}

fn st(id: &str, label: &str, elements: &str) -> Subtask {
    subtask(id, label, Some(elements))
}

fn on_screen(mut st: Subtask, screen: &str) -> Subtask {
    st.screen = Some(screen.into());
    st
}

fn with_covers(mut st: Subtask, covers: &[&str]) -> Subtask {
    st.covers = Some(covers.iter().map(|c| c.to_string()).collect());
    st
}

fn plan(subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Dashboard".into(),
            width: 1440.0,
            height: 900.0,
            layout: Some("horizontal".into()),
            gap: Some(0.0),
            padding: None,
            fill: None,
        },
        subtasks,
        style_guide_name: None,
    }
}

fn ids(plan: &OrchestratorPlan) -> Vec<&str> {
    plan.subtasks.iter().map(|st| st.id.as_str()).collect()
}

/// arena-d01 0927a run-2: `main (主内容区) elements="(container)"`.
fn sample_bare_container() -> OrchestratorPlan {
    plan(vec![
        st(
            "sidebar",
            "左侧栏",
            "品牌 logo 区, 六个导航项（概览/数据报表/用户管理/订单管理/渠道分析/系统设置）, 底部用户头像+姓名+角色",
        ),
        st("main", "主内容区", "(container)"),
        st("topbar", "顶部工具栏", "页面标题, 搜索框, 时间范围选择器, 通知铃铛, 导出按钮"),
        st(
            "kpis",
            "KPI 卡片行",
            "四个 KPI 卡（GMV、活跃用户、订单量、转化率），每卡含数值+环比涨跌+迷你趋势",
        ),
        st(
            "charts",
            "图表区",
            "左侧折线图（GMV 趋势，带图例与时间轴），右侧柱状图（渠道订单对比，带数值标注），两图并排",
        ),
        st(
            "table",
            "数据表",
            "8 行 × 6 列数据表（日期、渠道、订单量、GMV、转化率、客单价），表头排序箭头，行悬停态，底部分页器",
        ),
    ])
}

/// arena-d01 0926e run-1: every item is a sibling's label, tersely reworded.
fn sample_layout_list() -> OrchestratorPlan {
    plan(vec![
        st(
            "sidebar",
            "左侧栏",
            "品牌区（logo+名称）、六个垂直导航项（数据概览、用户分析、订单管理、内容运营、渠道投放、系统设置，当前项高亮）、底部用户卡片（头像+用户名+角色）、退出按钮",
        ),
        st("main", "主内容区", "垂直布局：顶部工具栏、KPI 卡行、图表行、数据表"),
        st(
            "toolbar",
            "顶部工具栏",
            "页面标题「运营数据总览」、日期范围选择器、时间维度切换（日/周/月）、搜索框、导出按钮、刷新按钮、通知铃铛",
        ),
        st(
            "kpis",
            "KPI 卡片行",
            "四个 KPI 卡并排：今日活跃用户（数值+环比涨幅）、今日订单量（数值+环比涨幅）、今日营收（数值+环比降幅）、转化率（数值+环比涨幅），每卡含迷你趋势标识",
        ),
        st(
            "charts",
            "图表区",
            "并排两卡：左侧折线图（近30日营收趋势，含图例+悬浮提示），右侧柱状图（各渠道订单量对比，含图例）",
        ),
        st(
            "brief-section-1",
            "折线图+柱图并排",
            "the brief explicitly requires this section: 折线图+柱图并排 — build exactly what it names, with every item and count the brief gives for it",
        ),
        st(
            "table",
            "数据表",
            "数据表标题「渠道明细」、8行6列表格（渠道、访问量、订单量、营收、转化率、环比），行悬停态、状态标签、底部分页控件（每页条数、页码、上一页/下一页）",
        ),
    ])
}

/// arena-d02 0926b run-2: `container for header, filter bar, table, pagination`.
fn sample_container_for() -> OrchestratorPlan {
    plan(vec![
        st(
            "sidebar",
            "Sidebar with Grouped Nav",
            "brand block, grouped nav (Main: Dashboard, Analytics; Management: Users active, Roles, Teams; System: Settings, Audit Log), footer user profile mini-card",
        ),
        st("main", "Main Column", "container for header, filter bar, table, pagination"),
        st(
            "header",
            "Header Bar",
            "page title 'Users', search input with icon, notification bell with dot, avatar with name + chevron, primary 'Invite user' button",
        ),
        st(
            "filters",
            "Filter Bar",
            "three select dropdowns (Role, Status, Team), results count text, clear-filters link button",
        ),
        st(
            "table",
            "Users Table",
            "table header row (checkbox, Name, Email, Role, Team, Status, Actions), 10 user rows with avatar + name + email, role text, team text, status pills",
        ),
        st(
            "pagination",
            "Pagination Footer",
            "rows-per-page select ('10 per page'), range text '1–10 of 84', previous/next page buttons with page numbers",
        ),
    ])
}

#[test]
fn drops_a_bare_container_marker() {
    let mut plan = sample_bare_container();
    assert_eq!(drop_umbrella_subtasks(&mut plan), 1);
    assert_eq!(ids(&plan), ["sidebar", "topbar", "kpis", "charts", "table"]);
}

#[test]
fn drops_a_layout_list_naming_only_siblings() {
    let mut plan = sample_layout_list();
    assert_eq!(drop_umbrella_subtasks(&mut plan), 1);
    assert_eq!(
        ids(&plan),
        [
            "sidebar",
            "toolbar",
            "kpis",
            "charts",
            "brief-section-1",
            "table"
        ]
    );
}

#[test]
fn drops_a_container_for_list() {
    let mut plan = sample_container_for();
    let before: Vec<Subtask> = plan
        .subtasks
        .iter()
        .filter(|st| st.id != "main")
        .cloned()
        .collect();
    assert_eq!(drop_umbrella_subtasks(&mut plan), 1);
    assert_eq!(plan.subtasks, before);
}

#[test]
fn keeps_a_main_subtask_with_its_own_content() {
    let mut plan = sample_layout_list();
    plan.subtasks[1].elements = Some("垂直布局：顶部工具栏、KPI 卡行、实时告警列表、数据表".into());
    assert_eq!(drop_umbrella_subtasks(&mut plan), 0);
    assert_eq!(plan.subtasks.len(), 7);
}

#[test]
fn keeps_a_subtask_without_elements() {
    let mut plan = sample_bare_container();
    plan.subtasks[1].elements = None;
    assert_eq!(drop_umbrella_subtasks(&mut plan), 0);
    plan.subtasks[1].elements = Some("   ".into());
    assert_eq!(drop_umbrella_subtasks(&mut plan), 0);
}

#[test]
fn keeps_an_umbrella_with_only_one_sibling() {
    let mut plan = plan(vec![
        st("main", "Main Column", "container for header"),
        st("header", "Header Bar", "title, search"),
    ]);
    assert_eq!(drop_umbrella_subtasks(&mut plan), 0);
    let mut bare = plan.clone();
    bare.subtasks[0].elements = Some("(container)".into());
    assert_eq!(drop_umbrella_subtasks(&mut bare), 0);
}

#[test]
fn never_matches_across_screens() {
    // The items name subtasks of the settings screen; the umbrella's own
    // screen has two other sections that it does not name.
    let mut plan = plan(vec![
        on_screen(st("home-hero", "Hero", "headline, CTA"), "Home"),
        on_screen(
            st(
                "home-main",
                "Main Column",
                "container for profile card, preferences list",
            ),
            "Home",
        ),
        on_screen(st("home-feed", "Feed", "post cards"), "Home"),
        on_screen(st("profile", "Profile Card", "avatar, name"), "Settings"),
        on_screen(st("prefs", "Preferences List", "toggles"), "Settings"),
    ]);
    assert_eq!(drop_umbrella_subtasks(&mut plan), 0);
    assert_eq!(plan.subtasks.len(), 5);
}

#[test]
fn a_bare_marker_needs_two_siblings_on_its_own_screen() {
    let mut plan = plan(vec![
        on_screen(st("a-main", "Main", "(container)"), "A"),
        on_screen(st("a-body", "Body", "text"), "A"),
        on_screen(st("b-1", "One", "text"), "B"),
        on_screen(st("b-2", "Two", "text"), "B"),
    ]);
    assert_eq!(drop_umbrella_subtasks(&mut plan), 0);
}

#[test]
fn keeps_an_umbrella_whose_covers_no_sibling_can_take() {
    let mut plan = sample_container_for();
    plan.subtasks[1] = with_covers(plan.subtasks[1].clone(), &["activity timeline"]);
    assert_eq!(drop_umbrella_subtasks(&mut plan), 0);
    assert_eq!(plan.subtasks.len(), 6);
}

#[test]
fn moves_covers_onto_the_siblings_that_own_them() {
    let mut plan = sample_layout_list();
    plan.subtasks[1] = with_covers(plan.subtasks[1].clone(), &["KPI 卡行", "数据表"]);
    plan.subtasks[6] = with_covers(plan.subtasks[6].clone(), &["数据表"]);
    assert_eq!(drop_umbrella_subtasks(&mut plan), 1);
    let kpis = plan.subtasks.iter().find(|s| s.id == "kpis").unwrap();
    assert_eq!(kpis.covers.as_deref(), Some(&["KPI 卡行".to_string()][..]));
    let table = plan.subtasks.iter().find(|s| s.id == "table").unwrap();
    assert_eq!(table.covers.as_deref(), Some(&["数据表".to_string()][..]));
}

#[test]
fn keeps_an_umbrella_when_one_item_names_no_sibling_in_its_own_words() {
    // arena-d01 0926d run-2: `图表行` refers to the `charts (折线图+柱图并排)`
    // sibling, but only across languages. Unprovable, so it stays.
    let mut plan = plan(vec![
        st("sidebar", "左侧栏", "品牌 logo 区, 六个导航项"),
        st(
            "main",
            "主内容区",
            "垂直堆叠容器: 顶部工具栏、KPI 卡行、图表行、数据表",
        ),
        st("topbar", "顶部工具栏", "页面标题, 日期范围选择器, 搜索框"),
        st("kpis", "KPI 卡片", "4 个 KPI 卡"),
        st("charts", "折线图+柱图并排", "左卡折线图; 右卡柱状图"),
        st("table", "数据表", "6 列表头, 8 行数据"),
    ]);
    assert_eq!(drop_umbrella_subtasks(&mut plan), 0);
}

#[test]
fn keeps_navigation_chrome_listing_real_items() {
    // A header whose parts were also planned as siblings still owns them.
    let mut plan = plan(vec![
        st("header", "Header", "logo, search, avatar"),
        st("logo", "Logo", "wordmark"),
        st("search", "Search", "input"),
        st("avatar", "Avatar", "image"),
    ]);
    assert_eq!(drop_umbrella_subtasks(&mut plan), 0);
}

#[test]
fn content_items_strip_framing() {
    assert!(content_items("(container)").is_empty());
    assert!(content_items("（主容器）").is_empty());
    assert!(content_items("wrapper").is_empty());
    assert_eq!(
        content_items("垂直布局：顶部工具栏、KPI 卡行、图表行、数据表"),
        ["顶部工具栏", "KPI 卡行", "图表行", "数据表"]
    );
    assert_eq!(
        content_items("container for header, filter bar, table and pagination"),
        ["header", "filter bar", "table", "pagination"]
    );
    // A colon whose head is content, not framing, is left alone.
    assert_eq!(content_items("四个 KPI 卡并排：今日活跃用户").len(), 1);
}

#[test]
fn tolerant_names_set_generic_suffixes_aside() {
    assert!(names_match("KPI 卡行", "KPI 卡片行"));
    assert!(names_match("图表行", "图表区"));
    assert!(names_match("filter bar", "Filter Bar"));
    assert!(names_match("kpi row", "kpis"));
    assert!(names_match("table", "Users Table"));
    assert!(!names_match("实时告警列表", "数据表"));
    assert!(!names_match("图行", "图区"), "one Han char is not a name");
    assert!(!names_match("list", "playlist"));
}

#[test]
fn drops_a_layout_phrase_marker() {
    // arena-d02 0927b run-1: `main (Main Column) elements="vertical container"`
    // was kept, then retried for being blank and drew the users table twice.
    let mut p = plan(vec![
        st("sidebar", "Sidebar", "brand block, grouped vertical nav"),
        st("main", "Main Column", "vertical container"),
        st(
            "header",
            "Header",
            "page title 'Users', search input, avatar",
        ),
        st("filterbar", "Filter Bar", "three select dropdowns"),
        st("table", "Users Table", "10 data rows with status pills"),
    ]);
    assert_eq!(drop_umbrella_subtasks(&mut p), 1);
    assert_eq!(ids(&p), vec!["sidebar", "header", "filterbar", "table"]);
}

#[test]
fn framing_phrases_need_only_layout_words_around_a_wrapper() {
    for framing in [
        "vertical container",
        "main content wrapper",
        "full-width vertical stack",
        "垂直堆叠容器",
        "主内容容器",
        "纵向布局",
    ] {
        assert!(is_framing_phrase(&normalize_name(framing)), "{framing}");
    }
    for content in [
        "card container",
        "hero content",
        "main content",
        "卡片容器",
        "商品列表布局",
    ] {
        assert!(!is_framing_phrase(&normalize_name(content)), "{content}");
    }
}

#[test]
fn drops_an_umbrella_whose_list_ends_in_a_framing_tail() {
    // arena-d03 0927c run-1: the last item carried "三块的布局容器", so
    // "告警列表" never matched its sibling and the dashboard drew twice.
    let mut p = plan(vec![
        st(
            "sidebar",
            "侧边导航栏",
            "品牌 Logo 区块、垂直导航项、底部用户资料块",
        ),
        st(
            "main",
            "主内容区",
            "垂直容器，包含状态摘要、设备网格、告警列表三块的布局容器",
        ),
        st("summary", "顶部状态摘要三卡", "三张摘要卡"),
        st("device-grid", "设备网格", "12 个设备卡片网格"),
        st("alerts", "右侧告警列表", "5 条告警项"),
    ]);
    assert_eq!(drop_umbrella_subtasks(&mut p), 1);
    assert_eq!(ids(&p), vec!["sidebar", "summary", "device-grid", "alerts"]);
}

#[test]
fn framing_tails_are_stripped_only_when_a_name_remains() {
    assert_eq!(strip_framing_tail("告警列表三块的布局容器"), "告警列表");
    assert_eq!(strip_framing_tail("卡片区的容器"), "卡片区");
    assert_eq!(strip_framing_tail("容器"), "容器");
    assert_eq!(strip_framing_tail("商品网格"), "商品网格");
}
