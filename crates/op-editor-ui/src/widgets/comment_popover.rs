//! Comment popover — the card that reads, answers and resolves a thread.
//!
//! It opens next to a pin (or a draft's anchor) and shows the message
//! history, a single-line input, and the lifecycle buttons for the
//! thread's current status. Its height follows the wrapped message text,
//! which only a paint pass can measure, so [`paint_comment_popover`]
//! records the card and control rects on `CommentUiState` and the press
//! path resolves against those (`CommentUiState::popover_hit`).
//!
//! Hosts paint it at the bottom of the top-most overlay band — above
//! every panel, below notices and modals — and route presses to it at the
//! matching place in the press ladder.

use op_editor_core::comment_ui::{CommentPopover, CommentPopoverHit};
use op_editor_core::comments::{CommentAuthorKind, CommentStatus};
use op_editor_core::EditorState;
use op_i18n::{translate, Locale};

use super::canvas_viewport_overlay::{wrap_text_in_family, WrapFont};
use crate::layout_scene::LayoutScene;
use crate::theme::Theme;
use crate::widgets::comment_pins::{open_popover_tip, PIN_SIZE};
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::host_canvas_geometry::canvas_rect;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{text_metrics, PaintCx};
use crate::{Color, Point2D, Rect, TextLayout};

const WIDTH: f32 = 300.0;
const PAD: f32 = 12.0;
const HEADER_H: f32 = 32.0;
const FONT: f32 = 12.0;
const LINE_H: f32 = 17.0;
const AUTHOR_H: f32 = 18.0;
const MESSAGE_GAP: f32 = 8.0;
const MAX_MESSAGES_H: f32 = 260.0;
const INPUT_H: f32 = 32.0;
const ACTIONS_H: f32 = 28.0;
const BUTTON_PAD_X: f32 = 10.0;
const ICON_HIT: f32 = 24.0;
const RADIUS: f32 = 10.0;
const GAP: f32 = 8.0;

/// One message ready to paint: author label and wrapped lines.
struct PaintedMessage {
    author: String,
    agent: bool,
    lines: Vec<String>,
}

/// Paint the open popover (if any) and cache its hit regions on
/// `state.editor_ui.comments`. A thread that no longer exists (deleted,
/// or undone away) closes the popover instead.
pub fn paint_comment_popover(
    cx: &mut PaintCx<'_>,
    state: &mut EditorState,
    scene: &LayoutScene,
    viewport_width: f32,
    viewport_height: f32,
) {
    let comments = &mut state.editor_ui.comments;
    comments.popover_rect = None;
    comments.popover_hits.clear();
    let Some(open) = comments.open.clone() else {
        return;
    };
    let thread = match &open {
        CommentPopover::Thread { id } => match state.comment(id) {
            Some(thread) => Some(thread.clone()),
            None => {
                op_editor_core::comment_ui::close_comment_popover(state);
                return;
            }
        },
        CommentPopover::Draft { .. } => None,
    };
    let Some(tip) = open_popover_tip(state, scene, viewport_width, viewport_height) else {
        return;
    };
    let theme = theme_for(&state.editor_ui);
    let locale = state.editor_ui.effective_locale();
    let canvas = canvas_rect(state, viewport_width, viewport_height);

    let text_w = WIDTH - PAD * 2.0;
    let messages: Vec<PaintedMessage> = thread
        .as_ref()
        .map(|t| t.messages.as_slice())
        .unwrap_or(&[])
        .iter()
        .map(|m| PaintedMessage {
            author: author_label(locale, &m.author.name, m.author.kind),
            agent: m.author.kind == CommentAuthorKind::Agent,
            lines: wrap_text_in_family(
                cx.backend,
                &m.text,
                &WrapFont {
                    font_size: FONT,
                    weight: 400,
                    letter_spacing: 0.0,
                    family: "system-ui",
                    italic: false,
                },
                text_w,
            ),
        })
        .collect();
    let full_messages_h: f32 = messages
        .iter()
        .map(|m| AUTHOR_H + m.lines.len() as f32 * LINE_H + MESSAGE_GAP)
        .sum();
    let messages_h = full_messages_h.min(MAX_MESSAGES_H);
    let height = HEADER_H + messages_h + INPUT_H + GAP + ACTIONS_H + PAD;

    let rect = place(canvas, tip, height);
    cx.backend.fill_drop_shadow(
        rect,
        RADIUS,
        12.0,
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.18,
        },
    );
    cx.backend.fill_round_rect(rect, RADIUS, theme.popover);
    cx.backend
        .stroke_round_rect(rect, RADIUS, theme.border, 1.0);
    let mut hits: Vec<(CommentPopoverHit, Rect)> = Vec::new();

    // Header: title + status, then delete / close icons.
    let header = Rect::xywh(rect.origin.x, rect.origin.y, WIDTH, HEADER_H);
    let title = match &thread {
        Some(t) => format!(
            "{} · {}",
            translate(locale, "comments.title"),
            status_label(locale, t.status)
        ),
        None => translate(locale, "comments.title").to_owned(),
    };
    draw_label(
        cx,
        &title,
        Point2D::new(
            header.origin.x + PAD,
            jian_widgets::centered_text_baseline_y(header, FONT),
        ),
        theme.popover_foreground,
        600,
    );
    let close = icon_rect(header, 0);
    paint_icon_button(
        cx,
        &theme,
        close,
        Icon::Close,
        state_hover(state, &CommentPopoverHit::Close),
    );
    hits.push((CommentPopoverHit::Close, close));
    if thread.is_some() {
        let delete = icon_rect(header, 1);
        paint_icon_button(
            cx,
            &theme,
            delete,
            Icon::Trash,
            state_hover(state, &CommentPopoverHit::Delete),
        );
        hits.push((CommentPopoverHit::Delete, delete));
    }

    // Messages, newest at the bottom; older ones scroll off the top.
    let messages_rect = Rect::xywh(
        rect.origin.x + PAD,
        rect.origin.y + HEADER_H,
        text_w,
        messages_h,
    );
    if !messages.is_empty() {
        cx.backend.save();
        cx.backend.clip_rect(messages_rect);
        let mut y = messages_rect.origin.y + messages_h - full_messages_h;
        for m in &messages {
            let author_color = if m.agent {
                theme.primary
            } else {
                theme.popover_foreground
            };
            draw_label(
                cx,
                &m.author,
                Point2D::new(messages_rect.origin.x, y + 13.0),
                author_color,
                600,
            );
            y += AUTHOR_H;
            for line in &m.lines {
                draw_label(
                    cx,
                    line,
                    Point2D::new(messages_rect.origin.x, y + 12.5),
                    theme.popover_foreground,
                    400,
                );
                y += LINE_H;
            }
            y += MESSAGE_GAP;
        }
        cx.backend.restore();
    }

    // Input.
    let input = Rect::xywh(
        rect.origin.x + PAD,
        messages_rect.origin.y + messages_h,
        text_w,
        INPUT_H,
    );
    paint_input(cx, &theme, state, locale, input, thread.is_some());
    hits.push((CommentPopoverHit::Input, input));

    // Actions: lifecycle buttons on the left, Post / Reply on the right.
    let row = Rect::xywh(
        rect.origin.x + PAD,
        input.origin.y + INPUT_H + GAP,
        text_w,
        ACTIONS_H,
    );
    let submit_key = if thread.is_some() {
        "comments.reply"
    } else {
        "comments.post"
    };
    let submit_label = translate(locale, submit_key);
    let submit_w = button_width(cx, submit_label);
    let submit = Rect::xywh(
        row.origin.x + row.size.x - submit_w,
        row.origin.y,
        submit_w,
        ACTIONS_H,
    );
    let has_text = !state.editor_ui.comments.input.text().trim().is_empty();
    paint_button(
        cx,
        &theme,
        submit,
        submit_label,
        true,
        has_text,
        state_hover(state, &CommentPopoverHit::Submit),
    );
    hits.push((CommentPopoverHit::Submit, submit));
    if let Some(t) = &thread {
        let mut x = row.origin.x;
        for (status, key) in status_actions(t.status) {
            let label = translate(locale, key);
            let w = button_width(cx, label);
            if x + w > submit.origin.x - GAP {
                break;
            }
            let button = Rect::xywh(x, row.origin.y, w, ACTIONS_H);
            let hit = CommentPopoverHit::SetStatus(*status);
            paint_button(
                cx,
                &theme,
                button,
                label,
                false,
                true,
                state_hover(state, &hit),
            );
            hits.push((hit, button));
            x += w + 6.0;
        }
    }

    let comments = &mut state.editor_ui.comments;
    comments.popover_rect = Some(to_array(rect));
    comments.popover_hits = hits.into_iter().map(|(h, r)| (h, to_array(r))).collect();
}

/// Buttons offered for a status, in display order.
pub fn status_actions(status: CommentStatus) -> &'static [(CommentStatus, &'static str)] {
    match status {
        CommentStatus::Pending => &[
            (CommentStatus::Addressed, "comments.markAddressed"),
            (CommentStatus::Archived, "comments.archive"),
        ],
        CommentStatus::Addressed => &[
            (CommentStatus::Pending, "comments.reopen"),
            (CommentStatus::Archived, "comments.archive"),
        ],
        CommentStatus::Archived => &[(CommentStatus::Pending, "comments.reopen")],
    }
}

fn status_label(locale: Locale, status: CommentStatus) -> &'static str {
    translate(
        locale,
        match status {
            CommentStatus::Pending => "comments.statusPending",
            CommentStatus::Addressed => "comments.statusAddressed",
            CommentStatus::Archived => "comments.statusArchived",
        },
    )
}

fn author_label(locale: Locale, name: &str, kind: CommentAuthorKind) -> String {
    let name = name.trim();
    match kind {
        CommentAuthorKind::User if name.is_empty() => translate(locale, "comments.you").to_owned(),
        CommentAuthorKind::User => name.to_owned(),
        CommentAuthorKind::Agent if name.is_empty() => {
            translate(locale, "comments.agent").to_owned()
        }
        CommentAuthorKind::Agent => format!("{name} · {}", translate(locale, "comments.agent")),
    }
}

/// Right of the tip when it fits, else left; clamped into the canvas.
fn place(canvas: Rect, tip: Point2D, height: f32) -> Rect {
    let right = tip.x + PIN_SIZE + GAP;
    let canvas_right = canvas.origin.x + canvas.size.x;
    let x = if right + WIDTH <= canvas_right {
        right
    } else {
        tip.x - GAP - WIDTH
    };
    let x = x.clamp(canvas.origin.x, (canvas_right - WIDTH).max(canvas.origin.x));
    let max_y = (canvas.origin.y + canvas.size.y - height).max(canvas.origin.y);
    let y = (tip.y - PIN_SIZE).clamp(canvas.origin.y, max_y);
    Rect::xywh(x, y, WIDTH, height)
}

fn icon_rect(header: Rect, index_from_right: usize) -> Rect {
    let x =
        header.origin.x + header.size.x - PAD + 4.0 - ICON_HIT * (index_from_right as f32 + 1.0);
    Rect::xywh(
        x,
        header.origin.y + (HEADER_H - ICON_HIT) / 2.0,
        ICON_HIT,
        ICON_HIT,
    )
}

fn state_hover(state: &EditorState, hit: &CommentPopoverHit) -> bool {
    state.editor_ui.comments.hover.as_ref() == Some(hit)
}

fn paint_icon_button(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, icon: Icon, hovered: bool) {
    if hovered {
        cx.backend.fill_round_rect(rect, 6.0, theme.button_hover);
    }
    let glyph = 14.0;
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            rect.origin.x + (ICON_HIT - glyph) / 2.0,
            rect.origin.y + (ICON_HIT - glyph) / 2.0,
        ),
        glyph,
        theme.muted_foreground,
        1.5,
    );
}

fn button_width(cx: &mut PaintCx<'_>, label: &str) -> f32 {
    text_metrics::measure_chrome(cx.backend, label, FONT) + BUTTON_PAD_X * 2.0
}

fn paint_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    label: &str,
    primary: bool,
    enabled: bool,
    hovered: bool,
) {
    let (fill, text) = match (primary, enabled) {
        (true, true) => (theme.primary, theme.primary_foreground),
        (true, false) => (theme.muted, theme.muted_foreground),
        (false, _) if hovered => (theme.button_hover, theme.popover_foreground),
        (false, _) => (theme.secondary, theme.secondary_foreground),
    };
    cx.backend.fill_round_rect(rect, 6.0, fill);
    let x = text_metrics::centered_text_x(cx.backend, label, FONT, rect);
    draw_label(
        cx,
        label,
        Point2D::new(x, jian_widgets::centered_text_baseline_y(rect, FONT)),
        text,
        500,
    );
}

fn paint_input(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &EditorState,
    locale: Locale,
    rect: Rect,
    replying: bool,
) {
    cx.backend.fill_round_rect(rect, 6.0, theme.background);
    cx.backend.stroke_round_rect(rect, 6.0, theme.ring, 1.0);
    let input = &state.editor_ui.comments.input;
    let baseline = jian_widgets::centered_text_baseline_y(rect, FONT);
    let inner_w = rect.size.x - 16.0;
    let text = input.text();
    if text.is_empty() {
        let key = if replying {
            "comments.placeholderReply"
        } else {
            "comments.placeholderNew"
        };
        draw_label(
            cx,
            translate(locale, key),
            Point2D::new(rect.origin.x + 8.0, baseline),
            theme.muted_foreground,
            400,
        );
    }
    // Single line: keep the caret end visible by dropping leading chars.
    let before_caret = &text[..input.caret().min(text.len())];
    let mut start = 0;
    while start < before_caret.len()
        && text_metrics::measure_chrome(cx.backend, &before_caret[start..], FONT) > inner_w
    {
        start += before_caret[start..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(1);
    }
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        rect.origin.x + 4.0,
        rect.origin.y,
        rect.size.x - 8.0,
        rect.size.y,
    ));
    if !text.is_empty() {
        draw_label(
            cx,
            &text[start..],
            Point2D::new(rect.origin.x + 8.0, baseline),
            theme.foreground,
            400,
        );
    }
    // Steady caret: the popover schedules no blink wake-ups.
    {
        let caret_x = rect.origin.x
            + 8.0
            + text_metrics::measure_chrome(cx.backend, &before_caret[start..], FONT);
        cx.backend.fill_rect(
            Rect::xywh(caret_x, rect.origin.y + 8.0, 1.0, rect.size.y - 16.0),
            theme.foreground,
        );
    }
    cx.backend.restore();
}

fn draw_label(cx: &mut PaintCx<'_>, text: &str, at: Point2D, color: Color, weight: u16) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        FONT,
        color.to_jian(),
        Point2D::new(0.0, 0.0),
    )
    .with_font_weight(weight);
    cx.backend.draw_text(&layout, at);
}

fn to_array(r: Rect) -> [f32; 4] {
    [r.origin.x, r.origin.y, r.size.x, r.size.y]
}

#[cfg(test)]
#[path = "comment_popover_tests.rs"]
mod tests;
