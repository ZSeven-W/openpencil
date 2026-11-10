//! Paint of the motion timeline panel, from the shared layout walk.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::motion_timeline::MotionTimelineView;
use crate::widgets::motion_timeline_layout::{TimelineLayout, KEYFRAME_R};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

/// Tick spacings the ruler picks from: the first at least this many
/// pixels apart.
const TICK_STEPS_MS: [f64; 8] = [50.0, 100.0, 200.0, 250.0, 500.0, 1000.0, 2000.0, 5000.0];
const MIN_TICK_PX: f32 = 56.0;

pub fn paint_motion_timeline(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    view: &MotionTimelineView,
) {
    let layout = TimelineLayout::new(rect, view);
    cx.backend.save();
    cx.backend.clip_rect(rect);
    cx.backend.fill_rect(rect, theme.background);
    cx.backend.fill_rect(layout.labels, theme.card);
    line(
        cx,
        theme.border,
        rect.origin,
        Point2D::new(rect.origin.x + rect.size.x, rect.origin.y),
    );
    let header_bottom = layout.header.origin.y + layout.header.size.y;
    line(
        cx,
        theme.border,
        Point2D::new(rect.origin.x, header_bottom),
        Point2D::new(rect.origin.x + rect.size.x, header_bottom),
    );
    paint_header(cx, theme, &layout, view);
    paint_ruler(cx, theme, &layout, view);
    paint_lanes(cx, theme, &layout, view);
    paint_playhead(cx, theme, &layout, view);
    cx.backend.restore();
}

fn paint_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    layout: &TimelineLayout,
    view: &MotionTimelineView,
) {
    if let Some(scope) = &view.scope_label {
        text(
            cx,
            scope,
            layout.scope.origin.x + 12.0,
            layout.scope,
            12.0,
            theme.foreground,
        );
    }
    for (tab, rect) in &layout.tabs {
        let active = *tab == view.tab;
        if active {
            cx.backend.fill_round_rect(*rect, 6.0, theme.muted);
        }
        let color = if active {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        text(
            cx,
            &view.tab_label(*tab),
            rect.origin.x + 10.0,
            *rect,
            12.0,
            color,
        );
    }
    let time = format!(
        "{:.2} s / {:.2} s",
        view.playhead_ms / 1000.0,
        view.clip_ms / 1000.0
    );
    text(
        cx,
        &time,
        layout.time.origin.x + 4.0,
        layout.time,
        12.0,
        theme.muted_foreground,
    );
    if view.playing {
        // Pause: two bars.
        let r = layout.play;
        let (cx0, cy) = (r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0);
        cx.backend
            .fill_rect(Rect::xywh(cx0 - 5.0, cy - 6.0, 3.5, 12.0), theme.foreground);
        cx.backend
            .fill_rect(Rect::xywh(cx0 + 1.5, cy - 6.0, 3.5, 12.0), theme.foreground);
    } else {
        icon(cx, Icon::Play, layout.play, theme.foreground);
    }
    let loop_color = if view.looping {
        theme.primary
    } else {
        theme.muted_foreground
    };
    icon(cx, Icon::RefreshCw, layout.looping, loop_color);
    icon(cx, Icon::Minus, layout.zoom_out, theme.muted_foreground);
    icon(cx, Icon::Plus, layout.zoom_in, theme.muted_foreground);
}

fn paint_ruler(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    layout: &TimelineLayout,
    view: &MotionTimelineView,
) {
    let step = TICK_STEPS_MS
        .into_iter()
        .find(|step| *step as f32 * view.zoom >= MIN_TICK_PX)
        .unwrap_or(10_000.0);
    let ruler = layout.ruler;
    let last_ms = layout.ms_for_x(ruler.origin.x + ruler.size.x);
    let mut tick = 0.0;
    while tick <= last_ms {
        let x = layout.x_for_ms(tick);
        let bottom = ruler.origin.y + ruler.size.y;
        line(
            cx,
            theme.border,
            Point2D::new(x, bottom - 6.0),
            Point2D::new(x, bottom),
        );
        text(
            cx,
            &seconds_label(tick),
            x + 3.0,
            ruler,
            10.0,
            theme.muted_foreground,
        );
        tick += step;
    }
}

fn paint_lanes(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    layout: &TimelineLayout,
    view: &MotionTimelineView,
) {
    if let Some(message) = view.empty_message() {
        text(
            cx,
            message,
            layout.lanes.origin.x + 16.0,
            Rect::xywh(
                layout.lanes.origin.x,
                layout.lanes.origin.y,
                layout.lanes.size.x,
                32.0,
            ),
            12.0,
            theme.muted_foreground,
        );
        return;
    }
    let run = Color {
        a: 0.18,
        ..theme.primary
    };
    let iteration = Color {
        a: 0.45,
        ..theme.primary
    };
    for (index, (row, row_layout)) in view.rows.iter().zip(&layout.rows).enumerate() {
        if index % 2 == 1 {
            cx.backend.fill_rect(
                row_layout.lane,
                Color {
                    a: 0.04,
                    ..theme.foreground
                },
            );
        }
        text(
            cx,
            &row.label,
            row_layout.label.origin.x + 12.0,
            row_layout.label,
            12.0,
            theme.foreground,
        );
        cx.backend.fill_round_rect(row_layout.strip, 4.0, run);
        cx.backend
            .fill_round_rect(row_layout.iteration, 4.0, iteration);
        for (keyframe, center) in row_layout.keyframes.iter().enumerate() {
            let key = op_editor_core::editor_ui_state::TimelineKeyframeRef {
                node_id: row.strip.node_id.clone(),
                animation: row.strip.animation,
                keyframe,
            };
            let color = if view.is_selected(&key) {
                theme.primary
            } else {
                theme.foreground
            };
            diamond(cx, *center, KEYFRAME_R, color);
        }
    }
}

fn paint_playhead(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    layout: &TimelineLayout,
    view: &MotionTimelineView,
) {
    let x = layout.x_for_ms(view.playhead_ms);
    let top = layout.ruler.origin.y;
    let bottom = layout.rect.origin.y + layout.rect.size.y;
    line(
        cx,
        theme.destructive,
        Point2D::new(x, top),
        Point2D::new(x, bottom),
    );
    diamond(cx, Point2D::new(x, top + 5.0), 5.0, theme.destructive);
}

/// A ruler tick in seconds with only the decimals it needs: `0 s`,
/// `0.25 s`, `0.5 s`, `1 s`.
pub(crate) fn seconds_label(ms: f64) -> String {
    let fixed = format!("{:.2}", ms / 1000.0);
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    format!("{trimmed} s")
}

fn line(cx: &mut PaintCx<'_>, color: Color, from: Point2D, to: Point2D) {
    cx.backend.stroke_line(from, to, color, 1.0);
}

fn diamond(cx: &mut PaintCx<'_>, center: Point2D, r: f32, color: Color) {
    cx.backend.fill_polygon(
        &[
            Point2D::new(center.x, center.y - r),
            Point2D::new(center.x + r, center.y),
            Point2D::new(center.x, center.y + r),
            Point2D::new(center.x - r, center.y),
        ],
        color,
    );
}

fn icon(cx: &mut PaintCx<'_>, icon: Icon, rect: Rect, color: Color) {
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            rect.origin.x + (rect.size.x - 14.0) / 2.0,
            rect.origin.y + (rect.size.y - 14.0) / 2.0,
        ),
        14.0,
        color,
        1.4,
    );
}

/// Text vertically centred in `row`, starting at `x`.
fn text(cx: &mut PaintCx<'_>, value: &str, x: f32, row: Rect, size: f32, color: Color) {
    let layout = TextLayout::single_run(
        value,
        "system-ui",
        size,
        color.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(x, jian_widgets::centered_text_baseline_y(row, size)),
    );
}
