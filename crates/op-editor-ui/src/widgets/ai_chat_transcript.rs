//! Chat-transcript layout, paint + hit-test for
//! [`super::ai_chat_panel::AIChatPlaceholder`].
//!
//! The transcript replaces the old single-line message bubbles with
//! a structured view: each assistant message can carry a collapsible
//! thinking block and a collapsible tool-call panel above its answer
//! text, each user message can carry an image strip, and the
//! trailing in-flight message shows a streaming animation.
//!
//! Layout is fully deterministic — wrapping uses a fixed per-glyph
//! *unit* budget, never live text measurement — so paint and
//! hit-test compute identical rects without sharing a backend.

use super::ai_chat_panel::to_jian_color;
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::chat::{ChatMessage, ChatRole};

use super::ai_chat_transcript_steps::{
    extract_step_blocks, split_design_progress, strip_tool_call_xml, ParsedStep, ParsedStepStatus,
};
use super::ai_chat_transcript_tools::tool_lines;

/// Body text size used throughout the transcript.
const BODY_FONT: f32 = 12.0;
/// Height of one wrapped text line.
const LINE_H: f32 = 16.0;
/// Inner padding inside a bubble / block box.
const BUBBLE_PAD: f32 = 8.0;
/// Text shown in the TS-style empty streaming assistant pill.
const TYPING_LABEL: &str = "Thinking";
/// Horizontal padding inside the empty streaming assistant pill.
const TYPING_PAD_X: f32 = 10.0;
/// Vertical padding inside the empty streaming assistant pill.
const TYPING_PAD_Y: f32 = 4.0;
/// Gap between the "Thinking" label and animated dots.
const TYPING_LABEL_DOT_GAP: f32 = 6.0;
/// Diameter of one animated typing dot.
const TYPING_DOT: f32 = 4.0;
/// Horizontal gap between animated typing dots.
const TYPING_DOT_GAP: f32 = 2.0;
/// Height of a collapsible header row (thinking / tool-calls).
const HEADER_H: f32 = 22.0;
/// Vertical gap between two messages.
const MSG_GAP: f32 = 10.0;
/// Vertical gap between sub-blocks within one message.
const SUB_GAP: f32 = 4.0;
/// Height of one compact design-progress step row.
const ACTION_STEP_H: f32 = 28.0;
/// Height of one detail line under a progress step.
const ACTION_DETAIL_LINE_H: f32 = 14.0;
/// Gap between the progress title row and detail lines.
const ACTION_DETAIL_GAP: f32 = 4.0;
/// Vertical gap between compact design-progress rows.
const ACTION_STEP_GAP: f32 = 4.0;
/// Side length of an image thumbnail box.
const IMG_THUMB: f32 = 60.0;
/// Gap between image thumbnails.
const IMG_GAP: f32 = 4.0;
/// Approximate device px per wrap *unit* at [`BODY_FONT`]. Drives the
/// unit budget handed to [`wrap_units`].
const CHAR_UNIT_PX: f32 = 6.6;
/// Bubble width as a fraction of the transcript body width.
const BUBBLE_FRAC: f32 = 0.84;

/// What a click inside the transcript resolved to. Both variants
/// carry the index into the full `messages` slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptHit {
    /// The thinking-block header — toggle its collapsed state.
    ToggleThinking(usize),
    /// The tool-calls panel header — toggle its collapsed state.
    ToggleToolCalls(usize),
}

/// A collapsible block (thinking text or tool-call list) — a
/// clickable `header` row plus an optional `body` box. When
/// `collapsed`, `body` has zero height and `lines` is empty.
pub(crate) struct Collapsible {
    pub header: Rect,
    pub label: String,
    pub collapsed: bool,
    pub body: Rect,
    pub lines: Vec<String>,
}

/// One compact design-progress row, matching the TS chat's action
/// step treatment rather than dumping progress into reasoning text.
pub(crate) struct ActionStep {
    pub rect: Rect,
    pub label: String,
    pub details: Vec<String>,
    pub done: bool,
    pub active: bool,
    pub failed: bool,
}

/// The answer-text bubble of a message. `typing` is set on an
/// in-flight message with no text yet — paint shows animated dots.
pub(crate) struct TextBubble {
    pub rect: Rect,
    pub lines: Vec<String>,
    pub typing: bool,
}

/// One fully-placed message in the transcript — absolute rects ready
/// for paint, with the interactive headers exposed for hit-test.
pub(crate) struct TranscriptItem {
    pub msg_index: usize,
    pub role: ChatRole,
    pub steps: Vec<ActionStep>,
    pub thinking: Option<Collapsible>,
    pub tools: Option<Collapsible>,
    pub bubble: Option<TextBubble>,
    /// Absolute thumbnail rects, parallel to `messages[i].images`
    /// (truncated to whatever fits — see [`build_item`]).
    pub images: Vec<Rect>,
    pub streaming: bool,
}

fn rect_contains(r: Rect, x: f32, y: f32) -> bool {
    x >= r.origin.x && x <= r.origin.x + r.size.x && y >= r.origin.y && y <= r.origin.y + r.size.y
}

/// Wrap-unit budget for an inner text width.
fn unit_budget(inner_w: f32) -> u32 {
    (inner_w / CHAR_UNIT_PX).floor().max(1.0) as u32
}

fn text_unit_width(text: &str) -> f32 {
    text.chars().map(char_display_units).sum::<u32>() as f32 * CHAR_UNIT_PX
}

fn typing_dots_width() -> f32 {
    3.0 * TYPING_DOT + 2.0 * TYPING_DOT_GAP
}

fn typing_pill_width() -> f32 {
    2.0 * TYPING_PAD_X + text_unit_width(TYPING_LABEL) + TYPING_LABEL_DOT_GAP + typing_dots_width()
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

fn step_state(
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

/// Place one message starting at `top`. Returns the item and the
/// `y` immediately below it (before the inter-message gap).
fn build_item(
    msg: &ChatMessage,
    msg_index: usize,
    top: f32,
    body: Rect,
    locale: op_editor_core::Locale,
) -> (TranscriptItem, f32) {
    let is_user = msg.role == ChatRole::User;
    let bubble_w = if is_user {
        body.size.x * BUBBLE_FRAC
    } else {
        body.size.x
    };
    let x = if is_user {
        body.origin.x + body.size.x - bubble_w
    } else {
        body.origin.x
    };
    let budget = unit_budget(bubble_w - 2.0 * BUBBLE_PAD);
    let mut y = top;
    let (mut progress_steps, thinking_text) = split_design_progress(&msg.thinking);
    let visible_content = if is_user {
        msg.content.clone()
    } else {
        let display_content = strip_tool_call_xml(&msg.content);
        let extracted = extract_step_blocks(&display_content, msg.streaming);
        progress_steps.extend(extracted.steps);
        extracted.visible_text
    };

    let build_collapsible = |present: bool,
                             collapsed: bool,
                             label: String,
                             lines_fn: &dyn Fn() -> Vec<String>,
                             y: &mut f32|
     -> Option<Collapsible> {
        if !present {
            return None;
        }
        let header = Rect::xywh(x, *y, bubble_w, HEADER_H);
        *y += HEADER_H;
        let (body_rect, lines) = if collapsed {
            (Rect::xywh(x, *y, bubble_w, 0.0), Vec::new())
        } else {
            let lines = lines_fn();
            let h = lines.len() as f32 * LINE_H + 2.0 * BUBBLE_PAD;
            let r = Rect::xywh(x, *y, bubble_w, h);
            *y += h;
            (r, lines)
        };
        *y += SUB_GAP;
        Some(Collapsible {
            header,
            label,
            collapsed,
            body: body_rect,
            lines,
        })
    };

    let mut steps = Vec::new();
    let total_steps = progress_steps.len();
    for (i, step) in progress_steps.iter().enumerate() {
        let (done, active, failed) = step_state(step, msg.streaming, i, total_steps);
        let details: Vec<String> = step
            .details
            .iter()
            .flat_map(|line| wrap_units(line, budget.saturating_sub(4)))
            .collect();
        let step_h = action_step_height(details.len());
        steps.push(ActionStep {
            rect: Rect::xywh(x, y, bubble_w, step_h),
            label: step.title.clone(),
            details,
            done,
            active,
            failed,
        });
        y += step_h + ACTION_STEP_GAP;
    }

    let thinking = build_collapsible(
        !thinking_text.trim().is_empty(),
        msg.thinking_collapsed,
        op_i18n::translate(locale, "ai.thinkingProcess").to_string(),
        &|| wrap_units(&thinking_text, budget),
        &mut y,
    );
    let tools = build_collapsible(
        !msg.tool_calls.is_empty(),
        msg.tools_collapsed,
        op_i18n::translate(locale, "ai.toolCalls")
            .replace("{{count}}", &msg.tool_calls.len().to_string()),
        &|| {
            tool_lines(
                &msg.tool_calls,
                budget,
                if msg.streaming { "running" } else { "done" },
            )
        },
        &mut y,
    );

    let typing = msg.streaming
        && msg.content.is_empty()
        && steps.is_empty()
        && thinking.is_none()
        && tools.is_none();
    let bubble = if typing {
        let r = Rect::xywh(
            x,
            y,
            typing_pill_width().min(body.size.x),
            LINE_H + 2.0 * TYPING_PAD_Y,
        );
        y += r.size.y;
        Some(TextBubble {
            rect: r,
            lines: Vec::new(),
            typing: true,
        })
    } else if !visible_content.is_empty() {
        let lines = wrap_units(&visible_content, budget);
        let h = if is_user {
            lines.len() as f32 * LINE_H + 2.0 * BUBBLE_PAD
        } else {
            lines.len() as f32 * LINE_H
        };
        let r = Rect::xywh(x, y, bubble_w, h);
        y += h;
        Some(TextBubble {
            rect: r,
            lines,
            typing: false,
        })
    } else {
        None
    };

    let mut images = Vec::new();
    if !msg.images.is_empty() {
        if bubble.is_some() {
            y += SUB_GAP;
        }
        let per_row = (bubble_w / (IMG_THUMB + IMG_GAP)).floor().max(1.0) as usize;
        for i in 0..msg.images.len() {
            let col = i % per_row;
            let row = i / per_row;
            images.push(Rect::xywh(
                x + col as f32 * (IMG_THUMB + IMG_GAP),
                y + row as f32 * (IMG_THUMB + IMG_GAP),
                IMG_THUMB,
                IMG_THUMB,
            ));
        }
        let rows = msg.images.len().div_ceil(per_row);
        y += rows as f32 * (IMG_THUMB + IMG_GAP) - IMG_GAP;
    }

    (
        TranscriptItem {
            msg_index,
            role: msg.role,
            steps,
            thinking,
            tools,
            bubble,
            images,
            streaming: msg.streaming,
        },
        y,
    )
}

fn action_step_height(detail_count: usize) -> f32 {
    if detail_count == 0 {
        ACTION_STEP_H
    } else {
        ACTION_STEP_H + ACTION_DETAIL_GAP + detail_count as f32 * ACTION_DETAIL_LINE_H
    }
}

/// Lay out the tail of `messages` that fits inside `body_rect`,
/// top-aligned. Each item carries absolute rects ready for paint and
/// hit-test.
pub(crate) fn build_transcript(
    messages: &[ChatMessage],
    body_rect: Rect,
    locale: op_editor_core::Locale,
) -> Vec<TranscriptItem> {
    if messages.is_empty() {
        return Vec::new();
    }
    // Pass 1 — walk backward summing heights to find the first
    // message that still fits.
    let mut start = messages.len();
    let mut used = 0.0f32;
    for i in (0..messages.len()).rev() {
        let (_, bottom) = build_item(&messages[i], i, 0.0, body_rect, locale);
        let h = bottom
            + if start == messages.len() {
                0.0
            } else {
                MSG_GAP
            };
        if used + h > body_rect.size.y && start != messages.len() {
            break;
        }
        used += h;
        start = i;
    }
    // Pass 2 — place the visible tail from the body top.
    let mut items = Vec::new();
    let mut top = body_rect.origin.y;
    for (i, msg) in messages.iter().enumerate().skip(start) {
        let (item, bottom) = build_item(msg, i, top, body_rect, locale);
        items.push(item);
        top = bottom + MSG_GAP;
    }
    items
}

/// Resolve a click inside the transcript body to a [`TranscriptHit`]
/// — only the collapsible headers are interactive.
pub(crate) fn transcript_hit(
    messages: &[ChatMessage],
    body_rect: Rect,
    x: f32,
    y: f32,
    locale: op_editor_core::Locale,
) -> Option<TranscriptHit> {
    // Paint clips the transcript to `body_rect`; gate hit-test the
    // same way so a click in the body/input gap can't toggle a
    // header that an over-tall latest message placed off-screen.
    if !rect_contains(body_rect, x, y) {
        return None;
    }
    for item in build_transcript(messages, body_rect, locale) {
        if let Some(t) = &item.thinking {
            if rect_contains(t.header, x, y) {
                return Some(TranscriptHit::ToggleThinking(item.msg_index));
            }
        }
        if let Some(t) = &item.tools {
            if rect_contains(t.header, x, y) {
                return Some(TranscriptHit::ToggleToolCalls(item.msg_index));
            }
        }
    }
    None
}

/// Display width of one character, in wrap *units*. Wide scripts
/// (CJK, kana, Hangul, full-width punctuation, emoji) occupy two
/// units; everything else one. A deliberate estimate — the
/// transcript body is clipped, so a mild over/under-shoot is safe.
fn char_display_units(c: char) -> u32 {
    let cp = c as u32;
    let wide = matches!(cp,
        0x1100..=0x115F   // Hangul Jamo
        | 0x2E80..=0x303E // CJK radicals + symbols
        | 0x3041..=0x33FF // Hiragana / Katakana / CJK marks
        | 0x3400..=0x4DBF // CJK Ext-A
        | 0x4E00..=0x9FFF // CJK Unified
        | 0xA000..=0xA4CF // Yi
        | 0xAC00..=0xD7A3 // Hangul syllables
        | 0xF900..=0xFAFF // CJK compat ideographs
        | 0xFE30..=0xFE4F // CJK compat forms
        | 0xFF00..=0xFF60 // full-width forms
        | 0xFFE0..=0xFFE6
        | 0x1F000..=0x1FAFF // emoji + symbols
    );
    if wide {
        2
    } else {
        1
    }
}

/// Greedy word-wrap `text` to lines no wider than `max_units`.
/// Existing `\n` always breaks; a run that overflows breaks at the
/// last space when there is one, otherwise mid-glyph (so a single
/// long token can't escape the bubble). Empty input yields one
/// empty line so a blank message still occupies a row.
pub(crate) fn wrap_units(text: &str, max_units: u32) -> Vec<String> {
    let budget = max_units.max(1);
    let mut out = Vec::new();
    for raw in text.split('\n') {
        let mut line = String::new();
        let mut line_units = 0u32;
        // Byte offset of the last space seen in `line`, so an
        // overflow can rewind to a word boundary instead of
        // breaking mid-word.
        let mut last_space: Option<usize> = None;
        for c in raw.chars() {
            let u = char_display_units(c);
            if line_units + u > budget && !line.is_empty() {
                match last_space {
                    Some(idx) if idx > 0 => {
                        let rest: String = line[idx..].trim_start().to_string();
                        line.truncate(idx);
                        out.push(std::mem::take(&mut line));
                        line = rest;
                        line_units = line.chars().map(char_display_units).sum();
                    }
                    _ => {
                        out.push(std::mem::take(&mut line));
                        line_units = 0;
                    }
                }
                last_space = None;
            }
            if c == ' ' {
                last_space = Some(line.len());
            }
            line.push(c);
            line_units += u;
        }
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Draw one wrapped text line. Small shared helper so the bubble,
/// thinking body and tool list paint identically.
fn draw_line(
    cx: &mut PaintCx<'_>,
    text: &str,
    x: f32,
    baseline_y: f32,
    size: f32,
    color: crate::Color,
) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
}

fn paint_action_step(cx: &mut PaintCx<'_>, theme: &Theme, step: &ActionStep) {
    cx.backend.fill_round_rect(step.rect, 6.0, theme.muted);
    cx.backend
        .stroke_round_rect(step.rect, 6.0, theme.border, 1.0);
    let icon_box = Rect::xywh(
        step.rect.origin.x + 10.0,
        step.rect.origin.y + 7.0,
        14.0,
        14.0,
    );
    if step.failed {
        draw_icon(
            cx.backend,
            Icon::XCircle,
            Point2D::new(icon_box.origin.x, icon_box.origin.y),
            14.0,
            theme.destructive,
            1.7,
        );
    } else if step.done {
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(icon_box.origin.x, icon_box.origin.y),
            14.0,
            theme.primary,
            2.1,
        );
    } else {
        let color = if step.active {
            theme.primary
        } else {
            theme.muted_foreground
        };
        let r = if step.active { 4.0 } else { 3.0 };
        let center = Point2D::new(icon_box.origin.x + 7.0, icon_box.origin.y + 7.0);
        cx.backend.fill_oval(
            Rect::xywh(center.x - r, center.y - r, r * 2.0, r * 2.0),
            color,
        );
    }
    let label_color = if step.active {
        theme.foreground
    } else if step.failed {
        theme.destructive
    } else {
        theme.muted_foreground
    };
    draw_line(
        cx,
        &step.label,
        step.rect.origin.x + 32.0,
        step.rect.origin.y + 18.0,
        11.0,
        label_color,
    );
    if !step.details.is_empty() {
        cx.backend.save();
        cx.backend.clip_rect(step.rect);
        let mut baseline = step.rect.origin.y + ACTION_STEP_H + ACTION_DETAIL_GAP + 9.0;
        for line in &step.details {
            draw_line(
                cx,
                line,
                step.rect.origin.x + 32.0,
                baseline,
                10.0,
                theme.muted_foreground,
            );
            baseline += ACTION_DETAIL_LINE_H;
        }
        cx.backend.restore();
    }
}

/// Paint a collapsible block — header row (chevron + label) plus,
/// when expanded, a tinted body box with the wrapped lines.
fn paint_collapsible(cx: &mut PaintCx<'_>, theme: &Theme, block: &Collapsible) {
    cx.backend.fill_round_rect(block.header, 6.0, theme.muted);
    let icon = if block.collapsed {
        Icon::ChevronRight
    } else {
        Icon::ChevronDown
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            block.header.origin.x + 6.0,
            block.header.origin.y + (HEADER_H - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.5,
    );
    draw_line(
        cx,
        &block.label,
        block.header.origin.x + 26.0,
        block.header.origin.y + HEADER_H / 2.0 + 4.0,
        11.0,
        theme.muted_foreground,
    );
    if !block.collapsed && !block.lines.is_empty() {
        cx.backend
            .fill_round_rect(block.body, 6.0, theme.background);
        // Clip to the body box — a pathological unbroken token can't
        // bleed past the estimated wrap width.
        cx.backend.save();
        cx.backend.clip_rect(block.body);
        let mut baseline = block.body.origin.y + BUBBLE_PAD + 11.0;
        for line in &block.lines {
            draw_line(
                cx,
                line,
                block.body.origin.x + BUBBLE_PAD,
                baseline,
                BODY_FONT,
                theme.muted_foreground,
            );
            baseline += LINE_H;
        }
        cx.backend.restore();
    }
}

/// Paint the animated "assistant is typing" dots after the label.
fn paint_typing_dots(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    start_x: f32,
    center_y: f32,
    now_ms: u64,
) {
    let phase = (now_ms / 280) % 3;
    for i in 0..3u64 {
        let active = i == phase;
        let r = if active {
            TYPING_DOT * 0.6
        } else {
            TYPING_DOT * 0.5
        };
        let mut color = theme.muted_foreground;
        color.a *= if active { 0.9 } else { 0.7 };
        let cx_dot = start_x + r + i as f32 * (TYPING_DOT + TYPING_DOT_GAP);
        cx.backend.fill_oval(
            Rect::xywh(cx_dot - r, center_y - r, r * 2.0, r * 2.0),
            color,
        );
    }
}

/// Paint the chat transcript — the tail of `messages` that fits in
/// `body_rect`, with collapsible thinking / tool blocks, image
/// thumbnails and the streaming animation on the in-flight message.
pub(crate) fn paint_transcript(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    body_rect: Rect,
    messages: &[ChatMessage],
    now_ms: u64,
    locale: op_editor_core::Locale,
) {
    cx.backend.save();
    cx.backend.clip_rect(body_rect);
    for item in build_transcript(messages, body_rect, locale) {
        for step in &item.steps {
            paint_action_step(cx, theme, step);
        }
        if let Some(block) = &item.thinking {
            paint_collapsible(cx, theme, block);
        }
        if let Some(block) = &item.tools {
            paint_collapsible(cx, theme, block);
        }
        if let Some(bubble) = &item.bubble {
            let (bg, fg) = match item.role {
                ChatRole::User => (theme.primary, theme.primary_foreground),
                ChatRole::Assistant => (theme.muted, theme.foreground),
            };
            if bubble.typing {
                cx.backend
                    .fill_round_rect(bubble.rect, bubble.rect.size.y / 2.0, bg);
                let text_x = bubble.rect.origin.x + TYPING_PAD_X;
                let baseline = bubble.rect.origin.y + TYPING_PAD_Y + 11.0;
                draw_line(
                    cx,
                    TYPING_LABEL,
                    text_x,
                    baseline,
                    BODY_FONT,
                    theme.muted_foreground,
                );
                paint_typing_dots(
                    cx,
                    theme,
                    text_x + text_unit_width(TYPING_LABEL) + TYPING_LABEL_DOT_GAP,
                    bubble.rect.origin.y + bubble.rect.size.y / 2.0,
                    now_ms,
                );
            } else {
                // Clip to the bubble — over-long tokens stay inside.
                cx.backend.save();
                cx.backend.clip_rect(bubble.rect);
                if item.role == ChatRole::User {
                    cx.backend.fill_round_rect(bubble.rect, 8.0, bg);
                }
                let text_x = match item.role {
                    ChatRole::User => bubble.rect.origin.x + BUBBLE_PAD,
                    ChatRole::Assistant => bubble.rect.origin.x,
                };
                let mut baseline = bubble.rect.origin.y
                    + match item.role {
                        ChatRole::User => BUBBLE_PAD + 11.0,
                        ChatRole::Assistant => 11.0,
                    };
                for line in &bubble.lines {
                    draw_line(cx, line, text_x, baseline, BODY_FONT, fg);
                    baseline += LINE_H;
                }
                // Streaming caret — a blinking bar after the last
                // line's (estimated) end. Same unit metric as wrap.
                if item.streaming && jian_core::anim::blink_visible(now_ms, 0, 500) {
                    let last = bubble.lines.last().map(String::as_str).unwrap_or("");
                    let units: u32 = last.chars().map(char_display_units).sum();
                    let caret_x = text_x + units as f32 * CHAR_UNIT_PX;
                    let caret_y = baseline - LINE_H - 9.0;
                    cx.backend
                        .fill_rect(Rect::xywh(caret_x, caret_y, 2.0, 13.0), fg);
                }
                cx.backend.restore();
            }
        }
        // Image thumbnails — a framed box with the decoded image
        // drawn on top (a no-op draw on backends without an image
        // pipeline leaves just the frame).
        let msg_images = &messages[item.msg_index].images;
        for (rect, img) in item.images.iter().zip(msg_images.iter()) {
            cx.backend.fill_round_rect(*rect, 6.0, theme.muted);
            draw_icon(
                cx.backend,
                Icon::ImagePlus,
                Point2D::new(
                    rect.origin.x + rect.size.x / 2.0 - 11.0,
                    rect.origin.y + rect.size.y / 2.0 - 11.0,
                ),
                22.0,
                theme.muted_foreground,
                1.5,
            );
            cx.backend.draw_image(*rect, img.id, &img.data);
            cx.backend.stroke_round_rect(*rect, 6.0, theme.border, 1.0);
        }
    }
    cx.backend.restore();
}

#[cfg(test)]
#[path = "ai_chat_transcript_tests.rs"]
mod tests;
