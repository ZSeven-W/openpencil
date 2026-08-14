//! Disabled primary action used by touch builds without an auth runtime.

use crate::{Point2D, Rect, RenderBackend, TextLayout, Theme};

pub(super) fn paint(backend: &mut dyn RenderBackend, theme: &Theme, button: Rect, label: &str) {
    backend.fill_round_rect(button, 11.0, theme.muted);
    backend.stroke_round_rect(button, 11.0, theme.border, 1.0);
    let font_size = 12.0;
    let weight = 600;
    let width = backend.measure_text_weighted(label, font_size, weight);
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        font_size,
        theme.muted_foreground.to_jian(),
        Point2D::ZERO,
    )
    .with_font_weight(weight);
    backend.draw_text(
        &layout,
        Point2D::new(
            button.origin.x + (button.size.x - width) / 2.0,
            button.origin.y + button.size.y / 2.0 + 4.0,
        ),
    );
}
