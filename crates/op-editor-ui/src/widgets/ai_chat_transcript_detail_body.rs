//! The structured body every expandable chat detail paints: a step's
//! progress details, a quality run's repairs, a tool call's arguments and
//! result.
//!
//! Details arrive as plain text lines. Painted as grey rows they read as one
//! undifferentiated block, and tool calls showed their raw JSON. Each line
//! is classified instead and painted for what it is:
//!
//! ```text
//! Checked layout, overflow, hierarchy        Text: a muted row
//! (● layout 3) (● structure 7)               Counts: one chip per category
//! ● Home  n1 · hero-bleed                    Repair: node, then the pass,
//!   gap 0 → 16                                 the new value emphasised
//! (3 screens) (41 elements)                  Stats: facts as neutral chips
//! skills    (cjk-typography) (icon-buttons)  Field: key column, value as chips,
//! retry #1  geometry echo → retry              a change,
//! Args                                         or a JSON object's fields
//!   nodeId  n1
//! ```
//!
//! Every row kind has a fixed height, so a step's height is known when the
//! transcript is laid out; only horizontal fitting is measured at paint
//! time (`ai_chat_transcript_detail_paint`).

use super::ai_chat_transcript::ACTION_DETAIL_LINE_H;
use super::ai_chat_transcript_receipts::pluralized;
use super::ai_chat_transcript_steps::wrap_detail;
use super::ai_chat_transcript_text::wrap_units;
use serde_json::Value;

pub(crate) use super::ai_chat_transcript_detail_paint::paint_detail_body;

/// The repair families the quality passes report, in their display order.
pub(crate) const CATEGORIES: [&str; 5] =
    ["layout", "overflow", "hierarchy", "structure", "palette"];
pub(crate) const CHIP_H: f32 = 18.0;
pub(crate) const CHIP_ROW_H: f32 = CHIP_H + 8.0;
pub(crate) const REPAIR_ROW_H: f32 = 34.0;
pub(crate) const CHANGE_ROW_H: f32 = 16.0;
/// Width of a field's key column.
pub(crate) const KEY_W: f32 = 66.0;
/// A JSON value shows at most this many fields before summarizing the rest.
const MAX_FIELDS: usize = 8;
/// A free-text field value shows at most this many rows.
const MAX_VALUE_ROWS: usize = 4;
/// Values longer than this are shortened when a JSON field is summarized.
const SUMMARY_CHARS: usize = 44;
/// Chip-list items longer than this make the value plain text instead.
const CHIP_MAX_CHARS: usize = 32;

/// One line of a detail body.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DetailLine {
    Repair(Repair),
    /// `repairs: layout 3, structure 7`.
    Counts(Vec<(String, usize)>),
    /// Two or more short `·`-separated facts: `3 screens · 41 elements`.
    Stats(Vec<String>),
    /// `key: value`.
    Field {
        key: String,
        value: FieldValue,
        alert: bool,
    },
    /// `before → after` with no key.
    Change {
        before: String,
        after: String,
    },
    /// Anything else, pre-wrapped into rows.
    Text {
        rows: Vec<String>,
        alert: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FieldValue {
    /// A list (`a, b, c`) or a single short token.
    Chips(Vec<String>),
    Change {
        before: String,
        after: String,
    },
    /// A JSON object's top-level fields with summarized values, and how many
    /// were left out.
    Fields {
        fields: Vec<(String, String)>,
        more: usize,
    },
    Text(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Repair {
    pub category: String,
    pub pass: String,
    /// Node name (or bare id when it has none) and id, when the repair has a
    /// node.
    pub node: Option<(String, Option<String>)>,
    /// The changed field and its old value (`gap 0`), or the whole change
    /// when it has no `→`.
    pub before: String,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DetailBody {
    pub lines: Vec<DetailLine>,
    pub height: f32,
}

/// The body for a step's `details` (each entry may hold several
/// `\n`-separated lines). `alert` tints its text, for a failed step. `None`
/// when there is nothing to show.
pub(crate) fn detail_body(details: &[String], budget: u32, alert: bool) -> Option<DetailBody> {
    let lines: Vec<(&str, bool)> = details
        .iter()
        .flat_map(|detail| detail.split('\n'))
        .map(|line| (line, alert))
        .collect();
    detail_body_lines(&lines, budget)
}

/// [`detail_body`] with a tint per line (a tool call's failed result).
pub(crate) fn detail_body_lines(lines: &[(&str, bool)], budget: u32) -> Option<DetailBody> {
    let lines: Vec<DetailLine> = lines
        .iter()
        .map(|(line, alert)| (line.trim(), *alert))
        .filter(|(line, _)| !line.is_empty())
        .map(|(line, alert)| parse_line(line, budget, alert))
        .collect();
    if lines.is_empty() {
        return None;
    }
    let height = lines.iter().map(line_height).sum::<f32>() + 4.0;
    Some(DetailBody { lines, height })
}

fn parse_line(line: &str, budget: u32, alert: bool) -> DetailLine {
    if let Some(repair) = parse_repair(line) {
        return DetailLine::Repair(repair);
    }
    if let Some(counts) = parse_counts(line) {
        return DetailLine::Counts(counts);
    }
    let line = pluralized(line);
    if let Some((key, value)) = split_field(&line) {
        return DetailLine::Field {
            key: key.to_owned(),
            value: parse_value(value, budget),
            alert,
        };
    }
    if !alert {
        if let Some((before, after)) = line.split_once(" → ") {
            return DetailLine::Change {
                before: before.to_owned(),
                after: after.to_owned(),
            };
        }
        if let Some(stats) = parse_stats(&line) {
            return DetailLine::Stats(stats);
        }
    }
    DetailLine::Text {
        rows: wrap_detail(&line, budget),
        alert,
    }
}

fn parse_repair(line: &str) -> Option<Repair> {
    let parts: Vec<&str> = line.split(" · ").collect();
    let (category, pass) = (*parts.first()?, *parts.get(1)?);
    if !CATEGORIES.contains(&category) || parts.len() < 3 {
        return None;
    }
    let (node, rest) = match parts.get(2) {
        Some(label) if parts.len() > 3 && label.ends_with(']') => {
            (Some(node_label(label)), &parts[3..])
        }
        _ => (None, &parts[2..]),
    };
    let change = rest.join(" · ");
    let (before, after) = match change.split_once(" → ") {
        Some((before, after)) => (before.to_owned(), Some(after.to_owned())),
        None => (change, None),
    };
    Some(Repair {
        category: category.to_owned(),
        pass: pass.to_owned(),
        node,
        before,
        after,
    })
}

/// `Home [n1]` → (`Home`, `n1`); `[n28]` → (`n28`, none).
fn node_label(label: &str) -> (String, Option<String>) {
    match label.rsplit_once(" [") {
        Some((name, id)) => (name.to_owned(), Some(id.trim_end_matches(']').to_owned())),
        None => (
            label
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_owned(),
            None,
        ),
    }
}

fn parse_counts(line: &str) -> Option<Vec<(String, usize)>> {
    let list = line.strip_prefix("repairs: ")?;
    list.split(", ")
        .map(|entry| {
            let (category, count) = entry.rsplit_once(' ')?;
            Some((category.to_owned(), count.parse().ok()?))
        })
        .collect()
}

/// `key: value` with a short, word-like key (`skills`, `retry #1`, `Args`);
/// a URL or prose with a colon later in it is not a field.
fn split_field(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(": ")?;
    let mut chars = key.chars();
    let word_like = chars.next().is_some_and(char::is_alphabetic)
        && chars.all(|c| c.is_alphanumeric() || matches!(c, ' ' | '#' | '-' | '_'));
    (word_like && key.chars().count() <= 20 && !value.trim().is_empty())
        .then(|| (key, value.trim()))
}

fn parse_value(value: &str, budget: u32) -> FieldValue {
    if value.starts_with('{') || value.starts_with('[') {
        if let Ok(json) = serde_json::from_str::<Value>(value) {
            return json_value(&json, budget);
        }
    }
    if let Some((before, after)) = value.split_once(" → ") {
        return FieldValue::Change {
            before: before.to_owned(),
            after: after.to_owned(),
        };
    }
    let items: Vec<&str> = value.split(", ").map(str::trim).collect();
    let chip_like = |item: &&str| {
        !item.is_empty() && item.chars().count() <= CHIP_MAX_CHARS && !item.contains(": ")
    };
    let single_token = items.len() == 1 && !value.contains(' ');
    if (items.len() > 1 || single_token) && items.iter().all(chip_like) {
        return FieldValue::Chips(items.into_iter().map(str::to_owned).collect());
    }
    FieldValue::Text(value_rows(value, budget))
}

fn json_value(json: &Value, budget: u32) -> FieldValue {
    match json {
        Value::Object(map) => FieldValue::Fields {
            fields: map
                .iter()
                .take(MAX_FIELDS)
                .map(|(key, value)| (key.clone(), summarize(value)))
                .collect(),
            more: map.len().saturating_sub(MAX_FIELDS),
        },
        Value::Array(items)
            if items
                .iter()
                .all(|item| !item.is_object() && !item.is_array()) =>
        {
            FieldValue::Chips(items.iter().take(MAX_FIELDS).map(summarize).collect())
        }
        other => FieldValue::Text(value_rows(&summarize(other), budget)),
    }
}

/// A one-glance rendering of a JSON value: strings unquoted, containers by
/// what they are or how big they are.
pub(crate) fn summarize(value: &Value) -> String {
    let text = match value {
        Value::Null => "(unset)".to_owned(),
        Value::String(text) => text.replace('\n', " "),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Array(items)
            if items
                .iter()
                .all(|item| !item.is_object() && !item.is_array()) =>
        {
            format!(
                "[{}]",
                items.iter().map(summarize).collect::<Vec<_>>().join(", ")
            )
        }
        Value::Array(items) => match items.len() {
            1 => "1 item".to_owned(),
            count => format!("{count} items"),
        },
        Value::Object(map) => map
            .get("type")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| match map.len() {
                1 => "1 field".to_owned(),
                count => format!("{count} fields"),
            }),
    };
    if text.chars().count() <= SUMMARY_CHARS {
        return text;
    }
    text.chars().take(SUMMARY_CHARS).collect::<String>() + "…"
}

/// Two or more short facts separated by middle dots.
fn parse_stats(line: &str) -> Option<Vec<String>> {
    let parts: Vec<&str> = line.split(" · ").map(str::trim).collect();
    (parts.len() >= 2
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().count() <= 28))
    .then(|| parts.into_iter().map(str::to_owned).collect())
}

/// A free-text value wrapped to the column beside the key.
fn value_rows(value: &str, budget: u32) -> Vec<String> {
    let full = ((budget as f32 * 1.2) as u32).saturating_sub(4);
    let key_units = ((KEY_W + 6.0) / 5.5) as u32;
    let mut rows = wrap_units(value, full.saturating_sub(key_units).max(8));
    if rows.len() > MAX_VALUE_ROWS {
        rows.truncate(MAX_VALUE_ROWS);
        if let Some(last) = rows.last_mut() {
            last.push('…');
        }
    }
    rows
}

pub(crate) fn line_height(line: &DetailLine) -> f32 {
    let rows = |count: usize| count.max(1) as f32 * ACTION_DETAIL_LINE_H;
    match line {
        DetailLine::Repair(_) => REPAIR_ROW_H,
        DetailLine::Counts(_) | DetailLine::Stats(_) => CHIP_ROW_H,
        DetailLine::Change { .. } => CHANGE_ROW_H,
        DetailLine::Text { rows: lines, .. } => rows(lines.len()),
        DetailLine::Field { value, .. } => match value {
            FieldValue::Chips(_) => CHIP_ROW_H,
            FieldValue::Change { .. } => CHANGE_ROW_H,
            FieldValue::Text(lines) => rows(lines.len()) + 2.0,
            FieldValue::Fields { fields, more } => {
                rows(1)
                    + fields.len() as f32 * ACTION_DETAIL_LINE_H
                    + if *more > 0 { ACTION_DETAIL_LINE_H } else { 0.0 }
                    + 2.0
            }
        },
    }
}

#[cfg(test)]
#[path = "ai_chat_transcript_detail_body_tests.rs"]
mod tests;
