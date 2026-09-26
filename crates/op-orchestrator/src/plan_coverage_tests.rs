use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

const APP11_BRIEF: &str = "日历 App 主页（375×812）：月视图网格、今日日程列表四条（时间+标题+地点+颜色条）、新建按钮、底部导航。";

// The ten CJK suite briefs (openpencil-arena/prompts/web-app-v1, app-11 already above).
const APP01_BRIEF: &str = "外卖 App 首页（375×812）：顶部地址与搜索、分类九宫格、限时优惠横幅、商家列表五个（图+名称+评分+配送费+时长）、底部导航四个标签。";
const APP02_BRIEF: &str = "健身 App 主页（375×812）：今日环形进度、本周条形图七天、训练计划三卡、最近记录列表四条、底部导航。";
const APP04_BRIEF: &str = "音乐播放器播放页（375×812）：大专辑封面、歌名与歌手、进度条与时间、播放控制五按钮、歌词预览两行、底部操作行。";
const APP07_BRIEF: &str = "打车 App 主页（375×812）：地图占位区、当前位置卡、目的地输入、车型选择三档横向卡（图标+名称+预估价）、优惠券条、确认叫车按钮、底部导航。";
const APP10_BRIEF: &str = "智能家居 App 首页（375×812）：房间横向标签、环境数据三卡（温度/湿度/空气）、设备网格六个带开关、场景模式四个、底部导航。";
const APP13_BRIEF: &str = "社交 App 动态流（375×812）：顶部故事横向列表、动态卡三条（头像+用户名+时间+正文+图+互动行）、底部导航五个标签。";
const APP16_BRIEF: &str = "App 设置页（375×812）：账户区带头像、四组设置分组共十二行（图标+标题+右侧值或开关）、退出登录按钮。不要底部导航。";
const APP19_BRIEF: &str = "理财 App 持仓页（375×812）：总资产卡带涨跌、收益折线图、持仓列表五条（名称+份额+市值+涨跌幅）、快捷操作三按钮、底部导航。";
const APP20_BRIEF: &str = "二手交易 App 首页（375×812）：搜索栏、分类图标八个、附近好物瀑布流六卡（图+标题+价格+距离）、底部导航五标签。";

fn subtask(id: &str, label: &str, elements: Option<&str>) -> Subtask {
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

fn app11_three_subtask_plan() -> OrchestratorPlan {
    plan(vec![
        subtask(
            "header",
            "Calendar Header",
            Some("month title, prev/next controls"),
        ),
        subtask(
            "month-grid",
            "Month View Grid",
            Some("weekday labels, date cells"),
        ),
        subtask(
            "bottom-nav",
            "Bottom Navigation Bar",
            Some("home, calendar, profile tabs"),
        ),
    ])
}

#[test]
fn required_sections_extracts_todays_schedule_from_app11_brief() {
    let sections = required_sections(APP11_BRIEF);
    assert!(
        sections.iter().any(|section| section.contains("今日日程")),
        "app-11 brief must yield 今日日程, got {sections:?}"
    );
}

#[test]
fn required_sections_empty_when_brief_has_no_enumeration() {
    assert!(required_sections("a landing page").is_empty());
    assert!(required_sections("日历 App 主页").is_empty());
}

#[test]
fn required_sections_english_with_list() {
    assert_eq!(
        required_sections("a landing page with hero, pricing table and FAQ"),
        vec!["hero", "pricing table", "FAQ"]
    );
}

#[test]
fn required_sections_drops_sentence_length_items() {
    let sections =
        required_sections("包含这是一段明显超过十二个汉字所以应该被丢掉的描述、短标题。");
    assert!(
        !sections
            .iter()
            .any(|section| section.contains("超过十二个")),
        "sentence-length CJK item must be dropped, got {sections:?}"
    );
    assert!(
        sections.iter().any(|section| section == "短标题"),
        "short CJK item must be kept, got {sections:?}"
    );

    let english = required_sections(
        "including this is a very long sentence that should not count, hero and FAQ",
    );
    assert_eq!(english, vec!["hero", "FAQ"]);
}

#[test]
fn missing_sections_app11_three_subtask_plan_drops_schedule() {
    let required = required_sections(APP11_BRIEF);
    let missing = missing_sections(&required, &app11_three_subtask_plan());
    // Required items are emitted whole now (今日日程列表, not 今日日程), and
    // 新建按钮 is a required section the three-subtask plan never names.
    assert_eq!(missing, vec!["今日日程列表", "新建按钮"]);
}

#[test]
fn missing_sections_schedule_list_covers_日程_via_synonym() {
    let required = vec!["日程".to_string()];
    let covered = plan(vec![subtask(
        "agenda",
        "Upcoming",
        Some("schedule list with times"),
    )]);
    assert!(missing_sections(&required, &covered).is_empty());
}

#[test]
fn required_sections_numbered_cjk_parts_and_bullets() {
    assert_eq!(
        required_sections("页面有3个部分：头部、内容、底栏"),
        vec!["头部", "内容", "底栏"]
    );
    assert_eq!(
        required_sections("- hero\n- pricing table\n- FAQ"),
        vec!["hero", "pricing table", "FAQ"]
    );
}

#[test]
fn required_sections_on_the_ten_suite_briefs() {
    let cases: &[(&str, &[&str])] = &[
        (
            APP01_BRIEF,
            &[
                "顶部地址与搜索",
                "分类九宫格",
                "限时优惠横幅",
                "商家列表",
                "底部导航",
            ],
        ),
        (
            APP02_BRIEF,
            &[
                "今日环形进度",
                "本周条形图",
                "训练计划",
                "最近记录列表",
                "底部导航",
            ],
        ),
        (
            APP04_BRIEF,
            &[
                "大专辑封面",
                "歌名与歌手",
                "进度条与时间",
                "播放控制",
                "歌词预览",
                "底部操作行",
            ],
        ),
        (
            APP07_BRIEF,
            &[
                "地图占位区",
                "当前位置卡",
                "目的地输入",
                "车型选择",
                "优惠券条",
                "确认叫车按钮",
                "底部导航",
            ],
        ),
        (
            APP10_BRIEF,
            &[
                "房间横向标签",
                "环境数据",
                "设备网格",
                "场景模式",
                "底部导航",
            ],
        ),
        (
            APP11_BRIEF,
            &["月视图网格", "今日日程列表", "新建按钮", "底部导航"],
        ),
        (APP13_BRIEF, &["顶部故事横向列表", "动态卡", "底部导航"]),
        (APP16_BRIEF, &["账户区", "设置分组", "退出登录按钮"]),
        (
            APP19_BRIEF,
            &["总资产卡", "收益折线图", "持仓列表", "快捷操作", "底部导航"],
        ),
        (
            APP20_BRIEF,
            &["搜索栏", "分类图标", "附近好物瀑布流", "底部导航"],
        ),
    ];
    for (brief, expected) in cases {
        let mut actual = required_sections(brief);
        actual.sort();
        let mut expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
        expected.sort();
        assert_eq!(actual, expected, "brief: {brief}");
    }
}

#[test]
fn required_sections_never_requires_negated_sections() {
    // The suite brief excludes the bottom nav after a sentence end; a inline
    // variant keeps it inside the same enumerated list.
    for brief in [
        APP16_BRIEF,
        "App 设置页：账户区、设置分组、退出登录按钮，不要底部导航",
    ] {
        let sections = required_sections(brief);
        assert!(
            !sections.iter().any(|section| section.contains("底部导航")),
            "不要底部导航 must never become a required section, got {sections:?}"
        );
        assert!(
            sections.iter().any(|section| section == "退出登录按钮"),
            "real sections around the negation must survive, got {sections:?}"
        );
    }
}

// ── motion50 fix 3: motion clauses and paren fragments are not sections ─────

const MOTION50_APP01_BRIEF: &str = "冥想 App 三屏可交互原型（各 375×812）：首页(问候+今日推荐卡+课程列表四条)、呼吸练习屏(大呼吸圆环+计时+暂停)、完成屏(时长与连续天数统计)。交互：首页「开始今日练习」onTap 进呼吸屏，呼吸屏「完成」onTap 进完成屏。动效：呼吸圆环 mount 用 emphasizedDecelerate 缩放淡入 500ms，课程卡 inView fade-up 交错 delayMs 60ms 递增，完成屏统计数字滚动。深色午夜蓝渐变 + 柔光玻璃卡，一个签名瞬间：呼吸圆环外的三层同心光晕。";

const MOTION50_WEB07_BRIEF: &str = "开源项目官网（1440 宽长页）：英雄(项目名+一句话+安装命令块+GitHub 星标 count-up)、特性六格、代码示例区、生态 logo、贡献者头像墙、页脚。滚动动效：安装块 mount、特性格 inView 交错、代码区 sticky 随滚动高亮不同行。终端深色 + 等宽字。";

const MOTION50_OTHER03_BRIEF: &str = "知识卡片一组三张（1080×1350）：主题「如何做好一次设计评审」，每张含标题、三条要点、底部署名条。动效：标题逐字揭示、要点 inView 交错。高对比撞色 + 大字号，适合社媒。";

/// motion50 lane1/web-07: the motion sentence's clauses ("安装块 mount",
/// "特性格 inView 交错", "代码区 sticky 随滚动高亮不同行") are animation
/// directions, not sections — and the hero's parenthesised "count-up" note
/// must not drag the genuine 英雄 section down with it.
#[test]
fn motion_direction_clauses_are_never_sections() {
    let sections = required_sections(MOTION50_WEB07_BRIEF);
    for phantom in ["mount", "inView", "sticky", "count-up"] {
        assert!(
            !sections.iter().any(|s| s.contains(phantom)),
            "{phantom} clause must not become a section, got {sections:?}"
        );
    }
    for genuine in [
        "英雄",
        "特性六格",
        "代码示例区",
        "生态 logo",
        "贡献者头像墙",
        "页脚",
    ] {
        assert!(
            sections.iter().any(|s| s == genuine),
            "genuine section {genuine} must survive, got {sections:?}"
        );
    }
}

/// motion50 app-01: splitting "完成屏(时长与连续天数统计)" on 与 used to leave
/// two unbalanced-paren fragments. Parenthetical notes are now stripped
/// before splitting, so neither fragment appears and the genuine 完成屏 screen
/// survives (it used to be dropped along with its fragments).
#[test]
fn unbalanced_paren_fragments_are_never_sections() {
    let sections = required_sections(MOTION50_APP01_BRIEF);
    assert_eq!(
        sections,
        vec!["首页", "呼吸练习屏", "完成屏"],
        "got {sections:?}"
    );
    // A genuinely unbalanced fragment (paren crossing the list boundary) is
    // still screened out.
    let fragments = required_sections("首页：完成屏(时长、连续天数统计");
    assert!(!fragments.iter().any(|s| s.contains('(')), "{fragments:?}");
}

/// motion50 lane0/other-03: "每张含标题" is a per-item descriptor clause (a
/// 含-led sentence fragment), not a section noun; the motion sentence's
/// "标题逐字揭示 / 要点 inView 交错" are animation directions.
#[test]
fn per_item_descriptor_and_motion_clauses_are_never_sections() {
    let sections = required_sections(MOTION50_OTHER03_BRIEF);
    for phantom in ["每张含标题", "逐字揭示", "inView"] {
        assert!(
            !sections.iter().any(|s| s.contains(phantom)),
            "{phantom} must not become a section, got {sections:?}"
        );
    }
    for genuine in ["三条要点", "底部署名条"] {
        assert!(
            sections.iter().any(|s| s == genuine),
            "genuine section {genuine} must survive, got {sections:?}"
        );
    }
}

/// The gate must keep firing on a genuinely missing section.
#[test]
fn a_truly_missing_priced_section_still_fires() {
    let required = required_sections("产品官网（1440）：包含定价三档、关于我们、页脚");
    assert!(
        required.iter().any(|s| s.contains("定价")),
        "定价 must stay a required section, got {required:?}"
    );
    let covering_except_pricing = plan(vec![subtask(
        "about",
        "关于我们 + 页脚",
        Some("team intro and footer links"),
    )]);
    let missing = missing_sections(&required, &covering_except_pricing);
    assert!(
        missing.iter().any(|s| s.contains("定价")),
        "coverage gate must still report the missing 定价 section, got {missing:?}"
    );
}

// ── covers backfill (plan-coverage v2): trust the planner's own claims ──────

/// The 0919 GLM-Flash web-11 brief: the plan names every section (English
/// labels, Chinese section titles inside elements), yet the gate reported
/// 英雄 / 色板与字阶展示 / 快速开始代码块 / 页脚 missing — synonym and language
/// drift the substring matcher can never bridge.
const WEB11_BRIEF: &str = "设计系统文档站首页（1440 宽长页）：英雄（标题+搜索框）、组件网格九个、设计原则三条、色板与字阶展示、快速开始代码块、页脚。交互：组件卡 hover 显示描述，代码块可复制。滚动动效：组件格 inView 交错，色板 mount 逐块展开，代码块 sticky。风格由你决定，要求信息清晰、层级分明。";

fn covered_subtask(id: &str, label: &str, elements: Option<&str>, covers: &[&str]) -> Subtask {
    let mut st = subtask(id, label, elements);
    st.covers = Some(covers.iter().map(|entry| entry.to_string()).collect());
    st
}

/// The plan the planner actually ships for web-11 — English labels, free-text
/// elements, and each subtask carrying the brief section it covers verbatim.
fn web11_plan(covers: bool) -> OrchestratorPlan {
    let backfill = |id: &str, label: &str, elements: &str, section: &str| {
        if covers {
            covered_subtask(id, label, Some(elements), &[section])
        } else {
            subtask(id, label, Some(elements))
        }
    };
    plan(vec![
        backfill(
            "hero",
            "Hero Section",
            "eyebrow, headline, large search input",
            "英雄",
        ),
        backfill(
            "grid",
            "Component Grid",
            "cards with mini previews",
            "组件网格",
        ),
        backfill(
            "principles",
            "Design Principles",
            "numbered principle cards",
            "设计原则",
        ),
        backfill(
            "tokens",
            "Color Palette & Type Scale",
            "swatch ramp rows, type specimens",
            "色板与字阶展示",
        ),
        backfill(
            "quickstart",
            "Quick Start Code Block",
            "numbered steps, tabbed code panel",
            "快速开始代码块",
        ),
        backfill(
            "footer",
            "Footer",
            "brand block, link columns, copyright",
            "页脚",
        ),
    ])
}

/// web-11 with the planner's covers backfill: every required section is
/// claimed verbatim, so the gate must pass. (Red before the fix: the
/// substring matcher cannot see 英雄 in "Hero Section" etc.)
#[test]
fn web11_plan_with_covers_backfill_passes_the_gate() {
    let required = required_sections(WEB11_BRIEF);
    for section in [
        "英雄",
        "组件网格",
        "设计原则",
        "色板与字阶展示",
        "快速开始代码块",
        "页脚",
    ] {
        assert!(
            required.iter().any(|s| s == section),
            "web-11 brief must require {section}, got {required:?}"
        );
    }
    let missing = missing_sections(&required, &web11_plan(true));
    assert!(
        missing.is_empty(),
        "covers backfill must cover every web-11 section, still missing {missing:?}"
    );
}

/// The same plan with every covers stripped: no subtask backfills, so the
/// legacy substring verdict must hold unchanged — this is the old-model /
/// old-prompt fallback path, identical to pre-fix behavior.
#[test]
fn web11_plan_without_covers_keeps_the_legacy_verdict() {
    let required = required_sections(WEB11_BRIEF);
    let missing = missing_sections(&required, &web11_plan(false));
    assert_eq!(
        missing,
        vec![
            "英雄",
            "组件网格",
            "设计原则",
            "色板与字阶展示",
            "快速开始代码块",
            "页脚",
        ],
        "without covers the English-labeled plan must still report every section missing"
    );
}

/// covers entries match on normalized EQUALITY, never as substrings: a
/// subtask claiming `定价` does not satisfy the required `定价三档`.
#[test]
fn covers_entry_is_equality_not_substring() {
    let required = vec!["定价三档".to_string()];
    let pricing_only = plan(vec![covered_subtask("pricing", "Pricing", None, &["定价"])]);
    let missing = missing_sections(&required, &pricing_only);
    assert!(
        missing.iter().any(|s| s == "定价三档"),
        "covers [定价] must NOT cover the required 定价三档, got {missing:?}"
    );
}

/// Normalization before that equality: whitespace, full/half-width
/// punctuation, and case differences do not break a covers match.
#[test]
fn covers_match_after_whitespace_and_punctuation_normalization() {
    let required = vec!["英雄区".to_string()];
    for entry in ["英雄 区", "英雄区。", "英雄区，"] {
        let covered = plan(vec![covered_subtask(
            "hero",
            "Hero Section",
            None,
            &[entry],
        )]);
        assert!(
            missing_sections(&required, &covered).is_empty(),
            "covers [{entry}] must cover 英雄区 after normalization"
        );
    }
    let required = vec!["FAQ".to_string()];
    let covered = plan(vec![covered_subtask("faq", "FAQ Section", None, &["faq"])]);
    assert!(
        missing_sections(&required, &covered).is_empty(),
        "covers [faq] must cover FAQ after lowercasing"
    );
}

/// The diagnostic contract: `check_coverage` says WHICH subtask covered each
/// section and whether that came from the covers backfill or the legacy
/// substring text.
#[test]
fn coverage_check_reports_the_source_per_section() {
    let required = vec!["英雄".to_string(), "定价三档".to_string()];
    let mixed = plan(vec![
        covered_subtask("hero", "Hero Section", None, &["英雄"]),
        subtask("pricing", "Pricing", Some("定价三档 pricing cards")),
    ]);
    let check = check_coverage(&required, &mixed);
    assert!(check.missing.is_empty());
    assert_eq!(
        check.covered_by,
        vec![
            (
                "英雄".to_string(),
                "hero".to_string(),
                CoverageSource::Covers
            ),
            (
                "定价三档".to_string(),
                "pricing".to_string(),
                CoverageSource::Text
            ),
        ],
        "covered-by must attribute hero to covers and 定价三档 to text, got {:?}",
        check.covered_by
    );
    assert_eq!(check.covered_by_line(), "hero(covers) pricing(text)");
}

/// arena-m01 (GLM-5.3-Flash): a three-page delivery brief planned every
/// section under its page's `screen`, yet the gate listed 商家详情页 as
/// missing and re-planned, and the re-plan came back missing two pages.
#[test]
fn a_page_named_by_the_brief_is_covered_by_its_screen() {
    let on = |id: &str, label: &str, screen: &str| {
        let mut st = subtask(id, label, None);
        st.screen = Some(screen.into());
        st
    };
    let plan = plan(vec![
        on("home-header", "顶部地址与搜索", "首页"),
        on("store-hero", "商家头图与评分行", "商家详情"),
        on("dish-list", "菜品列表", "商家详情"),
        on("fee-summary", "费用明细", "订单确认页"),
    ]);
    let required = vec!["商家详情页".to_string(), "订单确认页".to_string()];
    let check = check_coverage(&required, &plan);
    assert!(check.missing.is_empty(), "{:?}", check.missing);
}

/// arena-w02: "folder sidebar with counts" — once the planner writes the
/// badges into the sidebar's elements, the gate must see them.
#[test]
fn a_detail_written_into_its_sections_elements_is_covered() {
    let plan = plan(vec![subtask(
        "sidebar",
        "Folder Sidebar",
        Some("vertical folder nav: Inbox, Starred, Sent with an unread count badge on each row"),
    )]);
    let check = check_coverage(&["counts".to_string()], &plan);
    assert!(check.missing.is_empty(), "{:?}", check.missing);
}
