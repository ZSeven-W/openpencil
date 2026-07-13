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

use crate::Rect;

use super::ai_chat_transcript::{normalize_narration_markdown, TextBubble, LINE_H};
use super::ai_chat_transcript_steps::strip_tool_call_xml;
use super::ai_chat_transcript_text::wrap_units;
use super::ai_chat_transcript_tools::{build_tool_panel, ToolPanel, ToolPanelLayout};

/// Gap between a prose segment and the adjacent tool panel.
const FLOW_GAP: f32 = 6.0;

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
            y = next_y - header_h;
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
    let lines = wrap_units(visible, budget);
    let h = lines.len() as f32 * LINE_H;
    bubbles.push(TextBubble {
        rect: Rect::xywh(x, *y, width, h),
        lines,
        typing: false,
        completion: None,
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
