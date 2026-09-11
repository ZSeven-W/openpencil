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
