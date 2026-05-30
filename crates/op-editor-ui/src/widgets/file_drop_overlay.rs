//! Full-canvas overlay shown while a file is dragged over the window.
//!
//! Painted as the top-most layer (above every panel / modal) so the
//! user sees an unambiguous drop target. Driven by
//! `EditorState.editor_ui.file_drop_active`, which the desktop runner
//! toggles on the platform's `HoveredFile` / `HoveredFileCancelled`
//! drag events.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel_inputs::to_jian_color;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

/// Paint the drop overlay across `canvas_rect` (the editor's canvas
/// region — i.e. excluding the rails so the highlight reads as "drop
/// onto the canvas").
pub fn paint_file_drop_overlay(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    locale: op_editor_core::Locale,
    canvas_rect: Rect,
) {
    let p = theme.primary;
    // 1. A subtle primary-tinted scrim over the whole canvas.
    backend.fill_rect(
        canvas_rect,
        Color {
            r: p.r,
            g: p.g,
            b: p.b,
            a: 0.10,
        },
    );
    // 2. An inset rounded "drop zone" border in the accent colour.
    let inset = 14.0;
    let border = Rect {
        origin: Point2D::new(canvas_rect.origin.x + inset, canvas_rect.origin.y + inset),
        size: Point2D::new(
            (canvas_rect.size.x - inset * 2.0).max(0.0),
            (canvas_rect.size.y - inset * 2.0).max(0.0),
        ),
    };
    backend.stroke_round_rect(border, 16.0, theme.primary, 2.5);

    // 3. A centred card: download icon + label.
    let label_text = op_i18n::translate(locale, "dialog.dropToOpen");
    let label_w = backend.measure_text(label_text, 14.0);
    let card_w = (label_w + 56.0).max(220.0);
    let card_h = 104.0;
    let cx = canvas_rect.origin.x + canvas_rect.size.x / 2.0;
    let cy = canvas_rect.origin.y + canvas_rect.size.y / 2.0;
    let card = Rect {
        origin: Point2D::new(cx - card_w / 2.0, cy - card_h / 2.0),
        size: Point2D::new(card_w, card_h),
    };
    backend.fill_round_rect(card, 14.0, theme.popover);
    backend.stroke_round_rect(card, 14.0, theme.border, 1.0);

    // Download glyph (arrow into a tray) reads as "drop here".
    let icon = 30.0;
    draw_icon(
        backend,
        Icon::Download,
        Point2D::new(cx - icon / 2.0, card.origin.y + 20.0),
        icon,
        theme.primary,
        2.0,
    );
    // Label below the icon, horizontally centred.
    let label = TextLayout::single_run(
        label_text,
        "system-ui",
        14.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    backend.draw_text(
        &label,
        Point2D::new(cx - label_w / 2.0, card.origin.y + 80.0),
    );
}
