use crate::Rect;
use op_editor_core::{ChatActivity, ChatActivityStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParsedStepStatus {
    Pending,
    Streaming,
    Done,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedStep {
    pub title: String,
    pub status: Option<ParsedStepStatus>,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepExtraction {
    pub steps: Vec<ParsedStep>,
    pub visible_text: String,
}

/// One compact design-progress row, matching the TS chat's action
/// step treatment rather than dumping progress into reasoning text.
pub(crate) struct ActionStep {
    pub rect: Rect,
    /// Stable slot in the message's combined legacy/activity override list.
    pub source_index: usize,
    pub label: String,
    pub details: Vec<String>,
    pub expanded: bool,
    pub done: bool,
    pub active: bool,
    pub failed: bool,
    /// True only for a failed row backed by a persisted
    /// `ChatMessage::failed_subtasks` entry — gates the row's "Retry" icon
    /// so it never paints a dead button for a failure with nothing to
    /// replay (e.g. every activity flipped to `Error` by a whole-run
    /// catastrophic failure, which produces no `RunSummary` and therefore
    /// no persisted subtask spec).
    pub retryable: bool,
    /// The structured body painted instead of `details` when they are
    /// quality-repair records.
    pub receipt: Option<super::ai_chat_transcript_receipt_body::ReceiptBody>,
}

/// The font size step detail rows paint at, smaller than the body text.
pub(crate) const DETAIL_FONT: f32 = 10.0;

/// Wrap a step's detail text into rows. Each `\n`-separated line is one
/// item (a quality pass reports one repair per line): it wraps at the detail
/// font's own width, which holds more than the body-text `budget` it is
/// derived from, and its continuation rows hang under the first so the
/// items stay apart.
pub(crate) fn wrap_detail(text: &str, budget: u32) -> Vec<String> {
    use super::ai_chat_transcript::BODY_FONT;
    use super::ai_chat_transcript_text::wrap_units;
    const HANG: &str = "   ";
    let width = ((budget as f32 * BODY_FONT / DETAIL_FONT) as u32)
        .saturating_sub(4)
        .max(12);
    text.split('\n')
        .flat_map(|line| {
            wrap_units(line, width)
                .into_iter()
                .enumerate()
                .map(|(index, row)| {
                    if index == 0 {
                        row
                    } else {
                        format!("{HANG}{row}")
                    }
                })
        })
        .collect()
}

/// [`action_step_height`] for a row that may carry a structured repair body.
pub(crate) fn step_height(
    expanded: bool,
    detail_count: usize,
    receipt: Option<&super::ai_chat_transcript_receipt_body::ReceiptBody>,
) -> f32 {
    use super::ai_chat_transcript::{ACTION_DETAIL_GAP, ACTION_STEP_H};
    match receipt {
        Some(body) if expanded => ACTION_STEP_H + ACTION_DETAIL_GAP + body.height,
        _ => action_step_height(expanded, detail_count),
    }
}

/// Height of one step row: the header alone when collapsed (or with nothing
/// to reveal), plus one line per detail row when expanded.
pub(crate) fn action_step_height(expanded: bool, detail_count: usize) -> f32 {
    use super::ai_chat_transcript::{ACTION_DETAIL_GAP, ACTION_DETAIL_LINE_H, ACTION_STEP_H};
    if !expanded || detail_count == 0 {
        ACTION_STEP_H
    } else {
        ACTION_STEP_H + ACTION_DETAIL_GAP + detail_count as f32 * ACTION_DETAIL_LINE_H
    }
}

pub(crate) fn activity_step(activity: &ChatActivity) -> ParsedStep {
    let status = match activity.status {
        ChatActivityStatus::Pending => ParsedStepStatus::Pending,
        ChatActivityStatus::Running => ParsedStepStatus::Streaming,
        ChatActivityStatus::Done => ParsedStepStatus::Done,
        ChatActivityStatus::Error => ParsedStepStatus::Error,
    };
    ParsedStep {
        title: activity.title.clone(),
        status: Some(status),
        // One entry, newlines intact. A row's detail may be an itemized list
        // — the quality passes write one line per applied repair — and the
        // layout step splits it: `wrap_units` breaks on `\n` before wrapping,
        // so each line becomes its own expandable row. Do not "helpfully"
        // pre-split here; that would duplicate the split, not enable it.
        details: activity.detail.iter().cloned().collect(),
    }
}

pub(crate) fn strip_tool_call_xml(text: &str) -> String {
    let mut cleaned = text.to_string();
    for tag in ["function_calls", "result", "inference_process", "parameter"] {
        cleaned = strip_closed_blocks(&cleaned, tag);
    }
    cleaned = strip_closed_blocks(&cleaned, "invoke");
    cleaned = strip_unclosed_block(&cleaned, "invoke");
    for tag in ["invoke", "parameter", "function_calls"] {
        cleaned = strip_open_tags(&cleaned, tag);
        cleaned = strip_close_tags(&cleaned, tag);
    }
    for tag in ["search_quality_reflection", "thought_process"] {
        cleaned = strip_simple_tags(&cleaned, tag);
    }
    cleaned = cleaned.replace("<!-- APPLIED -->", "");
    collapse_blank_lines(&cleaned).trim().to_string()
}

pub(crate) fn split_design_progress(thinking: &str) -> (Vec<ParsedStep>, String) {
    let mut steps = Vec::new();
    let mut rest = Vec::new();
    for line in thinking.lines() {
        let trimmed = line.trim();
        if let Some(label) = trimmed.strip_prefix('•').map(str::trim) {
            if !label.is_empty() {
                steps.push(ParsedStep {
                    title: label.to_string(),
                    status: None,
                    details: Vec::new(),
                });
            }
        } else if let Some(detail) = trimmed.strip_prefix('▸').map(str::trim) {
            // Indented `▸` sub-line — Component 5's per-subtask detail
            // (skills / dropped / retry). Attach it to the most recent
            // step so the checklist can reveal it on expand.
            if !detail.is_empty() {
                if let Some(step) = steps.last_mut() {
                    step.details.push(detail.to_string());
                } else {
                    rest.push(line.trim_end().to_string());
                }
            }
        } else if !trimmed.is_empty() {
            rest.push(line.trim_end().to_string());
        }
    }
    (steps, rest.join("\n"))
}

pub(crate) fn extract_step_blocks(text: &str, is_streaming: bool) -> StepExtraction {
    let mut steps = Vec::new();
    let mut visible = String::new();
    let mut cursor = 0usize;
    let mut keep_from = 0usize;

    while let Some(open) = find_ascii_ci(text, "<step", cursor) {
        let Some(tag_end_rel) = text[open..].find('>') else {
            visible.push_str(&text[keep_from..open]);
            if is_streaming {
                steps.push(parsed_step("", "", true));
            }
            return finish_extraction(steps, visible);
        };
        let tag_end = open + tag_end_rel;
        let attrs = &text[open + "<step".len()..tag_end];
        let content_start = tag_end + 1;

        if let Some(close) = find_ascii_ci(text, "</step>", content_start) {
            visible.push_str(&text[keep_from..open]);
            steps.push(parsed_step(attrs, &text[content_start..close], false));
            let close_end = close + "</step>".len();
            cursor = close_end;
            keep_from = close_end;
        } else {
            visible.push_str(&text[keep_from..open]);
            if is_streaming {
                steps.push(parsed_step(attrs, &text[content_start..], true));
            }
            return finish_extraction(steps, visible);
        }
    }

    visible.push_str(&text[keep_from..]);
    finish_extraction(steps, visible)
}

fn finish_extraction(steps: Vec<ParsedStep>, visible: String) -> StepExtraction {
    StepExtraction {
        steps,
        visible_text: visible.trim().to_string(),
    }
}

fn strip_closed_blocks(input: &str, tag: &str) -> String {
    let open_pat = format!("<{tag}");
    let close_pat = format!("</{tag}>");
    let mut out = String::new();
    let mut cursor = 0usize;

    while let Some(open) = find_ascii_ci(input, &open_pat, cursor) {
        if !is_open_tag_at(input, open, tag) {
            out.push_str(&input[cursor..open + 1]);
            cursor = open + 1;
            continue;
        }
        let Some(tag_end_rel) = input[open..].find('>') else {
            break;
        };
        let content_start = open + tag_end_rel + 1;
        let Some(close) = find_ascii_ci(input, &close_pat, content_start) else {
            break;
        };
        out.push_str(&input[cursor..open]);
        cursor = close + close_pat.len();
    }

    out.push_str(&input[cursor..]);
    out
}

fn strip_unclosed_block(input: &str, tag: &str) -> String {
    let open_pat = format!("<{tag}");
    let mut cursor = 0usize;
    while let Some(open) = find_ascii_ci(input, &open_pat, cursor) {
        if is_open_tag_at(input, open, tag) {
            return input[..open].to_string();
        }
        cursor = open + 1;
    }
    input.to_string()
}

fn strip_open_tags(input: &str, tag: &str) -> String {
    let open_pat = format!("<{tag}");
    let mut out = String::new();
    let mut cursor = 0usize;

    while let Some(open) = find_ascii_ci(input, &open_pat, cursor) {
        if !is_open_tag_at(input, open, tag) {
            out.push_str(&input[cursor..open + 1]);
            cursor = open + 1;
            continue;
        }
        let Some(tag_end_rel) = input[open..].find('>') else {
            break;
        };
        out.push_str(&input[cursor..open]);
        cursor = open + tag_end_rel + 1;
    }

    out.push_str(&input[cursor..]);
    out
}

fn strip_close_tags(input: &str, tag: &str) -> String {
    let close_pat = format!("</{tag}>");
    let mut out = String::new();
    let mut cursor = 0usize;

    while let Some(close) = find_ascii_ci(input, &close_pat, cursor) {
        out.push_str(&input[cursor..close]);
        cursor = close + close_pat.len();
    }

    out.push_str(&input[cursor..]);
    out
}

fn strip_simple_tags(input: &str, tag: &str) -> String {
    strip_close_tags(&strip_open_tags(input, tag), tag)
}

fn is_open_tag_at(input: &str, open: usize, tag: &str) -> bool {
    let Some(after) = input.get(open + 1 + tag.len()..) else {
        return false;
    };
    after
        .chars()
        .next()
        .is_none_or(|c| c == '>' || c == '/' || c.is_ascii_whitespace())
}

fn collapse_blank_lines(input: &str) -> String {
    let mut out = Vec::new();
    let mut blank = false;
    for line in input.lines().map(str::trim_end) {
        if line.trim().is_empty() {
            if !blank && !out.is_empty() {
                out.push(String::new());
            }
            blank = true;
        } else {
            out.push(line.to_string());
            blank = false;
        }
    }
    out.join("\n")
}

fn parsed_step(attrs: &str, content: &str, partial: bool) -> ParsedStep {
    let default_title = if partial { "Design" } else { "Processing" };
    ParsedStep {
        title: attr_value(attrs, "title")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default_title.to_string()),
        status: attr_value(attrs, "status").and_then(|s| parse_status(s.trim())),
        details: step_details(content),
    }
}

fn step_details(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_status(value: &str) -> Option<ParsedStepStatus> {
    match value.to_ascii_lowercase().as_str() {
        "pending" => Some(ParsedStepStatus::Pending),
        "streaming" => Some(ParsedStepStatus::Streaming),
        "done" => Some(ParsedStepStatus::Done),
        "error" => Some(ParsedStepStatus::Error),
        _ => None,
    }
}

fn attr_value(attrs: &str, key: &str) -> Option<String> {
    let mut cursor = 0usize;
    while let Some(found_rel) = find_ascii_ci(attrs, key, cursor) {
        let found = found_rel;
        let before_ok = found == 0
            || attrs[..found]
                .chars()
                .next_back()
                .is_none_or(|c| c.is_ascii_whitespace());
        let after_key = found + key.len();
        let after_ok = attrs[after_key..]
            .chars()
            .next()
            .is_none_or(|c| c.is_ascii_whitespace() || c == '=');
        if !before_ok || !after_ok {
            cursor = after_key;
            continue;
        }

        let mut rest = attrs[after_key..].trim_start();
        if !rest.starts_with('=') {
            cursor = after_key;
            continue;
        }
        rest = rest[1..].trim_start();
        let quote = rest.chars().next()?;
        if quote == '"' || quote == '\'' {
            let value_start = quote.len_utf8();
            let value = &rest[value_start..];
            return value.find(quote).map(|end| value[..end].to_string());
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }
    None
}

fn find_ascii_ci(haystack: &str, needle: &str, start: usize) -> Option<usize> {
    let hay = haystack.get(start..)?;
    let hay = hay.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    hay.find(&needle).map(|idx| start + idx)
}

fn progress_failed(label: &str) -> bool {
    let lower = label.to_ascii_lowercase();
    lower.contains("failed") || lower.starts_with("error:")
}

fn progress_terminal(label: &str) -> bool {
    let lower = label.to_ascii_lowercase();
    progress_failed(label)
        || lower.contains(" done")
        || lower.ends_with("done")
        || lower.contains("ready")
        || lower.contains("applied")
        || lower.contains("captured")
        || lower.contains("skipped")
}

pub(crate) fn step_state(
    step: &ParsedStep,
    streaming: bool,
    index: usize,
    total: usize,
) -> (bool, bool, bool) {
    match step.status {
        Some(ParsedStepStatus::Done) => (true, false, false),
        Some(ParsedStepStatus::Error) => (true, false, true),
        Some(ParsedStepStatus::Streaming) => (false, streaming, false),
        Some(ParsedStepStatus::Pending) => (false, false, false),
        None => {
            let failed = progress_failed(&step.title);
            let done = failed || !streaming || index + 1 < total || progress_terminal(&step.title);
            let active = streaming && index + 1 == total && !done;
            (done, active, failed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_closed_step_blocks_and_strips_visible_text() {
        let extracted = extract_step_blocks(
            r#"before <step title="Check" status="done">ok</step> after"#,
            false,
        );

        assert_eq!(extracted.visible_text, "before  after");
        assert_eq!(
            extracted.steps,
            vec![ParsedStep {
                title: "Check".into(),
                status: Some(ParsedStepStatus::Done),
                details: vec!["ok".into()],
            }]
        );
    }

    #[test]
    fn extracts_partial_streaming_step() {
        let extracted =
            extract_step_blocks(r#"<step title="Sketch" status="streaming">drawing"#, true);

        assert!(extracted.visible_text.is_empty());
        assert_eq!(extracted.steps[0].title, "Sketch");
        assert_eq!(extracted.steps[0].status, Some(ParsedStepStatus::Streaming));
    }

    #[test]
    fn split_design_progress_attaches_detail_sublines_to_owning_step() {
        let thinking = "• Subtask `header` — 顶部问候栏  ·  6 skills · 5200/8000 tok\n  ▸ skills: cjk-typography, mobile-app\n  ▸ dropped: examples (budget)";

        let (steps, rest) = split_design_progress(thinking);

        assert_eq!(steps.len(), 1);
        assert_eq!(
            steps[0].details,
            vec![
                "skills: cjk-typography, mobile-app".to_string(),
                "dropped: examples (budget)".to_string(),
            ]
        );
        assert!(rest.is_empty(), "detail lines must not leak into rest");
    }

    #[test]
    fn split_design_progress_detail_with_no_preceding_step_goes_to_rest() {
        // A `▸` line with no owning step must not panic; it falls into rest.
        let thinking = "  ▸ orphan detail\n• Step one";

        let (steps, rest) = split_design_progress(thinking);

        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].title, "Step one");
        assert!(steps[0].details.is_empty());
        assert!(rest.contains("orphan detail"));
    }

    #[test]
    fn split_design_progress_non_detail_lines_still_create_steps() {
        let thinking = "• Step A\nsome free text\n• Step B";

        let (steps, rest) = split_design_progress(thinking);

        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].title, "Step A");
        assert_eq!(steps[1].title, "Step B");
        assert!(rest.contains("some free text"));
    }
}
