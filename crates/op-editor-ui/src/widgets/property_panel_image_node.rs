//! Image-node specific section for the native property panel.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::NodeSnapshot;
use crate::widgets::property_panel_image_preview::paint_image_preview;
use crate::widgets::property_panel_inputs::{
    paint_section_divider, paint_section_label, to_jian_color, INPUT_HEIGHT, INPUT_RADIUS, PAD_X,
    SECTION_GAP,
};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

#[allow(clippy::too_many_arguments)]
pub fn paint_image_node_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(
        cx,
        theme,
        op_i18n::translate(locale, "image.title"),
        x,
        y,
        width,
    );
    let usable_w = width - PAD_X * 2.0;
    let row = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w, INPUT_HEIGHT),
    };
    cx.backend.fill_round_rect(row, INPUT_RADIUS, theme.muted);
    cx.backend
        .stroke_round_rect(row, INPUT_RADIUS, theme.border, 1.0);

    let summary = snapshot.image_fill.as_ref();
    let thumb = Rect {
        origin: Point2D::new(row.origin.x + 6.0, row.origin.y + 5.0),
        size: Point2D::new(20.0, 20.0),
    };
    let painted = summary
        .and_then(|summary| {
            summary
                .image_url
                .as_deref()
                .map(|src| paint_image_preview(cx, thumb, src, summary))
        })
        .unwrap_or(false);
    if !painted {
        draw_icon(
            cx.backend,
            Icon::ImagePlus,
            Point2D::new(thumb.origin.x, thumb.origin.y),
            18.0,
            theme.muted_foreground,
            1.4,
        );
    }
    let label_key = summary
        .map(|summary| summary.mode.label_key())
        .unwrap_or("image.fill");
    let label = TextLayout::single_run(
        op_i18n::translate(locale, label_key),
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(row.origin.x + 32.0, row.origin.y + 19.0),
    );

    y += INPUT_HEIGHT + 34.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}
