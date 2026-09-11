//! Post-subtask gate: generated copy must match the brief's language.

use crate::plan::{RetryFeedback, Subtask};
use crate::subtask_completeness::CompletenessFailure;
use crate::types::{DesignRequest, DocSink, SubtaskOutcome};
use jian_ops_schema::node::{PenNode, TextContent};
use op_editor_core::{NodeId, PenNodeExt};
use regex::Regex;
use std::sync::LazyLock;

/// Script class used by the output-language gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Cjk,
    Latin,
}

/// Counts from a copy-language mismatch that crossed the retry threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MismatchReport {
    pub checked: usize,
    pub mismatched: usize,
    pub samples: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageFailure {
    pub checked: usize,
    pub mismatched: usize,
    pub feedback: String,
}

/// Latin tokens allowed in CJK-brief UI copy: short acronyms, units, and
/// product/brand names. Whole-node or whitespace-token match, case-insensitive.
const LATIN_IN_CJK_EXCEPTIONS: &[&str] = &[
    "AI", "VIP", "4K", "2K", "8K", "HD", "HDR", "UI", "UX", "iOS", "iPad", "iPhone", "macOS",
    "tvOS", "watchOS", "Android", "YouTube", "TikTok",
];

const CJK_RATIO: f64 = 0.30;
const LATIN_ASCII_RATIO: f64 = 0.70;
const LATIN_CJK_MAX: f64 = 0.05;

static SKIP_TIME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\d{1,2}:\d{2}(?::\d{2})?(?:\s*[ap]m)?$").expect("valid time skip")
});
static SKIP_PRICE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[¥$€£]\s*\d[\d,.]*$|^\d[\d,.]*\s*[元块]$").expect("valid price skip")
});
static SKIP_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(https?://|www\.)|\.[a-z]{2,4}(/|$)|://").expect("valid url skip")
});
static SKIP_HANDLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^@[\w.]+$").expect("valid handle skip"));
static SKIP_NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^[+-]?\d{1,3}(?:,\d{3})*(?:\.\d+)?\s*[kmb%]?$|^\d[\d,.]*$")
        .expect("valid number skip")
});
static SKIP_UNIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\d[\d,.]*\s*(?:px|pt|dp|sp|ms|s|min|hr|h|kg|km|m|cm|mm|gb|mb|kb)$")
        .expect("valid unit skip")
});
static ASKS_ENGLISH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)英文(?:界面|文案|版|UI|copy)?|(?:in|use|using|into)\s+English|English\s+(?:UI|copy|interface|version|labels?)|en-US|en_US",
    )
    .expect("valid English-request pattern")
});
static ASKS_CHINESE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)中文(?:界面|文案|版|UI|copy)?|(?:in|use|using|into)\s+Chinese|Chinese\s+(?:UI|copy|interface|version|labels?)|zh-CN|zh_CN|zh-Hans",
    )
    .expect("valid Chinese-request pattern")
});
static BILINGUAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)双语|bilingual").expect("valid bilingual pattern"));

/// Classify the brief as CJK, Latin, or mixed (`None` → gate off).
pub fn brief_language(brief: &str) -> Option<Lang> {
    classify(brief)
}

/// Compare eligible text-node copy against `expected`. Reports when at least
/// four nodes were checked and more than half are confidently the other class.
pub fn copy_language_mismatch(nodes: &[&PenNode], expected: Lang) -> Option<MismatchReport> {
    let mut texts = Vec::new();
    for node in nodes {
        collect_text_contents(node, &mut texts);
    }
    let mut checked = 0usize;
    let mut mismatched = 0usize;
    let mut samples = Vec::new();
    for text in texts {
        if is_skipped_copy(&text) {
            continue;
        }
        checked += 1;
        if expected == Lang::Cjk && is_cjk_brief_latin_exception(&text) {
            continue;
        }
        let Some(lang) = classify(&text) else {
            continue;
        };
        if lang != expected {
            mismatched += 1;
            if samples.len() < 4 {
                samples.push(text);
            }
        }
    }
    (checked >= 4 && mismatched * 2 > checked).then_some(MismatchReport {
        checked,
        mismatched,
        samples,
    })
}

pub(crate) fn language_feedback(expected: Lang, report: &MismatchReport) -> String {
    let samples = report
        .samples
        .iter()
        .take(2)
        .map(|sample| format!("\"{sample}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let example = if samples.is_empty() {
        String::new()
    } else {
        format!(" (e.g. {samples})")
    };
    match expected {
        Lang::Cjk => format!(
            "The brief is written in Chinese, but {} of {} text nodes are in English{example}. Rewrite all user-facing copy in Chinese; keep brand names and acronyms as they are.",
            report.mismatched, report.checked
        ),
        Lang::Latin => format!(
            "The brief is written in English, but {} of {} text nodes are in Chinese{example}. Rewrite all user-facing copy in English; keep brand names and acronyms as they are.",
            report.mismatched, report.checked
        ),
    }
}

pub(crate) fn expected_output_language(request: &DesignRequest) -> Option<Lang> {
    let lang = brief_language(&request.prompt)?;
    if brief_asks_for_other_language(&request.prompt, lang) {
        return None;
    }
    Some(lang)
}

pub(crate) fn mismatch_attempt(
    sink: &dyn DocSink,
    request: &DesignRequest,
    outcome: &SubtaskOutcome,
) -> Option<LanguageFailure> {
    if outcome.node_count == 0 {
        return None;
    }
    let expected = expected_output_language(request)?;
    let mut roots = Vec::new();
    for root_id in &outcome.inserted_root_ids {
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.clone()),
        ) else {
            continue;
        };
        roots.push(root);
    }
    let report = copy_language_mismatch(&roots, expected)?;
    Some(LanguageFailure {
        checked: report.checked,
        mismatched: report.mismatched,
        feedback: language_feedback(expected, &report),
    })
}

pub(crate) fn inspect_insert_gates(
    sink: &dyn DocSink,
    request: &DesignRequest,
    subtask: &Subtask,
    outcome: &SubtaskOutcome,
) -> (Option<CompletenessFailure>, Option<LanguageFailure>) {
    (
        crate::subtask_completeness::incomplete_attempt(sink, subtask, outcome),
        mismatch_attempt(sink, request, outcome),
    )
}

pub(crate) fn retain_mismatch_outcome(outcome: &mut SubtaskOutcome, failure: &LanguageFailure) {
    outcome.error = Some(failure.feedback.clone());
    outcome.subtask = None;
}

pub(crate) fn is_language_mismatch_outcome(outcome: &SubtaskOutcome) -> bool {
    outcome
        .error
        .as_deref()
        .is_some_and(|error| error.starts_with("The brief is written in "))
        && outcome.node_count > 0
}

pub(crate) fn retry_feedback_for_gates(
    completeness: Option<&CompletenessFailure>,
    language: Option<&LanguageFailure>,
    self_check: Option<String>,
) -> Option<RetryFeedback> {
    if let Some(failure) = completeness {
        Some(RetryFeedback::Completeness(failure.feedback.clone()))
    } else if let Some(failure) = language {
        Some(RetryFeedback::Language(failure.feedback.clone()))
    } else {
        self_check.map(RetryFeedback::SelfCheck)
    }
}

fn classify(text: &str) -> Option<Lang> {
    let (cjk, ascii, other) = letter_counts(text);
    let letters = cjk + ascii + other;
    if letters == 0 {
        return None;
    }
    let cjk_ratio = cjk as f64 / letters as f64;
    let ascii_ratio = ascii as f64 / letters as f64;
    if cjk_ratio >= CJK_RATIO {
        return Some(Lang::Cjk);
    }
    if ascii_ratio >= LATIN_ASCII_RATIO && cjk_ratio < LATIN_CJK_MAX {
        return Some(Lang::Latin);
    }
    // Chinese briefs routinely embed English product words. CJK characters
    // are denser than Latin letters, so CJK still wins when CJK chars are
    // ≥ 30% of (CJK chars + Latin words).
    let latin_words = latin_word_count(text);
    let units = cjk + latin_words;
    if cjk > 0 && units > 0 && (cjk as f64 / units as f64) >= CJK_RATIO {
        return Some(Lang::Cjk);
    }
    None
}

fn letter_counts(text: &str) -> (usize, usize, usize) {
    let mut cjk = 0usize;
    let mut ascii = 0usize;
    let mut other = 0usize;
    for ch in text.chars() {
        if is_cjk(ch) {
            cjk += 1;
        } else if ch.is_ascii_alphabetic() {
            ascii += 1;
        } else if ch.is_alphabetic() {
            other += 1;
        }
    }
    (cjk, ascii, other)
}

fn latin_word_count(text: &str) -> usize {
    text.split(|ch: char| !ch.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .count()
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{3040}'..='\u{30FF}'
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{20000}'..='\u{2FA1F}'
    )
}

fn collect_text_contents(node: &PenNode, out: &mut Vec<String>) {
    if crate::cleanup::is_status_bar(node) {
        return;
    }
    if let PenNode::Text(text) = node {
        let content = flatten_text(&text.content);
        if !content.trim().is_empty() {
            out.push(content);
        }
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_text_contents(child, out);
        }
    }
}

fn flatten_text(content: &TextContent) -> String {
    match content {
        TextContent::Plain(text) => text.clone(),
        TextContent::Styled(runs) => runs.iter().map(|run| run.text.as_str()).collect(),
    }
}

fn is_skipped_copy(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.chars().count() <= 2 {
        return true;
    }
    SKIP_NUMBER.is_match(trimmed)
        || SKIP_TIME.is_match(trimmed)
        || SKIP_PRICE.is_match(trimmed)
        || SKIP_URL.is_match(trimmed)
        || SKIP_HANDLE.is_match(trimmed)
        || SKIP_UNIT.is_match(trimmed)
}

fn is_cjk_brief_latin_exception(text: &str) -> bool {
    let trimmed = text.trim();
    if LATIN_IN_CJK_EXCEPTIONS
        .iter()
        .any(|token| token.eq_ignore_ascii_case(trimmed))
    {
        return true;
    }
    if is_short_acronym(trimmed) {
        return true;
    }
    is_product_name(trimmed)
}

fn is_short_acronym(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || chars.len() > 5 {
        return false;
    }
    let caps_or_digits = chars
        .iter()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit());
    let has_letter = chars.iter().any(|ch| ch.is_ascii_uppercase());
    let has_digit = chars.iter().any(|ch| ch.is_ascii_digit());
    // HOME / INBOX are 4–5 letter English UI labels, not acronyms like AI/VIP/4K.
    caps_or_digits && has_letter && (chars.len() <= 3 || has_digit)
}

fn is_product_name(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() || words.len() > 2 {
        return false;
    }
    words.iter().all(|word| {
        LATIN_IN_CJK_EXCEPTIONS
            .iter()
            .any(|token| token.eq_ignore_ascii_case(word))
            || has_internal_capital(word)
    })
}

fn has_internal_capital(word: &str) -> bool {
    let mut chars = word.chars();
    let Some(_) = chars.next() else {
        return false;
    };
    let rest: Vec<char> = chars.collect();
    rest.iter().any(|ch| ch.is_ascii_uppercase()) && rest.iter().any(|ch| !ch.is_ascii_uppercase())
}

fn brief_asks_for_other_language(brief: &str, expected: Lang) -> bool {
    if BILINGUAL.is_match(brief) {
        return true;
    }
    match expected {
        Lang::Cjk => ASKS_ENGLISH.is_match(brief),
        Lang::Latin => ASKS_CHINESE.is_match(brief),
    }
}

#[cfg(test)]
#[path = "output_language_tests.rs"]
mod tests;
