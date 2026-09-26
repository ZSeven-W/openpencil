//! Last-resort matching tier for the plan-coverage gate: reduce a required
//! section to its core noun and look for that in each subtask.
//!
//! A brief names a section together with where it sits, how many items it
//! holds and what state it is drawn in — `右侧告警列表五条`,
//! `下方数据表 8 行 6 列`, `右侧任务详情抽屉打开状态`, `FAQ six items`,
//! `three feature sections alternating image/text`. A planner labels the same
//! section by its noun alone (`告警列表面板`, `数据表`, `任务详情抽屉`,
//! `FAQ Section`, `Feature Section 1..3`). Every miss of that kind cost a
//! re-plan and, when the re-plan read the same, an appended duplicate
//! subtask (arena-d01 drew its data table twice). This tier strips exactly
//! those three kinds of qualifier — position, count, state — and nothing
//! that names what the section is.

use crate::plan::{OrchestratorPlan, Subtask};
use crate::plan_coverage::{
    alias_matches, aliases_for, english_singular, han_count, subtask_haystack,
};
use crate::plan_coverage_text::{contains_term, label_names_section, strip_position_words};
use regex::Regex;
use std::sync::LazyLock;

/// A CJK count with its classifier, optionally distributive
/// (`五条`, `8 行`, `各四张`, `12 个`).
static CJK_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"各?(?:[0-9]+|[零〇一二两三四五六七八九十百]+)\s*(?:个|条|张|项|页|行|列|位|款|篇|栏|卡|组|档|天|块|种|套|排|屏)",
    )
    .expect("valid CJK count phrase")
});

const EN_NUMBER: &str =
    r"(?:[0-9]+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)";

/// `three feature sections` → `feature sections`.
static EN_LEADING_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^{EN_NUMBER}\s+")).expect("valid English leading count")
});

/// `faq six items` → `faq`, `pricing three tiers` → `pricing`.
static EN_TRAILING_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"\s+{EN_NUMBER}\s+[a-z]+$")).expect("valid English trailing count")
});

/// A descriptor that follows the noun: `footer with four columns`,
/// `feature sections alternating image/text`, `a table of 10 rows`.
static EN_DESCRIPTOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+(?:with|alternating|of|featuring|showing|containing)\s.*$")
        .expect("valid English descriptor tail")
});

/// A trailing state/variant word: `drawer open state`, `row hover`.
static EN_STATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+(?:open|opened|expanded|collapsed|selected|active|hover|hovered)(?:\s+state)?$")
        .expect("valid English state tail")
});

/// State/variant suffixes a brief appends to the section it wants drawn in
/// that state (`任务详情抽屉打开状态`), and `切换`, which names the
/// interaction of a tab row (`标签切换`) rather than the row itself.
const CJK_STATE_SUFFIXES: &[&str] = &[
    "打开状态",
    "展开状态",
    "收起状态",
    "折叠状态",
    "选中状态",
    "激活状态",
    "悬停状态",
    "默认状态",
    "打开态",
    "展开态",
    "收起态",
    "折叠态",
    "选中态",
    "激活态",
    "悬停态",
    "默认态",
    "切换",
];

/// The chart kinds a compound brief section (`折线图+柱图并排`) is made of.
const CHART_WORDS: &[&str] = &["图表", "chart"];

/// The id of the first subtask that covers `section` once its position,
/// count and state qualifiers are set aside; `None` when nothing does, or
/// when the section has no such qualifier to set aside.
pub(crate) fn core_covering_subtask(section: &str, plan: &OrchestratorPlan) -> Option<String> {
    if han_count(section) > 0 {
        let core = cjk_core(section)?;
        return plan
            .subtasks
            .iter()
            .find(|st| cjk_core_covered(&core, st))
            .map(|st| st.id.clone());
    }
    let core = english_core(section)?;
    if let Some(st) = plan.subtasks.iter().find(|st| {
        let haystack = subtask_haystack(st);
        aliases_for(&core)
            .iter()
            .any(|alias| alias_matches(&haystack, alias))
    }) {
        return Some(st.id.clone());
    }
    numbered_subtasks(&core, plan)
}

/// A compound section made only of chart kinds (`折线图+柱图并排`) is the
/// chart row a plan labels `图表区` / `Charts Row` — when that subtask says
/// nothing finer. Once its elements name a chart kind, the part-by-part
/// match decides: `图表区` planning only a line chart does not cover a line
/// chart AND a bar chart.
pub(crate) fn chart_row_subtask(parts: &[String], plan: &OrchestratorPlan) -> Option<String> {
    if !parts.iter().all(|part| part.ends_with('图')) {
        return None;
    }
    plan.subtasks
        .iter()
        .find(|st| {
            let label = st.label.to_lowercase();
            let names_a_kind = st
                .elements
                .as_deref()
                .is_some_and(|elements| parts.iter().any(|part| elements.contains(part.as_str())));
            !names_a_kind && CHART_WORDS.iter().any(|word| contains_term(&label, word))
        })
        .map(|st| st.id.clone())
}

/// `右侧告警列表五条` → `告警列表`; `看板三列各四张任务卡` → `看板`;
/// `四个 KPI 卡` → `KPI 卡`; `右侧任务详情抽屉打开状态` → `任务详情抽屉`.
///
/// When a count splits the section, the words before it name the section
/// and the words after describe its items (`看板` holds `任务卡`), so the
/// head is the core; a leading count (`四个 KPI 卡`) is just dropped.
pub(crate) fn cjk_core(section: &str) -> Option<String> {
    let text = strip_state_suffix(section.trim());
    let text = match CJK_COUNT.find(&text) {
        Some(found) if han_count(text[..found.start()].trim()) >= 2 => {
            text[..found.start()].trim().to_string()
        }
        Some(_) => collapse_spaces(&CJK_COUNT.replace_all(&text, " ")),
        None => text,
    };
    let core = strip_state_suffix(&strip_position_words(&text));
    (!core.is_empty() && core != section.trim()).then_some(core)
}

fn cjk_core_covered(core: &str, st: &Subtask) -> bool {
    // A two-character core (`看板`) is only trusted in a label: in elements
    // it is as likely a tab caption (`看板 / 列表 / 日历`) as the section.
    let short = han_count(core) < 3 && !core.chars().any(|ch| ch.is_ascii_alphabetic());
    let haystack = if short {
        st.label.to_lowercase()
    } else {
        subtask_haystack(st)
    };
    aliases_for(core)
        .iter()
        .any(|alias| alias_matches(&haystack, alias))
        || (!short && han_subsequence(core, &haystack, 2))
        || label_names_section(core, &st.label)
}

/// The core of an English section, lowercased: leading/trailing counts and
/// trailing descriptors and state words dropped.
pub(crate) fn english_core(section: &str) -> Option<String> {
    let lower = collapse_spaces(&section.to_lowercase());
    let text = EN_DESCRIPTOR.replace(&lower, "");
    let text = EN_LEADING_COUNT.replace(&text, "");
    let text = EN_TRAILING_COUNT.replace(&text, "");
    let text = EN_STATE.replace(&text, "");
    let core = text.trim().to_string();
    (!core.is_empty() && core != lower).then_some(core)
}

/// `feature sections` is covered by `feature-1`, `feature-2`, `feature-3`:
/// a plural section the planner split into numbered subtasks.
fn numbered_subtasks(core: &str, plan: &OrchestratorPlan) -> Option<String> {
    let last = core.split_whitespace().last()?;
    english_singular(last)?;
    let stem = core.split_whitespace().next()?;
    let stem = english_singular(stem).unwrap_or_else(|| stem.to_string());
    let numbered: Vec<&Subtask> = plan
        .subtasks
        .iter()
        .filter(|st| {
            st.id
                .to_lowercase()
                .strip_prefix(&stem)
                .map(|rest| rest.trim_start_matches(['-', '_']))
                .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit()))
        })
        .collect();
    (numbered.len() >= 2).then(|| numbered[0].id.clone())
}

fn strip_state_suffix(text: &str) -> String {
    let mut text = text.trim().to_string();
    loop {
        let Some(head) = CJK_STATE_SUFFIXES
            .iter()
            .find_map(|suffix| text.strip_suffix(suffix))
            .map(str::trim)
            .filter(|head| han_count(head) >= 2)
        else {
            return text;
        };
        text = head.to_string();
    }
}

/// Whether the Han characters of `core` occur in order inside `haystack`
/// within a window at most `slack` characters longer than `core`:
/// `数据表` in `数据明细表`.
fn han_subsequence(core: &str, haystack: &str, slack: usize) -> bool {
    let needle: Vec<char> = core.chars().filter(|ch| !ch.is_whitespace()).collect();
    if needle.len() < 3 || needle.iter().any(|ch| !crate::plan_coverage::is_han(*ch)) {
        return false;
    }
    let hay: Vec<char> = haystack.chars().collect();
    let window = needle.len() + slack;
    (0..hay.len())
        .filter(|&i| hay[i] == needle[0])
        .any(|start| {
            let mut matched = 0;
            for ch in hay.iter().skip(start).take(window) {
                if *ch == needle[matched] {
                    matched += 1;
                    if matched == needle.len() {
                        return true;
                    }
                }
            }
            false
        })
}

fn collapse_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[path = "plan_coverage_core_tests.rs"]
mod tests;
