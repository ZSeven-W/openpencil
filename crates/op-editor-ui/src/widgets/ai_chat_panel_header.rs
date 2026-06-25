//! Header tab-row painter for [`super::ai_chat_panel::AIChatPlaceholder`].
//!
//! Extracted from `ai_chat_panel.rs` (which was approaching the 800-line cap)
//! to keep that file under the limit — same pattern as `ai_chat_panel_footer.rs`.
//!
//! ## Layout (left→right inside the tab-row zone)
//!
//! ```text
//! [chevron][  tab 0  ][  tab 1 (active)  ][ tab 2 ][ + ]
//! ```
//!
//! Tab-row zone: `x = panel_left + PAD + CHEVRON_W + PILL_GAP` to
//!               `x = right_edge - NEW_CHAT_D - MAXIMIZE_GAP - MAXIMIZE_W - PILL_RIGHT_GAP`
//!
//! Tabs lay out left→right; leftmost tabs clip under the chevron if they
//! overflow (simple left-clip, no scrollbar). The active tab is a rounded
//! pill with brighter text (+ running spinner). Inactive tabs are muted
//! text only. Hovering any tab reveals a small × close glyph at its right.
//!
//! ## Hit geometry (mirrored in `ai_chat_panel_hit.rs`)
//!
//! `tab_row_rects` returns the same rects used by the painter so hit-test
//! and paint stay pixel-perfect in sync.

use super::ai_chat_panel::{ChatTabInfo, HEADER_HEIGHT, PAD};
use crate::theme::Theme;
use crate::widgets::ai_chat_panel_controls::chat_neutral_feedback_color;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

// ── Layout constants ─────────────────────────────────────────────────────────

/// Width of the collapse chevron icon.
pub(crate) const CHEVRON_W: f32 = 18.0;
/// Gap between chevron and the first tab.
pub(crate) const PILL_GAP: f32 = 6.0;
/// Gap between the last tab and the maximize button.
pub(crate) const PILL_RIGHT_GAP: f32 = 8.0;
/// Diameter of the new-chat "+" circle.
pub(crate) const NEW_CHAT_D: f32 = 24.0;
/// Gap between the "+" circle and the maximize icon.
pub(crate) const MAXIMIZE_GAP: f32 = 6.0;
/// Width of the maximize / minimize icon.
pub(crate) const MAXIMIZE_W: f32 = 18.0;
/// Height of the active-tab pill.
const PILL_H: f32 = 26.0;
/// Corner radius of the active-tab pill.
const PILL_RADIUS: f32 = 8.0;
/// Maximum width per tab (title is ellipsized to fit).
pub(crate) const TAB_MAX_W: f32 = 120.0;
/// Horizontal text padding inside a tab on each side.
const TAB_PAD_X: f32 = 8.0;
/// Width of the × close glyph.
const CLOSE_W: f32 = 12.0;
/// Gap between the tab title and the × glyph.
const CLOSE_GAP: f32 = 4.0;
/// Width reserved for the × (or spinner) at the right edge of a tab when present.
pub(crate) const TAB_RIGHT_INSET: f32 = CLOSE_W + CLOSE_GAP;
/// Font size for tab title text.
pub(crate) const TAB_FONT_SIZE: f32 = 12.0;
/// Diameter of the running-spinner inside the active tab.
const SPINNER_D: f32 = 10.0;

// ── Computed tab-row extents ─────────────────────────────────────────────────

/// Left x-coordinate where the tab row starts (just right of the chevron).
pub(crate) fn tab_row_left(rect: Rect) -> f32 {
    rect.origin.x + PAD + CHEVRON_W + PILL_GAP
}

/// Right x-coordinate where the tab row ends (just left of the maximize icon).
pub(crate) fn tab_row_right(rect: Rect) -> f32 {
    let right_edge = rect.origin.x + rect.size.x - PAD;
    right_edge - NEW_CHAT_D - MAXIMIZE_GAP - MAXIMIZE_W - PILL_RIGHT_GAP
}

/// Available width for all tabs combined.
pub(crate) fn tab_row_width(rect: Rect) -> f32 {
    (tab_row_right(rect) - tab_row_left(rect)).max(0.0)
}

// ── Rect computation (used by BOTH painter and hit-tester) ───────────────────

/// Rects describing a single rendered tab.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TabRect {
    /// Full tab bounding rect (for hit-test of the tab body).
    pub(crate) body: Rect,
    /// Sub-rect of the × close glyph (inside `body`, at its right edge).
    /// Only relevant for paint/hit when the tab is hovered.
    pub(crate) close: Rect,
}

/// Compute the layout rects for all tabs in the tab row.
///
/// Tabs are laid out left→right, each `TAB_MAX_W` wide or narrower if the
/// zone is small. When all tabs together overflow the zone, tabs shift so
/// the rightmost (usually the active) tab stays visible at the right edge.
///
/// Returns one `TabRect` per tab.
pub(crate) fn tab_row_rects(rect: Rect, tab_count: usize) -> Vec<TabRect> {
    if tab_count == 0 {
        return Vec::new();
    }
    let avail_w = tab_row_width(rect);
    // Per-tab width: at most TAB_MAX_W, sharing avail_w equally when tight.
    let tab_w = {
        let share = avail_w / tab_count as f32;
        TAB_MAX_W.min(share.max(24.0))
    };
    let total_w = tab_w * tab_count as f32;

    // Prefer starting at tab_row_left; right-align when tabs overflow.
    let start_x = if total_w <= avail_w {
        tab_row_left(rect)
    } else {
        tab_row_right(rect) - total_w
    };

    let tab_h = PILL_H;
    let tab_y = rect.origin.y + (HEADER_HEIGHT - tab_h) / 2.0;
    let close_size = CLOSE_W;
    let close_y = tab_y + (tab_h - close_size) / 2.0;

    (0..tab_count)
        .map(|i| {
            let x = start_x + i as f32 * tab_w;
            let body = Rect {
                origin: Point2D::new(x, tab_y),
                size: Point2D::new(tab_w, tab_h),
            };
            // × is at the right inset of the tab body.
            let close = Rect {
                origin: Point2D::new(x + tab_w - TAB_RIGHT_INSET, close_y),
                size: Point2D::new(close_size, close_size),
            };
            TabRect { body, close }
        })
        .collect()
}

// ── Tooltip geometry ─────────────────────────────────────────────────────────

/// Height of the "New Chat ⌘T" tooltip box.
pub(crate) const TOOLTIP_H: f32 = 28.0;
/// Width of the "New Chat ⌘T" tooltip box.
pub(crate) const TOOLTIP_W: f32 = 100.0;

/// Rect of the tooltip that appears below-left of the new-chat "+" circle
/// when it is hovered.
pub(crate) fn new_chat_tooltip_rect(rect: Rect) -> Rect {
    let right_edge = rect.origin.x + rect.size.x - PAD;
    let new_chat_x = right_edge - NEW_CHAT_D;
    // Bottom of the "+" circle, but at least at the header bottom so the
    // tooltip is always fully below the header row (required even when
    // NEW_CHAT_D is small enough that the circle's bottom sits above HEADER_HEIGHT).
    let circle_bottom = rect.origin.y + (HEADER_HEIGHT + NEW_CHAT_D) / 2.0;
    let new_chat_bottom = circle_bottom.max(rect.origin.y + HEADER_HEIGHT);
    Rect {
        origin: Point2D::new(new_chat_x + NEW_CHAT_D - TOOLTIP_W, new_chat_bottom + 4.0),
        size: Point2D::new(TOOLTIP_W, TOOLTIP_H),
    }
}

// ── Paint ─────────────────────────────────────────────────────────────────────

/// Paint the full tab row between the chevron and the maximize/+ buttons.
///
/// `tabs` = lightweight per-tab info snapshots; `active_index` = which is
/// active; `tab_hover` = which tab the cursor is over; `is_running` =
/// whether the active tab has agents running (shows spinner instead of ×);
/// `now_ms` = animation clock for spinner rotation.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_header_tabs(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    tabs: &[ChatTabInfo],
    active_index: usize,
    tab_hover: Option<usize>,
    is_running: bool,
    now_ms: u64,
) {
    let tab_count = tabs.len();
    if tab_count == 0 {
        return;
    }

    let rects = tab_row_rects(rect, tab_count);
    let zone_left = tab_row_left(rect);
    let zone_right = tab_row_right(rect);

    // Clip so left-overflowing tabs are hidden behind the chevron.
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        zone_left,
        rect.origin.y,
        (zone_right - zone_left).max(0.0),
        HEADER_HEIGHT,
    ));

    for (i, (tr, tab)) in rects.iter().zip(tabs.iter()).enumerate() {
        let is_active = i == active_index;
        let is_hovered = tab_hover == Some(i);

        if is_active {
            // Rounded pill fill for the active tab.
            cx.backend
                .fill_round_rect(tr.body, PILL_RADIUS, theme.secondary);
        } else if is_hovered {
            // Subtle hover tint for inactive hovered tab.
            cx.backend.fill_round_rect(
                tr.body,
                PILL_RADIUS,
                chat_neutral_feedback_color(theme, false),
            );
        }

        // Title text — ellipsized to fit inside the tab.
        // Reserve space for × (or spinner) when the tab is active or hovered.
        let show_right_inset = is_active || is_hovered;
        let right_inset = if show_right_inset {
            TAB_RIGHT_INSET + 4.0
        } else {
            TAB_PAD_X
        };
        let title_max_w = (tr.body.size.x - TAB_PAD_X - right_inset).max(0.0);
        let title_text = crate::util::ellipsize_to_width(&tab.title, title_max_w, |s| {
            cx.backend.measure_text(s, TAB_FONT_SIZE)
        });
        let text_color = if is_active {
            theme.foreground
        } else {
            Color {
                a: 0.55,
                ..theme.muted_foreground
            }
        };
        let title_layout = TextLayout::single_run(
            &title_text,
            "system-ui",
            TAB_FONT_SIZE,
            text_color.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        // Baseline-relative vertical center: center + fs * 0.35.
        let baseline_y = tr.body.origin.y + PILL_H / 2.0 + TAB_FONT_SIZE * 0.35;
        cx.backend
            .draw_text(&title_layout, Point2D::new(tr.body.origin.x + TAB_PAD_X, baseline_y));

        if is_active && is_running {
            // Spinner arc at the active tab's right inset.
            let spinner_cx =
                tr.body.origin.x + tr.body.size.x - TAB_RIGHT_INSET + CLOSE_W / 2.0 - 2.0;
            let spinner_cy = tr.body.origin.y + PILL_H / 2.0;
            let r = SPINNER_D / 2.0;
            let angle = (now_ms % 1000) as f32 / 1000.0 * std::f32::consts::TAU;
            cx.backend.stroke_oval(
                Rect::xywh(spinner_cx - r, spinner_cy - r, SPINNER_D, SPINNER_D),
                Color {
                    a: 0.3,
                    ..theme.muted_foreground
                },
                1.0,
            );
            cx.backend.stroke_line(
                Point2D::new(spinner_cx, spinner_cy),
                Point2D::new(spinner_cx + angle.cos() * r, spinner_cy + angle.sin() * r),
                Color {
                    a: 0.85,
                    ..theme.primary
                },
                1.5,
            );
        } else if is_hovered {
            // × close glyph — shown ONLY while the tab is hovered (the inset is
            // still reserved on the active tab so the title doesn't reflow).
            draw_icon(
                cx.backend,
                Icon::Close,
                Point2D::new(tr.close.origin.x, tr.close.origin.y),
                CLOSE_W,
                if !is_active {
                    theme.foreground
                } else {
                    theme.muted_foreground
                },
                1.4,
            );
        }
    }

    cx.backend.restore();
}

/// Paint the "New Chat ⌘T" dark tooltip below-left of the "+" button.
/// Appears only when the "+" button is hovered.
pub(crate) fn paint_new_chat_tooltip(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect) {
    let tooltip = new_chat_tooltip_rect(rect);
    const TOOLTIP_RADIUS: f32 = 6.0;

    // Dark tooltip background (dark even in light theme — matches standard tooltip style).
    let bg = Color {
        r: 0.12,
        g: 0.12,
        b: 0.14,
        a: 0.97,
    };
    cx.backend.fill_round_rect(tooltip, TOOLTIP_RADIUS, bg);

    // "New Chat" label on the left.
    let label = "New Chat";
    let font_sz = 11.0;
    let label_layout = TextLayout::single_run(
        label,
        "system-ui",
        font_sz,
        Color::WHITE.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let label_x = tooltip.origin.x + 8.0;
    let label_baseline_y = tooltip.origin.y + TOOLTIP_H / 2.0 + font_sz * 0.35;
    cx.backend
        .draw_text(&label_layout, Point2D::new(label_x, label_baseline_y));

    // Keycap chips: ⌘ and T, right-aligned inside the tooltip.
    let chip_h: f32 = 16.0;
    let chip_radius = 3.0;
    let chip_font = 10.0;
    let chip_bg = Color {
        r: 0.28,
        g: 0.28,
        b: 0.32,
        a: 1.0,
    };
    let chip_text_color = Color {
        r: 0.85,
        g: 0.85,
        b: 0.88,
        a: 1.0,
    };
    let chip_y = tooltip.origin.y + (TOOLTIP_H - chip_h) / 2.0;
    let pad_x = 5.0;
    let gap = 3.0;

    let cmd_sym = "⌘";
    let t_sym = "T";
    let cmd_text_w = cx.backend.measure_text(cmd_sym, chip_font);
    let t_text_w = cx.backend.measure_text(t_sym, chip_font);
    let cmd_w = cmd_text_w + pad_x * 2.0;
    let t_w = t_text_w + pad_x * 2.0;
    let chips_total = cmd_w + gap + t_w;
    let chips_right = tooltip.origin.x + tooltip.size.x - 6.0;
    let cmd_x = chips_right - chips_total;
    let t_x = cmd_x + cmd_w + gap;

    // ⌘ chip.
    let cmd_rect = Rect::xywh(cmd_x, chip_y, cmd_w, chip_h);
    cx.backend.fill_round_rect(cmd_rect, chip_radius, chip_bg);
    let cmd_layout = TextLayout::single_run(
        cmd_sym,
        "system-ui",
        chip_font,
        chip_text_color.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &cmd_layout,
        Point2D::new(cmd_x + pad_x, chip_y + chip_h / 2.0 + chip_font * 0.35),
    );

    // T chip.
    let t_rect = Rect::xywh(t_x, chip_y, t_w, chip_h);
    cx.backend.fill_round_rect(t_rect, chip_radius, chip_bg);
    let t_layout = TextLayout::single_run(
        t_sym,
        "system-ui",
        chip_font,
        chip_text_color.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &t_layout,
        Point2D::new(t_x + pad_x, chip_y + chip_h / 2.0 + chip_font * 0.35),
    );

    // Suppress unused-variable warning for `theme` — it is kept as a param
    // for future styling.
    let _ = theme;
}

/// Determine which tab (if any) the cursor hits, and whether it is on the
/// close × sub-rect. Returns `(tab_index, over_close_button)`.
///
/// `tab_hover` is the currently-tracked hover index (from `EditorUiState`);
/// the × is only "live" when the cursor is on the tab that is already hovered.
pub(crate) fn tab_hit_at(
    rect: Rect,
    tab_count: usize,
    point: Point2D,
    tab_hover: Option<usize>,
) -> Option<(usize, bool)> {
    let zone_left = tab_row_left(rect);
    let zone_right = tab_row_right(rect);
    if point.x < zone_left || point.x > zone_right {
        return None;
    }
    let rects = tab_row_rects(rect, tab_count);
    for (i, tr) in rects.iter().enumerate() {
        if tr.body.contains(point) {
            // The × is only active when this tab is the currently hovered one.
            let over_close = tab_hover == Some(i) && tr.close.contains(point);
            return Some((i, over_close));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::ai_chat_panel::AI_CHAT_WIDTH;
    use crate::widgets::ai_chat_panel::AI_CHAT_HEIGHT;

    fn panel_rect() -> Rect {
        Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT)
    }

    fn make_tabs(n: usize) -> Vec<ChatTabInfo> {
        (0..n)
            .map(|i| ChatTabInfo {
                title: format!("Tab {i}"),
            })
            .collect()
    }

    #[test]
    fn tab_row_rects_single_tab_starts_at_zone_left() {
        let rect = panel_rect();
        let rects = tab_row_rects(rect, 1);
        assert_eq!(rects.len(), 1);
        let expected_x = tab_row_left(rect);
        assert!(
            (rects[0].body.origin.x - expected_x).abs() < 0.01,
            "single tab should start at tab_row_left"
        );
        assert!(rects[0].body.size.x > 0.0);
    }

    #[test]
    fn tab_row_rects_two_tabs_are_adjacent_and_non_overlapping() {
        let rect = panel_rect();
        let rects = tab_row_rects(rect, 2);
        assert_eq!(rects.len(), 2);
        let t0 = &rects[0];
        let t1 = &rects[1];
        // Adjacent: tab 1 starts where tab 0 ends.
        assert!(
            (t1.body.origin.x - (t0.body.origin.x + t0.body.size.x)).abs() < 0.01,
            "tabs must be adjacent: t0_right={}, t1_left={}",
            t0.body.origin.x + t0.body.size.x,
            t1.body.origin.x
        );
        // Equal width.
        assert!(
            (t0.body.size.x - t1.body.size.x).abs() < 0.01,
            "two tabs should share the zone equally"
        );
    }

    #[test]
    fn tab_row_rects_active_tab_pill_does_not_overlap_inactive() {
        let rect = panel_rect();
        let rects = tab_row_rects(rect, 2);
        // Verify no overlap between the two tabs.
        let r0_right = rects[0].body.origin.x + rects[0].body.size.x;
        let r1_left = rects[1].body.origin.x;
        assert!(
            r0_right <= r1_left + 0.01,
            "tab 0 right ({r0_right}) must not exceed tab 1 left ({r1_left})"
        );
    }

    #[test]
    fn tab_row_rects_close_rect_inside_body() {
        let rect = panel_rect();
        let rects = tab_row_rects(rect, 2);
        for tr in &rects {
            assert!(tr.close.origin.x >= tr.body.origin.x - 0.01);
            assert!(
                tr.close.origin.x + tr.close.size.x <= tr.body.origin.x + tr.body.size.x + 0.01
            );
        }
    }

    #[test]
    fn tab_hit_at_body_returns_correct_index() {
        let rect = panel_rect();
        let tabs = make_tabs(2);
        let _ = tabs; // tabs used for count below
        let rects = tab_row_rects(rect, 2);
        let center1 = Point2D::new(
            rects[1].body.origin.x + rects[1].body.size.x / 2.0,
            rects[1].body.origin.y + rects[1].body.size.y / 2.0,
        );
        let result = tab_hit_at(rect, 2, center1, None);
        assert_eq!(result, Some((1, false)));
    }

    #[test]
    fn tab_hit_at_close_requires_hover_state() {
        let rect = panel_rect();
        let rects = tab_row_rects(rect, 2);
        let close_center = Point2D::new(
            rects[0].close.origin.x + rects[0].close.size.x / 2.0,
            rects[0].close.origin.y + rects[0].close.size.y / 2.0,
        );
        // Without hover on tab 0, over_close=false.
        assert_eq!(tab_hit_at(rect, 2, close_center, None), Some((0, false)));
        // With hover on tab 0, over_close=true.
        assert_eq!(tab_hit_at(rect, 2, close_center, Some(0)), Some((0, true)));
        // With hover on a different tab, over_close stays false.
        assert_eq!(tab_hit_at(rect, 2, close_center, Some(1)), Some((0, false)));
    }

    #[test]
    fn tab_hit_outside_zone_returns_none() {
        let rect = panel_rect();
        // Far left of tab zone (inside chevron area).
        let p = Point2D::new(rect.origin.x + PAD, rect.origin.y + HEADER_HEIGHT / 2.0);
        assert_eq!(tab_hit_at(rect, 2, p, None), None);
    }

    #[test]
    fn new_chat_tooltip_rect_is_below_header_and_within_panel() {
        let rect = panel_rect();
        let tip = new_chat_tooltip_rect(rect);
        // The tooltip must appear below the header row, not just below the
        // panel top edge — "below header" means y >= rect.origin.y + HEADER_HEIGHT.
        assert!(
            tip.origin.y >= rect.origin.y + HEADER_HEIGHT,
            "tooltip must be below the header (y={} < rect.y={} + HEADER_HEIGHT={})",
            tip.origin.y,
            rect.origin.y,
            HEADER_HEIGHT,
        );
        assert!(
            tip.origin.x + tip.size.x <= rect.origin.x + rect.size.x + 2.0,
            "tooltip must not exceed panel right edge"
        );
        assert!(tip.size.x > 0.0 && tip.size.y > 0.0);
    }
}
