//! Plan-coverage gate: the brief enumerated a section the plan never named.
//!
//! Distinct from [`crate::subtask_completeness`]: that gate checks a subtask
//! that promised N repeated items and delivered 0–1. This one fires when the
//! section was never planned at all.

use crate::plan::OrchestratorPlan;
use regex::Regex;
use std::sync::LazyLock;

/// Bidirectional synonym groups. A required section that overlaps a group is
/// covered when any member of that group appears in a subtask label/elements.
const SYNONYM_GROUPS: &[&[&str]] = &[
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

const TYPE_SUFFIXES: &[&str] = &[
    "区域", "模块", "部分", "列表", "网格", "section", "area", "区",
];

static CJK_INTRO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(需要有|包含|包括|含有|分为|有)([^。；\n]+)")
        .expect("valid CJK intro-list pattern")
});
static CJK_PARTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:[0-9]+|[一二三四五六七八九十]+)\s*个\s*(?:部分|区域|模块)[：:]\s*([^。；\n]+)")
        .expect("valid CJK numbered-parts pattern")
});
static CJK_COLON: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[：:]\s*([^。；\n]*、[^。；\n]+)").expect("valid CJK colon-enumeration pattern")
});
static CJK_SPLIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"、|，|以及|及|与|和").expect("valid CJK item split"));
static EN_WITH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:with|including|containing)\s+(.+?)(?:[.;\n]|$)")
        .expect("valid English with/including list")
});
static EN_SECTIONS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bsections?:\s*(.+?)(?:[.;\n]|$)").expect("valid English sections: list")
});
static BULLET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:[-*•]|[0-9]+[.)]|[①②③④⑤⑥⑦⑧⑨⑩])\s+(.+?)\s*$")
        .expect("valid bullet/numbered line")
});
static PARENS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"（[^）]*）|\([^)]*\)").expect("valid parenthetical strip"));
/// `共<count>…` tail: `四组设置分组共十二行` → `四组设置分组`.
static TRAILING_SHARED_TAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"共(?:[0-9]+|[零〇一二两三四五六七八九十]+)[^\s共（）()、，。；]*$")
        .expect("valid 共-count tail")
});
/// Trailing quantity phrase: `<count>[measure][标签|带开关|带涨跌|按钮|横向卡|纵向卡]`.
/// `底部导航四个标签` → `底部导航`, `播放控制五按钮` → `播放控制`, `车型选择三档横向卡` → `车型选择`.
static TRAILING_COUNT_PHRASE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:[0-9]+|[零〇一二两三四五六七八九十]+)\s*[个条张项页行位款篇栏卡组档天]?\s*(?:标签|带开关|带涨跌|按钮|横向卡|纵向卡)?$",
    )
    .expect("valid trailing count phrase")
});
/// Descriptive `带…` tail without a count: `总资产卡带涨跌` → `总资产卡`, `账户区带头像` → `账户区`.
/// Only stripped when the remainder keeps ≥3 CJK chars (guard in `strip_trailing_junk`).
static TRAILING_DESC_TAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"带[^\s（）()、，。；]{1,2}$").expect("valid trailing 带 tail"));
/// Leading `<count><measure>` prefix: `四组设置分组` → `设置分组` (remainder ≥3 CJK chars).
static LEADING_COUNT_PHRASE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[0-9]+|[零〇一二两三四五六七八九十]+)\s*[个条张项页行位款篇栏卡组档天]")
        .expect("valid leading count phrase")
});
static CJK_YOU_PARTS_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[0-9]+|[一二三四五六七八九十]+)\s*个\s*(?:部分|区域|模块)")
        .expect("valid 有-parts skip")
});

/// Extract section nouns the brief EXPLICITLY enumerates. High-precision only.
pub fn required_sections(brief: &str) -> Vec<String> {
    // (raw item, declared): declared items come from an explicit `N个部分：` list,
    // so even short generic nouns (头部、内容) are trustworthy sections.
    let mut raw_items: Vec<(String, bool)> = Vec::new();

    for captures in CJK_PARTS.captures_iter(brief) {
        if let Some(list) = captures.get(1) {
            raw_items.extend(
                split_cjk_items(list.as_str())
                    .into_iter()
                    .map(|item| (item, true)),
            );
        }
    }

    for captures in CJK_INTRO.captures_iter(brief) {
        let Some(intro) = captures.get(1) else {
            continue;
        };
        let Some(list) = captures.get(2) else {
            continue;
        };
        if intro.as_str() == "有" {
            if let Some(prev) = brief[..intro.start()].chars().last() {
                if matches!(prev, '没' | '所' | '还' | '只' | '拥' | '持') {
                    continue;
                }
            }
            let trimmed = list.as_str().trim_start();
            if CJK_YOU_PARTS_PREFIX.is_match(trimmed) {
                continue;
            }
            if !list.as_str().contains('、') {
                continue;
            }
        } else if !has_cjk_list_separator(list.as_str()) {
            continue;
        }
        raw_items.extend(
            split_cjk_items(list.as_str())
                .into_iter()
                .map(|item| (item, false)),
        );
    }

    for captures in CJK_COLON.captures_iter(brief) {
        if let Some(list) = captures.get(1) {
            raw_items.extend(
                split_cjk_items(list.as_str())
                    .into_iter()
                    .map(|item| (item, false)),
            );
        }
    }

    for captures in EN_WITH.captures_iter(brief) {
        if let Some(list) = captures.get(1) {
            let text = list.as_str();
            if has_english_list_separator(text) {
                raw_items.extend(split_english_items(text).into_iter().map(|i| (i, false)));
            }
        }
    }

    for captures in EN_SECTIONS.captures_iter(brief) {
        if let Some(list) = captures.get(1) {
            raw_items.extend(
                split_english_items(list.as_str())
                    .into_iter()
                    .map(|item| (item, false)),
            );
        }
    }

    let bullets: Vec<String> = BULLET
        .captures_iter(brief)
        .filter_map(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        .collect();
    if bullets.len() >= 2 {
        raw_items.extend(bullets.into_iter().map(|item| (item, false)));
    }

    dedupe_normalized(
        raw_items
            .into_iter()
            .filter_map(|(item, declared)| normalize_section(&item, declared)),
    )
}

/// Required sections the plan's subtask labels/elements do not cover.
pub fn missing_sections(required: &[String], plan: &OrchestratorPlan) -> Vec<String> {
    let haystack = plan_haystack(plan);
    required
        .iter()
        .filter(|section| !section_covered(section, &haystack))
        .cloned()
        .collect()
}

/// Prompt-side paragraph appended to a one-shot re-plan request.
pub fn coverage_feedback(missing: &[String]) -> String {
    format!(
        "The brief explicitly asks for these sections, which the plan does not cover: {}. Add one subtask per missing section (keep the existing ones).",
        missing.join(", ")
    )
}

fn plan_haystack(plan: &OrchestratorPlan) -> String {
    let mut haystack = String::new();
    for subtask in &plan.subtasks {
        haystack.push_str(&subtask.label);
        haystack.push(' ');
        if let Some(elements) = &subtask.elements {
            haystack.push_str(elements);
            haystack.push(' ');
        }
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

fn alias_matches(haystack: &str, alias: &str) -> bool {
    let needle = alias.to_lowercase();
    if needle.is_empty() {
        return false;
    }
    haystack.contains(&needle) || all_tokens_present(haystack, &needle)
}

fn all_tokens_present(haystack: &str, phrase: &str) -> bool {
    let tokens = tokens(phrase);
    !tokens.is_empty() && tokens.iter().all(|token| haystack.contains(token))
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

fn aliases_for(section: &str) -> Vec<String> {
    let mut aliases = vec![section.to_string()];
    let lower = section.to_lowercase();
    for group in SYNONYM_GROUPS {
        let overlaps = group.iter().any(|term| {
            let term_lower = term.to_lowercase();
            lower == term_lower || lower.contains(&term_lower)
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

/// Normalise a raw enumerated item into a required-section name.
///
/// Quantity/descriptor tails are stripped (`商家列表五个` → `商家列表`,
/// `底部导航四个标签` → `底部导航`) but the item itself is emitted WHOLE —
/// type suffixes (`列表`/`网格`/…) are never stripped here; the suffix-stripped
/// head is only used as an extra matching alias in [`section_covered`].
fn normalize_section(raw: &str, declared: bool) -> Option<String> {
    let mut text = raw
        .trim()
        .trim_matches(|ch| {
            matches!(
                ch,
                '"' | '\'' | '“' | '”' | '‘' | '’' | '。' | '.' | '；' | ';'
            )
        })
        .to_string();
    text = PARENS.replace_all(&text, "").into_owned();
    let text = strip_trailing_junk(text.trim());
    let text = strip_leading_count(text.trim());
    let text = text.trim();
    if text.is_empty()
        || is_negated_section(text)
        || is_sentence_length(text)
        || is_short_field_noun(text, declared)
    {
        return None;
    }
    Some(text.to_string())
}

/// Strip trailing quantity/descriptor tails: `共…` tail, then `<count>[measure][tail]`,
/// then a bare `带…` tail when the remainder keeps ≥3 CJK chars.
fn strip_trailing_junk(text: &str) -> String {
    let stripped = TRAILING_SHARED_TAIL.replace(text, "").into_owned();
    let stripped = TRAILING_COUNT_PHRASE
        .replace(stripped.trim(), "")
        .into_owned();
    let trimmed = stripped.trim();
    if let Some(mat) = TRAILING_DESC_TAIL.find(trimmed) {
        let head = trimmed[..mat.start()].trim();
        if han_count(head) >= 3 {
            return head.to_string();
        }
    }
    trimmed.to_string()
}

/// Strip a leading `<count><measure>` prefix (`四组设置分组` → `设置分组`)
/// only when the remainder keeps ≥3 CJK chars.
fn strip_leading_count(text: &str) -> String {
    let Some(mat) = LEADING_COUNT_PHRASE.find(text) else {
        return text.to_string();
    };
    let rest = text[mat.end()..].trim();
    if han_count(rest) >= 3 {
        rest.to_string()
    } else {
        text.to_string()
    }
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

/// A section the brief explicitly excluded (`不要底部导航`) is never required.
fn is_negated_section(text: &str) -> bool {
    text.starts_with("不要")
        || text.starts_with("不用")
        || text.starts_with("无需")
        || text.starts_with("别加")
        || text.starts_with("别放")
        || text.starts_with("没有")
}

/// Very short CJK items are usually text fields, not sections (`歌名`, `时间`,
/// `搜索`). Kept when they carry a type suffix (`区块`) or are known section
/// nouns from the synonym table (`日程`), except `搜索`, which stays too
/// generic. Declared `N个部分：` items bypass this screen.
fn is_short_field_noun(text: &str, declared: bool) -> bool {
    if declared {
        return false;
    }
    let han = han_count(text);
    if han == 0 || han > 2 {
        return false;
    }
    let lower = text.to_lowercase();
    if TYPE_SUFFIXES
        .iter()
        .any(|suffix| lower != *suffix && lower.ends_with(suffix))
    {
        return false;
    }
    if text == "搜索" {
        return true;
    }
    han == 2 && SYNONYM_GROUPS.iter().any(|group| group.contains(&text))
}

fn is_sentence_length(text: &str) -> bool {
    let han = han_count(text);
    let words = text.split_whitespace().count();
    han > 12 || (han == 0 && words > 5)
}

fn han_count(text: &str) -> usize {
    text.chars().filter(|ch| is_han(*ch)).count()
}

fn is_han(ch: char) -> bool {
    matches!(
        ch,
        '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}'
    )
}

fn has_cjk_list_separator(text: &str) -> bool {
    text.contains('、')
        || text.contains('，')
        || text.contains('和')
        || text.contains('与')
        || text.contains('及')
}

fn has_english_list_separator(text: &str) -> bool {
    text.contains(',') || text.to_ascii_lowercase().contains(" and ")
}

fn split_cjk_items(list: &str) -> Vec<String> {
    // Split on every separator, remembering the conjunction that followed each
    // piece: a `与/和` pair with a ≤2-char side is kept whole (they are the text
    // fields of one section, `歌名与歌手`), not split into bare nouns.
    let mut pieces: Vec<(&str, &str)> = Vec::new();
    let mut start = 0;
    for sep in CJK_SPLIT.find_iter(list) {
        pieces.push((list[start..sep.start()].trim(), sep.as_str()));
        start = sep.end();
    }
    pieces.push((list[start..].trim(), ""));

    let mut items: Vec<String> = Vec::new();
    let mut index = 0;
    while index < pieces.len() {
        let (first, mut sep) = pieces[index];
        index += 1;
        if first.is_empty() {
            continue;
        }
        let mut item = first.to_string();
        while matches!(sep, "与" | "和") && index < pieces.len() {
            let (next, next_sep) = pieces[index];
            if next.is_empty() || (han_count(&item) > 2 && han_count(next) > 2) {
                break;
            }
            item.push_str(sep);
            item.push_str(next);
            sep = next_sep;
            index += 1;
        }
        items.push(item);
    }
    items
}

fn split_english_items(list: &str) -> Vec<String> {
    let mut items = Vec::new();
    for comma_part in list.split(',') {
        let trimmed = comma_part
            .trim()
            .trim_start_matches("and ")
            .trim_start_matches("And ")
            .trim();
        if trimmed.is_empty() {
            continue;
        }
        for and_part in trimmed.split(" and ") {
            let item = and_part.trim();
            if !item.is_empty() {
                items.push(item.to_string());
            }
        }
    }
    items
}

fn dedupe_normalized(items: impl Iterator<Item = String>) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        if !out
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&item))
        {
            out.push(item);
        }
    }
    out
}

#[cfg(test)]
#[path = "plan_coverage_tests.rs"]
mod tests;
