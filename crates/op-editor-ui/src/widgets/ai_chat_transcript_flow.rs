//! Interleaved narration ↔ tool-call flow for design-loop messages.
//!
//! The reference transcript alternates prose paragraphs with compact
//! per-call verb chips at the exact point of the story where each call
//! happened. `ChatToolCall.content_offset` (stamped by the host when the
//! call arrives mid-stream) records that point as a byte offset into the
//! message content; this module splits the content at those offsets and
//! lays out prose segments and headerless tool panels in document order.
//! Messages whose calls carry no offsets keep the classic grouped
//! "N tool calls" panel.

use op_editor_core::chat::ChatMessage;
use op_editor_core::ChatActivity;

use crate::Rect;

use super::ai_chat_transcript::{
    action_step_height, streaming_caret_visible, ActionStep, TextBubble, TranscriptItem,
    ACTION_STEP_GAP, LINE_H,
};
use super::ai_chat_transcript_richtext::{layout_rich, paint_rich, rich_height, rich_line_width};
use super::ai_chat_transcript_steps::{activity_step, step_state, strip_tool_call_xml};
use super::ai_chat_transcript_text::wrap_units;
use super::ai_chat_transcript_tools::{
    build_tool_panel, paint_tool_panel, ToolPanel, ToolPanelLayout, CARD_GAP,
};
use crate::theme::Theme;
use crate::widgets::PaintCx;

/// Breathing room between a prose paragraph and the adjacent tool panel.
/// Applied SYMMETRICALLY: the panel builder already appends a trailing
/// `CARD_GAP`, so the flow subtracts it before adding this — otherwise the
/// gap below a chip ran 4px wider than the gap above it and the chip read as
/// belonging to the paragraph BELOW it (user report 2026-07-12).
const FLOW_GAP: f32 = 10.0;

/// The interleaved flow applies when any call knows WHERE in the narration
/// it happened. (User messages and non-loop assistant turns never stamp
/// offsets, so they keep the grouped panel.)
pub(crate) fn should_interleave(msg: &ChatMessage) -> bool {
    !msg.tool_calls.is_empty()
        && msg
            .tool_calls
            .iter()
            .any(|call| call.content_offset.is_some())
}

/// CLI-backed design turns carry the same chronological anchor as built-in
/// tool calls, but on provider-neutral activities. Legacy rows without an
/// offset keep their grouped checklist layout.
pub(crate) fn should_interleave_activities(msg: &ChatMessage) -> bool {
    !msg.activities.is_empty()
        && msg
            .activities
            .iter()
            .any(|activity| activity.content_offset.is_some())
}

/// Lay out narration and CLI design activities in one ordered flow. The
/// activity cards and built-in tool cards remain different adapters, while
/// their placement obeys the same byte-offset timeline contract.
pub(crate) fn build_activity_flow(
    msg: &ChatMessage,
    x: f32,
    mut y: f32,
    width: f32,
    budget: u32,
    source_index_base: usize,
) -> (Vec<TextBubble>, Vec<ActionStep>, f32) {
    let content = msg.content.as_str();
    let groups = activity_groups(&msg.activities, content);
    let mut bubbles = Vec::new();
    let mut steps = Vec::new();
    let mut cursor = 0usize;

    for (offset, first_index, len) in groups {
        y = push_prose(
            &content[cursor..offset],
            x,
            &mut y,
            width,
            budget,
            &mut bubbles,
        );
        cursor = offset;
        for index in first_index..first_index + len {
            let source_index = source_index_base + index;
            let parsed = activity_step(&msg.activities[index]);
            let details: Vec<String> = parsed
                .details
                .iter()
                .flat_map(|line| wrap_units(line, budget.saturating_sub(4)))
                .collect();
            let (done, active, failed) =
                step_state(&parsed, msg.streaming, index, msg.activities.len());
            let expanded = msg
                .action_step_expanded_overrides
                .get(source_index)
                .copied()
                .flatten()
                .unwrap_or(active || failed);
            let height = action_step_height(expanded, details.len());
            // `index` is a direct `msg.activities` index here (unlike the
            // legacy thinking-text-derived steps in `build_item`), so the
            // retry lookup is exact — this is the path a real design-turn
            // message actually renders through: `upsert_activity` always
            // stamps `content_offset`, so `should_interleave_activities` is
            // true for every classic-orchestrator turn.
            let retryable = failed
                && msg
                    .failed_subtasks
                    .iter()
                    .any(|p| p.subtask_id == msg.activities[index].id);
            steps.push(ActionStep {
                rect: Rect::xywh(x, y, width, height),
                source_index,
                label: parsed.title,
                details,
                expanded,
                done,
                active,
                failed,
                retryable,
            });
            y += height + ACTION_STEP_GAP;
        }
        y += (FLOW_GAP - ACTION_STEP_GAP).max(0.0);
    }

    y = push_prose(&content[cursor..], x, &mut y, width, budget, &mut bubbles);
    (bubbles, steps, y)
}

fn activity_groups(activities: &[ChatActivity], content: &str) -> Vec<(usize, usize, usize)> {
    let mut groups = Vec::new();
    let mut last_offset = 0usize;
    for (index, activity) in activities.iter().enumerate() {
        let offset = activity
            .content_offset
            .map(|offset| clamp_to_char_boundary(content, offset as usize))
            .unwrap_or(last_offset)
            .max(last_offset);
        last_offset = offset;
        match groups.last_mut() {
            Some((group_offset, _, len)) if *group_offset == offset => *len += 1,
            _ => groups.push((offset, index, 1)),
        }
    }
    groups
}

/// Lay out the interleaved flow starting at `y`. Returns the prose bubbles,
/// the headerless per-group tool panels, and the y below the flow.
pub(crate) fn build_flow(
    msg: &ChatMessage,
    x: f32,
    mut y: f32,
    width: f32,
    budget: u32,
) -> (Vec<TextBubble>, Vec<ToolPanel>, f32) {
    let content = msg.content.as_str();
    let default_status = if msg.streaming { "running" } else { "done" };

    // Group consecutive calls by offset: a run of calls with no prose
    // between them stacks into one panel. Offsets are stamped from the
    // monotonically-growing content, so arrival order == offset order; a
    // missing offset inherits its predecessor's so the call stays grouped
    // with its neighbors instead of jumping to the top.
    let mut groups: Vec<(usize, usize, usize)> = Vec::new(); // (offset, first_index, len)
    let mut last_offset = 0usize;
    for (index, call) in msg.tool_calls.iter().enumerate() {
        let offset = call
            .content_offset
            .map(|o| clamp_to_char_boundary(content, o as usize))
            .unwrap_or(last_offset)
            .max(last_offset);
        last_offset = offset;
        match groups.last_mut() {
            Some((group_offset, _, len)) if *group_offset == offset => *len += 1,
            _ => groups.push((offset, index, 1)),
        }
    }

    let mut bubbles = Vec::new();
    let mut panels = Vec::new();
    let mut cursor = 0usize;
    for &(offset, first_index, len) in &groups {
        y = push_prose(
            &content[cursor..offset],
            x,
            &mut y,
            width,
            budget,
            &mut bubbles,
        );
        cursor = offset;
        let (panel, next_y) = build_tool_panel(
            &msg.tool_calls[first_index..first_index + len],
            ToolPanelLayout {
                collapsed: false,
                label: String::new(),
                x,
                y,
                width,
                budget,
                default_status,
                expanded_overrides: &msg.tool_call_expanded_overrides,
                first_index,
            },
        );
        if let Some(mut panel) = panel {
            // Headerless: zero the header band build_tool_panel reserved and
            // pull the cards up into its place.
            let header_h = panel.header.size.y;
            panel.header.size.y = 0.0;
            shift_panel_up(&mut panel, header_h);
            // The panel builder trails the last card with TWO CARD_GAPs (one
            // from the per-card loop, one closing the body); drop both so
            // FLOW_GAP alone sets the distance.
            y = next_y - header_h - 2.0 * CARD_GAP;
            panels.push(panel);
        }
        y += FLOW_GAP;
    }
    y = push_prose(&content[cursor..], x, &mut y, width, budget, &mut bubbles);
    (bubbles, panels, y)
}

fn push_prose(
    raw: &str,
    x: f32,
    y: &mut f32,
    width: f32,
    budget: u32,
    bubbles: &mut Vec<TextBubble>,
) -> f32 {
    let visible = normalize_narration_markdown(&strip_tool_call_xml(raw));
    let visible = visible.trim();
    if visible.is_empty() {
        return *y;
    }
    // The narration keeps its markdown typography — bold labels, code chips,
    // bulleted lists with a hanging indent (the reference reads as a typed
    // document, not a grey wall).
    let rich = layout_rich(visible, budget);
    let h = rich_height(&rich);
    bubbles.push(TextBubble {
        rect: Rect::xywh(x, *y, width, h),
        lines: wrap_units(visible, budget),
        rich,
        typing: false,
    });
    *y += h + FLOW_GAP;
    *y
}

fn shift_panel_up(panel: &mut ToolPanel, dy: f32) {
    panel.body.origin.y -= dy;
    for card in &mut panel.cards {
        card.rect.origin.y -= dy;
        card.header.origin.y -= dy;
        card.body.origin.y -= dy;
    }
}

/// Floor a stamped byte offset to the nearest char boundary (defensive: the
/// host stamps `content.len()` between whole streamed chunks, but a foreign
/// provider could split a multi-byte char across chunks).
fn clamp_to_char_boundary(s: &str, mut offset: usize) -> usize {
    offset = offset.min(s.len());
    while offset > 0 && !s.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

/// Paint the interleaved flow: prose paragraphs (with the streaming caret on
/// the last one) and headerless tool panels, all pre-placed at absolute rects.
pub(crate) fn paint_flow(cx: &mut PaintCx<'_>, theme: &Theme, item: &TranscriptItem, now_ms: u64) {
    for (flow_index, bubble) in item.flow_bubbles.iter().enumerate() {
        cx.backend.save();
        cx.backend.clip_rect(bubble.rect);
        paint_rich(cx, theme, &bubble.rich, bubble.rect.origin);
        if item.streaming
            && flow_index + 1 == item.flow_bubbles.len()
            && streaming_caret_visible(now_ms)
        {
            let caret_x = bubble.rich.last().map(rich_line_width).unwrap_or(0.0);
            let caret_y =
                bubble.rect.origin.y + (bubble.rich.len().saturating_sub(1)) as f32 * LINE_H + 2.0;
            cx.backend.fill_rect(
                Rect::xywh(bubble.rect.origin.x + caret_x, caret_y, 2.0, 13.0),
                theme.foreground,
            );
        }
        cx.backend.restore();
    }
    for panel in &item.flow_panels {
        paint_tool_panel(cx, theme, panel, now_ms);
    }
}

/// Repair the ONE thing the stream gets wrong: two bold headings glued
/// together ("**Batch 1****Batch 2**") or a heading opening right after a
/// colon. The markers themselves are LEFT IN — the transcript now renders
/// markdown (see `ai_chat_transcript_richtext`), so stripping them here would
/// throw away the typography before it is ever laid out.
pub(crate) fn normalize_narration_markdown(text: &str) -> String {
    text.replace("****", "**\n**").replace(":**", ":\n**")
}
