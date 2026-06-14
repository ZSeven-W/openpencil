//! Layer-section paint for the right property panel.

use crate::theme::Theme;
use crate::widgets::property_panel_inputs::{
    paint_section_divider, paint_section_label, INPUT_HEIGHT, INPUT_RADIUS, PAD_X, SECTION_GAP,
};
use crate::widgets::property_panel_sections::{EditContext, PropertyLabels};
use crate::widgets::property_panel_snapshot::{EllipseArcSummary, NodeSnapshot};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::PropertyFocus;

#[allow(clippy::too_many_arguments)]
pub fn paint_layer_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    labels: &PropertyLabels,
    edit: &EditContext<'_>,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.layer, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let opacity_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    paint_labeled_input(
        cx,
        theme,
        edit,
        opacity_rect,
        labels.opacity,
        PropertyFocus::Opacity,
        "100",
        Some("%"),
    );

    if let Some(sides) = snapshot.polygon_sides {
        let sides_rect = Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        };
        let value = sides.to_string();
        paint_labeled_input(
            cx,
            theme,
            edit,
            sides_rect,
            labels.polygon_sides,
            PropertyFocus::PolygonSides,
            &value,
            None,
        );
    }

    y += INPUT_HEIGHT;
    if let Some(arc) = snapshot.ellipse_arc {
        y += 6.0;
        paint_ellipse_arc_row(cx, theme, edit, labels, arc, x + PAD_X, y, usable_w);
        y += INPUT_HEIGHT;
    }

    y += 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

#[allow(clippy::too_many_arguments)]
fn paint_ellipse_arc_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    arc: EllipseArcSummary,
    x: f32,
    y: f32,
    width: f32,
) {
    let col_w = (width - 12.0) / 3.0;
    let values = [
        (
            labels.ellipse_start,
            PropertyFocus::EllipseStart,
            format_number(arc.start_deg),
            Some("°"),
        ),
        (
            labels.ellipse_sweep,
            PropertyFocus::EllipseSweep,
            format_number(arc.sweep_deg),
            Some("°"),
        ),
        (
            labels.ellipse_inner_radius,
            PropertyFocus::EllipseInnerRadius,
            format_number(arc.inner_percent),
            Some("%"),
        ),
    ];
    for (i, (label, focus, value, unit)) in values.iter().enumerate() {
        paint_labeled_input(
            cx,
            theme,
            edit,
            Rect {
                origin: Point2D::new(x + i as f32 * (col_w + 6.0), y),
                size: Point2D::new(col_w, INPUT_HEIGHT),
            },
            label,
            *focus,
            value,
            *unit,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_labeled_input(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    edit: &EditContext<'_>,
    rect: Rect,
    prefix: &str,
    focus: PropertyFocus,
    fallback: &str,
    suffix: Option<&str>,
) {
    let value = edit.value_for(focus, fallback);
    let focused = edit.focus == Some(focus);
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    if focused {
        cx.backend
            .stroke_round_rect(rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    cx.backend.save();
    cx.backend.clip_rect(rect);

    let baseline_y = rect.origin.y + 19.0;
    let prefix_x = rect.origin.x + 10.0;
    let prefix_layout = TextLayout::single_run(
        prefix,
        "system-ui",
        12.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&prefix_layout, Point2D::new(prefix_x, baseline_y));
    let prefix_w = cx.backend.measure_text(prefix, 12.0);
    let value_x = prefix_x + prefix_w + 8.0;
    if !edit.paint_input_view_at(
        cx,
        theme,
        focus,
        Rect {
            origin: Point2D::new(value_x, rect.origin.y),
            size: Point2D::new(
                (rect.origin.x + rect.size.x - 18.0 - value_x).max(0.0),
                rect.size.y,
            ),
        },
        12.0,
        0.0,
        baseline_y,
    ) {
        edit.paint_selection_at(
            cx,
            theme,
            focus,
            value,
            value_x,
            baseline_y,
            12.0,
            rect.origin.x + rect.size.x - 8.0,
        );
        let value_layout = TextLayout::single_run(
            value,
            "system-ui",
            12.0,
            (theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&value_layout, Point2D::new(value_x, baseline_y));
        if let Some(pos) = edit.caret_at(focus) {
            let w = cx
                .backend
                .measure_text(&value[..pos.min(value.len())], 12.0);
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(value_x + w, rect.origin.y + 6.0),
                    size: Point2D::new(1.5, INPUT_HEIGHT - 12.0),
                },
                theme.foreground,
            );
        }
    }
    if let Some(unit) = suffix {
        let value_w = cx.backend.measure_text(value, 12.0);
        let unit_layout = TextLayout::single_run(
            unit,
            "system-ui",
            12.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &unit_layout,
            Point2D::new(value_x + value_w + 6.0, baseline_y),
        );
    }
    cx.backend.restore();
}

fn format_number(value: f32) -> String {
    if value.fract().abs() < f32::EPSILON {
        format!("{}", value.round() as i32)
    } else {
        format!("{value:.2}")
    }
}
