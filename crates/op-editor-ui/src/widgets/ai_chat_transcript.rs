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

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::chat::{ChatMessage, ChatRole, ChatTranscriptSelection};

use super::ai_chat_transcript_cache::CanonicalTranscript;
use super::ai_chat_transcript_completion::{
    completion_card_rect, paint_completion_card, parse_completion_summary, CompletionSummary,
};
use super::ai_chat_transcript_design::{
    applied_design_block_label, extract_design_json_blocks, paint_design_block,
    place_design_blocks, DesignBlock,
};
pub(crate) use super::ai_chat_transcript_hit::{transcript_hit, TranscriptHit};
use super::ai_chat_transcript_identity::{
    layout_agent_identity, paint_agent_identity, AgentIdentityHeader,
};
use super::ai_chat_transcript_paint_parts::{paint_action_step, paint_collapsible};
use super::ai_chat_transcript_selection::paint_user_bubble_selection;
pub(crate) use super::ai_chat_transcript_selection::transcript_text_offset_at;
use super::ai_chat_transcript_steps::{
    extract_step_blocks, split_design_progress, strip_tool_call_xml, ParsedStep, ParsedStepStatus,
};
use super::ai_chat_transcript_text::char_display_units;
pub(crate) use super::ai_chat_transcript_text::wrap_units;
use super::ai_chat_transcript_tools::{
    build_tool_panel, paint_tool_panel, ToolPanel, ToolPanelLayout,
};

/// Body text size used throughout the transcript.
pub(crate) const BODY_FONT: f32 = 12.0;
/// Height of one wrapped text line.
pub(crate) const LINE_H: f32 = 16.0;
/// Inner padding inside a collapsible block box (thinking / tool body).
pub(crate) const BUBBLE_PAD: f32 = 8.0;
/// Inner padding for user message bubbles — matches the #27 generous
/// padding spec (~14px). Larger than BUBBLE_PAD so the user bubble
/// feels spacious without inflating assistant / block bodies.
pub(crate) const USER_BUBBLE_PAD: f32 = 14.0;
const TYPING_LABEL: &str = "Thinking";
const AUTOMATED_ACTION_LABEL: &str = "(Automated action completed)";
const TYPING_PAD_X: f32 = 10.0;
const TYPING_PAD_Y: f32 = 4.0;
const TYPING_LABEL_DOT_GAP: f32 = 6.0;
const TYPING_DOT: f32 = 4.0;
const TYPING_DOT_GAP: f32 = 2.0;
/// Height of a collapsible header row (thinking / tool-calls).
pub(crate) const HEADER_H: f32 = 22.0;
/// Vertical gap between two messages.
pub(crate) const MSG_GAP: f32 = 12.0;
/// Vertical gap between sub-blocks within one message.
const SUB_GAP: f32 = 4.0;
/// Height of one design-progress step row (#27 restyle: 48px for comfortable
/// vertical padding, matching the tool/step card height spec).
pub(crate) const ACTION_STEP_H: f32 = 48.0;
/// Height of one detail line under a progress step.
pub(crate) const ACTION_DETAIL_LINE_H: f32 = 14.0;
/// Gap between the progress title row and detail lines.
pub(crate) const ACTION_DETAIL_GAP: f32 = 4.0;
/// Vertical gap between compact design-progress rows.
const ACTION_STEP_GAP: f32 = 4.0;
/// Side length of an image thumbnail box.
const IMG_THUMB: f32 = 60.0;
/// Gap between image thumbnails.
const IMG_GAP: f32 = 4.0;
/// Approximate device px per wrap *unit* at [`BODY_FONT`]. Drives the
/// unit budget handed to [`wrap_units`].
pub(crate) const CHAR_UNIT_PX: f32 = 6.6;
/// Maximum user bubble width as a fraction of the transcript body width.
const USER_BUBBLE_MAX_FRAC: f32 = 0.78;
/// Minimum user bubble width so very short prompts still read as a chip.
const USER_BUBBLE_MIN_W: f32 = 56.0;

fn streaming_caret_visible(now_ms: u64) -> bool {
    jian_core::anim::blink_visible(now_ms, 0, jian_core::text_input::CARET_BLINK_PERIOD_MS)
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
    pub expanded: bool,
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
    pub completion: Option<CompletionSummary>,
}

/// One fully-placed message in the transcript — absolute rects ready
/// for paint, with the interactive headers exposed for hit-test.
pub(crate) struct TranscriptItem {
    pub msg_index: usize,
    pub role: ChatRole,
    pub agent_identity: Option<AgentIdentityHeader>,
    pub steps: Vec<ActionStep>,
    pub thinking: Option<Collapsible>,
    pub tools: Option<ToolPanel>,
    pub design_blocks: Vec<DesignBlock>,
    pub bubble: Option<TextBubble>,
    /// Absolute thumbnail rects, parallel to `messages[i].images`
    /// (truncated to whatever fits — see [`build_item`]).
    pub images: Vec<Rect>,
    pub streaming: bool,
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
pub(crate) fn build_item(
    msg: &ChatMessage,
    msg_index: usize,
    top: f32,
    body: Rect,
    locale: op_editor_core::Locale,
) -> (TranscriptItem, f32) {
    let is_user = msg.role == ChatRole::User;
    let mut bubble_w = if is_user {
        body.size.x * USER_BUBBLE_MAX_FRAC
    } else {
        body.size.x
    };
    let mut x = if is_user {
        body.origin.x + body.size.x - bubble_w
    } else {
        body.origin.x
    };
    let mut y = top;
    let (agent_identity, next_y) = layout_agent_identity(msg, x, y, bubble_w);
    y = next_y;
    let (mut progress_steps, thinking_text) = split_design_progress(&msg.thinking);
    let raw_visible_content = if is_user {
        msg.content.clone()
    } else {
        let display_content = strip_tool_call_xml(&msg.content);
        let extracted = extract_step_blocks(&display_content, msg.streaming);
        progress_steps.extend(extracted.steps);
        normalize_narration_markdown(&extracted.visible_text)
    };
    let design_applied =
        !is_user && (msg.content.contains("<!-- APPLIED -->") || msg.content.contains('\u{2705}'));
    let (visible_content, mut pending_design_blocks) = if is_user {
        (raw_visible_content, Vec::new())
    } else {
        let extracted = extract_design_json_blocks(&raw_visible_content, msg.streaming);
        // Suppress the in-chat design card WHILE STREAMING — no transient
        // "Generating design..." card (the "Pencil it out" checklist + the
        // live canvas already convey progress). Completed blocks still render.
        let blocks = if msg.streaming {
            Vec::new()
        } else {
            extracted.blocks
        };
        (extracted.visible_text, blocks)
    };
    if design_applied {
        for block in &mut pending_design_blocks {
            block.applied = true;
            block.label = applied_design_block_label(locale, block.element_count);
        }
    }
    // No streaming "Generating design..." placeholder card — the fixed
    // "Pencil it out" checklist already shows live progress, so this
    // duplicate card was removed (user directive 2026-06-22).
    let mut user_bubble_lines = if is_user && !visible_content.is_empty() {
        let max_w = body.size.x * USER_BUBBLE_MAX_FRAC;
        let max_budget = unit_budget(max_w - 2.0 * USER_BUBBLE_PAD);
        let lines = wrap_units(&visible_content, max_budget);
        let content_w = lines
            .iter()
            .map(|line| text_unit_width(line))
            .fold(0.0, f32::max);
        bubble_w = (content_w + 2.0 * USER_BUBBLE_PAD)
            .max(USER_BUBBLE_MIN_W)
            .min(max_w);
        x = body.origin.x + body.size.x - bubble_w;
        Some(lines)
    } else {
        None
    };
    // budget for word-wrapping inside the bubble (user uses USER_BUBBLE_PAD,
    // assistant uses full width with no inset).
    let budget = if is_user {
        unit_budget(bubble_w - 2.0 * USER_BUBBLE_PAD)
    } else {
        unit_budget(bubble_w - 2.0 * BUBBLE_PAD)
    };
    let has_progress_steps = !progress_steps.is_empty();
    let completion_summary = (!is_user)
        .then(|| parse_completion_summary(&visible_content))
        .flatten();

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
    let inline_steps: Vec<&ParsedStep> = progress_steps
        .iter()
        .filter(|step| !step.details.is_empty())
        .collect();
    let total_steps = inline_steps.len();
    for (i, step) in inline_steps.iter().copied().enumerate() {
        let (done, active, failed) = step_state(step, msg.streaming, i, total_steps);
        let details: Vec<String> = step
            .details
            .iter()
            .flat_map(|line| wrap_units(line, budget.saturating_sub(4)))
            .collect();
        // Default: expanded only while this step is the active/streaming
        // one. A user click records a per-step override (collapse/expand).
        let expanded = msg
            .action_step_expanded_overrides
            .get(i)
            .copied()
            .flatten()
            .unwrap_or(active);
        let step_h = action_step_height(expanded, details.len());
        steps.push(ActionStep {
            rect: Rect::xywh(x, y, bubble_w, step_h),
            label: step.title.clone(),
            details,
            expanded,
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
    let (tools, next_y) = build_tool_panel(
        &msg.tool_calls,
        ToolPanelLayout {
            collapsed: msg.tools_collapsed,
            label: op_i18n::translate(locale, "ai.toolCalls")
                .replace("{{count}}", &msg.tool_calls.len().to_string()),
            x,
            y,
            width: bubble_w,
            budget,
            default_status: if msg.streaming { "running" } else { "done" },
            expanded_overrides: &msg.tool_call_expanded_overrides,
        },
    );
    y = next_y;
    let (design_blocks, next_y) = place_design_blocks(
        pending_design_blocks,
        x,
        y,
        bubble_w,
        SUB_GAP,
        &msg.design_block_expanded_overrides,
    );
    y = next_y;

    let typing = msg.streaming
        && msg.content.is_empty()
        && steps.is_empty()
        && !has_progress_steps
        && thinking.is_none()
        && tools.is_none();
    let automated_placeholder = !is_user
        && !msg.streaming
        && !msg.content.trim().is_empty()
        && visible_content.is_empty()
        && steps.is_empty()
        && !has_progress_steps
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
            completion: None,
        })
    } else if automated_placeholder {
        let lines = vec![AUTOMATED_ACTION_LABEL.to_string()];
        let r = Rect::xywh(x, y, bubble_w, LINE_H);
        y += r.size.y;
        Some(TextBubble {
            rect: r,
            lines,
            typing: false,
            completion: None,
        })
    } else if let Some(summary) = completion_summary {
        let r = completion_card_rect(x, y, bubble_w);
        y += r.size.y;
        Some(TextBubble {
            rect: r,
            lines: Vec::new(),
            typing: false,
            completion: Some(summary),
        })
    } else if !visible_content.is_empty() {
        let lines = user_bubble_lines
            .take()
            .unwrap_or_else(|| wrap_units(&visible_content, budget));
        let h = if is_user {
            // #27 restyle: generous 14px padding for the user bubble.
            lines.len() as f32 * LINE_H + 2.0 * USER_BUBBLE_PAD
        } else {
            lines.len() as f32 * LINE_H
        };
        let r = Rect::xywh(x, y, bubble_w, h);
        y += h;
        Some(TextBubble {
            rect: r,
            lines,
            typing: false,
            completion: None,
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
            agent_identity,
            steps,
            thinking,
            tools,
            design_blocks,
            bubble,
            images,
            streaming: msg.streaming,
        },
        y,
    )
}

fn action_step_height(expanded: bool, detail_count: usize) -> f32 {
    if !expanded || detail_count == 0 {
        ACTION_STEP_H
    } else {
        ACTION_STEP_H + ACTION_DETAIL_GAP + detail_count as f32 * ACTION_DETAIL_LINE_H
    }
}

/// Light markdown normalization for streamed narration — the panel paints
/// plain text, so raw `**bold**` markers and back-to-back bold headings
/// ("**Batch 1**" glued straight onto "**Batch 2**") rendered as asterisk
/// soup (measured 2026-07-12). Full markdown is out of scope; this strips
/// emphasis/backtick markers, re-breaks adjacent bold headings onto their
/// own lines, and turns `- ` bullets into `• `.
pub(crate) fn normalize_narration_markdown(text: &str) -> String {
    // Adjacent closing/opening bold with nothing between = two headings the
    // stream glued together; give the second its own paragraph.
    let mut out = text.replace("****", "**\n**");
    // A bold opener directly after a colon or period also reads as a new
    // heading in the measured streams.
    out = out.replace(":**", ":\n**");
    // Strip the emphasis/code markers themselves.
    out = out.replace("**", "").replace('`', "");
    // Bullets.
    let mut lines: Vec<String> = Vec::new();
    for line in out.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("- ") {
            let indent = &line[..line.len() - trimmed.len()];
            lines.push(format!("{indent}\u{2022} {rest}"));
        } else {
            lines.push(line.to_string());
        }
    }
    lines.join(
        "
",
    )
}

/// Total height (px) of the full transcript laid out from the body top.
/// Served from the shared per-frame cache — no separate layout pass. The
/// production max-scroll clamp (`AIChatPlaceholder::transcript_scroll_max`) now
/// resolves owner-scoped directly for the same value, so this UNOWNED helper is
/// retained only for the transcript layout tests.
#[cfg(test)]
pub(crate) fn transcript_content_height(
    messages: &[ChatMessage],
    body_rect: Rect,
    locale: op_editor_core::Locale,
) -> f32 {
    super::ai_chat_transcript_cache::unowned_for_tests(messages, body_rect, locale).total_height
}

/// Effective scroll offset to render at, given an already-resolved canonical
/// build: the pinned-to-bottom maximum while `pinned`, otherwise the stored
/// `offset` clamped to the scrollable range. Taking the resolved build (rather
/// than `messages`) is what lets the paint / hit entry points fingerprint the
/// transcript once and reuse it for both the scroll clamp and the layout.
pub(crate) fn effective_offset_of(
    canonical: &CanonicalTranscript,
    body_rect: Rect,
    offset: f32,
    pinned: bool,
) -> f32 {
    let max = (canonical.total_height - body_rect.size.y).max(0.0);
    if pinned {
        max
    } else {
        offset.clamp(0.0, max)
    }
}

/// Draw one wrapped text line. Small shared helper so the bubble,
/// thinking body and tool list paint identically.
pub(crate) fn draw_line(
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
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
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

/// Paint the transcript from an already-resolved canonical (scroll-0) build.
/// The caller fingerprints the transcript once (via
/// [`unowned_for_tests`]) and threads the build here, so paint never
/// re-hashes; scroll is applied with a `translate`, not a rebuild. `messages`
/// is still needed for the live text / image content the layout indexes into.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_transcript_with_selection(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    body_rect: Rect,
    messages: &[ChatMessage],
    canonical: &CanonicalTranscript,
    now_ms: u64,
    design_hover: Option<(usize, usize)>,
    selection: Option<ChatTranscriptSelection>,
    scroll_offset: f32,
) {
    cx.backend.save();
    // Clip in screen space, THEN shift by the scroll so the shared canonical
    // (scroll-0) build paints at the right place — no scroll-applied rebuild.
    cx.backend.clip_rect(body_rect);
    if scroll_offset != 0.0 {
        cx.backend.translate(Point2D::new(0.0, -scroll_offset));
    }
    for item in &canonical.items {
        // Ours: sub-agent identity header (name + colour chip) above the
        // item's steps — the canonical build lays it out, paint replays it.
        if let Some(identity) = &item.agent_identity {
            paint_agent_identity(cx, theme, identity);
        }
        for step in &item.steps {
            paint_action_step(cx, theme, step);
        }
        if let Some(block) = &item.thinking {
            paint_collapsible(cx, theme, block);
        }
        if let Some(block) = &item.tools {
            paint_tool_panel(cx, theme, block);
        }
        for (block_index, block) in item.design_blocks.iter().enumerate() {
            // Design-hover is paint-only: it reveals the per-block copy icon and
            // never moves a rect, so it stays out of the layout cache and is
            // resolved here against the live hover instead.
            let copy_visible = design_hover == Some((item.msg_index, block_index));
            paint_design_block(cx, theme, block, copy_visible);
        }
        if let Some(bubble) = &item.bubble {
            // #27 restyle: user bubble = medium-gray (theme.user_bubble),
            // assistant text = plain (no bubble background).
            let (bg, fg) = match item.role {
                ChatRole::User => (theme.user_bubble, theme.user_bubble_foreground),
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
            } else if let Some(summary) = bubble.completion {
                paint_completion_card(cx, theme, bubble.rect, summary);
            } else {
                // Clip to the bubble — over-long tokens stay inside.
                cx.backend.save();
                cx.backend.clip_rect(bubble.rect);
                if item.role == ChatRole::User {
                    // #27 restyle: rounded-rect bubble with ~14px radius.
                    cx.backend.fill_round_rect(bubble.rect, 14.0, bg);
                }
                if item.role == ChatRole::User {
                    // Bounds-defense: index the live slice with `.get` — the
                    // cached build can outlive a shrink of `messages`.
                    if let (Some(selection), Some(message)) = (
                        selection.filter(|selection| selection.message_index == item.msg_index),
                        messages.get(item.msg_index),
                    ) {
                        paint_user_bubble_selection(cx, theme, &message.content, bubble, selection);
                    }
                }
                // #27 restyle: user bubble text uses generous USER_BUBBLE_PAD inset.
                let text_x = match item.role {
                    ChatRole::User => bubble.rect.origin.x + USER_BUBBLE_PAD,
                    ChatRole::Assistant => bubble.rect.origin.x,
                };
                let mut baseline = bubble.rect.origin.y
                    + match item.role {
                        ChatRole::User => USER_BUBBLE_PAD + 11.0,
                        ChatRole::Assistant => 11.0,
                    };
                for line in &bubble.lines {
                    draw_line(cx, line, text_x, baseline, BODY_FONT, fg);
                    baseline += LINE_H;
                }
                // Streaming caret — a blinking bar after the last
                // line's (estimated) end. Same unit metric as wrap.
                if item.streaming && streaming_caret_visible(now_ms) {
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
        // Bounds-defense: a cached build item may reference a message index past
        // the end of a shrunken live slice; fall back to no thumbnails rather
        // than panicking on an out-of-bounds index.
        let msg_images = messages
            .get(item.msg_index)
            .map(|m| m.images.as_slice())
            .unwrap_or(&[]);
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

/// Test-only builders / painters, split into a sibling file to keep this
/// production module under the 800-line cap. Re-exported so existing
/// `build_transcript` / `paint_transcript` / `transcript_effective_offset`
/// call sites resolve through `ai_chat_transcript` unchanged.
#[cfg(test)]
#[path = "ai_chat_transcript_test_builders.rs"]
mod test_builders;
#[cfg(test)]
pub(crate) use test_builders::{
    build_transcript, build_transcript_with_design_hover, paint_transcript,
    paint_transcript_with_design_hover, transcript_effective_offset,
};

#[cfg(test)]
#[path = "ai_chat_transcript_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "ai_chat_transcript_scroll_tests.rs"]
mod scroll_tests;

#[cfg(test)]
#[path = "ai_chat_transcript_cache_tests.rs"]
mod cache_tests;

#[cfg(test)]
#[path = "ai_chat_transcript_copy_tests.rs"]
mod copy_tests;

#[cfg(test)]
#[path = "ai_chat_transcript_progress_tests.rs"]
mod progress_tests;
