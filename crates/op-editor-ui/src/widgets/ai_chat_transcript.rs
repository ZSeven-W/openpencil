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
use op_editor_core::chat::{ChatMessage, ChatRole, ChatToolCall};

/// Body text size used throughout the transcript.
const BODY_FONT: f32 = 12.0;
/// Height of one wrapped text line.
const LINE_H: f32 = 16.0;
/// Inner padding inside a bubble / block box.
const BUBBLE_PAD: f32 = 8.0;
/// Height of a collapsible header row (thinking / tool-calls).
const HEADER_H: f32 = 22.0;
/// Vertical gap between two messages.
const MSG_GAP: f32 = 10.0;
/// Vertical gap between sub-blocks within one message.
const SUB_GAP: f32 = 4.0;
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

/// Format the tool-call list into display lines: one `→ name` line
/// per call, followed by the call's args wrapped across as many
/// lines as needed. The expanded panel shows the full arguments,
/// capped at [`MAX_ARG_LINES`] per call so one giant JSON blob can't
/// flood the narrow panel.
fn tool_lines(calls: &[ChatToolCall], budget: u32) -> Vec<String> {
    /// Per-call ceiling on wrapped argument lines.
    const MAX_ARG_LINES: usize = 8;
    let mut lines = Vec::new();
    for c in calls {
        lines.push(format!("→ {}", c.name));
        let compact: String = c.args.split_whitespace().collect::<Vec<_>>().join(" ");
        if !compact.is_empty() {
            let wrapped = wrap_units(&compact, budget.saturating_sub(2).max(1));
            let shown = wrapped.len().min(MAX_ARG_LINES);
            for line in &wrapped[..shown] {
                lines.push(format!("  {line}"));
            }
            if wrapped.len() > shown {
                lines.push("  …".to_string());
            }
        }
    }
    lines
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
    let bubble_w = body.size.x * BUBBLE_FRAC;
    let x = if is_user {
        body.origin.x + body.size.x - bubble_w
    } else {
        body.origin.x
    };
    let budget = unit_budget(bubble_w - 2.0 * BUBBLE_PAD);
    let mut y = top;

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

    let thinking = build_collapsible(
        !msg.thinking.is_empty(),
        msg.thinking_collapsed,
        op_i18n::translate(locale, "ai.thinkingProcess").to_string(),
        &|| wrap_units(&msg.thinking, budget),
        &mut y,
    );
    let tools = build_collapsible(
        !msg.tool_calls.is_empty(),
        msg.tools_collapsed,
        op_i18n::translate(locale, "ai.toolCalls")
            .replace("{{count}}", &msg.tool_calls.len().to_string()),
        &|| tool_lines(&msg.tool_calls, budget),
        &mut y,
    );

    let typing = msg.streaming && msg.content.is_empty();
    let bubble = if typing {
        let r = Rect::xywh(x, y, bubble_w, LINE_H + 2.0 * BUBBLE_PAD);
        y += r.size.y;
        Some(TextBubble {
            rect: r,
            lines: Vec::new(),
            typing: true,
        })
    } else if !msg.content.is_empty() {
        let lines = wrap_units(&msg.content, budget);
        let h = lines.len() as f32 * LINE_H + 2.0 * BUBBLE_PAD;
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
            thinking,
            tools,
            bubble,
            images,
            streaming: msg.streaming,
        },
        y,
    )
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
    for i in start..messages.len() {
        let (item, bottom) = build_item(&messages[i], i, top, body_rect, locale);
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

/// Paint the animated "assistant is typing" dots inside `rect`.
fn paint_typing_dots(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, now_ms: u64) {
    let phase = (now_ms / 280) % 3;
    let cy = rect.origin.y + rect.size.y / 2.0;
    for i in 0..3u64 {
        let active = i == phase;
        let r = if active { 3.5 } else { 2.5 };
        let color = if active {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        let cx_dot = rect.origin.x + BUBBLE_PAD + 6.0 + i as f32 * 10.0;
        cx.backend
            .fill_oval(Rect::xywh(cx_dot - r, cy - r, r * 2.0, r * 2.0), color);
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
            cx.backend.fill_round_rect(bubble.rect, 8.0, bg);
            if bubble.typing {
                paint_typing_dots(cx, theme, bubble.rect, now_ms);
            } else {
                // Clip to the bubble — over-long tokens stay inside.
                cx.backend.save();
                cx.backend.clip_rect(bubble.rect);
                let mut baseline = bubble.rect.origin.y + BUBBLE_PAD + 11.0;
                for line in &bubble.lines {
                    draw_line(
                        cx,
                        line,
                        bubble.rect.origin.x + BUBBLE_PAD,
                        baseline,
                        BODY_FONT,
                        fg,
                    );
                    baseline += LINE_H;
                }
                // Streaming caret — a blinking bar after the last
                // line's (estimated) end. Same unit metric as wrap.
                if item.streaming && jian_core::anim::blink_visible(now_ms, 0, 500) {
                    let last = bubble.lines.last().map(String::as_str).unwrap_or("");
                    let units: u32 = last.chars().map(char_display_units).sum();
                    let caret_x = bubble.rect.origin.x + BUBBLE_PAD + units as f32 * CHAR_UNIT_PX;
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
mod tests {
    use super::*;

    #[test]
    fn wrap_units_breaks_ascii_at_word_boundaries() {
        // Budget 10 units — "hello world" (11) must split after the
        // space, not mid-word.
        let lines = wrap_units("hello world", 10);
        assert_eq!(lines, vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn wrap_units_preserves_explicit_newlines() {
        let lines = wrap_units("a\nb", 80);
        assert_eq!(lines, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn wrap_units_counts_cjk_as_two_units_each() {
        // Five CJK glyphs = 10 units. Budget 6 fits 3 per line.
        let lines = wrap_units("设计登录页", 6);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].chars().count(), 3);
        assert_eq!(lines[1].chars().count(), 2);
    }

    #[test]
    fn wrap_units_hard_breaks_a_token_with_no_spaces() {
        // No space to rewind to — a long token still gets chopped so
        // it cannot overflow the bubble.
        let lines = wrap_units("aaaaaaaa", 3);
        assert!(lines.len() >= 3);
        assert!(lines.iter().all(|l| l.chars().count() <= 3));
    }

    #[test]
    fn wrap_units_empty_text_yields_one_empty_line() {
        assert_eq!(wrap_units("", 40), vec![String::new()]);
    }

    fn body() -> Rect {
        Rect::xywh(0.0, 0.0, 340.0, 300.0)
    }

    #[test]
    fn build_transcript_empty_messages_is_empty() {
        assert!(build_transcript(&[], body(), op_editor_core::Locale::EnUs).is_empty());
    }

    #[test]
    fn streaming_message_with_no_text_yields_a_typing_bubble() {
        let msgs = vec![ChatMessage::assistant_streaming()];
        let items = build_transcript(&msgs, body(), op_editor_core::Locale::EnUs);
        assert_eq!(items.len(), 1);
        assert!(items[0].streaming);
        let bubble = items[0].bubble.as_ref().expect("typing bubble present");
        assert!(bubble.typing, "empty in-flight message shows typing dots");
        assert!(bubble.lines.is_empty());
    }

    #[test]
    fn assistant_thinking_collapsed_has_header_but_no_body_lines() {
        let mut m = ChatMessage::assistant("the answer");
        m.thinking = "a long private chain of reasoning".into();
        // Default: thinking_collapsed == true.
        let items = build_transcript(std::slice::from_ref(&m), body(), op_editor_core::Locale::EnUs);
        let t = items[0].thinking.as_ref().expect("thinking block present");
        assert!(t.collapsed);
        assert!(t.lines.is_empty(), "collapsed body carries no lines");
        assert!(t.header.size.y > 0.0, "header is still clickable");
        assert!((t.body.size.y - 0.0).abs() < 1e-4, "collapsed body is flat");
    }

    #[test]
    fn assistant_thinking_expanded_has_wrapped_body_lines() {
        let mut m = ChatMessage::assistant("the answer");
        m.thinking = "a long private chain of reasoning that must wrap \
                      across several lines inside the narrow panel"
            .into();
        m.thinking_collapsed = false;
        let items = build_transcript(std::slice::from_ref(&m), body(), op_editor_core::Locale::EnUs);
        let t = items[0].thinking.as_ref().unwrap();
        assert!(!t.collapsed);
        assert!(t.lines.len() > 1, "long reasoning wraps to many lines");
        assert!(t.body.size.y > 0.0);
    }

    #[test]
    fn tool_calls_block_header_label_counts_the_calls() {
        let mut m = ChatMessage::assistant("done");
        m.tool_calls = vec![
            ChatToolCall {
                name: "insert_node".into(),
                args: "{}".into(),
            },
            ChatToolCall {
                name: "set_fill_hex".into(),
                args: "{}".into(),
            },
        ];
        let items =
            build_transcript(std::slice::from_ref(&m), body(), op_editor_core::Locale::EnUs);
        let t = items[0].tools.as_ref().expect("tools block present");
        // The header label substitutes the call count into the
        // `ai.toolCalls` template's `{{count}}` placeholder.
        let expected = op_i18n::translate(op_editor_core::Locale::EnUs, "ai.toolCalls")
            .replace("{{count}}", "2");
        assert_eq!(t.label, expected, "header label counts the calls");
    }

    #[test]
    fn user_message_images_get_one_thumbnail_rect_each() {
        let mut m = ChatMessage::user("look");
        for i in 0..3 {
            m.images.push(op_editor_core::ChatImage {
                id: i,
                name: format!("{i}.png"),
                media_type: "image/png".into(),
                data: vec![1],
            });
        }
        let items = build_transcript(std::slice::from_ref(&m), body(), op_editor_core::Locale::EnUs);
        assert_eq!(items[0].images.len(), 3, "one thumbnail rect per image");
        // Thumbnails do not overlap.
        let (a, b) = (items[0].images[0], items[0].images[1]);
        assert!(a.origin.x != b.origin.x || a.origin.y != b.origin.y);
    }

    #[test]
    fn transcript_hit_resolves_a_click_on_the_thinking_header() {
        let mut m = ChatMessage::assistant("answer");
        m.thinking = "reasoning".into();
        let msgs = std::slice::from_ref(&m);
        let header = build_transcript(msgs, body(), op_editor_core::Locale::EnUs)[0]
            .thinking
            .as_ref()
            .unwrap()
            .header;
        let cx = header.origin.x + header.size.x / 2.0;
        let cy = header.origin.y + header.size.y / 2.0;
        assert_eq!(
            transcript_hit(msgs, body(), cx, cy, op_editor_core::Locale::EnUs),
            Some(TranscriptHit::ToggleThinking(0))
        );
    }

    #[test]
    fn transcript_hit_resolves_a_click_on_the_tool_header() {
        let mut m = ChatMessage::assistant("answer");
        m.tool_calls = vec![ChatToolCall {
            name: "insert_node".into(),
            args: "{}".into(),
        }];
        let msgs = std::slice::from_ref(&m);
        let header = build_transcript(msgs, body(), op_editor_core::Locale::EnUs)[0]
            .tools
            .as_ref()
            .unwrap()
            .header;
        let cx = header.origin.x + header.size.x / 2.0;
        let cy = header.origin.y + header.size.y / 2.0;
        assert_eq!(
            transcript_hit(msgs, body(), cx, cy, op_editor_core::Locale::EnUs),
            Some(TranscriptHit::ToggleToolCalls(0))
        );
    }

    #[test]
    fn transcript_hit_misses_when_the_click_is_not_on_a_header() {
        let m = ChatMessage::assistant("plain answer, no thinking, no tools");
        let msgs = std::slice::from_ref(&m);
        // Click far below the single short message.
        assert_eq!(transcript_hit(msgs, body(), 20.0, 280.0, op_editor_core::Locale::EnUs), None);
    }
}
