//! Slides panel — the left rail's page-navigator tab.
//!
//! One row per top-level board, in page order, and a row is a CARD: a
//! rounded surface one step off the rail, holding a real rendered
//! thumbnail of the board with a round slide-number chip riding its
//! top-left corner. Clicking a row frames that board; dragging one
//! reorders the deck; the bar pinned to the rail's bottom edge presents
//! the deck and exports it. It is the
//! deck's only navigator — what a slide IS, which one the camera is on
//! and how a reorder commits all come from
//! [`crate::widgets::deck_boards`], so the rail can never disagree with
//! the presentation about the order.
//!
//! **Rows carry no name.** The board's name is already on the board, as
//! the frame label the canvas paints above it; repeating it under every
//! thumbnail bought a second text baseline, a truncation rule and a
//! taller row for information the user is looking straight at. The list
//! is a sequence of pictures — the eye counts positions and recognises
//! slides, which is what a navigator is for.
//!
//! **This widget paints a thumbnail PLACEHOLDER, never a thumbnail.**
//! Rendering a board is platform work — a second skia surface per board
//! — so the host paints its cached rasters into [`SlidesPanelLayout::thumb_rect`]
//! after the widget has painted. A host without a local renderer (the
//! browser) simply paints nothing there and the placeholder stands.
//! Because the blit lands ON the plate, everything that has to sit above
//! a thumbnail lives in [`SlidesPanel::paint_overlay`], which every host
//! calls after its own blit — see that method for the contract.
//!
//! Every rect is derived WITHOUT measuring text — rows are a fixed
//! height for a given rail width — so the host's hit-test and the paint
//! pass compute the same layout from the same four inputs (panel rect,
//! row count, board aspect, scroll offset).

use jian_widgets::centered_text_baseline_y;
use op_editor_core::{SlidesDrag, SlidesPanelTarget};

use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::slides_panel_actions::{
    SlidesActionLabels, SlidesActionLayout, SlidesActionState, ACTION_BAR_HEIGHT,
};
use crate::widgets::text_metrics;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout, Theme};

/// Height of the tab row that heads the rail.
pub const SLIDES_TAB_ROW_HEIGHT: f32 = 36.0;
/// Touch rail tab row. Its 4pt vertical insets leave 44pt tab targets.
pub const TOUCH_SLIDES_TAB_ROW_HEIGHT: f32 = 52.0;
/// Minimum width and height of every touch rail tab target.
pub const TOUCH_SLIDES_TAB_TARGET: f32 = 44.0;
/// Short alias used inside this module's geometry.
const TAB_ROW_HEIGHT: f32 = SLIDES_TAB_ROW_HEIGHT;
/// Height of the bar pinned to the rail's bottom edge.
///
/// Kept as the module's own name because the list band is laid out at
/// `panel − tab row − this`, which is the whole reason the last
/// thumbnail is never covered by the bar.
pub const FOOTER_HEIGHT: f32 = ACTION_BAR_HEIGHT;
const TAB_INSET_X: f32 = 8.0;
const TAB_INSET_Y: f32 = 5.0;
const TAB_RADIUS: f32 = 6.0;
const TAB_FONT: f32 = 12.0;
/// Glyph size for a tab in icon mode.
const TAB_ICON_SIZE: f32 = 14.0;
/// Padding inside a tab, around whatever it holds.
const TAB_PAD_X: f32 = 8.0;
/// Gap between a tab's glyph and its label, when it keeps one.
const TAB_ICON_GAP: f32 = 5.0;

/// Margin between the rail's edge and a card. Shared with the action
/// bar so the buttons line up with the cards above them.
pub(super) const ROW_PAD_X: f32 = 10.0;
/// Padding inside a card, around the thumbnail plate.
const CARD_PAD: f32 = 10.0;
const CARD_RADIUS: f32 = 10.0;
/// Ring width on the selected card.
const CARD_STROKE: f32 = 2.0;
/// Vertical gap between cards.
const ROW_GAP: f32 = 8.0;
/// Corner radius of the thumbnail plate inside a card. Public because a
/// host blitting a rendered board has to clip to the same corners the
/// plate was drawn with, or the picture paints square over a round hole.
pub const SLIDE_THUMB_RADIUS: f32 = 5.0;
/// Diameter of the round slide-number chip.
const CHIP_D: f32 = 22.0;
/// Chip inset from the card's top-left corner.
const CHIP_INSET: f32 = 8.0;
const CHIP_FONT: f32 = 11.0;
/// Opacity of the chip's disc. The chip floats over a rendered board
/// whose colours belong to the DOCUMENT, not the editor, so it carries
/// its own scrim instead of a theme surface — one disc that stays
/// legible over a white cover slide and a black one, in either theme.
const CHIP_SCRIM_ALPHA: f32 = 0.72;
/// Opacity of the hairline around the disc. A dark scrim alone has no
/// shape on a dark slide — the number reads but the chip does not — and
/// a dark deck is exactly the case this panel was built for. The
/// hairline draws the circle there; the scrim draws it on a light one.
const CHIP_EDGE_ALPHA: f32 = 0.18;
/// Rail width the box height is struck at — the shipped default for
/// `layer_panel_width`. It is a REFERENCE, not a constraint: the rail
/// resizes freely and the height below does not follow it.
const REFERENCE_RAIL_W: f32 = 240.0;
/// Fixed height of every row's thumbnail box.
///
/// **The one number that keeps the list a list.** Rows hold this height
/// whether the page carries 16:9 decks, 3:4 cards or 9:19.5 phone
/// screens, and whether the rail is dragged to 180 px or 480 — each
/// board is fitted INTO the box, never the box to the board. Deriving
/// it from the rail's width instead (so a deck always filled its card)
/// looked right at one width and absurd at another: a rail dragged wide
/// gave 500 px slides and a list you could no longer read as a
/// sequence.
///
/// The value is 16:9 at [`REFERENCE_RAIL_W`], so a deck — the common
/// case — fills its card exactly at the width the rail actually opens
/// at, and any other width only changes how much margin sits either
/// side of the picture.
pub const THUMB_BOX_H: f32 =
    (REFERENCE_RAIL_W - (ROW_PAD_X + CARD_PAD) * 2.0) / DEFAULT_BOARD_ASPECT;
/// Fallback board aspect (16:9) for a deck whose boards have no
/// resolvable bounds yet — the scene may not have been built when the
/// first frame paints.
pub const DEFAULT_BOARD_ASPECT: f32 = 16.0 / 9.0;
/// How far a press has to travel before it stops being a click. Matches
/// the canvas node-drag threshold so the two gestures feel the same.
pub const DRAG_THRESHOLD_PX: f32 = 3.0;
const DROP_BAR_H: f32 = 2.0;
const GHOST_ALPHA: f32 = 0.35;

#[path = "slides_panel_tabs.rs"]
mod tabs;

pub use tabs::{text_tabs_fit, SlidesPanelTabs};

/// Where the slides tab's rows, list viewport and action bar sit.
///
/// Built once per event / per paint and shared by both, which is what
/// keeps a row's painted rect and its click target identical.
#[derive(Debug, Clone, PartialEq)]
pub struct SlidesPanelLayout {
    pub panel: Rect,
    pub tabs: SlidesPanelTabs,
    /// The clipped, scrolling band the rows live in.
    pub list: Rect,
    /// The bar pinned to the rail's bottom edge, and everything on it.
    pub actions: SlidesActionLayout,
    /// How far the row stack is scrolled up.
    pub offset: f32,
    pub count: usize,
    /// The thumbnail BOX every row gets — the same for all of them (see
    /// `new`). Individual boards are letterboxed inside it.
    pub thumb_box: Point2D,
    /// Each board's picture size inside the box, in board order.
    thumbs: Vec<Point2D>,
}

impl SlidesPanelLayout {
    /// Lay the slides tab out inside `panel`.
    ///
    /// `aspects` is one width / height per board, in page order.
    ///
    /// **Rows are a FIXED height and boards are fitted into them.** The
    /// height is [`THUMB_BOX_H`] and nothing — not the rail's width,
    /// not the shapes on the page — moves it; only the box's WIDTH
    /// tracks the rail, so dragging the rail wider spreads margin
    /// around the pictures rather than growing them. See [`THUMB_BOX_H`]
    /// for why. It also keeps the drag arithmetic a division instead of
    /// a scan.
    ///
    /// `tabs` is passed in rather than derived because the tab row's
    /// own geometry depends on the labels, and labels are i18n — which
    /// this module deliberately knows nothing about. The flow resolves
    /// both from one place so paint and hit-test cannot disagree.
    ///
    /// `None` when the rail is too small to show a row — a list that
    /// cannot show a slide is worse than no list: it is a strip that
    /// eats clicks and explains nothing.
    ///
    /// **The list band is the panel less the tab row AND less the action
    /// bar.** That subtraction is the only thing keeping the last
    /// thumbnail clear of a bar that does not scroll; every rect that
    /// scrolls is derived from `list`, so there is nowhere else the two
    /// could disagree.
    pub fn new(
        panel: Rect,
        tabs: SlidesPanelTabs,
        aspects: &[f32],
        offset: f32,
        actions: SlidesActionState,
    ) -> Option<Self> {
        let box_w = panel.size.x - (ROW_PAD_X + CARD_PAD) * 2.0;
        if box_w <= 0.0 {
            return None;
        }
        let thumb_box = Point2D::new(box_w, THUMB_BOX_H);
        let thumbs = aspects.iter().map(|a| fit_into(thumb_box, *a)).collect();
        let count = aspects.len();
        let tabs_height = tabs.row.size.y;
        let list_top = panel.origin.y + tabs_height;
        let list_h = (panel.size.y - tabs_height - FOOTER_HEIGHT).max(0.0);
        if list_h <= 0.0 {
            return None;
        }
        let list = Rect {
            origin: Point2D::new(panel.origin.x, list_top),
            size: Point2D::new(panel.size.x, list_h),
        };
        let mut layout = Self {
            panel,
            tabs,
            list,
            actions: SlidesActionLayout::new(panel, list_top, actions),
            offset: 0.0,
            count,
            thumb_box,
            thumbs,
        };
        layout.offset = offset.clamp(0.0, layout.max_scroll());
        Some(layout)
    }

    /// Height of one row: the card, which is the thumbnail box plus its
    /// padding. A constant — the same for every row, on every page, at
    /// every rail width.
    pub fn row_height(&self) -> f32 {
        THUMB_BOX_H + CARD_PAD * 2.0
    }

    /// Row pitch — a row plus the gap that follows it.
    fn row_stride(&self) -> f32 {
        self.row_height() + ROW_GAP
    }

    /// Total height of the row stack, including the trailing padding
    /// that keeps the last row clear of the footer.
    pub fn content_height(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }
        ROW_GAP + self.count as f32 * self.row_stride()
    }

    /// How far the list can scroll.
    pub fn max_scroll(&self) -> f32 {
        (self.content_height() - self.list.size.y).max(0.0)
    }

    /// Rect of row `index`, in screen space. Not clamped to the list: a
    /// row scrolled out of view still has a position, which is what lets
    /// the drag arithmetic work on rows the user cannot see.
    pub fn row_rect(&self, index: usize) -> Rect {
        Rect {
            origin: Point2D::new(
                self.list.origin.x,
                self.list.origin.y - self.offset + ROW_GAP + index as f32 * self.row_stride(),
            ),
            size: Point2D::new(self.list.size.x, self.row_height()),
        }
    }

    /// The painted card of row `index` — the rounded surface the
    /// thumbnail sits inside.
    ///
    /// Narrower than [`Self::row_rect`] on purpose: the card is the
    /// visual row, the row rect is the CLICK target, and letting a
    /// press in the margin beside a card still hit it is what stops the
    /// list feeling fiddly at the rail's edges.
    pub fn card_rect(&self, index: usize) -> Rect {
        let row = self.row_rect(index);
        Rect {
            origin: Point2D::new(row.origin.x + ROW_PAD_X, row.origin.y),
            size: Point2D::new((row.size.x - ROW_PAD_X * 2.0).max(0.0), row.size.y),
        }
    }

    /// The fixed-size box row `index` gives its thumbnail — the card
    /// less its padding, the same shape in every row whatever the board
    /// inside it looks like.
    pub fn thumb_box_rect(&self, index: usize) -> Rect {
        let card = self.card_rect(index);
        Rect {
            origin: Point2D::new(card.origin.x + CARD_PAD, card.origin.y + CARD_PAD),
            size: self.thumb_box,
        }
    }

    /// The round slide-number chip on row `index`, riding the card's
    /// top-left corner over the thumbnail.
    pub fn chip_rect(&self, index: usize) -> Rect {
        let card = self.card_rect(index);
        Rect {
            origin: Point2D::new(card.origin.x + CHIP_INSET, card.origin.y + CHIP_INSET),
            size: Point2D::new(CHIP_D, CHIP_D),
        }
    }

    /// Where row `index`'s board actually paints: its own aspect scaled
    /// to fit [`Self::thumb_box_rect`] and centred in it. This is the
    /// rect a host blits its rendered board into, so the picture never
    /// stretches to a shape the board is not.
    ///
    /// **The box's height leads.** It is the fixed side, so a board
    /// normally fills the row's full height and takes whatever width its
    /// aspect asks for — a tall phone screen becomes a narrow strip
    /// centred in a wide card, which is exactly the reading a navigator
    /// wants. Width only takes over when the aspect would overrun the
    /// box, which is what stops a 16:9 board spilling out of a rail
    /// dragged to its minimum; it letterboxes there instead.
    pub fn thumb_rect(&self, index: usize) -> Rect {
        let boxed = self.thumb_box_rect(index);
        let size = self
            .thumbs
            .get(index)
            .copied()
            .unwrap_or_else(|| fit_into(self.thumb_box, DEFAULT_BOARD_ASPECT));
        Rect {
            origin: Point2D::new(
                boxed.origin.x + (boxed.size.x - size.x) / 2.0,
                boxed.origin.y + (boxed.size.y - size.y) / 2.0,
            ),
            size,
        }
    }

    /// The part of row `index`'s thumbnail that is inside the list
    /// band, or `None` when none of it is.
    ///
    /// A host blitting a rendered board MUST clip to this rather than to
    /// [`Self::thumb_rect`]: the widget clips its own placeholder to the
    /// band, so an unclipped blit would put the last row's picture over
    /// the footer while the placeholder under it stopped at the edge.
    pub fn visible_thumb_rect(&self, index: usize) -> Option<Rect> {
        let thumb = self.thumb_rect(index);
        let top = thumb.origin.y.max(self.list.origin.y);
        let bottom = (thumb.origin.y + thumb.size.y).min(self.list.origin.y + self.list.size.y);
        (bottom > top).then(|| Rect {
            origin: Point2D::new(thumb.origin.x, top),
            size: Point2D::new(thumb.size.x, bottom - top),
        })
    }

    /// The rows with any part inside the list band, paired with their
    /// rects. Hosts render thumbnails for exactly these, so an
    /// off-screen slide never costs a raster.
    pub fn visible_rows(&self) -> Vec<(usize, Rect)> {
        (0..self.count)
            .map(|index| (index, self.row_rect(index)))
            .filter(|(_, rect)| {
                rect.origin.y + rect.size.y > self.list.origin.y
                    && rect.origin.y < self.list.origin.y + self.list.size.y
            })
            .collect()
    }

    /// Which row `point` lands on. Only the part of a row inside the
    /// band counts — a half-scrolled row must not be clickable where it
    /// is not painted.
    pub fn row_at(&self, point: Point2D) -> Option<usize> {
        if !contains(self.list, point) {
            return None;
        }
        self.visible_rows()
            .into_iter()
            .find_map(|(index, rect)| contains(rect, point).then_some(index))
    }

    /// What `point` lands on anywhere in the panel.
    ///
    /// Reverse paint order, like every other hit-test in the app: the
    /// open export dropdown is drawn last and over the thumbnails, so it
    /// answers first — and it answers for its CHROME too, by returning
    /// `None` without falling through. A press on the menu's padding
    /// must not reach the slide row it happens to be covering.
    pub fn hit(&self, point: Point2D) -> Option<SlidesPanelTarget> {
        if self.actions.over_menu(point) {
            return self.actions.menu_row_at(point);
        }
        if let Some(tab) = self.tabs.hit(point) {
            return Some(tab);
        }
        if let Some(button) = self.actions.button_at(point) {
            return Some(button);
        }
        self.row_at(point).map(SlidesPanelTarget::Slide)
    }

    /// Whether `point` is anywhere on the panel. The press path uses
    /// this to decide the press is the panel's business and must not
    /// reach the surfaces below.
    pub fn contains_point(&self, point: Point2D) -> bool {
        contains(self.panel, point)
    }

    /// The slot a row dropped at `pointer_y` would be inserted before,
    /// in the range `0..=count`. Counted from row CENTRES, so the drop
    /// flips as the dragged row passes the middle of its neighbour.
    pub fn insertion_slot(&self, pointer_y: f32) -> usize {
        let content_y = pointer_y - (self.list.origin.y - self.offset) - ROW_GAP;
        (0..self.count)
            .filter(|index| {
                let centre = *index as f32 * self.row_stride() + self.row_height() / 2.0;
                content_y > centre
            })
            .count()
    }

    /// Screen y of the bar marking `slot`, clamped into the band so it
    /// stays visible at either end of a scrolled list.
    fn drop_bar_y(&self, slot: usize) -> f32 {
        let content_y = if slot == 0 {
            ROW_GAP / 2.0
        } else {
            ROW_GAP + (slot - 1) as f32 * self.row_stride() + self.row_height() + ROW_GAP / 2.0
        };
        (self.list.origin.y - self.offset + content_y).clamp(
            self.list.origin.y,
            self.list.origin.y + self.list.size.y - DROP_BAR_H,
        )
    }
}

/// Whether a drag has travelled far enough to be a reorder rather than
/// a click that has not been released yet.
pub fn drag_is_live(drag: &SlidesDrag) -> bool {
    (drag.pointer_y - drag.press_y).abs() > DRAG_THRESHOLD_PX
}

/// The panel, ready to paint.
pub struct SlidesPanel<'a> {
    /// The slide the camera is looking at, if any board resolves.
    pub active: Option<usize>,
    pub hover: Option<SlidesPanelTarget>,
    pub drag: Option<SlidesDrag>,
    /// Whether the host will paint a rendered board over the thumbnail
    /// plate. When it will not, the plate carries a faint slide glyph
    /// instead of staying empty — a bare plate reads as broken, and the
    /// chip already says which slide it is.
    pub thumbnails_supported: bool,
    pub layers_label: &'a str,
    pub slides_label: &'a str,
    /// Labels for the bottom action bar and its dropdown.
    pub actions: SlidesActionLabels<'a>,
}

impl SlidesPanel<'_> {
    pub fn paint(&self, cx: &mut PaintCx<'_>, layout: &SlidesPanelLayout, theme: &Theme) {
        cx.backend.fill_rect(layout.panel, theme.card);
        // Right-edge hairline, so the rail reads as a distinct surface
        // from the canvas next to it — same as the layers tab.
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(
                    layout.panel.origin.x + layout.panel.size.x - 1.0,
                    layout.panel.origin.y,
                ),
                size: Point2D::new(1.0, layout.panel.size.y),
            },
            theme.border,
        );
        layout
            .tabs
            .paint(cx, theme, self.hover, self.layers_label, self.slides_label);

        cx.backend.save();
        cx.backend.clip_rect(layout.list);
        for (index, _) in layout.visible_rows() {
            self.paint_row(cx, theme, layout, index);
        }
        cx.backend.restore();

        crate::widgets::slides_panel_actions::paint_bar(
            cx,
            theme,
            &layout.actions,
            self.actions,
            self.hover,
        );
    }

    /// Everything that has to sit ON TOP of a thumbnail: the number
    /// chips, the ghost of a carried row, and the drop bar.
    ///
    /// **Every host calls this exactly once per frame, after
    /// [`Self::paint`] and after its own thumbnail blit** — that is the
    /// whole reason it is a second method rather than the tail of
    /// `paint`. The chip rides the card's top-left corner, which is
    /// inside the picture, so anything drawn here during `paint` would
    /// be buried by the blit that follows. Splitting it is what keeps
    /// the two hosts identical: the browser, which never blits, gets the
    /// same pixels from the same two calls back to back.
    ///
    /// Once per frame, not "at least once": the carried row's ghost is a
    /// translucent wash, so a second pass would darken it rather than
    /// leave it alone.
    pub fn paint_overlay(&self, cx: &mut PaintCx<'_>, layout: &SlidesPanelLayout, theme: &Theme) {
        cx.backend.save();
        cx.backend.clip_rect(layout.list);
        let dragging = self.drag.filter(drag_is_live);
        for (index, _) in layout.visible_rows() {
            let ghosted = dragging.is_some_and(|drag| drag.from == index);
            if ghosted {
                // Wash the card back towards the rail. Done here rather
                // than by fading each piece as it paints, because the
                // heaviest thing on a carried row is the host's blitted
                // board — which the widget never draws and so cannot
                // fade at the source.
                cx.backend.fill_round_rect(
                    layout.card_rect(index),
                    CARD_RADIUS,
                    fade(theme.card, 1.0 - GHOST_ALPHA),
                );
            }
            self.paint_number_chip(cx, layout, index, ghosted);
        }
        if let Some(drag) = dragging {
            let slot = layout.insertion_slot(drag.pointer_y);
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(layout.list.origin.x + ROW_PAD_X, layout.drop_bar_y(slot)),
                    size: Point2D::new((layout.list.size.x - ROW_PAD_X * 2.0).max(0.0), DROP_BAR_H),
                },
                DROP_BAR_H / 2.0,
                theme.primary,
            );
        }
        cx.backend.restore();

        // Outside the list clip, and last of everything: the dropdown
        // hangs off a control on the rail's bottom edge, so it opens
        // upward and covers the thumbnails it grew into. Painting it
        // inside the clip above would let the list band crop the very
        // rows it is meant to overlay.
        crate::widgets::slides_panel_actions::paint_menu(
            cx,
            theme,
            &layout.actions,
            self.actions,
            self.hover,
        );
    }

    /// One card: its surface, its selection ring, and the plate the
    /// thumbnail lands on.
    ///
    /// Nothing here fades for a carried row — a ghost has to cover the
    /// host's blitted board too, so it is applied once, on top, in
    /// [`Self::paint_overlay`].
    fn paint_row(
        &self,
        cx: &mut PaintCx<'_>,
        theme: &Theme,
        layout: &SlidesPanelLayout,
        index: usize,
    ) {
        let active = self.active == Some(index);
        let hovered = self.hover == Some(SlidesPanelTarget::Slide(index));
        let card = layout.card_rect(index);
        // The card surface, a step off the rail so the list reads as a
        // stack of slides rather than pictures loose on the panel.
        // Selection lifts the same surface another step and rings it,
        // which is why hover under a selected row changes nothing: the
        // card is already as far forward as it goes.
        let surface = match (active, hovered) {
            (true, _) => theme.row_selected,
            (false, true) => theme.accent,
            (false, false) => theme.muted,
        };
        cx.backend.fill_round_rect(card, CARD_RADIUS, surface);
        if active {
            cx.backend
                .stroke_round_rect(card, CARD_RADIUS, theme.primary, CARD_STROKE);
        }
        // Thumbnail plate — the board's own fitted rect, not the box
        // around it, so it sits exactly where the host will blit. A tall
        // board gets a narrow plate rather than a wide one with its
        // picture floating inside. Recessed (the rail's own tone, darker
        // than the card) so an unfilled plate reads as a well.
        let thumb = layout.thumb_rect(index);
        cx.backend
            .fill_round_rect(thumb, SLIDE_THUMB_RADIUS, theme.card);
        if !self.thumbnails_supported {
            // No renderer will cover this plate. The chip already names
            // the slide, so this is a texture, not a second label.
            let size = (thumb.size.y * 0.3).min(32.0);
            draw_icon(
                cx.backend,
                Icon::PresentationScreen,
                Point2D::new(
                    thumb.origin.x + (thumb.size.x - size) / 2.0,
                    thumb.origin.y + (thumb.size.y - size) / 2.0,
                ),
                size,
                fade(theme.muted_foreground, 0.4),
                1.5,
            );
        }
    }

    /// The slide number, as a dark disc over the card's top-left corner.
    fn paint_number_chip(
        &self,
        cx: &mut PaintCx<'_>,
        layout: &SlidesPanelLayout,
        index: usize,
        ghosted: bool,
    ) {
        let alpha = if ghosted { GHOST_ALPHA } else { 1.0 };
        let chip = layout.chip_rect(index);
        cx.backend.fill_round_rect(
            chip,
            CHIP_D / 2.0,
            fade(CHIP_DISC, CHIP_SCRIM_ALPHA * alpha),
        );
        cx.backend.stroke_round_rect(
            chip,
            CHIP_D / 2.0,
            fade(CHIP_INK, CHIP_EDGE_ALPHA * alpha),
            1.0,
        );
        let number = format!("{}", index + 1);
        let width = text_metrics::measure_chrome_weighted(cx.backend, &number, CHIP_FONT, 600);
        cx.backend.draw_text(
            &TextLayout::single_run(
                &number,
                "system-ui",
                CHIP_FONT,
                fade(CHIP_INK, alpha).to_jian(),
                Point2D::ZERO,
            )
            .with_font_weight(600),
            Point2D::new(
                chip.origin.x + (chip.size.x - width) / 2.0,
                centered_text_baseline_y(chip, CHIP_FONT),
            ),
        );
    }
}

/// The chip's disc and its label. Fixed rather than themed — see
/// [`CHIP_SCRIM_ALPHA`].
const CHIP_DISC: Color = Color {
    r: 0.06,
    g: 0.06,
    b: 0.07,
    a: 1.0,
};
const CHIP_INK: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

/// The largest `aspect`-shaped rectangle that fits inside `boxed`.
///
/// A non-finite or non-positive aspect falls back to 16:9 rather than
/// producing a NaN rect — an unresolved board still has to have somewhere
/// to paint, and the scene has not built one on the first frame after a
/// document opens.
fn fit_into(boxed: Point2D, aspect: f32) -> Point2D {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        DEFAULT_BOARD_ASPECT
    };
    let by_width = Point2D::new(boxed.x, boxed.x / aspect);
    if by_width.y <= boxed.y {
        by_width
    } else {
        Point2D::new(boxed.y * aspect, boxed.y)
    }
}

fn fade(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

pub(super) fn contains(rect: Rect, point: Point2D) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.x
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.y
}

#[cfg(test)]
#[path = "slides_panel_tests.rs"]
mod slides_panel_tests;
