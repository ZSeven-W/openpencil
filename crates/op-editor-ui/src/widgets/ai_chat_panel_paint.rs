//! Body painter for [`super::ai_chat_panel::AIChatPlaceholder`] — the
//! empty-state example-card grid. The active-transcript painter
//! lives in `ai_chat_transcript.rs`. Split out of `ai_chat_panel.rs`
//! to keep that file under the 800-line cap.

use super::ai_chat_panel::{to_jian_color, ExampleCard, HEADER_HEIGHT, PAD};
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

pub(crate) const EXAMPLE_CARD_GAP: f32 = 8.0;
pub(crate) const EXAMPLE_CARD_HEIGHT: f32 = 58.0;
const EXAMPLE_CARD_PAD: f32 = 12.0;

pub(crate) fn example_card_rects(rect: Rect) -> [Rect; 4] {
    let grid_y = rect.origin.y + HEADER_HEIGHT + 32.0;
    let card_w = (rect.size.x - PAD * 2.0 - EXAMPLE_CARD_GAP) / 2.0;
    std::array::from_fn(|i| {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        Rect {
            origin: Point2D::new(
                rect.origin.x + PAD + col * (card_w + EXAMPLE_CARD_GAP),
                grid_y + row * (EXAMPLE_CARD_HEIGHT + EXAMPLE_CARD_GAP),
            ),
            size: Point2D::new(card_w, EXAMPLE_CARD_HEIGHT),
        }
    })
}

fn with_alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

/// Paint the floating chat panel shell. TS uses
/// `rounded-xl border bg-card/95 shadow-2xl backdrop-blur-sm`;
/// Skia here has no blur primitive, so we layer translucent rounded
/// rects behind the card to give the panel a comparable lift.
pub(crate) fn paint_panel_surface(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect) {
    let shadow_outer = Rect {
        origin: Point2D::new(rect.origin.x, rect.origin.y + 18.0),
        size: rect.size,
    };
    let shadow_inner = Rect {
        origin: Point2D::new(rect.origin.x, rect.origin.y + 8.0),
        size: rect.size,
    };
    cx.backend
        .fill_round_rect(shadow_outer, 14.0, with_alpha(Color::BLACK, 0.14));
    cx.backend
        .fill_round_rect(shadow_inner, 14.0, with_alpha(Color::BLACK, 0.2));
    cx.backend
        .fill_round_rect(rect, 14.0, with_alpha(theme.card, 0.95));
    cx.backend.stroke_round_rect(rect, 14.0, theme.border, 1.0);
}

/// Paint the empty-state hint line + the 2×2 example-card grid.
pub(crate) fn paint_examples(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    hint_label: &str,
    tip_label: &str,
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
    let hint_w = cx.backend.measure_text(hint_label, 12.0);
    cx.backend.draw_text(
        &hint,
        Point2D::new(rect.origin.x + (rect.size.x - hint_w) / 2.0, hint_y),
    );

    let card_bg = with_alpha(theme.muted, 0.3);
    for (card, ex) in example_card_rects(rect).iter().zip(examples.iter()) {
        cx.backend.fill_round_rect(*card, 8.0, card_bg);
        cx.backend.stroke_round_rect(*card, 8.0, theme.border, 1.0);
        let emoji_layout = TextLayout::single_run(
            ex.emoji,
            "system-ui",
            14.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &emoji_layout,
            Point2D::new(
                card.origin.x + EXAMPLE_CARD_PAD,
                card.origin.y + EXAMPLE_CARD_PAD + 10.0,
            ),
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
            Point2D::new(
                card.origin.x + EXAMPLE_CARD_PAD + 20.0,
                card.origin.y + EXAMPLE_CARD_PAD + 10.0,
            ),
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
            Point2D::new(
                card.origin.x + EXAMPLE_CARD_PAD,
                card.origin.y + EXAMPLE_CARD_PAD + 28.0,
            ),
        );
    }
    let tip = TextLayout::single_run(
        tip_label,
        "system-ui",
        10.0,
        to_jian_color(with_alpha(theme.muted_foreground, 0.5)),
        Point2D::new(0.0, 0.0),
    );
    let tip_w = cx.backend.measure_text(tip_label, 10.0);
    cx.backend.draw_text(
        &tip,
        Point2D::new(
            rect.origin.x + (rect.size.x - tip_w) / 2.0,
            rect.origin.y
                + HEADER_HEIGHT
                + 32.0
                + EXAMPLE_CARD_HEIGHT * 2.0
                + EXAMPLE_CARD_GAP * 2.0
                + 22.0,
        ),
    );
}
