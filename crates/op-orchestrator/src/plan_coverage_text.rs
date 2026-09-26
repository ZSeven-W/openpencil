//! Text screens for the plan-coverage gate: motion-clause detection, brief
//! markup stripping, and word-bounded matching. Split out of
//! [`crate::plan_coverage`] to keep the spine under the file-size cap.

use crate::design_type::contains_word;
use regex::Regex;
use std::sync::LazyLock;

/// Motion/interaction direction words (motion50 fix 3, lane1/web-07 et al.).
/// A candidate clause carrying one of these is an animation/interaction
/// instruction (`安装块 mount`, `课程卡 inView 上浮交错`, `代码区 sticky 随
/// 滚动高亮不同行`, `9）:封面`-style residue aside), never a layout section —
/// generating one phantom requirement used to burn a pointless
/// `PlanCoverageRetry` on nearly every brief. Matched lowercase; ASCII words
/// match on word boundaries so `items`/`forms`/`programs` never read as `ms`.
const MOTION_WORDS: &[&str] = &[
    "mount",
    "inview",
    "transition",
    "ontap",
    "pressed",
    "hover",
    "视差",
    "parallax",
    "sticky",
    "交错",
    "stagger",
    "滚动",
    "数字滚动",
    "count-up",
    "逐字",
    "逐词",
    "揭示",
    "动效",
    "动画",
    "淡入",
    "滑入",
    "缩放",
    "easing",
];

/// Scroll phrases that describe a section's LAYOUT (a horizontal card rail),
/// not an animation. `横向滚动的课程卡片轨道` is a section; only the bare
/// `滚动` inside it looked like motion (arena-m02: the rail was silently
/// dropped from the required list).
const LAYOUT_SCROLL_PHRASES: &[&str] = &[
    "横向滚动",
    "水平滚动",
    "左右滚动",
    "横向滑动",
    "左右滑动",
    "可横向滚动",
    "可滚动",
];

/// `500ms` / `60 ms` — a duration, the only form in which `ms` is motion.
static DURATION_MS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[0-9]\s*ms\b").expect("valid duration pattern"));

/// Markdown emphasis markers (`**横向课程轨道**`, `__hero__`). A brief that
/// bolds the section it cares most about must not lose it to the markers.
static EMPHASIS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\*\*|__").expect("valid emphasis pattern"));

/// A leading English article on an enumerated item (`a weekly chart`).
static LEADING_ARTICLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:a|an|the)\s+").expect("valid article pattern"));

/// A clause carrying an animation/interaction direction word is not a section.
/// Runs on the PAREN-STRIPPED text so a genuine section whose parenthetical
/// note mentions motion (`英雄(… count-up)`) survives (motion50 fix 3).
pub(crate) fn is_motion_clause(text: &str) -> bool {
    let mut lower = text.to_lowercase();
    for phrase in LAYOUT_SCROLL_PHRASES {
        lower = lower.replace(phrase, " ");
    }
    DURATION_MS.is_match(&lower)
        || MOTION_WORDS
            .iter()
            .any(|needle| contains_word(&lower, needle))
}

/// Remove Markdown emphasis markers from the whole brief.
pub(crate) fn strip_emphasis(brief: &str) -> String {
    EMPHASIS.replace_all(brief, "").into_owned()
}

/// Drop one leading English article.
pub(crate) fn strip_leading_article(text: &str) -> String {
    LEADING_ARTICLE.replace(text, "").into_owned()
}

/// Substring match for CJK needles; for ASCII needles the match must START at
/// a word boundary — `list` must not match `playlist`, `chart` must not match
/// `flowchart` — while its end may run on, so `bottom nav` still matches
/// `bottom navigation` and `card` matches `cards`.
pub(crate) fn contains_term(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    if !needle.is_ascii() {
        return haystack.contains(needle);
    }
    let bytes = haystack.as_bytes();
    haystack.match_indices(needle).any(|(start, _)| {
        start == 0 || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_')
    })
}

/// The noun head of a `<modifier>的<noun>` section:
/// `横向滚动的课程卡片轨道` → `课程卡片轨道`. `None` when there is no `的` or
/// the head is too short to be a section on its own (≥3 CJK chars).
pub(crate) fn de_head(section: &str) -> Option<String> {
    let (_, head) = section.rsplit_once('的')?;
    let head = head.trim();
    let han = head
        .chars()
        .filter(|ch| crate::plan_coverage::is_han(*ch))
        .count();
    (han >= 3).then(|| head.to_string())
}

/// Whether a subtask `label` names the required `section` in fewer words.
///
/// Planners label a section tersely or reorder its words: the brief asks for
/// `右侧任务详情抽屉打开状态` and the plan says `右侧任务详情抽屉`; the brief
/// says `侧栏项目列表` and the plan says `项目侧栏`. Substring and synonym
/// matching miss both, so the gate appended duplicates of sections the plan
/// already had. A label counts when it is at least four Han characters, every
/// one of them occurs in the section (as a multiset), and together they make
/// up most of it.
///
/// Where on the screen a section sits is said either way round — the brief
/// asks for `顶部门店信息`, the plan says `门店信息头部` — so the position
/// words are set aside on both sides before comparing.
pub(crate) fn label_names_section(section: &str, label: &str) -> bool {
    const POSITION_WORDS: [&str; 5] = ["顶部", "头部", "顶端", "上方", "上部"];
    let han = |text: &str| -> Vec<char> {
        let mut text = text.to_string();
        for word in POSITION_WORDS {
            text = text.replace(word, "");
        }
        text.chars()
            .filter(|ch| crate::plan_coverage::is_han(*ch))
            .collect()
    };
    let label_chars = han(label);
    let mut section_chars = han(section);
    if label_chars.len() < 4 || section_chars.is_empty() {
        return false;
    }
    let total = section_chars.len();
    for ch in &label_chars {
        match section_chars.iter().position(|c| c == ch) {
            Some(i) => {
                section_chars.swap_remove(i);
            }
            None => return false,
        }
    }
    label_chars.len() * 10 >= total * 6
}

/// Whether a required `section` is really a page the plan put its subtasks
/// on. A multi-screen brief names its pages (`商家详情页`, `订单确认页`) and
/// the planner files the sections of each page under a `screen` — the page
/// itself is never a subtask of its own, so matching labels and elements
/// alone reported every page after the first as missing and re-planned.
pub(crate) fn screen_names_section(section: &str, screen: &str) -> bool {
    let strip = |text: &str| -> String {
        let mut text: String = text
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        for suffix in ["页面", "页", "screen", "page"] {
            if let Some(head) = text.strip_suffix(suffix) {
                text = head.to_string();
                break;
            }
        }
        text
    };
    let (section, screen) = (strip(section), strip(screen));
    let han = section
        .chars()
        .filter(|ch| crate::plan_coverage::is_han(*ch))
        .count();
    !section.is_empty()
        && (section == screen
            || (han >= 2 && screen.contains(&section))
            || label_names_section(&section, &screen))
}

/// The parts of a compound brief section — `折线图+柱图并排` asks for a line
/// chart AND a bar chart laid side by side, which a plan says as
/// "并排两卡：左侧折线图…右侧柱状图". Split on `+`, drop the trailing layout
/// word, and spell common chart short forms in full so each part can be
/// matched on its own. `None` unless there are at least two parts.
pub(crate) fn compound_section_parts(section: &str) -> Option<Vec<String>> {
    const LAYOUT_WORDS: [&str; 6] = ["并排", "并列", "左右", "上下", "两列", "横排"];
    const SHORT_FORMS: [(&str, &str); 4] = [
        ("柱图", "柱状图"),
        ("线图", "折线图"),
        ("饼图", "饼状图"),
        ("条图", "条形图"),
    ];
    if !section.contains(['+', '＋']) {
        return None;
    }
    let parts: Vec<String> = section
        .split(['+', '＋'])
        .map(|part| {
            let mut part = part.trim().to_string();
            for word in LAYOUT_WORDS {
                if let Some(head) = part.strip_suffix(word) {
                    part = head.trim().to_string();
                }
            }
            for (short, full) in SHORT_FORMS {
                if part == short || (part.ends_with(short) && !part.ends_with(full)) {
                    part = format!("{}{full}", &part[..part.len() - short.len()]);
                }
            }
            part
        })
        .filter(|part| !part.is_empty())
        .collect();
    (parts.len() >= 2).then_some(parts)
}

#[cfg(test)]
mod label_names_section_tests {
    use super::label_names_section;

    #[test]
    fn a_compound_section_splits_into_its_parts() {
        use super::compound_section_parts;
        assert_eq!(
            compound_section_parts("折线图+柱图并排"),
            Some(vec!["折线图".to_string(), "柱状图".to_string()])
        );
        assert_eq!(compound_section_parts("KPI 卡片"), None);
        assert_eq!(compound_section_parts("+"), None);
    }

    #[test]
    fn a_page_name_matches_the_screen_its_sections_sit_on() {
        use super::screen_names_section;
        assert!(screen_names_section("商家详情页", "商家详情"));
        assert!(screen_names_section("订单确认页", "订单确认页"));
        assert!(screen_names_section("首页", "首页"));
        assert!(!screen_names_section("商家详情页", "订单确认"));
    }

    #[test]
    fn position_words_may_sit_on_either_side() {
        // Measured (GLM-5.3-Flash): `store-header(门店信息头部)` was planned,
        // yet the gate reported `顶部门店信息` missing and re-planned.
        assert!(label_names_section("顶部门店信息", "门店信息头部"));
        assert!(!label_names_section("顶部门店信息", "顶部导航头部"));
    }

    #[test]
    fn a_terse_or_reordered_label_names_its_section() {
        assert!(label_names_section(
            "右侧任务详情抽屉打开状态",
            "右侧任务详情抽屉"
        ));
        assert!(label_names_section("侧栏项目列表", "项目侧栏"));
    }

    #[test]
    fn a_short_or_unrelated_label_does_not() {
        assert!(!label_names_section("今日目标环形进度", "今日"));
        assert!(!label_names_section("侧栏项目列表", "顶部标签切换"));
        assert!(!label_names_section("横向滚动的课程卡片轨道", "课程推荐"));
        assert!(!label_names_section("商家详情页面顶部横幅", "商家列表"));
    }
}
