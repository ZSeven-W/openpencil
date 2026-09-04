//! Layer-section paint for the right property panel.

use crate::theme::Theme;
use crate::util::format_panel_number as format_number;
use crate::widgets::property_panel_inputs::{
    paint_section_divider, paint_section_label, INPUT_HEIGHT, INPUT_RADIUS, PAD_X, SECTION_GAP,
};
use crate::widgets::property_panel_sections::{EditContext, PropertyLabels};
use crate::widgets::property_panel_snapshot::{EllipseArcSummary, NodeSnapshot};
use crate::widgets::text_metrics;
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
    show_compositing: bool,
    touch_controls: bool,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.layer, x, y, width);
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let opacity_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(
            opacity_input_width(width, snapshot.polygon_sides.is_some(), touch_controls),
            INPUT_HEIGHT,
        ),
    };
    let opacity_value = format_number(snapshot.opacity_percent);
    paint_labeled_input(
        cx,
        theme,
        edit,
        opacity_rect,
        labels.opacity,
        PropertyFocus::Opacity,
        &opacity_value,
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

    if show_compositing {
        y += COMPOSITING_ROW_GAP;
        crate::widgets::property_panel_compositing::paint_node_triggers(
            cx, theme, snapshot, locale, x, y, width,
        );
        y += INPUT_HEIGHT;
    }

    y += 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

/// The half-width desktop row leaves room for Polygon Sides. On the narrower
/// logical viewport produced by touch density there is no sibling for ordinary
/// nodes, so let Opacity use the otherwise-empty second column rather than
/// ellipsizing localized labels such as `不透明度`.
pub(crate) fn opacity_input_width(
    panel_width: f32,
    has_polygon_sides: bool,
    touch_controls: bool,
) -> f32 {
    let usable_w = panel_width - PAD_X * 2.0;
    if touch_controls && !has_polygon_sides {
        usable_w
    } else {
        (usable_w - 8.0) / 2.0
    }
}

const COMPOSITING_ROW_GAP: f32 = crate::widgets::property_panel_compositing::COMPOSITING_ROW_GAP;

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
    // The prefix label yields to the value: it is fitted to whatever is left
    // of the box after the value, its unit and the gaps between them. The
    // value is what the user is reading and editing, so a long localized
    // label ("Deckkraft") must ellipsize rather than push the digits under
    // the clip set above — which shears them with no ellipsis at all.
    let value_w = text_metrics::measure_chrome(cx.backend, value, 12.0);
    let suffix_w = suffix.map_or(0.0, |unit| {
        6.0 + text_metrics::measure_chrome(cx.backend, unit, 12.0)
    });
    let prefix_budget = (rect.size.x - 10.0 - 8.0 - value_w - suffix_w - 8.0).max(0.0);
    let prefix = text_metrics::fit_chrome(cx.backend, prefix, prefix_budget, 12.0);
    let prefix_layout = TextLayout::single_run(
        &prefix,
        "system-ui",
        12.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&prefix_layout, Point2D::new(prefix_x, baseline_y));
    let prefix_w = text_metrics::measure_chrome(cx.backend, &prefix, 12.0);
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
            let w = text_metrics::measure_chrome(cx.backend, &value[..pos.min(value.len())], 12.0);
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
        // The unit trails the value, but it is clamped inside the input's
        // right padding: the input is clipped to `rect`, so a wide value
        // would otherwise push the unit under the clip and shear it. Short
        // values — every ordinary case — are unaffected.
        let unit_w = text_metrics::measure_chrome(cx.backend, unit, 12.0);
        let unit_x = (value_x + value_w + 6.0).min(rect.origin.x + rect.size.x - 8.0 - unit_w);
        let unit_layout = TextLayout::single_run(
            unit,
            "system-ui",
            12.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&unit_layout, Point2D::new(unit_x, baseline_y));
    }
    cx.backend.restore();
}
