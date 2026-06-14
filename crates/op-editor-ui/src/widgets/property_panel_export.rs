//! Export-section paint code split out of
//! `property_panel_sections.rs` to honour the 800-line ceiling
//! (mirrors the `property_panel_fill.rs` split). Contains the
//! Export section itself (two dropdowns + the Export action
//! button) plus the inline scale / format select popups.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{EffectSummary, PropertyPanelAction};
use crate::widgets::property_panel_inputs::{
    paint_dropdown, paint_section_label, INPUT_HEIGHT, INPUT_RADIUS, PAD_X,
};
use crate::widgets::property_panel_layout::{
    action_button_rects_with_fill_picker, VisibleSections,
};
use crate::widgets::property_panel_sections::PropertyLabels;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

/// "1x" / "2x" / "3x" label for a raster export scale.
pub fn export_scale_label(scale: f32) -> &'static str {
    match crate::widgets::export_dialog::scale_index(scale) {
        1 => "1x",
        3 => "3x",
        _ => "2x",
    }
}

/// Paint the Export section: title, a row of two dropdowns (scale +
/// format) and a full-width Export action button below them.
// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub fn paint_export_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    format: op_editor_core::ExportFormat,
    scale: f32,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.export, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    // Left dropdown: scale. Right dropdown: format. Clicking either
    // opens its inline select popup (`paint_export_picker`); the
    // full Export modal is reached only via File ▸ Export Image.
    paint_dropdown(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        export_scale_label(scale),
    );
    paint_dropdown(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        format.label(),
    );
    y += INPUT_HEIGHT + 12.0;
    // Export action button — full-width primary button. Triggers a
    // direct export with the chosen scale + format (the host queues
    // `FileAction::ExportImageConfirm`, which pops the Save dialog).
    let btn = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w, INPUT_HEIGHT),
    };
    cx.backend.fill_round_rect(btn, INPUT_RADIUS, theme.primary);
    let tw = cx.backend.measure_text(labels.export, 13.0);
    let lbl = TextLayout::single_run(
        labels.export,
        "system-ui",
        13.0,
        (theme.primary_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &lbl,
        Point2D::new(btn.origin.x + (btn.size.x - tw) / 2.0, btn.origin.y + 19.0),
    );
    y += INPUT_HEIGHT + 12.0;
    y
}

/// Paint the Export section's inline scale / format select popups.
/// Called by `PropertyPanel::paint` AFTER every section so the
/// popup overlays them. Row geometry comes straight from
/// `action_button_rects_with_fill_picker` — the same walker
/// `hit_test_action` uses — so the painted rows and the clickable
/// hit-rects can never drift apart. `hover` is the row index the
/// cursor is over (within the open picker), or `None`.
#[allow(clippy::too_many_arguments)]
pub fn paint_export_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: VisibleSections,
    effects: &[EffectSummary],
    scale_open: bool,
    format_open: bool,
    current_scale: f32,
    current_format: op_editor_core::ExportFormat,
    hover: Option<usize>,
) {
    use PropertyPanelAction as A;
    let rects = action_button_rects_with_fill_picker(
        panel_rect,
        visible,
        effects,
        false,
        false,
        false,
        scale_open,
        format_open,
        false,
    );
    if scale_open {
        let rows: Vec<(&str, bool, Rect)> = rects
            .iter()
            .filter_map(|(a, r)| match a {
                A::SetExportScale(s) => Some((
                    export_scale_label(*s),
                    (*s - current_scale).abs() < 0.01,
                    *r,
                )),
                _ => None,
            })
            .collect();
        paint_select_popup(cx, theme, &rows, hover);
    }
    if format_open {
        let rows: Vec<(&str, bool, Rect)> = rects
            .iter()
            .filter_map(|(a, r)| match a {
                A::SetExportFormat(f) => Some((f.label(), *f == current_format, *r)),
                _ => None,
            })
            .collect();
        paint_select_popup(cx, theme, &rows, hover);
    }
}

/// Paint a select-popup background enclosing `rows` plus one label
/// per row. The active row is tinted + check-marked; the `hover`
/// row gets a lighter wash.
fn paint_select_popup(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rows: &[(&str, bool, Rect)],
    hover: Option<usize>,
) {
    let (Some((_, _, first)), Some((_, _, last))) = (rows.first(), rows.last()) else {
        return;
    };
    let bg = Rect {
        origin: Point2D::new(first.origin.x - 4.0, first.origin.y - 6.0),
        size: Point2D::new(
            first.size.x + 8.0,
            (last.origin.y + last.size.y) - (first.origin.y - 6.0) + 6.0,
        ),
    };
    cx.backend.fill_round_rect(bg, 8.0, theme.popover);
    cx.backend.stroke_round_rect(bg, 8.0, theme.border, 1.0);
    for (i, (label, active, row)) in rows.iter().enumerate() {
        // Active selection wins over hover (stronger tint); a plain
        // hovered row gets the lighter `accent` wash.
        let wash = if *active {
            Some(theme.row_selected_primary)
        } else if hover == Some(i) {
            Some(theme.accent)
        } else {
            None
        };
        if let Some(color) = wash {
            let sel = Rect {
                origin: Point2D::new(row.origin.x + 2.0, row.origin.y + 2.0),
                size: Point2D::new(row.size.x - 4.0, row.size.y - 4.0),
            };
            cx.backend.fill_round_rect(sel, 6.0, color);
        }
        let text_color = if *active {
            theme.primary
        } else {
            theme.foreground
        };
        let lbl = TextLayout::single_run(
            label,
            "system-ui",
            12.0,
            (text_color).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&lbl, Point2D::new(row.origin.x + 12.0, row.origin.y + 20.0));
        if *active {
            draw_icon(
                cx.backend,
                Icon::Check,
                Point2D::new(row.origin.x + row.size.x - 22.0, row.origin.y + 7.0),
                14.0,
                theme.primary,
                1.6,
            );
        }
    }
}
