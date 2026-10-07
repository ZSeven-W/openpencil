//! Promise-delivery checks for repeated items inside one planned subtask.

use crate::plan::Subtask;
use crate::types::{DocSink, SubtaskOutcome};
use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::PenNode;
use op_design_lint::node_util::is_node_visible;
use op_editor_core::{EditorState, NodeId, PenNodeExt};
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
    // Quoted text is literal copy the section displays ("title '8 条评论'"),
    // not a promise of that many sibling items.
    let text = mask_quoted_copy(&text);

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

    if let Some((rows, columns)) = grid_dimensions(&text) {
        counts.push(rows.saturating_mul(columns));
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
        collect_delivered_counts(sink.state(), root, &mut largest_family, &mut image_count);
    }
    collect_sibling_family(sink.state(), &roots, &mut largest_family);
    if largest_family >= 2 {
        largest_family
    } else {
        image_count
    }
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
    let mut delivered = delivered_item_count(sink, &outcome.inserted_root_ids);
    let text = mask_quoted_copy(&format!(
        "{}\n{}",
        subtask.label,
        subtask.elements.as_deref().unwrap_or("")
    ));
    if let Some((rows, columns)) = grid_dimensions(&text) {
        for id in &outcome.inserted_root_ids {
            if let Some(root) =
                op_editor_core::walkers::find_node(sink.state().active_children(), &NodeId::new(id))
            {
                delivered = delivered.max(delivered_grid_cells(root, rows, columns));
            }
        }
    }
    (delivered < expected).then(|| CompletenessFailure {
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

fn collect_delivered_counts(
    state: &EditorState,
    node: &PenNode,
    largest_family: &mut usize,
    image_count: &mut usize,
) {
    if !is_node_visible(node) || crate::cleanup::is_status_bar(node) {
        return;
    }
    if matches!(node, PenNode::Image(_)) {
        *image_count += 1;
    }
    let Some(children) = node.children() else {
        return;
    };
    let child_refs: Vec<&PenNode> = children.iter().collect();
    collect_sibling_family(state, &child_refs, largest_family);
    for child in children {
        if !is_node_visible(child) {
            continue;
        }
        collect_delivered_counts(state, child, largest_family, image_count);
    }
}

fn collect_sibling_family(state: &EditorState, children: &[&PenNode], largest_family: &mut usize) {
    let mut families: BTreeMap<String, usize> = BTreeMap::new();
    for child in children {
        if is_item_node(child) && is_node_visible(child) && !crate::cleanup::is_status_bar(child) {
            *families.entry(item_signature(state, child, 0)).or_default() += 1;
        }
    }
    if let Some(family) = families.values().copied().max() {
        *largest_family = (*largest_family).max(family);
    }
}

/// Reused and inline copies of the same item belong to one delivery family.
/// Bound alias lookup so malformed component cycles cannot recurse forever.
fn item_signature(state: &EditorState, node: &PenNode, depth: usize) -> String {
    if let PenNode::Ref(instance) = node {
        if depth < 8 {
            let target = NodeId::new(&instance.target);
            let master =
                op_editor_core::walkers::find_node(&state.doc.children, &target).or_else(|| {
                    state
                        .doc
                        .pages
                        .as_ref()
                        .into_iter()
                        .flatten()
                        .find_map(|p| op_editor_core::walkers::find_node(&p.children, &target))
                });
            if let Some(master) = master {
                return item_signature(state, master, depth + 1);
            }
        }
    }
    crate::cleanup::structural_signature(node)
}

fn is_item_node(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_) | PenNode::Rectangle(_) | PenNode::Image(_) | PenNode::Ref(_)
    )
}

fn grid_dimensions(text: &str) -> Option<(usize, usize)> {
    if !text.to_ascii_lowercase().contains("grid") {
        return None;
    }
    static GRID: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(\d[\d,]*)\s*[x×]\s*(\d[\d,]*)").expect("valid grid-count pattern")
    });
    GRID.captures_iter(text)
        .filter_map(|c| {
            Some((
                parse_count(c.get(1)?.as_str())?,
                parse_count(c.get(2)?.as_str())?,
            ))
        })
        .filter(|&(r, c)| (1..=50).contains(&r) && (1..=50).contains(&c))
        .max_by_key(|&(r, c)| r * c)
}

/// A row-major grid's cells may be nested under row wrappers, so its largest
/// sibling family is only the column count. Sum cells only for the explicitly
/// promised row shape; ordinary list metadata must not multiply the item count.
fn delivered_grid_cells(node: &PenNode, rows: usize, columns: usize) -> usize {
    if !is_node_visible(node) || crate::cleanup::is_status_bar(node) {
        return 0;
    }
    let children = node.children().map(Vec::as_slice).unwrap_or(&[]);
    let groups: Vec<_> = children
        .iter()
        .filter(|n| is_node_visible(n) && is_item_node(n))
        .collect();
    let mut total = 0;
    if groups.len() == rows
        && groups.iter().all(
            |n| matches!(n,PenNode::Frame(f) if f.container.layout==Some(LayoutMode::Horizontal)),
        )
    {
        let counts: Vec<_> = groups
            .iter()
            .map(|n| {
                n.children()
                    .map(Vec::as_slice)
                    .unwrap_or(&[])
                    .iter()
                    .filter(|n| is_node_visible(n) && is_item_node(n))
                    .count()
            })
            .collect();
        if counts.iter().all(|&n| n <= columns) {
            total = counts.into_iter().sum();
        }
    }
    children.iter().fold(total, |total, n| {
        total.max(delivered_grid_cells(n, rows, columns))
    })
}

/// Longest quoted run (in chars) treated as display copy; anything longer
/// is more likely a stray apostrophe pairing across real content.
const QUOTED_COPY_MAX_CHARS: usize = 40;

/// Blank out the inside of quoted copy, keeping byte offsets stable so the
/// span bookkeeping below still lines up with `text`.
fn mask_quoted_copy(text: &str) -> String {
    const PAIRS: &[(char, char)] = &[
        ('\'', '\''),
        ('"', '"'),
        ('“', '”'),
        ('‘', '’'),
        ('「', '」'),
        ('『', '』'),
    ];
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut masked = text.to_string().into_bytes();
    let mut i = 0;
    while i < chars.len() {
        let (_, open) = chars[i];
        let Some(&(_, close)) = PAIRS.iter().find(|(o, _)| *o == open) else {
            i += 1;
            continue;
        };
        // An ASCII apostrophe only opens a quote at a word start ("title 'x'"),
        // never inside a word ("user's").
        if open == '\'' && i > 0 && chars[i - 1].1.is_alphanumeric() {
            i += 1;
            continue;
        }
        let end = chars[i + 1..]
            .iter()
            .take(QUOTED_COPY_MAX_CHARS + 1)
            .position(|&(_, ch)| ch == close || ch == '\n')
            .map(|offset| i + 1 + offset)
            .filter(|&j| chars[j].1 == close);
        let Some(end) = end else {
            i += 1;
            continue;
        };
        if close == '\''
            && chars
                .get(end + 1)
                .is_some_and(|&(_, ch)| ch.is_alphanumeric())
        {
            i += 1;
            continue;
        }
        let from = chars[i + 1].0;
        let to = chars[end].0;
        masked[from..to].fill(b' ');
        i = end + 1;
    }
    String::from_utf8(masked).unwrap_or_else(|_| text.to_string())
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
    before.ends_with(['¥', '$', '€', '£', ':', '第'])
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
