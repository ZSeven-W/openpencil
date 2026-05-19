//! Body painter for [`super::ai_chat_panel::AIChatPlaceholder`] — the
//! empty-state example-card grid. The active-transcript painter
//! lives in `ai_chat_transcript.rs`. Split out of `ai_chat_panel.rs`
//! to keep that file under the 800-line cap.

use super::ai_chat_panel::{to_jian_color, ExampleCard, HEADER_HEIGHT, PAD};
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

/// Paint the empty-state hint line + the 2×2 example-card grid.
pub(crate) fn paint_examples(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    hint_label: &str,
    examples: &[ExampleCard; 4],
) {
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
    for (i, ex) in examples.iter().enumerate() {
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
            &ex.title,
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
            &ex.subtitle,
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
