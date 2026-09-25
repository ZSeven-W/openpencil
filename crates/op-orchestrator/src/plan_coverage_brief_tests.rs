//! Brief-extraction and matching regressions from arena-m02 (0925): the
//! bolded, parenthesised course rail vanished from the required list, and
//! generic synonyms let unrelated subtasks "cover" specific sections.

use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

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

#[test]
fn arena_m02_brief_requires_all_five_sections_including_the_bold_rail() {
    assert_eq!(
        required_sections(ARENA_M02_BRIEF),
        vec![
            "顶部问候与头像",
            "今日目标环形进度",
            "横向滚动的课程卡片轨道",
            "本周活动条形图",
            "底部导航",
        ]
    );
}

#[test]
fn english_brief_requires_its_enumerated_sections_without_articles() {
    assert_eq!(
        required_sections(ENGLISH_FITNESS_BRIEF),
        vec![
            "greeting header",
            "daily goal ring",
            "horizontally scrolling course carousel",
            "weekly activity bar chart",
            "bottom tab bar",
        ]
    );
}

/// `滚动` inside a layout phrase is not motion, but a real scroll-motion
/// clause still is.
#[test]
fn layout_scroll_phrases_survive_while_scroll_motion_is_still_dropped() {
    let sections = required_sections("首页：横向滚动的推荐卡片、随滚动淡入的标题、页脚");
    assert!(
        sections.iter().any(|s| s == "横向滚动的推荐卡片"),
        "{sections:?}"
    );
    assert!(
        !sections.iter().any(|s| s.contains("随滚动")),
        "{sections:?}"
    );
}

/// `ms` is only motion as a duration: `items` / `forms` are sections.
#[test]
fn ms_inside_english_words_is_not_a_motion_duration() {
    let sections = required_sections("a support page with contact forms, FAQ items and a footer");
    assert_eq!(sections, vec!["contact forms", "FAQ items", "footer"]);
    let motion = required_sections("a page with hero fade 300ms, pricing and footer");
    assert_eq!(motion, vec!["pricing", "footer"]);
}

/// A comma inside a parenthetical note never splits a section in two.
#[test]
fn commas_inside_parenthetical_notes_do_not_split_sections() {
    let sections = required_sections("首页：推荐轨道（六张，可横滑）、底部导航");
    assert_eq!(sections, vec!["推荐轨道", "底部导航"]);
}

/// A generic container word (`cards`, `list`) in some subtask never covers a
/// specific section that merely contains it.
#[test]
fn generic_synonyms_do_not_cover_specific_sections() {
    let required = vec![
        "横向滚动的课程卡片轨道".to_string(),
        "今日日程列表".to_string(),
    ];
    let unrelated = plan(vec![
        subtask("stats", "Stats", "three stat cards"),
        subtask("recent", "Recent", "activity list"),
    ]);
    assert_eq!(missing_sections(&required, &unrelated), required);
}

/// The `的` head carries identity: a plan naming `课程卡片轨道` covers the
/// brief's `横向滚动的课程卡片轨道`.
#[test]
fn de_head_of_a_modified_section_covers_it() {
    let required = vec!["横向滚动的课程卡片轨道".to_string()];
    let covering = plan(vec![subtask("rail", "课程卡片轨道", "six course cards")]);
    assert!(missing_sections(&required, &covering).is_empty());
}

/// English tokens must start a word: `chart` inside `flowchart` or `weekly`
/// inside `biweekly` does not cover `weekly chart`; inflections still do.
#[test]
fn english_tokens_must_start_a_word() {
    let required = vec!["weekly chart".to_string()];
    let near_miss = plan(vec![subtask("x", "Biweekly", "flowchart palette")]);
    assert_eq!(missing_sections(&required, &near_miss), required);
    let covering = plan(vec![subtask("x", "Charts", "weekly totals")]);
    assert!(missing_sections(&required, &covering).is_empty());
}
