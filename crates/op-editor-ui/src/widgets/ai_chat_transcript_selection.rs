use super::ai_chat_transcript::{TextBubble, BODY_FONT, CHAR_UNIT_PX, LINE_H, USER_BUBBLE_PAD};
use super::ai_chat_transcript_cache::CanonicalTranscript;
use super::ai_chat_transcript_text::char_display_units;
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use jian_core::text_input::prev_char_boundary;
use op_editor_core::chat::{ChatMessage, ChatRole, ChatTranscriptSelection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TranscriptTextHit {
    pub message_index: usize,
    pub offset: usize,
}

pub(crate) fn transcript_text_offset_at(
    messages: &[ChatMessage],
    canonical: &CanonicalTranscript,
    body_rect: Rect,
    point: Point2D,
    scroll_offset: f32,
) -> Option<TranscriptTextHit> {
    if !(body_rect).contains(point) {
        return None;
    }
    // Canonical layout is scroll-0 and already resolved by the caller; shift the
    // query point down by the scroll so it lands in the same coordinate space as
    // the cached rects. `messages` supplies the live per-message text content.
    let p = Point2D::new(point.x, point.y + scroll_offset);
    for item in &canonical.items {
        if item.role != ChatRole::User {
            continue;
        }
        let Some(bubble) = &item.bubble else {
            continue;
        };
        if bubble.typing || !(bubble.rect).contains(p) {
            continue;
        }
        // Bounds-defense: the cached build may carry more items than the live
        // `messages` slice if messages shrank between the resolve that produced
        // this build and this probe (owner scoping stops CROSS-panel pairing but
        // not a same-panel same-frame shrink). A miss is treated as no-hit — an
        // out-of-bounds index would otherwise panic.
        let Some(message) = messages.get(item.msg_index) else {
            continue;
        };
        let text = &message.content;
        if text.is_empty() || bubble.lines.is_empty() {
            continue;
        }
        // User bubble text starts at USER_BUBBLE_PAD (14px) inset (#27 restyle).
        let text_top = bubble.rect.origin.y + USER_BUBBLE_PAD;
        let line_idx = ((p.y - text_top).max(0.0) / LINE_H)
            .floor()
            .clamp(0.0, (bubble.lines.len() - 1) as f32) as usize;
        let line = &bubble.lines[line_idx];
        let line_offsets = line_start_offsets(text, &bubble.lines);
        let line_start = *line_offsets.get(line_idx).unwrap_or(&0);
        let line_offset = line_offset_at_x(line, p.x - (bubble.rect.origin.x + USER_BUBBLE_PAD));
        let offset = prev_char_boundary(text, (line_start + line_offset).min(text.len()));
        return Some(TranscriptTextHit {
            message_index: item.msg_index,
            offset,
        });
    }
    None
}

pub(crate) fn paint_user_bubble_selection(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    text: &str,
    bubble: &TextBubble,
    selection: ChatTranscriptSelection,
) {
    if selection.is_collapsed() || text.is_empty() || bubble.lines.is_empty() {
        return;
    }
    let (start, end) = selection.ordered();
    let start = prev_char_boundary(text, start.min(text.len()));
    let end = prev_char_boundary(text, end.min(text.len()));
    if start >= end {
        return;
    }
    let offsets = line_start_offsets(text, &bubble.lines);
    // #27 restyle: user bubble text uses USER_BUBBLE_PAD (14px) inset.
    let text_x = bubble.rect.origin.x + USER_BUBBLE_PAD;
    let mut baseline = bubble.rect.origin.y + USER_BUBBLE_PAD + 11.0;
    for (i, line) in bubble.lines.iter().enumerate() {
        let line_start = *offsets.get(i).unwrap_or(&0);
        let line_end = (line_start + line.len()).min(text.len());
        if end > line_start && start < line_end {
            let local_start =
                prev_char_boundary(line, start.saturating_sub(line_start).min(line.len()));
            let local_end =
                prev_char_boundary(line, end.saturating_sub(line_start).min(line.len()));
            let x0 = text_x + line_x_at_offset(line, local_start);
            let x1 = text_x + line_x_at_offset(line, local_end);
            if x1 > x0 {
                cx.backend.fill_round_rect(
                    Rect::xywh(
                        x0 - 2.0,
                        baseline - BODY_FONT - 2.0,
                        x1 - x0 + 4.0,
                        BODY_FONT + 6.0,
                    ),
                    3.0,
                    crate::widgets::text_selection::selection_color(theme),
                );
            }
        }
        baseline += LINE_H;
    }
}

fn line_start_offsets(text: &str, lines: &[String]) -> Vec<usize> {
    let mut out = Vec::with_capacity(lines.len());
    let mut search_from = 0;
    for line in lines {
        if line.is_empty() {
            out.push(search_from.min(text.len()));
            continue;
        }
        let next = text[search_from..]
            .find(line)
            .map(|rel| search_from + rel)
            .unwrap_or(search_from);
        out.push(next);
        search_from = (next + line.len()).min(text.len());
    }
    out
}

fn line_offset_at_x(line: &str, x: f32) -> usize {
    if x <= 0.0 {
        return 0;
    }
    let mut px = 0.0;
    for (idx, ch) in line.char_indices() {
        let w = char_display_units(ch) as f32 * CHAR_UNIT_PX;
        if x < px + w / 2.0 {
            return idx;
        }
        px += w;
    }
    line.len()
}

fn line_x_at_offset(line: &str, offset: usize) -> f32 {
    line.char_indices()
        .take_while(|(idx, _)| *idx < offset)
        .map(|(_, ch)| char_display_units(ch) as f32 * CHAR_UNIT_PX)
        .sum()
}
