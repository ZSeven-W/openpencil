//! Receipts: a `•` line followed by indented `▸` sub-lines inside an
//! assistant message's content.
//!
//! The agent loop appends its quality credential this way after a design run
//! ("Checked layout, … — 10 auto-repair(s) applied, no issues left" plus one
//! `▸` line per repair). The thinking stream already turns that shape into a
//! step; in the content it used to be painted as prose, an unformatted wall
//! whose wrapped lines lost their indent. Here each receipt becomes the same
//! collapsed accordion row the run's other steps use: the result leads the
//! header (a one-line header truncates, so it must not open with the scope),
//! and the scope plus the itemized lines are one click away. Receipts trail
//! the message body, where the loop appended them.

use super::ai_chat_transcript::{step_height, ActionStep, ACTION_STEP_GAP};
use super::ai_chat_transcript_detail_body::detail_body;
use super::ai_chat_transcript_steps::{wrap_detail, ParsedStep, ParsedStepStatus};
use crate::widgets::ai_chat_workspace_copy::progress_label;
use crate::Rect;
use op_editor_core::chat::ChatMessage;

/// Split `text` into the prose without its receipts and one step per
/// receipt. A `•` line only counts as a receipt when a `▸` line follows it,
/// so ordinary bulleted narration stays prose.
pub(crate) fn split_receipts(text: &str) -> (String, Vec<ParsedStep>) {
    let lines: Vec<&str> = text.lines().collect();
    let mut kept = Vec::new();
    let mut receipts = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let title = lines[index]
            .trim()
            .strip_prefix('•')
            .map(str::trim)
            .filter(|title| !title.is_empty());
        let detailed = lines
            .get(index + 1)
            .is_some_and(|next| next.trim_start().starts_with('▸'));
        let (Some(title), true) = (title, detailed) else {
            kept.push(lines[index]);
            index += 1;
            continue;
        };
        index += 1;
        let mut details = Vec::new();
        while let Some(detail) = lines
            .get(index)
            .and_then(|line| line.trim().strip_prefix('▸'))
        {
            let detail = detail.trim();
            if !detail.is_empty() {
                details.push(detail.to_owned());
            }
            index += 1;
        }
        receipts.push(receipt_step(title, details));
    }
    (kept.join("\n").trim_end().to_owned(), receipts)
}

/// Lay the receipts out below the message body as accordion rows. Their
/// expand state lives in the message's step overrides after every other
/// step, starting at `source_index_base`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn layout_receipts(
    receipts: &[ParsedStep],
    msg: &ChatMessage,
    locale: op_editor_core::Locale,
    x: f32,
    y: &mut f32,
    width: f32,
    budget: u32,
    source_index_base: usize,
) -> Vec<ActionStep> {
    let mut steps = Vec::new();
    for (offset, receipt) in receipts.iter().enumerate() {
        let source_index = source_index_base + offset;
        let details: Vec<String> = receipt
            .details
            .iter()
            .flat_map(|line| wrap_detail(&progress_label(locale, line), budget))
            .collect();
        let expanded = msg
            .action_step_expanded_overrides
            .get(source_index)
            .copied()
            .flatten()
            .unwrap_or(false);
        let body = detail_body(&receipt.details, budget, false);
        let height = step_height(expanded, details.len(), body.as_ref());
        steps.push(ActionStep {
            rect: Rect::xywh(x, *y, width, height),
            source_index,
            label: progress_label(locale, &receipt.title),
            details,
            expanded,
            done: true,
            active: false,
            failed: false,
            retryable: false,
            detail_body: body,
        });
        *y += height + ACTION_STEP_GAP;
    }
    steps
}

/// A receipt reads "scope — result": the header leads with the result and
/// the scope becomes the first detail row.
fn receipt_step(title: &str, mut details: Vec<String>) -> ParsedStep {
    let title = match title.split_once(" — ") {
        Some((scope, result)) if !scope.trim().is_empty() && !result.trim().is_empty() => {
            details.insert(0, scope.trim().to_owned());
            capitalized(&pluralized(result.trim()).replacen(", ", " · ", 1))
        }
        _ => title.to_owned(),
    };
    ParsedStep {
        title,
        status: Some(ParsedStepStatus::Done),
        details,
    }
}

/// The producer writes counts as `N noun(s)`, which reads robotic: write
/// the noun in the number its count asks for (`10 auto-repairs`,
/// `1 issue`).
pub(crate) fn pluralized(text: &str) -> String {
    let words: Vec<&str> = text.split(' ').collect();
    let mut out = Vec::with_capacity(words.len());
    for (index, word) in words.iter().enumerate() {
        let count = index
            .checked_sub(1)
            .and_then(|previous| words[previous].parse::<u64>().ok());
        let word = match (word.split_once("(s)"), count) {
            (Some((noun, rest)), Some(1)) => format!("{noun}{rest}"),
            (Some((noun, rest)), _) => format!("{noun}s{rest}"),
            (None, _) => (*word).to_owned(),
        };
        out.push(word);
    }
    out.join(" ")
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
#[path = "ai_chat_transcript_receipts_tests.rs"]
mod tests;
