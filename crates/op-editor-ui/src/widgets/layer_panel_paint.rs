//! Paint helpers + small leaf utilities for `LayerPanel`,
//! extracted to keep `layer_panel.rs` under the 800-line cap.

use crate::theme::Theme;
use crate::widgets::icons::draw_icon;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

use super::layer_panel::{LayerItem, LAYER_ROW_HEIGHT, ROW_PAD_X};

pub(super) const HEADER_FONT: f32 = 12.0;
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

pub(super) fn paint_drag_ghost(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ghost: &LayerItem,
    cursor_y: f32,
    panel_rect: Rect,
) {
    let row = Rect {
        origin: Point2D::new(panel_rect.origin.x + 6.0, cursor_y - LAYER_ROW_HEIGHT / 2.0),
        size: Point2D::new(panel_rect.size.x - 12.0, LAYER_ROW_HEIGHT - 4.0),
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
    let icon_x = row.origin.x + ROW_PAD_X + ghost.depth as f32 * 12.0 + 18.0;
    draw_icon(
        cx.backend,
        ghost.icon,
        Point2D::new(icon_x, row.origin.y + 6.0),
        14.0,
        fg,
        1.4,
    );
    let label = TextLayout::single_run(
        &ghost.label,
        "system-ui",
        ROW_FONT,
        (fg).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label, Point2D::new(icon_x + 20.0, row.origin.y + 17.0));
}

/// Paint an inline rename input — flat input look (no boxed
/// background) with a subtle primary underline + blinking caret.
/// Tightened caret-to-text gap and uses `blink_visible` so the
/// caret pulses at the same cadence as the chat / property input.
// Paint-context + geometry args threaded through; a struct adds no gain.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_rename_input(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    draft: &str,
    caret: usize,
    x: f32,
    row_y: f32,
    available_w: f32,
    now_ms: u64,
    caret_anchor_ms: u64,
    select_all: bool,
) {
    let input_w = available_w.max(40.0);
    let input_h = LAYER_ROW_HEIGHT - 4.0;
    // No truncation while editing — full draft scrolls so the caret
    // stays in view. Use the backend's real glyph measurement so CJK /
    // capital-heavy strings don't under-scroll (the 0.5-factor
    // heuristic does for narrow ASCII).
    let prefix: String = draft.chars().take(caret).collect();
    let prefix_w = cx.backend.measure_text(&prefix, ROW_FONT);
    let text_w = cx.backend.measure_text(draft, ROW_FONT);
    let caret_pad = 2.0;
    // Anchor the scroll to the caret (so mid-string editing keeps the
    // caret visible), clamped so we never scroll past the draft end.
    let max_scroll = (text_w + caret_pad - input_w).max(0.0);
    let scroll_x = (prefix_w + caret_pad - input_w).max(0.0).min(max_scroll);
    let clip = Rect {
        origin: Point2D::new(x - 2.0, row_y),
        size: Point2D::new(input_w + 4.0, input_h),
    };
    cx.backend.save();
    cx.backend.clip_rect(clip);
    // Ctrl/Cmd+A highlights the whole rename draft (painted behind it).
    if select_all {
        crate::widgets::text_selection::paint_single_line_selection(
            cx,
            theme,
            draft,
            x - scroll_x,
            row_y + 17.0,
            ROW_FONT,
            clip.origin.x + clip.size.x,
        );
    }
    let text = TextLayout::single_run(
        draft,
        "system-ui",
        ROW_FONT,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&text, Point2D::new(x - scroll_x, row_y + 17.0));
    if jian_core::anim::blink_visible(now_ms, caret_anchor_ms, 500) {
        let caret_x = x + prefix_w - scroll_x;
        let caret_rect = Rect {
            origin: Point2D::new(caret_x, row_y + 5.0),
            size: Point2D::new(1.0, input_h - 6.0),
        };
        cx.backend.fill_rect(caret_rect, theme.foreground);
    }
    cx.backend.restore();
    // Subtle underline indicates edit mode without the heavy ring.
    let underline = Rect {
        origin: Point2D::new(x - 2.0, row_y + input_h - 1.0),
        size: Point2D::new(input_w + 4.0, 1.0),
    };
    cx.backend.fill_rect(underline, theme.primary);
}

pub(super) fn paint_section_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    y: f32,
    _width: f32,
    label: &str,
) {
    let header_text = TextLayout::single_run(
        label,
        "system-ui",
        HEADER_FONT,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&header_text, Point2D::new(x + ROW_PAD_X, y + 19.0));
}
