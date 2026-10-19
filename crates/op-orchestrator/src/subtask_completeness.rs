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
        "The plan asks for {expected} items in this section; only {delivered} were delivered — preserve the planned grid/list layout and emit all {expected} items."
    )
}

/// Parse the largest repeated-item count promised by a subtask label/elements pair.
pub fn expected_item_count(subtask: &Subtask) -> Option<usize> {
    expected_count_with_map_markers(subtask, 0)
}

fn expected_count_with_map_markers(subtask: &Subtask, map_markers: usize) -> Option<usize> {
    let text = indicator::mask_counts(&local_item_text(subtask));

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
                && !text_line_count(
                    &text,
                    captures.name("lower").unwrap().start(),
                    captures.name("unit").unwrap(),
                )
                && !is_ignored_count(count)
                && !template_landmark_count(
                    &text,
                    captures.name("unit").unwrap().end(),
                    count,
                    map_markers,
                )
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
        if ignored_number_context(&text, count.start())
            || text_line_count(&text, count.start(), captures.name("unit").unwrap())
        {
            continue;
        }
        if let Some(value) = parse_count(count.as_str()) {
            if !is_ignored_count(value)
                && !template_landmark_count(
                    &text,
                    captures.name("unit").unwrap().end(),
                    value,
                    map_markers,
                )
            {
                counts.push(value);
            }
        }
    }

    static ENGLISH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"(?ix)(?P<count>\d[\d,]*(?:\s*(?:-|–|—|to)\s*\d[\d,]*)?)\s+(?P<unit>{ENGLISH_ITEM_WORDS})\b"
        ))
        .expect("valid English item-count pattern")
    });
    let english = &*ENGLISH;
    for captures in english.captures_iter(&text) {
        let Some(count) = captures.name("count") else {
            continue;
        };
        if ignored_number_context(&text, count.start())
            || text_line_count(&text, count.start(), captures.name("unit").unwrap())
        {
            continue;
        }
        if let Some(value) = parse_range_lower_bound(count.as_str()) {
            if !is_ignored_count(value) {
                counts.push(value);
            }
        }
    }

    static LIST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\blist\s+of\s+(\d[\d,]*(?:\s*(?:-|–|—|to)\s*\d[\d,]*)?|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\b")
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

/// Quotas and grid delivery must use the same local promises. Literal copy,
/// metadata values, and explicitly non-local references do not own items here.
fn local_item_text(subtask: &Subtask) -> String {
    let text = format!(
        "{}\n{}",
        subtask.label,
        subtask.elements.as_deref().unwrap_or("")
    );
    let text = statistics::mask_counts(&mask_metadata_copy(&mask_quoted_copy(&text)));
    scope::mask_nonlocal_references(&text, subtask)
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
    if outcome.node_count == 0 {
        return None;
    }
    if let Some(failure) = bottom_action::inspect(sink, subtask, outcome) {
        return Some(failure);
    }
    let named_delivered = match named_options::inspect(sink, subtask, outcome) {
        Ok(delivered) => delivered,
        Err(failure) => return Some(failure),
    };
    let map_markers = outcome
        .inserted_root_ids
        .iter()
        .filter_map(|id| {
            op_editor_core::walkers::find_node(sink.state().active_children(), &NodeId::new(id))
        })
        .map(template_marker_capacity)
        .max()
        .unwrap_or(0);
    let expected = expected_count_with_map_markers(subtask, map_markers)?;
    if expected < 2 {
        return None;
    }
    let mut delivered = delivered_item_count(sink, &outcome.inserted_root_ids).max(named_delivered);
    let text = local_item_text(subtask);
    if grid_dimensions(&text).is_none()
        && media_items::has_local_media_card_contract(&text, expected)
    {
        delivered = delivered.max(
            media_items::delivered_media_card_count(sink, &outcome.inserted_root_ids).unwrap_or(0),
        );
    }
    if grid_dimensions(&text).is_none() && list_rows::has_local_list_contract(&text, expected) {
        delivered = delivered.max(list_rows::delivered_list_row_count(
            sink,
            &outcome.inserted_root_ids,
        ));
    }
    if grid_dimensions(&text).is_none() && reviews::has_local_review_contract(&text, expected) {
        if let Some(proof) = reviews::delivered_reviews(sink, &outcome.inserted_root_ids) {
            tracing::debug!(cards = ?proof.card_ids, attachments = ?proof.attachment_ids,
                "proven sibling reviews with optional photo attachments");
            delivered = delivered.max(proof.card_ids.len());
        }
    }
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

fn template_landmark_count(text: &str, after_unit: usize, count: usize, capacity: usize) -> bool {
    if count > capacity {
        return false;
    }
    let after = text[after_unit..].trim_start();
    ["节点", "标记", "图钉", "定位点"].iter().any(|noun| {
        after.strip_prefix(noun).is_some_and(|rest| {
            // A marker's separate information cards still promise real items.
            !["卡", "信息", "列表"]
                .iter()
                .any(|suffix| rest.trim_start().starts_with(suffix))
        })
    })
}

fn template_marker_capacity(node: &PenNode) -> usize {
    if !is_node_visible(node) {
        return 0;
    }
    // The owned template adds these landmarks after section insertion. Only
    // explicit template slots qualify; arbitrary map images/names do not.
    if let PenNode::Image(image) = node {
        let query = image.image_search_query.as_deref().unwrap_or("").trim();
        return match query {
            "map-template:delivery" | "map-template:ride" => 3,
            "map-template:running" => 2,
            "map-template:location" => 1,
            _ => 0,
        };
    }
    node.children()
        .into_iter()
        .flatten()
        .map(template_marker_capacity)
        .max()
        .unwrap_or(0)
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
    if text.contains("九宫格") {
        return Some((3, 3));
    }
    if !text.to_ascii_lowercase().contains("grid") && !text.contains("网格") {
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

/// Grid cells may be grouped by rows or columns. Sum cells only for the
/// explicitly promised shape; list metadata must not multiply the item count.
fn delivered_grid_cells(node: &PenNode, rows: usize, columns: usize) -> usize {
    if !is_node_visible(node) || crate::cleanup::is_status_bar(node) {
        return 0;
    }
    let children = node.children().map(Vec::as_slice).unwrap_or(&[]);
    let groups: Vec<_> = children
        .iter()
        .filter(|n| is_node_visible(n) && is_item_node(n))
        .collect();
    let (group_count, cell_count, group_layout) = if matches!(node, PenNode::Frame(f) if f.container.layout == Some(LayoutMode::Horizontal))
    {
        (columns, rows, LayoutMode::Vertical)
    } else {
        (rows, columns, LayoutMode::Horizontal)
    };
    let mut total = 0;
    if groups.len() == group_count
        && groups.iter().all(
            |n| matches!(n,PenNode::Frame(f) if f.container.layout.as_ref()==Some(&group_layout)),
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
        if counts.iter().all(|&n| n <= cell_count) {
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

/// A short parenthesized metadata value is display copy, not a local item
/// promise. Keep ordinary parentheses and explicit lists inside metadata live.
fn mask_metadata_copy(text: &str) -> String {
    static VALUES: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\((?P<ascii>[^()（）\n]{1,160})\)|（(?P<cjk>[^()（）\n]{1,160})）")
            .expect("valid metadata parentheses")
    });
    static META_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)(?:\bmeta(?:data)?(?:\s+(?:row|text|label|summary))?|元信息(?:行|文字)?|元数据(?:行|文字)?|信息行)\s*[:：]?\s*$")
            .expect("valid metadata prefix")
    });
    static ITEM_INSTRUCTION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)\b(?:list|grid|rows?|cards?|tiles?|entries|render|generate|include|show|display|repeat|each|with|per)\b|列表|网格|卡片|逐项|每项|每个|包含|列出|生成|显示")
            .expect("valid repeated-item instruction")
    });
    let mut masked = text.as_bytes().to_vec();
    for captures in VALUES.captures_iter(text) {
        let whole = captures.get(0).unwrap();
        let body = captures
            .name("ascii")
            .or_else(|| captures.name("cjk"))
            .unwrap();
        if META_PREFIX.is_match(&text[..whole.start()]) && !ITEM_INSTRUCTION.is_match(body.as_str())
        {
            masked[body.range()].fill(b' ');
        }
    }
    String::from_utf8(masked).unwrap_or_else(|_| text.to_string())
}

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
    match normalized.to_ascii_lowercase().as_str() {
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        _ => parse_cjk_number(&normalized),
    }
}

fn ignored_number_context(text: &str, start: usize) -> bool {
    let before = text[..start].trim_end();
    if before.ends_with(['¥', '$', '€', '£', ':', '第']) {
        return true;
    }
    // A unitless design value can touch an item noun: "圆角16 卡片" is a
    // radius, not sixteen cards. Keep explicit classifiers authoritative,
    // e.g. "圆角 6 张卡片" still promises six cards.
    static DESIGN_VALUE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)(?:圆角|半径|字号|字重|行高|间距|内边距|外边距|宽度|高度|\b(?:cornerRadius|border-radius|radius|gap|padding|margin|fontSize|font-size|font-weight|line-height|width|height))\s*(?:[=：]|为)?\s*$",
        )
        .expect("valid design-value prefix")
    });
    static EXPLICIT_CLASSIFIER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(?:[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+)(?:\s*(?:-|–|—|至|到)\s*(?:[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+))?\s*[个条张项页行位款篇栏]",
        )
        .expect("valid explicit item classifier")
    });
    DESIGN_VALUE.is_match(before) && !EXPLICIT_CLASSIFIER.is_match(&text[start..])
}

/// A title/paragraph's wrapped lines are one text block, not repeated items.
/// Match only the local copy phrase; a later list in the same section still
/// promises rows, and non-line classifiers (reviews/products) remain intact.
fn text_line_count(text: &str, start: usize, unit: regex::Match<'_>) -> bool {
    if !matches!(
        unit.as_str().to_ascii_lowercase().as_str(),
        "行" | "row" | "rows"
    ) {
        return false;
    }
    static BEFORE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?ix)(?:标题|正文|段落|文案|描述|说明文字|简介|摘要|\b(?:title|subtitle|headline|heading|paragraph|body(?:\s+text)?|description|caption|copy|text))
              \s*(?:(?:最多|至多|最少|不超过|限于|限制为?|显示为?|占用?|分成?|换成?|换为)|(?:spans?|wraps?(?:\s+to)?|across|in|of|to|up\s+to|at\s+most))?
              \s*[（(:：]?\s*$",
        )
        .expect("valid copy line-count prefix")
    });
    static AFTER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?ix)^\s*(?:的\s*)?(?:商品标题|副标题|标题|正文|段落|文案|描述|说明文字|简介|摘要)
              |^\s*(?:of\s+)?(?:product\s+title|title|subtitle|headline|heading|paragraph|body(?:\s+text)?|description|caption|copy|text)\b",
        )
        .expect("valid copy line-count suffix")
    });
    static LIST_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"(?ix)^\s*(?:列表|条目|项目|网格|清单|卡片?|list\b|(?:{ENGLISH_ITEM_WORDS})\b)"
        ))
        .expect("valid repeated-copy list suffix")
    });
    let after = &text[unit.end()..];
    BEFORE.is_match(&text[..start])
        || AFTER
            .find(after)
            .is_some_and(|copy| !LIST_SUFFIX.is_match(&after[copy.end()..]))
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

#[cfg(test)]
#[path = "subtask_completeness_word_tests.rs"]
mod word_tests;

#[cfg(test)]
#[path = "subtask_completeness_text_line_tests.rs"]
mod text_line_tests;

#[path = "subtask_completeness_indicator.rs"]
mod indicator;

#[cfg(test)]
#[path = "subtask_completeness_indicator_tests.rs"]
mod indicator_tests;

#[path = "subtask_completeness_scope.rs"]
mod scope;

#[cfg(test)]
#[path = "subtask_completeness_scope_tests.rs"]
mod scope_tests;

#[path = "subtask_completeness_media_items.rs"]
mod media_items;

#[cfg(test)]
#[path = "subtask_completeness_media_items_tests.rs"]
mod media_items_tests;

#[cfg(test)]
#[path = "subtask_completeness_media_items_boundary_tests.rs"]
mod media_items_boundary_tests;

#[path = "subtask_completeness_list_rows.rs"]
mod list_rows;

#[cfg(test)]
#[path = "subtask_completeness_list_rows_tests.rs"]
mod list_rows_tests;

#[cfg(test)]
#[path = "subtask_completeness_list_rows_boundary_tests.rs"]
mod list_rows_boundary_tests;

#[path = "subtask_completeness_named_options.rs"]
mod named_options;

#[cfg(test)]
#[path = "subtask_completeness_named_options_tests.rs"]
mod named_options_tests;

#[cfg(test)]
#[path = "subtask_completeness_named_options_boundary_tests.rs"]
mod named_options_boundary_tests;

#[path = "subtask_completeness_bottom_action.rs"]
mod bottom_action;

#[cfg(test)]
#[path = "subtask_completeness_bottom_action_tests.rs"]
mod bottom_action_tests;

#[cfg(test)]
#[path = "subtask_completeness_bottom_action_boundary_tests.rs"]
mod bottom_action_boundary_tests;

#[path = "subtask_completeness_statistics.rs"]
mod statistics;

#[cfg(test)]
#[path = "subtask_completeness_statistics_tests.rs"]
mod statistics_tests;

#[path = "subtask_completeness_reviews.rs"]
mod reviews;

#[cfg(test)]
#[path = "subtask_completeness_reviews_tests.rs"]
mod reviews_tests;
