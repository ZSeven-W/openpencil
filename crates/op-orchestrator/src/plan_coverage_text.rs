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
