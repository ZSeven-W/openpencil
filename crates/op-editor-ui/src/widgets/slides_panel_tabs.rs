//! The left rail's tab row — the header that switches the rail between
//! the Agent conversation, the Layers tree and the slides navigator.
//!
//! Split out of `slides_panel.rs` at the 800-line ceiling.
//! `SlidesPanelTabs` is re-exported from the parent, so every import
//! path and test name is unchanged.
//!
//! The row has two modes and picks between them by MEASURING, never by
//! a width threshold — see [`SlidesPanelTabs::new`]. Which tabs EXIST
//! is the flow's decision ([`SlidesTabRow`]): the Chat tab is
//! desktop-only, the slides tab needs boards, Layers is always there.

use op_editor_core::{LeftPanelTab, SlidesPanelTarget};

use super::{
    contains, TAB_FONT, TAB_GAP, TAB_ICON_GAP, TAB_ICON_SIZE, TAB_INSET_X, TAB_INSET_Y, TAB_PAD_X,
    TAB_RADIUS, TAB_ROW_HEIGHT, TOUCH_SLIDES_TAB_ROW_HEIGHT, TOUCH_SLIDES_TAB_TARGET,
};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::text_metrics;
use crate::widgets::top_bar_geometry::estimated_text_width;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout, Theme};

/// The row's labels plus which of the optional tabs exist.
///
/// Resolved once by the flow for a document, then fed to layout AND
/// paint, so the rects the hit-test uses can never be laid out from a
/// different tab set than the one painted.
pub struct SlidesTabRow<'a> {
    pub chat_label: &'a str,
    pub layers_label: &'a str,
    pub slides_label: &'a str,
    /// The Chat tab is desktop-only: touch chrome hosts the
    /// conversation as its own bottom sheet, so the sheet's row keeps
    /// the two tabs it has always had.
    pub chat_available: bool,
    /// The slides tab needs boards to list — see the flow's
    /// `slides_tab_available`.
    pub slides_available: bool,
}

impl SlidesTabRow<'_> {
    /// The tabs on show, in row order: 对话 · 图层 · 幻灯片 (Pencil
    /// puts Agent first).
    fn tabs(&self) -> Vec<LeftPanelTab> {
        let mut tabs = Vec::with_capacity(3);
        if self.chat_available {
            tabs.push(LeftPanelTab::Chat);
        }
        tabs.push(LeftPanelTab::Layers);
        if self.slides_available {
            tabs.push(LeftPanelTab::Slides);
        }
        tabs
    }

    fn label(&self, tab: LeftPanelTab) -> &str {
        match tab {
            LeftPanelTab::Chat => self.chat_label,
            LeftPanelTab::Layers => self.layers_label,
            LeftPanelTab::Slides => self.slides_label,
        }
    }
}

/// Rects of the tab row that heads the rail.
///
/// Resolved on its own (not only as part of the full slides layout)
/// because the row also paints — and takes clicks — while the LAYERS
/// tab owns the rest of the rail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlidesPanelTabs {
    pub row: Rect,
    pub chat: Rect,
    pub layers: Rect,
    pub slides: Rect,
    /// True when the labels did not fit and the row fell back to
    /// icons. The ACTIVE tab keeps its label either way.
    pub compact: bool,
    /// The tab on show — the one that stays a labelled pill in compact
    /// mode. Stored because the tabs then have different widths, so
    /// the rects themselves depend on it.
    pub active: LeftPanelTab,
}

impl SlidesPanelTabs {
    /// Lay the tab row across the top of `panel`.
    ///
    /// **Which mode the row is in is decided by measuring, never by a
    /// width threshold.** A hardcoded "narrow means under 220 px" would
    /// be wrong the moment a locale has longer words than English, and
    /// wrong again the day a fourth tab lands. Instead the labels are
    /// costed against the space available and the row drops to icons
    /// only when they genuinely do not fit — so dragging the rail
    /// between its 240 and 440 limits switches modes at the right
    /// pixel, in every language.
    ///
    /// The cost uses [`estimated_text_width`], not the backend's real
    /// shaper, because this rect has to be identical at paint time and
    /// at hit-test time and the hit-test has no backend. The estimate
    /// charges a full em per non-ASCII character, so CJK labels are
    /// over-costed — which is the safe direction: the row drops to
    /// icons a few pixels early rather than painting a label that
    /// clips.
    pub fn new(panel: Rect, active: LeftPanelTab, tabs: &SlidesTabRow<'_>) -> Self {
        Self::with_row_height(panel, active, tabs, TAB_ROW_HEIGHT)
    }

    /// Touch variant with a 52pt row and 44pt tab targets.
    pub fn new_touch(panel: Rect, active: LeftPanelTab, tabs: &SlidesTabRow<'_>) -> Self {
        Self::with_row_height(panel, active, tabs, TOUCH_SLIDES_TAB_ROW_HEIGHT)
    }

    fn with_row_height(
        panel: Rect,
        active: LeftPanelTab,
        tabs: &SlidesTabRow<'_>,
        row_height: f32,
    ) -> Self {
        let row = Rect {
            origin: panel.origin,
            size: Point2D::new(panel.size.x, row_height),
        };
        let shown = tabs.tabs();
        let widest = shown
            .iter()
            .map(|tab| estimated_text_width(tabs.label(*tab), TAB_FONT))
            .fold(0.0_f32, f32::max);
        let inner_w = (row.size.x - TAB_INSET_X * 2.0).max(0.0);
        // ONE decision, made here: the rects and the `compact` flag the
        // painter branches on are both derived from it.
        let text_mode = text_tabs_fit(inner_w, shown.len(), widest);
        let rects = Self::layout_rects(row, active, tabs, &shown, text_mode);

        Self {
            row,
            chat: rect_for(&rects, LeftPanelTab::Chat),
            layers: rect_for(&rects, LeftPanelTab::Layers),
            slides: rect_for(&rects, LeftPanelTab::Slides),
            compact: !text_mode,
            active,
        }
    }

    /// Rects for the tabs on show, laid left to right in row order.
    fn layout_rects(
        row: Rect,
        active: LeftPanelTab,
        tabs: &SlidesTabRow<'_>,
        shown: &[LeftPanelTab],
        text_mode: bool,
    ) -> Vec<(LeftPanelTab, Rect)> {
        let inner_w = (row.size.x - TAB_INSET_X * 2.0).max(0.0);
        let x = row.origin.x + TAB_INSET_X;
        let inset_y = if row.size.y > TAB_ROW_HEIGHT {
            4.0
        } else {
            TAB_INSET_Y
        };
        let y = row.origin.y + inset_y;
        let h = (row.size.y - inset_y * 2.0).max(0.0);

        // Text mode: each tab takes the width of its OWN label, laid
        // left to right from the rail's inset. Equal thirds is what made
        // the row read as a segmented control bolted onto the rail
        // instead of part of it; natural widths let the labels sit like
        // the section headings they are, the way Pencil's Agent /
        // Layers / Slides row does. `text_tabs_fit` has already decided
        // the words fit, so the total cannot overrun the rail.
        if text_mode {
            let mut cursor = 0.0_f32;
            // A touch row still owes every tab a 44 pt target, so a short
            // label's natural width is floored there.
            let touch_floor = if row.size.y > TAB_ROW_HEIGHT {
                TOUCH_SLIDES_TAB_TARGET
            } else {
                0.0
            };
            return shown
                .iter()
                .map(|tab| {
                    let w = (estimated_text_width(tabs.label(*tab), TAB_FONT) + TAB_PAD_X * 2.0)
                        .max(touch_floor)
                        .min((inner_w - cursor).max(0.0));
                    let rect = Rect {
                        origin: Point2D::new(x + cursor, y),
                        size: Point2D::new(w, h),
                    };
                    cursor += w + TAB_GAP;
                    (*tab, rect)
                })
                .collect();
        }

        // Icon mode: the active tab keeps `[icon label]`, the rest
        // shrink to their glyph. Laid out left to right from the inner
        // edge, so the row reads as a toolbar rather than stretched
        // halves with nothing in them.
        let touch = row.size.y > TAB_ROW_HEIGHT;
        let icon_only_w = if touch {
            TOUCH_SLIDES_TAB_TARGET
        } else {
            TAB_ICON_SIZE + TAB_PAD_X * 2.0
        };
        let labelled_w = |label: &str| {
            (TAB_ICON_SIZE + TAB_ICON_GAP + estimated_text_width(label, TAB_FONT) + TAB_PAD_X * 2.0)
                .max(if touch { TOUCH_SLIDES_TAB_TARGET } else { 0.0 })
        };
        // Even one pill can overrun a very narrow rail, so each tab is
        // capped at what its predecessors left, minus the minimum
        // target the tabs still to come need on touch.
        let mut rects = Vec::with_capacity(shown.len());
        let mut cursor = 0.0_f32;
        for (index, tab) in shown.iter().enumerate() {
            let remaining = (inner_w - cursor).max(0.0);
            let reserve = if touch {
                TOUCH_SLIDES_TAB_TARGET * (shown.len() - 1 - index) as f32
            } else {
                0.0
            };
            let wanted = if *tab == active {
                labelled_w(tabs.label(*tab))
            } else {
                icon_only_w
            };
            let w = wanted.min((remaining - reserve).max(0.0));
            rects.push((
                *tab,
                Rect {
                    origin: Point2D::new(x + cursor, y),
                    size: Point2D::new(w, h),
                },
            ));
            cursor += w;
        }
        rects
    }

    fn rect_of(&self, tab: LeftPanelTab) -> Rect {
        match tab {
            LeftPanelTab::Chat => self.chat,
            LeftPanelTab::Layers => self.layers,
            LeftPanelTab::Slides => self.slides,
        }
    }

    /// Which tab `point` lands on, or `None` when it is off the row or
    /// on a tab this document does not show (its rect is zero-width).
    pub fn hit(&self, point: Point2D) -> Option<SlidesPanelTarget> {
        if !contains(self.row, point) {
            return None;
        }
        for tab in [
            LeftPanelTab::Chat,
            LeftPanelTab::Layers,
            LeftPanelTab::Slides,
        ] {
            if contains(self.rect_of(tab), point) {
                return Some(match tab {
                    LeftPanelTab::Chat => SlidesPanelTarget::ChatTab,
                    LeftPanelTab::Layers => SlidesPanelTarget::LayersTab,
                    LeftPanelTab::Slides => SlidesPanelTarget::SlidesTab,
                });
            }
        }
        None
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
    /// The labels MUST be the same set `new` was given — they decide
    /// the pill's width there and are drawn inside it here, so a
    /// different string would paint outside its own rect.
    pub fn paint(
        &self,
        cx: &mut PaintCx<'_>,
        theme: &Theme,
        hover: Option<SlidesPanelTarget>,
        tabs: &SlidesTabRow<'_>,
    ) {
        cx.backend.fill_rect(self.row, theme.card);
        // No segmented track. The row is a set of headings on the rail's
        // own ground, and a filled track around them is precisely what
        // made the tabs read as a control sitting on top of the panel
        // rather than the panel's own top edge.
        for (tab, icon) in [
            (LeftPanelTab::Chat, Icon::MessageCircle),
            (LeftPanelTab::Layers, Icon::LayersStack),
            (LeftPanelTab::Slides, Icon::PresentationScreen),
        ] {
            let rect = self.rect_of(tab);
            if rect.size.x <= 0.0 {
                continue;
            }
            let label = tabs.label(tab);
            let target = match tab {
                LeftPanelTab::Chat => SlidesPanelTarget::ChatTab,
                LeftPanelTab::Layers => SlidesPanelTarget::LayersTab,
                LeftPanelTab::Slides => SlidesPanelTarget::SlidesTab,
            };
            let selected = self.active == tab;
            if selected {
                // A soft fill and nothing else: the border used to draw a
                // second box inside the track and read as a button.
                cx.backend.fill_round_rect(rect, TAB_RADIUS, theme.muted);
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

fn rect_for(rects: &[(LeftPanelTab, Rect)], tab: LeftPanelTab) -> Rect {
    rects
        .iter()
        .find(|(candidate, _)| *candidate == tab)
        .map(|(_, rect)| *rect)
        .unwrap_or(Rect::ZERO)
}

/// Whether `count` equal-width text tabs fit in `inner_w` when the
/// widest label needs `widest_label_w`.
///
/// Equal widths are what makes the widest label govern all of them: a
/// tab is only as roomy as its share, so the row is in text mode only
/// when the LONGEST label clears the padding in ITS share.
pub fn text_tabs_fit(inner_w: f32, count: usize, widest_label_w: f32) -> bool {
    if count == 0 {
        return true;
    }
    let share = inner_w / count as f32;
    share >= widest_label_w + TAB_PAD_X * 2.0
}
