//! Promise-delivery checks for repeated items inside one planned subtask.

use crate::plan::Subtask;
use crate::types::{DocSink, SubtaskOutcome};
use jian_ops_schema::node::PenNode;
use op_design_lint::node_util::is_node_visible;
use op_editor_core::{NodeId, PenNodeExt};
use regex::Regex;
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::LazyLock;

const MEASURE_WORDS: &str = "个条张项页行位款篇栏卡";
const ENGLISH_ITEM_WORDS: &str =
    "cards?|items?|rows?|entries|tiles?|posts?|products?|merchants?|exercises?";

/// The exact retry message used when a promised repeated-item section is short.
pub(crate) fn completeness_feedback(expected: usize, delivered: usize) -> String {
    format!(
        "The plan asks for {expected} items in this section; only {delivered} were delivered — emit all {expected} as sibling items."
    )
}

/// Parse the largest repeated-item count promised by a subtask label/elements pair.
pub fn expected_item_count(subtask: &Subtask) -> Option<usize> {
    let mut text = subtask.label.clone();
    if let Some(elements) = &subtask.elements {
        text.push('\n');
        text.push_str(elements);
    }

    let mut counts = Vec::new();
    let mut ranged_spans: Vec<Range<usize>> = Vec::new();
    static CJK_RANGE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"(?P<lower>[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+)\s*(?:-|–|—|至|到)\s*(?:[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+)\s*(?P<unit>[{MEASURE_WORDS}])"
        ))
        .expect("valid CJK item-count range pattern")
    });
    let cjk_range = &*CJK_RANGE;
    for captures in cjk_range.captures_iter(&text) {
        if let Some(count) = captures.name("lower").and_then(|m| parse_count(m.as_str())) {
            if !ignored_number_context(&text, captures.name("lower").unwrap().start())
                && !is_ignored_count(count)
            {
                counts.push(count);
            }
        }
        if let Some(full) = captures.get(0) {
            ranged_spans.push(full.range());
        }
    }

    static CJK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"(?P<count>[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+)\s*(?P<unit>[{MEASURE_WORDS}])"
        ))
        .expect("valid CJK item-count pattern")
    });
    let cjk = &*CJK;
    for captures in cjk.captures_iter(&text) {
        let Some(full) = captures.get(0) else {
            continue;
        };
        if ranged_spans
            .iter()
            .any(|range| range.start <= full.start() && full.end() <= range.end)
        {
            continue;
        }
        let Some(count) = captures.name("count") else {
            continue;
        };
        if ignored_number_context(&text, count.start()) {
            continue;
        }
        if let Some(value) = parse_count(count.as_str()) {
            if !is_ignored_count(value) {
                counts.push(value);
            }
        }
    }

    static ENGLISH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"(?ix)(?P<count>\d[\d,]*(?:\s*(?:-|–|—|to)\s*\d[\d,]*)?)\s+(?:{ENGLISH_ITEM_WORDS})\b"
        ))
        .expect("valid English item-count pattern")
    });
    let english = &*ENGLISH;
    for captures in english.captures_iter(&text) {
        let Some(count) = captures.name("count") else {
            continue;
        };
        if ignored_number_context(&text, count.start()) {
            continue;
        }
        if let Some(value) = parse_range_lower_bound(count.as_str()) {
            if !is_ignored_count(value) {
                counts.push(value);
            }
        }
    }

    static LIST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\blist\s+of\s+(\d[\d,]*(?:\s*(?:-|–|—|to)\s*\d[\d,]*)?)")
            .expect("valid list-count pattern")
    });
    let list = &*LIST;
    for captures in list.captures_iter(&text) {
        if let Some(count) = captures
            .get(1)
            .filter(|m| !ignored_number_context(&text, m.start()))
            .and_then(|m| parse_range_lower_bound(m.as_str()))
        {
            if !is_ignored_count(count) {
                counts.push(count);
            }
        }
    }

    if text.to_ascii_lowercase().contains("grid") {
        static GRID: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?i)(\d[\d,]*)\s*[x×]\s*(\d[\d,]*)").expect("valid grid-count pattern")
        });
        let grid = &*GRID;
        for captures in grid.captures_iter(&text) {
            let rows = captures.get(1).and_then(|m| parse_count(m.as_str()));
            let columns = captures.get(2).and_then(|m| parse_count(m.as_str()));
            if let (Some(rows), Some(columns)) = (rows, columns) {
                if rows > 50 || columns > 50 {
                    continue;
                }
                counts.push(rows.saturating_mul(columns));
            }
        }
    }

    counts.into_iter().filter(|count| *count >= 2).max()
}

/// Count the largest repeated sibling family inside the inserted subtree(s).
pub fn delivered_item_count(sink: &dyn DocSink, inserted_root_ids: &[String]) -> usize {
    let mut largest_family = 0;
    let mut image_count = 0;
    let mut roots = Vec::new();
    for root_id in inserted_root_ids {
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.clone()),
        ) else {
            continue;
        };
        if !crate::cleanup::is_status_bar(root) {
            roots.push(root);
        }
        collect_delivered_counts(root, &mut largest_family, &mut image_count);
    }
    collect_sibling_family(&roots, &mut largest_family);
    if largest_family >= 2 {
        largest_family
    } else {
        image_count
    }
}

/// Return a completeness failure and remove its roots, or leave a complete
/// attempt untouched. The caller decides whether the last incomplete rung is kept.
pub(crate) fn reject_incomplete_attempt(
    sink: &mut dyn DocSink,
    subtask: &Subtask,
    outcome: &SubtaskOutcome,
) -> Option<CompletenessFailure> {
    let failure = incomplete_attempt(sink, subtask, outcome)?;
    rollback_inserted_roots(sink, &outcome.inserted_root_ids);
    Some(failure)
}

pub(crate) fn incomplete_attempt(
    sink: &dyn DocSink,
    subtask: &Subtask,
    outcome: &SubtaskOutcome,
) -> Option<CompletenessFailure> {
    let expected = expected_item_count(subtask)?;
    if outcome.node_count == 0 || expected < 2 {
        return None;
    }
    let delivered = delivered_item_count(sink, &outcome.inserted_root_ids);
    (delivered < expected.min(2)).then(|| CompletenessFailure {
        expected,
        delivered,
        feedback: completeness_feedback(expected, delivered),
    })
}

/// Mark the final incomplete non-empty outcome without deleting its last result.
pub(crate) fn retain_incomplete_outcome(
    outcome: &mut SubtaskOutcome,
    failure: &CompletenessFailure,
) {
    outcome.error = Some(failure.feedback.clone());
    outcome.subtask = None;
}

pub(crate) fn is_incomplete_outcome(outcome: &SubtaskOutcome) -> bool {
    outcome
        .error
        .as_deref()
        .is_some_and(|error| error.starts_with("The plan asks for "))
        && outcome.node_count > 0
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletenessFailure {
    pub expected: usize,
    pub delivered: usize,
    pub feedback: String,
}

pub(crate) fn rollback_inserted_roots(sink: &mut dyn DocSink, root_ids: &[String]) {
    sink.rollback_inserted_roots(root_ids);
}

fn collect_delivered_counts(node: &PenNode, largest_family: &mut usize, image_count: &mut usize) {
    if crate::cleanup::is_status_bar(node) {
        return;
    }
    if matches!(node, PenNode::Image(_)) {
        *image_count += 1;
    }
    let Some(children) = node.children() else {
        return;
    };
    let child_refs: Vec<&PenNode> = children.iter().collect();
    collect_sibling_family(&child_refs, largest_family);
    for child in children {
        if !is_node_visible(child) {
            continue;
        }
        collect_delivered_counts(child, largest_family, image_count);
    }
}

fn collect_sibling_family(children: &[&PenNode], largest_family: &mut usize) {
    let mut families: BTreeMap<String, usize> = BTreeMap::new();
    for child in children {
        if is_item_node(child) && is_node_visible(child) && !crate::cleanup::is_status_bar(child) {
            *families
                .entry(crate::cleanup::structural_signature(child))
                .or_default() += 1;
        }
    }
    if let Some(family) = families.values().copied().max() {
        *largest_family = (*largest_family).max(family);
    }
}

fn is_item_node(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_) | PenNode::Rectangle(_) | PenNode::Image(_)
    )
}

fn parse_count(raw: &str) -> Option<usize> {
    let normalized = raw.replace(',', "");
    if normalized.chars().all(|ch| ch.is_ascii_digit()) {
        return normalized.parse().ok();
    }
    parse_cjk_number(&normalized)
}

fn ignored_number_context(text: &str, start: usize) -> bool {
    let before = text[..start].trim_end();
    before.ends_with(['¥', '$', '€', '£', ':'])
}

fn is_ignored_count(value: usize) -> bool {
    (1900..=2100).contains(&value)
}

fn parse_range_lower_bound(raw: &str) -> Option<usize> {
    let lower = raw
        .split_once('-')
        .or_else(|| raw.split_once('–'))
        .or_else(|| raw.split_once('—'))
        .or_else(|| raw.split_once("to"))
        .map_or(raw, |(lower, _)| lower);
    parse_count(lower.trim())
}

fn parse_cjk_number(raw: &str) -> Option<usize> {
    let mut total = 0usize;
    let mut section = 0usize;
    let mut number = 0usize;
    for ch in raw.chars() {
        let digit = match ch {
            '零' | '〇' => Some(0),
            '一' => Some(1),
            '二' | '两' => Some(2),
            '三' => Some(3),
            '四' => Some(4),
            '五' => Some(5),
            '六' => Some(6),
            '七' => Some(7),
            '八' => Some(8),
            '九' => Some(9),
            _ => None,
        };
        if let Some(digit) = digit {
            number = number.saturating_mul(10).saturating_add(digit);
            continue;
        }
        let unit = match ch {
            '十' => 10,
            '百' => 100,
            '千' => 1_000,
            '万' => 10_000,
            '亿' => 100_000_000,
            _ => return None,
        };
        if unit >= 10_000 {
            total = total.saturating_add((section.saturating_add(number)).saturating_mul(unit));
            section = 0;
        } else {
            section = section.saturating_add(number.max(1).saturating_mul(unit));
        }
        number = 0;
    }
    Some(total.saturating_add(section).saturating_add(number))
}

#[cfg(test)]
#[path = "subtask_completeness_tests.rs"]
mod tests;
