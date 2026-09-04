//! The left rail's tab row — the two-tab header that switches the rail
//! between the Layers tree and the slides navigator.
//!
//! Split out of `slides_panel.rs` at the 800-line ceiling. Pure code
//! motion: `SlidesPanelTabs` is re-exported from the parent, so every
//! import path and test name is unchanged.
//!
//! The row has two modes and picks between them by MEASURING, never by
//! a width threshold — see [`SlidesPanelTabs::new`].

use op_editor_core::{LeftPanelTab, SlidesPanelTarget};

use super::{
    contains, TAB_FONT, TAB_ICON_GAP, TAB_ICON_SIZE, TAB_INSET_X, TAB_INSET_Y, TAB_PAD_X,
    TAB_RADIUS, TAB_ROW_HEIGHT, TOUCH_SLIDES_TAB_ROW_HEIGHT, TOUCH_SLIDES_TAB_TARGET,
};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::text_metrics;
use crate::widgets::top_bar_geometry::estimated_text_width;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout, Theme};

/// Rects of the two-tab row that heads the rail.
///
/// Resolved on its own (not only as part of the full slides layout)
/// because the row also paints — and takes clicks — while the LAYERS
/// tab owns the rest of the rail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlidesPanelTabs {
    pub row: Rect,
    pub layers: Rect,
    pub slides: Rect,
    /// True when the labels did not fit and the row fell back to
    /// icons. The ACTIVE tab keeps its label either way.
    pub compact: bool,
    /// The tab on show — the one that stays a labelled pill in compact
    /// mode. Stored because the two tabs then have different widths, so
    /// the rects themselves depend on it.
    pub active: LeftPanelTab,
}

impl SlidesPanelTabs {
    /// Lay the tab row across the top of `panel`.
    ///
    /// **Which mode the row is in is decided by measuring, never by a
    /// width threshold.** A hardcoded "narrow means under 220 px" would
    /// be wrong the moment a locale has longer words than English, and
    /// wrong again the day a third tab lands. Instead the labels are
    /// costed against the space available and the row drops to icons
    /// only when they genuinely do not fit — so dragging the rail
    /// between its 180 and 480 limits switches modes at the right
    /// pixel, in every language.
    ///
    /// The cost uses [`estimated_text_width`], not the backend's real
    /// shaper, because this rect has to be identical at paint time and
    /// at hit-test time and the hit-test has no backend. The estimate
    /// charges a full em per non-ASCII character, so CJK labels are
    /// over-costed — which is the safe direction: the row drops to
    /// icons a few pixels early rather than painting a label that
    /// clips.
    pub fn new(panel: Rect, active: LeftPanelTab, layers_label: &str, slides_label: &str) -> Self {
        Self::with_row_height(panel, active, layers_label, slides_label, TAB_ROW_HEIGHT)
    }

    /// Touch variant with a 52pt row and two 44pt tab targets.
    pub fn new_touch(
        panel: Rect,
        active: LeftPanelTab,
        layers_label: &str,
        slides_label: &str,
    ) -> Self {
        Self::with_row_height(
            panel,
            active,
            layers_label,
            slides_label,
            TOUCH_SLIDES_TAB_ROW_HEIGHT,
        )
    }

    fn with_row_height(
        panel: Rect,
        active: LeftPanelTab,
        layers_label: &str,
        slides_label: &str,
        row_height: f32,
    ) -> Self {
        let row = Rect {
            origin: panel.origin,
            size: Point2D::new(panel.size.x, row_height),
        };
        let inner_w = (row.size.x - TAB_INSET_X * 2.0).max(0.0);
        let x = row.origin.x + TAB_INSET_X;
        let inset_y = if row_height > TAB_ROW_HEIGHT {
            4.0
        } else {
            TAB_INSET_Y
        };
        let y = row.origin.y + inset_y;
        let h = (row_height - inset_y * 2.0).max(0.0);
        let widest = estimated_text_width(layers_label, TAB_FONT)
            .max(estimated_text_width(slides_label, TAB_FONT));

        if text_tabs_fit(inner_w, 2, widest) {
            let half = inner_w / 2.0;
            return Self {
                row,
                layers: Rect {
                    origin: Point2D::new(x, y),
                    size: Point2D::new(half, h),
                },
                slides: Rect {
                    origin: Point2D::new(x + half, y),
                    size: Point2D::new(half, h),
                },
                compact: false,
                active,
            };
        }

        // Icon mode: the active tab keeps `[icon label]`, the rest
        // shrink to their glyph. Laid out left to right from the inner
        // edge, so the row reads as a toolbar rather than as two
        // stretched halves with nothing in them.
        let touch = row_height > TAB_ROW_HEIGHT;
        let icon_only_w = if touch {
            TOUCH_SLIDES_TAB_TARGET
        } else {
            TAB_ICON_SIZE + TAB_PAD_X * 2.0
        };
        let labelled_w = |label: &str| {
            (TAB_ICON_SIZE + TAB_ICON_GAP + estimated_text_width(label, TAB_FONT) + TAB_PAD_X * 2.0)
                .max(if touch { TOUCH_SLIDES_TAB_TARGET } else { 0.0 })
        };
        let (wanted_layers_w, wanted_slides_w) = match active {
            LeftPanelTab::Layers => (labelled_w(layers_label), icon_only_w),
            LeftPanelTab::Slides => (icon_only_w, labelled_w(slides_label)),
        };
        // Even one pill can overrun a very narrow rail, so each tab is
        // capped at what its predecessors left rather than being
        // allowed to hang off the row's edge.
        let layers_cap = if touch && inner_w >= TOUCH_SLIDES_TAB_TARGET * 2.0 {
            inner_w - TOUCH_SLIDES_TAB_TARGET
        } else {
            inner_w
        };
        let layers_w = wanted_layers_w.min(layers_cap);
        let slides_w = wanted_slides_w.min((inner_w - layers_w).max(0.0));
        Self {
            row,
            layers: Rect {
                origin: Point2D::new(x, y),
                size: Point2D::new(layers_w, h),
            },
            slides: Rect {
                origin: Point2D::new(x + layers_w, y),
                size: Point2D::new(slides_w, h),
            },
            compact: true,
            active,
        }
    }

    /// Which tab `point` lands on, or `None` when it is off the row.
    pub fn hit(&self, point: Point2D) -> Option<SlidesPanelTarget> {
        if !contains(self.row, point) {
            return None;
        }
        if contains(self.layers, point) {
            return Some(SlidesPanelTarget::LayersTab);
        }
        contains(self.slides, point).then_some(SlidesPanelTarget::SlidesTab)
    }

    /// The rail below the tab row — what the Layers tree gets when it
    /// is the tab on show.
    pub fn content_rect(&self, panel: Rect) -> Rect {
        let row_height = self.row.size.y;
        Rect {
            origin: Point2D::new(panel.origin.x, panel.origin.y + row_height),
            size: Point2D::new(panel.size.x, (panel.size.y - row_height).max(0.0)),
        }
    }

    /// Paint the tab row. `hover` is whichever tab the cursor is over,
    /// if any.
    ///
    /// The labels MUST be the same pair `new` was given — they decide
    /// the pill's width there and are drawn inside it here, so a
    /// different string would paint outside its own rect.
    pub fn paint(
        &self,
        cx: &mut PaintCx<'_>,
        theme: &Theme,
        hover: Option<SlidesPanelTarget>,
        layers_label: &str,
        slides_label: &str,
    ) {
        cx.backend.fill_rect(self.row, theme.card);
        if !self.compact {
            // The segmented track, so the two tabs read as one control
            // rather than two loose buttons. Icon mode has no track:
            // there the unselected tabs are bare glyphs, and a rail
            // behind them would read as a button they are inside of.
            cx.backend.fill_round_rect(
                Rect {
                    origin: self.layers.origin,
                    size: Point2D::new(self.layers.size.x + self.slides.size.x, self.layers.size.y),
                },
                TAB_RADIUS,
                theme.muted,
            );
        }
        for (rect, tab, label, icon) in [
            (
                self.layers,
                LeftPanelTab::Layers,
                layers_label,
                Icon::LayersStack,
            ),
            (
                self.slides,
                LeftPanelTab::Slides,
                slides_label,
                Icon::PresentationScreen,
            ),
        ] {
            if rect.size.x <= 0.0 {
                continue;
            }
            let target = match tab {
                LeftPanelTab::Layers => SlidesPanelTarget::LayersTab,
                LeftPanelTab::Slides => SlidesPanelTarget::SlidesTab,
            };
            let selected = self.active == tab;
            if selected {
                cx.backend.fill_round_rect(rect, TAB_RADIUS, theme.card);
                cx.backend
                    .stroke_round_rect(rect, TAB_RADIUS, theme.border, 1.0);
            } else if hover == Some(target) {
                cx.backend
                    .fill_round_rect(rect, TAB_RADIUS, theme.button_hover);
            }
            let color = if selected {
                theme.foreground
            } else {
                theme.muted_foreground
            };
            let baseline = rect.origin.y + rect.size.y / 2.0 + TAB_FONT / 2.0 - 1.5;
            let icon_y = rect.origin.y + (rect.size.y - TAB_ICON_SIZE) / 2.0;

            if !self.compact {
                let width = text_metrics::measure_chrome_weighted(
                    cx.backend,
                    label,
                    TAB_FONT,
                    if selected { 600 } else { 400 },
                );
                cx.backend.draw_text(
                    &TextLayout::single_run(
                        label,
                        "system-ui",
                        TAB_FONT,
                        color.to_jian(),
                        Point2D::ZERO,
                    )
                    .with_font_weight(if selected { 600 } else { 400 }),
                    Point2D::new(rect.origin.x + (rect.size.x - width) / 2.0, baseline),
                );
                continue;
            }

            if !selected {
                // Glyph alone, centred in its square.
                draw_icon(
                    cx.backend,
                    icon,
                    Point2D::new(rect.origin.x + (rect.size.x - TAB_ICON_SIZE) / 2.0, icon_y),
                    TAB_ICON_SIZE,
                    color,
                    1.6,
                );
                continue;
            }
            // The one labelled pill: glyph then label, the pair centred
            // together so the pill does not look left-heavy.
            let label = crate::widgets::file_menu::truncate_to_width(
                cx,
                label,
                TAB_FONT,
                (rect.size.x - TAB_PAD_X * 2.0 - TAB_ICON_SIZE - TAB_ICON_GAP).max(0.0),
            );
            let label_w = text_metrics::measure_chrome_weighted(cx.backend, &label, TAB_FONT, 600);
            let content_w = TAB_ICON_SIZE + TAB_ICON_GAP + label_w;
            let icon_x = rect.origin.x + (rect.size.x - content_w) / 2.0;
            draw_icon(
                cx.backend,
                icon,
                Point2D::new(icon_x, icon_y),
                TAB_ICON_SIZE,
                color,
                1.6,
            );
            if !label.is_empty() {
                cx.backend.draw_text(
                    &TextLayout::single_run(
                        &label,
                        "system-ui",
                        TAB_FONT,
                        color.to_jian(),
                        Point2D::ZERO,
                    )
                    .with_font_weight(600),
                    Point2D::new(icon_x + TAB_ICON_SIZE + TAB_ICON_GAP, baseline),
                );
            }
        }
    }
}

/// Whether `count` equal-width text tabs fit in `inner_w` when the
/// widest label needs `widest_label_w`.
///
/// Equal widths are what makes the widest label govern all of them: a
/// tab is only as roomy as its share, so the row is in text mode only
/// when the LONGEST label clears the padding in ITS share.
///
/// Written over a count rather than hardcoding two so a third tab —
/// Pencil ships five — flows through unchanged and simply switches the
/// row to icons sooner.
pub fn text_tabs_fit(inner_w: f32, count: usize, widest_label_w: f32) -> bool {
    if count == 0 {
        return true;
    }
    let share = inner_w / count as f32;
    share >= widest_label_w + TAB_PAD_X * 2.0
}
