//! Plan-coverage gate: the brief enumerated a section the plan never named.
//!
//! Distinct from [`crate::subtask_completeness`]: that gate checks a subtask
//! that promised N repeated items and delivered 0–1. This one fires when the
//! section was never planned at all.

use crate::plan::{OrchestratorPlan, Subtask};
use crate::plan_coverage_core::{chart_row_subtask, core_covering_subtask};
pub use crate::plan_coverage_extract::{required_items, RequiredItem};
use crate::plan_coverage_text::{
    compound_section_parts, contains_term, de_head, label_names_section, screen_names_section,
};

/// Bidirectional synonym groups. A required section that overlaps a group is
/// covered when any member of that group appears in a subtask label/elements.
pub(crate) const SYNONYM_GROUPS: &[&[&str]] = &[
    &["日程", "schedule", "agenda"],
    &["底栏", "底部导航", "bottom nav", "tab bar"],
    &["状态栏", "status bar"],
    &["头部", "顶部", "header"],
    &["列表", "list"],
    &["卡片", "cards"],
    &["搜索", "search"],
    &["轮播", "banner", "carousel"],
    &["月历", "月视图", "month view", "month grid", "calendar"],
];

/// Short forms and the English names planners write for common section
/// nouns: the brief says `侧栏`, the plan `侧边导航栏` or `Sidebar`; the brief
/// says `顶部标签切换`, the plan `View Tabs`. Kept apart from
/// [`SYNONYM_GROUPS`] because that table also marks two-character field nouns
/// (`日程`) for the extraction screen, and `侧栏`/`页脚` are real sections.
const SECTION_SYNONYM_GROUPS: &[&[&str]] = &[
    &[
        "侧栏",
        "侧边栏",
        "侧边导航",
        "左侧栏",
        "左侧边栏",
        "左侧导航",
        "sidebar",
        "side nav",
    ],
    &["标签切换", "视图切换", "tabs", "tab switcher"],
    &["看板", "kanban", "board"],
    &["抽屉", "drawer"],
    &["数据表", "data table"],
    &["页脚", "footer"],
    &["订阅", "subscribe", "subscription", "newsletter"],
    &["评价", "testimonial"],
];

/// Synonym terms too generic to vouch for a longer section on their own: a
/// required `课程卡片轨道` is NOT covered by any subtask that mentions `cards`,
/// and `今日日程列表` is not covered by any `list`. These terms only expand
/// when the section IS the term. `看板` is here because a view switcher lists
/// it as a tab label (`看板 / 列表 / 日历`) in plans with no board at all.
const GENERIC_SYNONYM_TERMS: &[&str] = &[
    "列表", "list", "卡片", "cards", "头部", "顶部", "header", "看板", "kanban", "board",
];

pub(crate) const TYPE_SUFFIXES: &[&str] = &[
    "区域", "模块", "部分", "列表", "网格", "section", "area", "区",
];

/// Extract section nouns the brief EXPLICITLY enumerates. High-precision only.
pub fn required_sections(brief: &str) -> Vec<String> {
    required_items(brief)
        .into_iter()
        .map(|item| item.name)
        .collect()
}

/// The required items the brief enumerated as a detail of a sibling
/// (`右侧告警列表五条带时间与等级色标` → `等级色标`). They stay in
/// [`required_sections`] so a re-plan can fold them into their section, but
/// the appender never turns one into a subtask of its own.
pub fn required_section_details(brief: &str) -> Vec<RequiredItem> {
    required_items(brief)
        .into_iter()
        .filter(|item| item.detail_of.is_some())
        .collect()
}

/// Required sections the plan's subtask labels/elements do not cover.
pub fn missing_sections(required: &[String], plan: &OrchestratorPlan) -> Vec<String> {
    check_coverage(required, plan).missing
}

/// How one required section came to be covered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageSource {
    /// A subtask's `covers` backfill matched the section after normalization.
    Covers,
    /// The legacy label/elements substring match found it.
    Text,
}

impl CoverageSource {
    fn tag(self) -> &'static str {
        match self {
            CoverageSource::Covers => "covers",
            CoverageSource::Text => "text",
        }
    }
}

/// The gate's full verdict: the missing sections plus, for every covered
/// section, which subtask covered it and how — the `covered-by:` diagnostic
/// line's raw material.
#[derive(Debug, Clone, Default)]
pub struct CoverageCheck {
    pub missing: Vec<String>,
    /// `(section, subtask id, source)` per covered section, in required
    /// order.
    pub covered_by: Vec<(String, String, CoverageSource)>,
}

impl CoverageCheck {
    /// `hero(covers) tokens(text)` — one entry per covered section, `-` when
    /// nothing was covered.
    pub fn covered_by_line(&self) -> String {
        if self.covered_by.is_empty() {
            return "-".to_string();
        }
        self.covered_by
            .iter()
            .map(|(_, id, source)| format!("{id}({})", source.tag()))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Whether any subtask carries a non-empty `covers` backfill. Gates the
/// diagnostics: a plan without backfill logs exactly as it did before the
/// field existed.
pub fn plan_has_covers(plan: &OrchestratorPlan) -> bool {
    plan.subtasks.iter().any(covers_present)
}

/// The gate, two tiers per required section (plan-coverage v2):
///
/// 1. **Planner backfill** — any subtask whose `covers` entry equals the
///    section after normalization (whitespace/full-half-width punctuation
///    stripped, lowercased) covers it. The planner copies the brief's own
///    wording, so exact equality is the contract — never a substring.
/// 2. **Legacy substring** — the pre-v2 `label + elements` haystack match,
///    unchanged, as the fallback for old models that do not backfill.
///
/// Tier 1 is strictly additive: with no backfill anywhere it can never fire,
/// so a plan without `covers` gets byte-identical verdicts. Tier 2 then
/// widens step by step — compound parts, screens, terse labels, and last the
/// section's core once its position, count and state qualifiers are set
/// aside ([`crate::plan_coverage_core`]).
pub fn check_coverage(required: &[String], plan: &OrchestratorPlan) -> CoverageCheck {
    let haystack = plan_haystack(plan);
    let mut check = CoverageCheck::default();
    for section in required {
        if let Some(id) = covering_subtask_id(plan, |st| {
            covers_entry_matches(section, st.covers.as_deref())
        }) {
            check
                .covered_by
                .push((section.clone(), id, CoverageSource::Covers));
            continue;
        }
        if section_covered(section, &haystack) {
            // Annotate with the first subtask whose own text matches; the
            // verdict itself stays the whole-plan haystack's (all-tokens
            // matching may span subtask boundaries).
            let id =
                covering_subtask_id(plan, |st| section_covered(section, &subtask_haystack(st)))
                    .unwrap_or_else(|| "plan".to_string());
            check
                .covered_by
                .push((section.clone(), id, CoverageSource::Text));
            continue;
        }
        if let Some(parts) = compound_section_parts(section) {
            if let Some(id) = chart_row_subtask(&parts, plan) {
                check
                    .covered_by
                    .push((section.clone(), id, CoverageSource::Text));
                continue;
            }
            if parts.iter().all(|part| section_covered(part, &haystack)) {
                let id = covering_subtask_id(plan, |st| {
                    parts
                        .iter()
                        .all(|part| section_covered(part, &subtask_haystack(st)))
                })
                .unwrap_or_else(|| "plan".to_string());
                check
                    .covered_by
                    .push((section.clone(), id, CoverageSource::Text));
                continue;
            }
        }
        if let Some(id) = covering_subtask_id(plan, |st| {
            st.screen
                .as_deref()
                .is_some_and(|screen| screen_names_section(section, screen))
        }) {
            check
                .covered_by
                .push((section.clone(), id, CoverageSource::Text));
            continue;
        }
        if let Some(id) = covering_subtask_id(plan, |st| label_names_section(section, &st.label)) {
            check
                .covered_by
                .push((section.clone(), id, CoverageSource::Text));
            continue;
        }
        if let Some(id) = core_covering_subtask(section, plan) {
            check
                .covered_by
                .push((section.clone(), id, CoverageSource::Text));
            continue;
        }
        check.missing.push(section.clone());
    }
    check
}

/// Prompt-side paragraph appended to a one-shot re-plan request.
///
/// A missing item is often a detail OF a planned section rather than a
/// section of its own ("folder sidebar with counts" → the counts). Asked to
/// add a subtask per item, a planner split the sidebar's unread badges into
/// their own subtask, which the scaffold then laid out as a full-width band
/// in the main column and the three-pane mail layout collapsed. So a detail
/// goes into the subtask it belongs to; only a standalone section gets one.
pub fn coverage_feedback(missing: &[String]) -> String {
    format!(
        "The brief explicitly asks for these, which the plan does not cover: {}. For each one: if it is a detail of a section you already planned (e.g. counts inside a sidebar, a button inside a hero), add it to that subtask's elements; only if it is a standalone section, add one subtask for it. Keep the existing subtasks.",
        missing.join(", ")
    )
}

fn covering_subtask_id(
    plan: &OrchestratorPlan,
    matches: impl Fn(&Subtask) -> bool,
) -> Option<String> {
    plan.subtasks
        .iter()
        .find(|st| matches(st))
        .map(|st| st.id.clone())
}

fn covers_present(st: &Subtask) -> bool {
    st.covers
        .as_ref()
        .is_some_and(|entries| !entries.is_empty())
}

/// A `covers` entry matches the required section on normalized equality —
/// `定价` does NOT match `定价三档`.
fn covers_entry_matches(section: &str, covers: Option<&[String]>) -> bool {
    let target = normalize_for_equality(section);
    !target.is_empty()
        && covers.is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| normalize_for_equality(entry) == target)
        })
}

/// Normalize a section name / covers entry for equality: drop whitespace and
/// full/half-width punctuation, lowercase. `英雄 区` and `英雄区。` both
/// reduce to `英雄区`.
fn normalize_for_equality(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_whitespace() && !is_punctuation_for_equality(*ch))
        .flat_map(char::to_lowercase)
        .collect()
}

/// ASCII punctuation plus the common full-width/CJK punctuation forms.
fn is_punctuation_for_equality(ch: char) -> bool {
    ch.is_ascii_punctuation()
        || matches!(
            ch,
            '，' | '。'
                | '、'
                | '；'
                | '：'
                | '！'
                | '？'
                | '…'
                | '—'
                | '～'
                | '·'
                | '“'
                | '”'
                | '‘'
                | '’'
                | '（'
                | '）'
                | '《'
                | '》'
                | '「'
                | '」'
                | '『'
                | '』'
                | '【'
                | '】'
        )
}

fn plan_haystack(plan: &OrchestratorPlan) -> String {
    plan.subtasks
        .iter()
        .map(subtask_haystack)
        .collect::<Vec<_>>()
        .join("")
}

pub(crate) fn subtask_haystack(st: &Subtask) -> String {
    let mut haystack = String::new();
    haystack.push_str(&st.label);
    haystack.push(' ');
    if let Some(elements) = &st.elements {
        haystack.push_str(elements);
        haystack.push(' ');
    }
    haystack.to_lowercase()
}

fn section_covered(required: &str, haystack: &str) -> bool {
    if aliases_for(required)
        .iter()
        .any(|alias| alias_matches(haystack, alias))
    {
        return true;
    }
    // `横向滚动的课程卡片轨道` is covered by a plan naming its noun head
    // `课程卡片轨道` — the modifier is layout direction, not identity.
    if let Some(head) = de_head(required) {
        if aliases_for(&head)
            .iter()
            .any(|alias| alias_matches(haystack, alias))
        {
            return true;
        }
    }
    // A required `商家列表` is also covered when the plan names just the
    // suffix-stripped head `商家` (substring or synonym on the head).
    let head = strip_type_suffix(required);
    if head.is_empty() || head == required {
        return false;
    }
    aliases_for(&head)
        .iter()
        .any(|alias| alias_matches(haystack, alias))
}

pub(crate) fn alias_matches(haystack: &str, alias: &str) -> bool {
    let needle = alias.to_lowercase();
    if needle.is_empty() {
        return false;
    }
    contains_term(haystack, &needle) || all_tokens_present(haystack, &needle)
}

fn all_tokens_present(haystack: &str, phrase: &str) -> bool {
    let tokens = tokens(phrase);
    !tokens.is_empty() && tokens.iter().all(|token| contains_word(haystack, token))
}

/// `contains_term`, also accepting the singular of an English plural: the
/// brief asks for "counts" and the plan writes "an unread count badge".
fn contains_word(haystack: &str, token: &str) -> bool {
    contains_term(haystack, token)
        || english_singular(token).is_some_and(|singular| contains_term(haystack, &singular))
}

pub(crate) fn english_singular(token: &str) -> Option<String> {
    if !token.is_ascii() || token.len() <= 3 || token.ends_with("ss") {
        return None;
    }
    if let Some(stem) = token.strip_suffix("ies") {
        return Some(format!("{stem}y"));
    }
    if token.ends_with("ches") || token.ends_with("shes") {
        return Some(token[..token.len() - 2].to_string());
    }
    token.strip_suffix('s').map(str::to_string)
}

fn tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || is_han(ch) {
            current.push(ch);
        } else if !current.is_empty() {
            tokens.push(current.to_lowercase());
            current.clear();
        }
    }
    if !current.is_empty() {
        tokens.push(current.to_lowercase());
    }
    tokens
}

pub(crate) fn aliases_for(section: &str) -> Vec<String> {
    let mut aliases = vec![section.to_string()];
    let lower = section.to_lowercase();
    for group in SYNONYM_GROUPS.iter().chain(SECTION_SYNONYM_GROUPS) {
        let overlaps = group.iter().any(|term| {
            let term_lower = term.to_lowercase();
            lower == term_lower
                || (!GENERIC_SYNONYM_TERMS.contains(term) && contains_term(&lower, &term_lower))
        });
        if !overlaps {
            continue;
        }
        for term in *group {
            if !aliases
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(term))
            {
                aliases.push((*term).to_string());
            }
        }
    }
    aliases
}

fn strip_type_suffix(text: &str) -> String {
    let lower = text.to_lowercase();
    for suffix in TYPE_SUFFIXES {
        let suffix_lower = suffix.to_lowercase();
        if lower != suffix_lower && lower.ends_with(&suffix_lower) {
            let end = text.len().saturating_sub(suffix.len());
            if text.is_char_boundary(end) {
                return text[..end].trim().to_string();
            }
        }
    }
    text.to_string()
}

pub(crate) fn han_count(text: &str) -> usize {
    text.chars().filter(|ch| is_han(*ch)).count()
}

pub(crate) fn is_han(ch: char) -> bool {
    matches!(
        ch,
        '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}'
    )
}

#[cfg(test)]
#[path = "plan_coverage_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "plan_coverage_corpus_tests.rs"]
mod corpus_tests;

#[cfg(test)]
#[path = "plan_coverage_brief_tests.rs"]
mod brief_tests;
