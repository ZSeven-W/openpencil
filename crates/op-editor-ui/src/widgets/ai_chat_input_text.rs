//! Chat input text hit-testing and selection paint helpers.

use crate::theme::Theme;
use crate::widgets::ai_chat_panel::to_jian_color;
use crate::widgets::ai_chat_transcript::CHAR_UNIT_PX;
use crate::widgets::ai_chat_transcript_text::char_display_units;
use crate::widgets::canvas_viewport_overlay::wrap_text;
use crate::widgets::text_selection::selection_color;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::chat::{ChatInputSelection, ChatState};

pub(crate) const INPUT_LINE_H: f32 = 18.0;
pub(crate) const INPUT_TEXT_X_PAD: f32 = 8.0;
pub(crate) const INPUT_BASELINE_ASCENT: f32 = 14.0;
pub(crate) const INPUT_FONT: f32 = 14.0;
pub(crate) const INPUT_MAX_LINES: usize = 3;

pub(crate) fn input_text_x(input_rect: Rect) -> f32 {
    input_rect.origin.x + INPUT_TEXT_X_PAD
}

pub(crate) fn input_text_width(input_rect: Rect) -> f32 {
    (input_rect.size.x - INPUT_TEXT_X_PAD * 2.0).max(24.0)
}

pub(crate) fn input_first_baseline(visible_rows: usize, input_area_h: f32) -> f32 {
    let rows = visible_rows.max(1) as f32;
    let block_h = rows * INPUT_LINE_H;
    ((input_area_h - block_h) / 2.0 + INPUT_BASELINE_ASCENT)
        .clamp(INPUT_BASELINE_ASCENT, input_area_h - 4.0)
}

pub(crate) fn input_text_offset_at(text: &str, input_rect: Rect, point: Point2D) -> Option<usize> {
    if !rect_contains(input_rect, point) {
        return None;
    }
    if text.is_empty() {
        return Some(0);
    }
    let lines = wrap_input_approx(text, input_text_width(input_rect));
    if lines.is_empty() {
        return Some(0);
    }
    let visible_start = lines.len().saturating_sub(INPUT_MAX_LINES);
    let visible = &lines[visible_start..];
    let first_baseline = input_first_baseline(visible.len(), input_rect.size.y);
    let top = input_rect.origin.y + first_baseline - INPUT_BASELINE_ASCENT;
    let line_idx = ((point.y - top).max(0.0) / INPUT_LINE_H)
        .floor()
        .clamp(0.0, (visible.len() - 1) as f32) as usize;
    let all_offsets = line_start_offsets(text, &lines);
    let line_start = *all_offsets.get(visible_start + line_idx).unwrap_or(&0);
    let line = &visible[line_idx];
    let line_offset = line_offset_at_x(line, point.x - input_text_x(input_rect));
    Some(previous_char_boundary(
        text,
        (line_start + line_offset).min(text.len()),
    ))
}

pub(crate) fn paint_input_text_area(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &ChatState,
    input_rect: Rect,
    input_area_h: f32,
    now_ms: u64,
    placeholder: &str,
) {
    let text_x = input_text_x(input_rect);
    let text_w = input_text_width(input_rect);
    let is_placeholder = state.input.is_empty();
    let (text, color) = if is_placeholder {
        (placeholder, theme.muted_foreground)
    } else {
        (state.input.as_str(), theme.foreground)
    };
    let wrapped = wrap_text(cx.backend, text, INPUT_FONT, text_w, 400);
    let start = wrapped.len().saturating_sub(INPUT_MAX_LINES);
    let visible = &wrapped[start..];
    let first_baseline = input_first_baseline(visible.len(), input_area_h);

    cx.backend.save();
    cx.backend.clip_rect(input_rect);
    if !is_placeholder {
        let input_selection = if state.input_select_all {
            Some(ChatInputSelection {
                anchor: 0,
                focus: state.input.len(),
            })
        } else {
            state.input_selection
        };
        if let Some(selection) = input_selection {
            paint_wrapped_input_selection(
                cx,
                theme,
                state.input.as_str(),
                &wrapped,
                start,
                input_rect,
                first_baseline,
                selection,
            );
        }
    }
    for (i, line) in visible.iter().enumerate() {
        let baseline = input_rect.origin.y + first_baseline + i as f32 * INPUT_LINE_H;
        let label = TextLayout::single_run(
            line,
            "system-ui",
            INPUT_FONT,
            to_jian_color(color),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&label, Point2D::new(text_x, baseline));
    }
    cx.backend.restore();

    let selection_collapsed = state
        .input_selection
        .map(|selection| selection.is_collapsed())
        .unwrap_or(true);
    let caret_visible = state.focused
        && !state.input_select_all
        && selection_collapsed
        && jian_core::anim::blink_visible(now_ms, state.caret_anchor_ms, 500);
    if !caret_visible {
        return;
    }

    let (caret_x, caret_line) = if is_placeholder {
        (text_x, 0usize)
    } else if let Some(selection) = state.input_selection {
        input_caret_position(
            cx,
            state.input.as_str(),
            &wrapped,
            start,
            input_rect,
            selection.focus,
        )
        .unwrap_or((text_x, 0usize))
    } else {
        let last = visible.last().map(String::as_str).unwrap_or("");
        (
            text_x + cx.backend.measure_text(last, INPUT_FONT),
            visible.len().saturating_sub(1),
        )
    };
    let caret_top = input_rect.origin.y + first_baseline + caret_line as f32 * INPUT_LINE_H - 13.0;
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(caret_x, caret_top),
            size: Point2D::new(1.5, 17.0),
        },
        theme.foreground,
    );
}

pub(crate) fn paint_wrapped_input_selection(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    text: &str,
    wrapped: &[String],
    visible_start: usize,
    input_rect: Rect,
    first_baseline: f32,
    selection: ChatInputSelection,
) {
    if selection.is_collapsed() || text.is_empty() || wrapped.is_empty() {
        return;
    }
    let (start, end) = selection.ordered();
    let start = previous_char_boundary(text, start.min(text.len()));
    let end = previous_char_boundary(text, end.min(text.len()));
    if start >= end {
        return;
    }
    let offsets = line_start_offsets(text, wrapped);
    let text_x = input_text_x(input_rect);
    let visible = &wrapped[visible_start.min(wrapped.len())..];
    for (visible_i, line) in visible.iter().enumerate() {
        let line_i = visible_start + visible_i;
        let line_start = *offsets.get(line_i).unwrap_or(&0);
        let line_end = (line_start + line.len()).min(text.len());
        if end <= line_start || start >= line_end {
            continue;
        }
        let local_start =
            previous_char_boundary(line, start.saturating_sub(line_start).min(line.len()));
        let local_end =
            previous_char_boundary(line, end.saturating_sub(line_start).min(line.len()));
        let x0 = text_x + measure_prefix(cx, line, local_start);
        let x1 = text_x + measure_prefix(cx, line, local_end);
        if x1 <= x0 {
            continue;
        }
        let baseline = input_rect.origin.y + first_baseline + visible_i as f32 * INPUT_LINE_H;
        cx.backend.fill_round_rect(
            Rect::xywh(
                x0 - 2.0,
                baseline - INPUT_FONT - 2.0,
                x1 - x0 + 4.0,
                INPUT_FONT + 6.0,
            ),
            3.0,
            selection_color(theme),
        );
    }
}

pub(crate) fn input_caret_position(
    cx: &mut PaintCx<'_>,
    text: &str,
    wrapped: &[String],
    visible_start: usize,
    input_rect: Rect,
    offset: usize,
) -> Option<(f32, usize)> {
    if text.is_empty() {
        return Some((input_text_x(input_rect), 0));
    }
    if wrapped.is_empty() {
        return None;
    }
    let offset = previous_char_boundary(text, offset.min(text.len()));
    let offsets = line_start_offsets(text, wrapped);
    let mut line_i = visible_start.min(wrapped.len().saturating_sub(1));
    for i in visible_start..wrapped.len() {
        let start = *offsets.get(i).unwrap_or(&0);
        let end = (start + wrapped[i].len()).min(text.len());
        if offset >= start && offset <= end {
            line_i = i;
            break;
        }
    }
    let line = &wrapped[line_i];
    let line_start = *offsets.get(line_i).unwrap_or(&0);
    let local = previous_char_boundary(line, offset.saturating_sub(line_start).min(line.len()));
    Some((
        input_text_x(input_rect) + measure_prefix(cx, line, local),
        line_i.saturating_sub(visible_start),
    ))
}

fn wrap_input_approx(text: &str, max_w: f32) -> Vec<String> {
    let max_units = (max_w / CHAR_UNIT_PX).floor().max(1.0) as u32;
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut units = 0;
    for ch in text.chars() {
        if ch == '\n' {
            lines.push(std::mem::take(&mut line));
            units = 0;
            continue;
        }
        let ch_units = char_display_units(ch);
        if units > 0 && units + ch_units > max_units {
            lines.push(std::mem::take(&mut line));
            units = 0;
        }
        line.push(ch);
        units += ch_units;
    }
    lines.push(line);
    lines
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

fn measure_prefix(cx: &mut PaintCx<'_>, line: &str, offset: usize) -> f32 {
    if offset == 0 {
        return 0.0;
    }
    cx.backend
        .measure_text(&line[..offset.min(line.len())], INPUT_FONT)
}

fn previous_char_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn rect_contains(rect: Rect, point: Point2D) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.x
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.y
}
