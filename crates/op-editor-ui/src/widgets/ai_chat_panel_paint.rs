//! Body painters for [`super::ai_chat_panel::AIChatPlaceholder`] — the
//! empty-state example-card grid and the message-bubble transcript.
//! Split out of `ai_chat_panel.rs` to keep that file under the
//! 800-line cap.

use super::ai_chat_panel::{to_jian_color, EXAMPLES, HEADER_HEIGHT, PAD};
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::chat::{ChatMessage, ChatRole};

/// Paint the empty-state hint line + the 2×2 example-card grid.
pub(crate) fn paint_examples(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, hint_label: &str) {
    let hint = TextLayout::single_run(
        hint_label,
        "system-ui",
        12.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    let hint_y = rect.origin.y + HEADER_HEIGHT + 16.0;
    cx.backend.draw_text(
        &hint,
        Point2D::new(rect.origin.x + rect.size.x / 2.0 - 40.0, hint_y),
    );

    let grid_origin_y = hint_y + 16.0;
    let card_w = (rect.size.x - PAD * 2.0 - 8.0) / 2.0;
    let card_h = 70.0;
    for (i, ex) in EXAMPLES.iter().enumerate() {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        let card = Rect {
            origin: Point2D::new(
                rect.origin.x + PAD + col * (card_w + 8.0),
                grid_origin_y + row * (card_h + 8.0),
            ),
            size: Point2D::new(card_w, card_h),
        };
        cx.backend.fill_round_rect(card, 8.0, theme.muted);
        cx.backend.stroke_round_rect(card, 8.0, theme.border, 1.0);
        let emoji_layout = TextLayout::single_run(
            ex.emoji,
            "system-ui",
            14.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &emoji_layout,
            Point2D::new(card.origin.x + 12.0, card.origin.y + 22.0),
        );
        let title_layout = TextLayout::single_run(
            ex.title,
            "system-ui",
            12.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &title_layout,
            Point2D::new(card.origin.x + 36.0, card.origin.y + 22.0),
        );
        let subtitle_layout = TextLayout::single_run(
            ex.subtitle,
            "system-ui",
            11.0,
            to_jian_color(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &subtitle_layout,
            Point2D::new(card.origin.x + 36.0, card.origin.y + 42.0),
        );
    }
}

/// Paint the chat transcript — the most recent message bubbles that
/// fit within `body_rect`.
pub(crate) fn paint_messages(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    body_rect: Rect,
    messages: &[ChatMessage],
) {
    cx.backend.save();
    cx.backend.clip_rect(body_rect);
    let row_h = 38.0;
    let max_visible = (body_rect.size.y / row_h).floor() as usize;
    let start = messages.len().saturating_sub(max_visible.max(1));
    let mut y = body_rect.origin.y + 4.0;
    for msg in &messages[start..] {
        let bubble_rect = match msg.role {
            ChatRole::User => Rect {
                origin: Point2D::new(body_rect.origin.x + body_rect.size.x * 0.25, y),
                size: Point2D::new(body_rect.size.x * 0.75 - 4.0, row_h - 6.0),
            },
            ChatRole::Assistant => Rect {
                origin: Point2D::new(body_rect.origin.x, y),
                size: Point2D::new(body_rect.size.x * 0.75, row_h - 6.0),
            },
        };
        let bg = match msg.role {
            ChatRole::User => theme.primary,
            ChatRole::Assistant => theme.muted,
        };
        let fg = match msg.role {
            ChatRole::User => theme.primary_foreground,
            ChatRole::Assistant => theme.foreground,
        };
        cx.backend.fill_round_rect(bubble_rect, 8.0, bg);
        let layout = TextLayout::single_run(
            &msg.content,
            "system-ui",
            12.0,
            to_jian_color(fg),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &layout,
            Point2D::new(bubble_rect.origin.x + 10.0, bubble_rect.origin.y + 21.0),
        );
        y += row_h;
    }
    cx.backend.restore();
}
