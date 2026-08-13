//! Paint helpers + small leaf utilities for `LayerPanel`,
//! extracted to keep `layer_panel.rs` under the 800-line cap.

use crate::theme::Theme;
use crate::widgets::icons::draw_icon;
use crate::widgets::property_panel_text_input::paint_text_input_view_value;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use jian_core::text_input::TextInputState;

use super::layer_panel::LayerItem;
use super::layer_panel_metrics::{
    glyph_rect_in, layer_action_targets, layer_node_icon_x, LayerPanelMetrics,
};

pub(super) const ROW_FONT: f32 = 13.0;
/// Heuristic avg-char-width factor for system-ui at the row font.
/// 0.5 matches the rendered width closely enough that the caret
/// sits flush against the last glyph without a visible gap.
const TEXT_WIDTH_FACTOR: f32 = 0.5;

/// Approx pixel width — heuristic only, no Skia measurement.
pub(super) fn approx_text_width(s: &str, font_size: f32) -> f32 {
    s.chars().count() as f32 * font_size * TEXT_WIDTH_FACTOR
}

/// Truncate `s` to fit `max_w` at `font_size`, with `…` suffix.
pub(super) fn truncate_to_fit(s: &str, font_size: f32, max_w: f32) -> String {
    if approx_text_width(s, font_size) <= max_w {
        return s.to_string();
    }
    let approx_char_w = font_size * TEXT_WIDTH_FACTOR;
    let max_chars = ((max_w / approx_char_w).floor() as usize).saturating_sub(1);
    if max_chars == 0 {
        return "…".to_string();
    }
    let kept: String = s.chars().take(max_chars).collect();
    format!("{}…", kept)
}

pub(super) fn truncate_to_fit_measured(
    backend: &mut dyn RenderBackend,
    s: &str,
    font_size: f32,
    max_w: f32,
) -> String {
    const FONT_FAMILY: &str = "system-ui";
    if backend.measure_text_family(s, font_size, FONT_FAMILY) <= max_w {
        return s.to_string();
    }
    if backend.measure_text_family("…", font_size, FONT_FAMILY) > max_w {
        return "…".to_string();
    }

    let mut boundaries: Vec<usize> = s.char_indices().map(|(byte, _)| byte).collect();
    boundaries.push(s.len());
    let mut low = 0;
    let mut high = boundaries.len() - 1;
    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate = format!("{}…", &s[..boundaries[mid]]);
        if backend.measure_text_family(&candidate, font_size, FONT_FAMILY) <= max_w {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    format!("{}…", &s[..boundaries[low]])
}

pub(super) fn layer_trailing_icon_xs(row: Rect) -> (f32, f32) {
    let (eye, lock) = layer_action_targets(row, LayerPanelMetrics::DESKTOP);
    (
        glyph_rect_in(eye, LayerPanelMetrics::DESKTOP.trailing_glyph_size)
            .origin
            .x,
        glyph_rect_in(lock, LayerPanelMetrics::DESKTOP.trailing_glyph_size)
            .origin
            .x,
    )
}

pub(super) fn layer_action_gutter_left(row: Rect) -> f32 {
    let (eye_x, _) = layer_trailing_icon_xs(row);
    eye_x - 8.0
}

pub(super) fn layer_action_gutter_left_with_metrics(row: Rect, metrics: LayerPanelMetrics) -> f32 {
    if metrics.touch {
        layer_action_targets(row, metrics).0.origin.x
    } else {
        layer_action_gutter_left(row)
    }
}

pub(super) fn layer_content_clip_rect_with_metrics(
    row: Rect,
    renaming: bool,
    metrics: LayerPanelMetrics,
) -> Rect {
    let right_edge = if renaming {
        row.origin.x + row.size.x - 8.0
    } else {
        layer_action_gutter_left_with_metrics(row, metrics)
    };
    Rect {
        origin: row.origin,
        size: Point2D::new((right_edge - row.origin.x).max(0.0), row.size.y),
    }
}

pub(super) fn layer_label_available_width_with_metrics(
    row: Rect,
    label_x: f32,
    horizontal_offset: f32,
    renaming: bool,
    metrics: LayerPanelMetrics,
) -> f32 {
    let screen_label_x = label_x - horizontal_offset;
    let clip = layer_content_clip_rect_with_metrics(row, renaming, metrics);
    let right_edge = clip.origin.x + clip.size.x - if renaming { 2.0 } else { 0.0 };
    (right_edge - screen_label_x).max(0.0)
}

pub(super) fn paint_drag_ghost(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ghost: &LayerItem,
    cursor_y: f32,
    panel_rect: Rect,
    metrics: LayerPanelMetrics,
) {
    let row = Rect {
        origin: Point2D::new(
            panel_rect.origin.x + 6.0,
            cursor_y - metrics.layer_row_height / 2.0,
        ),
        size: Point2D::new(panel_rect.size.x - 12.0, metrics.layer_row_height - 4.0),
    };
    let bg = Color {
        a: 0.55,
        ..theme.row_selected
    };
    cx.backend.fill_round_rect(row, 6.0, bg);
    let fg = Color {
        a: 0.85,
        ..theme.foreground
    };
    cx.backend.save();
    cx.backend
        .clip_rect(layer_content_clip_rect_with_metrics(row, false, metrics));
    let indent = metrics.row_pad_x + ghost.depth as f32 * 12.0;
    let icon_x = layer_node_icon_x(row, indent, metrics);
    let icon_y = if metrics.touch {
        row.origin.y + (row.size.y - metrics.glyph_size) / 2.0
    } else {
        row.origin.y + 6.0
    };
    draw_icon(
        cx.backend,
        ghost.icon,
        Point2D::new(icon_x, icon_y),
        metrics.glyph_size,
        fg,
        1.4,
    );
    let label_x = icon_x + metrics.glyph_size + if metrics.touch { 8.0 } else { 6.0 };
    let available_w = layer_label_available_width_with_metrics(row, label_x, 0.0, false, metrics);
    let display = truncate_to_fit_measured(cx.backend, &ghost.label, metrics.row_font, available_w);
    let label = TextLayout::single_run(
        &display,
        "system-ui",
        metrics.row_font,
        (fg).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let baseline = if metrics.touch {
        jian_widgets::centered_text_baseline_y(row, metrics.row_font)
    } else {
        row.origin.y + 17.0
    };
    cx.backend
        .draw_text(&label, Point2D::new(label_x, baseline));
    cx.backend.restore();
}

/// Paint an inline rename input — flat input look (no boxed
/// background) with a subtle primary underline. Text editing,
/// selection, horizontal scroll, and caret blink are owned by
/// `TextInputView`.
#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(super) fn paint_rename_input(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    input: &TextInputState,
    x: f32,
    row_y: f32,
    available_w: f32,
    now_ms: u64,
) {
    paint_rename_input_with_metrics(
        cx,
        theme,
        input,
        x,
        row_y,
        available_w,
        now_ms,
        LayerPanelMetrics::DESKTOP,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_rename_input_with_metrics(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    input: &TextInputState,
    x: f32,
    row_y: f32,
    available_w: f32,
    now_ms: u64,
    metrics: LayerPanelMetrics,
) {
    if available_w <= 0.0 {
        return;
    }
    let input_w = available_w;
    let input_h = metrics.layer_row_height - 4.0;
    let rect = Rect {
        origin: Point2D::new(x - 2.0, row_y),
        size: Point2D::new(input_w + 4.0, input_h),
    };
    paint_text_input_view_value(
        cx,
        theme,
        input,
        rect,
        metrics.row_font,
        2.0,
        if metrics.touch {
            jian_widgets::centered_text_baseline_y(rect, metrics.row_font)
        } else {
            row_y + 17.0
        },
        now_ms,
    );
    // Subtle underline indicates edit mode without the heavy ring.
    let underline = Rect {
        origin: Point2D::new(x - 2.0, row_y + input_h - 1.0),
        size: Point2D::new(input_w + 4.0, 1.0),
    };
    cx.backend.fill_rect(underline, theme.primary);
}

pub(super) fn paint_section_header_with_metrics(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    width: f32,
    label: &str,
    metrics: LayerPanelMetrics,
) {
    let header = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(width, metrics.section_header_height),
    };
    let header_text = TextLayout::single_run(
        label,
        "system-ui",
        metrics.header_font,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let baseline = if metrics.touch {
        jian_widgets::centered_text_baseline_y(header, metrics.header_font)
    } else {
        y + 19.0
    };
    cx.backend
        .draw_text(&header_text, Point2D::new(x + metrics.row_pad_x, baseline));
}
