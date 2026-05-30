//! No-repo onboarding empty state for [`GitPanel`].
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. When the open document has no git history yet the
//! panel paints a centred clock, a heading, the Init / Open / Clone
//! cards and an "optional" note (TS parity with the TS
//! `git-panel-empty-state`). The Init card is disabled until the
//! document has a saved path; a blue hint pill explains why.

use crate::widgets::git_panel::{GitPanel, EMPTY_STATE_WIDTH};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

/// Icon-box side length for the heading clock glyph.
const EMPTY_ICON_BOX: f32 = 48.0;
const EMPTY_CARD_W: f32 = 96.0;
const EMPTY_CARD_H: f32 = 104.0;
const EMPTY_CARD_GAP: f32 = 8.0;
const EMPTY_CARD_ICON_BOX: f32 = 36.0;
/// Top offset (from the panel's top edge) of the card row.
const EMPTY_CARDS_TOP: f32 = 116.0;
/// The three onboarding cards: (icon, label key, description key).
/// Index 0 (Init) is gated on `has_saved_file`.
const EMPTY_CARDS: [(Icon, &str, &str); 3] = [
    (
        Icon::FilePlus,
        "git.empty.newCard",
        "git.empty.newCardDescription",
    ),
    (
        Icon::FolderOpen,
        "git.empty.openCard",
        "git.empty.openCardDescription",
    ),
    (
        Icon::GitFork,
        "git.empty.cloneCard",
        "git.empty.cloneCardDescription",
    ),
];

impl GitPanel<'_> {
    /// `true` when the panel shows the no-repo onboarding empty state
    /// (clock + Init/Open/Clone cards) — i.e. not loading, not in a
    /// repo, and not in the diff / merge views.
    pub(super) fn is_empty_state(&self) -> bool {
        !self.state.loading
            && !self.state.in_repo
            && self.state.diff.is_none()
            && self.state.merge_resolve.is_none()
    }

    /// The empty-state "Init" card rect (index 0), for the host's
    /// hover tracking — it drives `git_panel.empty_init_hovered`, which
    /// gates the disabled-Init hint pill. `None` outside the empty
    /// state. `rect` is the painted panel body ([`git_panel_rect`]).
    pub fn empty_init_card_rect(&self, rect: Rect) -> Option<Rect> {
        self.is_empty_state()
            .then(|| self.empty_state_rects(rect)[0])
    }

    /// The three card rects for the onboarding empty state — shared by
    /// paint + hit-test so they can't drift.
    pub(super) fn empty_state_rects(&self, rect: Rect) -> [Rect; 3] {
        let center_x = rect.origin.x + EMPTY_STATE_WIDTH / 2.0;
        let row_w = EMPTY_CARD_W * 3.0 + EMPTY_CARD_GAP * 2.0;
        let row_left = center_x - row_w / 2.0;
        let top = rect.origin.y + EMPTY_CARDS_TOP;
        let mut rects = [Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(0.0, 0.0),
        }; 3];
        for (i, r) in rects.iter_mut().enumerate() {
            r.origin = Point2D::new(row_left + i as f32 * (EMPTY_CARD_W + EMPTY_CARD_GAP), top);
            r.size = Point2D::new(EMPTY_CARD_W, EMPTY_CARD_H);
        }
        rects
    }

    /// Horizontally-centred text at `center_x` (baseline `y`).
    fn text_centered(
        &self,
        cx: &mut PaintCx<'_>,
        s: &str,
        center_x: f32,
        baseline_y: f32,
        size: f32,
        color: Color,
    ) {
        let w = cx.backend.measure_text(s, size);
        self.text(cx, s, center_x - w / 2.0, baseline_y, size, color);
    }

    /// Paint the no-repo onboarding empty state (TS parity).
    pub(super) fn paint_empty_state(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let center_x = rect.origin.x + EMPTY_STATE_WIDTH / 2.0;

        // Clock glyph in a rounded, ringed container.
        let box_y = rect.origin.y + 24.0;
        let box_rect = Rect {
            origin: Point2D::new(center_x - EMPTY_ICON_BOX / 2.0, box_y),
            size: Point2D::new(EMPTY_ICON_BOX, EMPTY_ICON_BOX),
        };
        cx.backend.fill_round_rect(box_rect, 14.0, self.theme.muted);
        cx.backend
            .stroke_round_rect(box_rect, 14.0, self.theme.border, 1.0);
        let hist = 22.0;
        draw_icon(
            cx.backend,
            Icon::History,
            Point2D::new(center_x - hist / 2.0, box_y + (EMPTY_ICON_BOX - hist) / 2.0),
            hist,
            self.theme.muted_foreground,
            1.5,
        );

        // Heading.
        self.text_centered(
            cx,
            self.t("git.empty.heading"),
            center_x,
            rect.origin.y + 98.0,
            13.0,
            self.theme.foreground,
        );

        // Init / Open / Clone cards.
        let cards = self.empty_state_rects(rect);
        for (i, (icon, label_key, desc_key)) in EMPTY_CARDS.iter().enumerate() {
            // Init (index 0) is disabled until the doc has a saved path.
            let enabled = i != 0 || self.state.has_saved_file;
            self.paint_empty_card(
                cx,
                cards[i],
                *icon,
                self.t(label_key),
                self.t(desc_key),
                enabled,
            );
        }

        // Footer note. Painted before the disabled-Init hint so the
        // blue pill reads on top of it (TS lets them overlap).
        self.text_centered(
            cx,
            self.t("git.empty.optional"),
            center_x,
            rect.origin.y + 248.0,
            11.0,
            self.theme.muted_foreground,
        );

        // Disabled-Init hint — a blue pill pointing up at the (greyed)
        // Init card, explaining why it's disabled. Shown only while the
        // cursor hovers that card (not persistently — TS parity).
        if !self.state.has_saved_file && self.state.empty_init_hovered {
            self.paint_disabled_init_hint(cx, rect, cards[0]);
        }
    }

    /// One onboarding card: rounded body + icon box + label + desc.
    /// `enabled == false` dims the glyph + label (the disabled Init).
    fn paint_empty_card(
        &self,
        cx: &mut PaintCx<'_>,
        card: Rect,
        icon: Icon,
        label: &str,
        desc: &str,
        enabled: bool,
    ) {
        cx.backend.fill_round_rect(card, 12.0, self.theme.card);
        cx.backend
            .stroke_round_rect(card, 12.0, self.theme.border, 1.0);
        let card_cx = card.origin.x + card.size.x / 2.0;

        let ib = EMPTY_CARD_ICON_BOX;
        let ib_rect = Rect {
            origin: Point2D::new(card_cx - ib / 2.0, card.origin.y + 16.0),
            size: Point2D::new(ib, ib),
        };
        cx.backend.fill_round_rect(ib_rect, 10.0, self.theme.muted);
        let fg = if enabled {
            self.theme.foreground
        } else {
            self.theme.muted_foreground
        };
        let isz = 18.0;
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(card_cx - isz / 2.0, ib_rect.origin.y + (ib - isz) / 2.0),
            isz,
            fg,
            1.75,
        );
        self.text_centered(cx, label, card_cx, card.origin.y + 68.0, 11.0, fg);
        self.text_centered(
            cx,
            desc,
            card_cx,
            card.origin.y + 82.0,
            9.0,
            self.theme.muted_foreground,
        );
    }

    /// The blue "save first" hint pill below the disabled Init card,
    /// with an up-caret pointing at it (TS parity). Clamped to stay
    /// inside the panel so the long localized string never bleeds out.
    fn paint_disabled_init_hint(&self, cx: &mut PaintCx<'_>, panel: Rect, init_card: Rect) {
        const PILL_H: f32 = 24.0;
        const CARET_H: f32 = 6.0;
        const CARET_HALF: f32 = 6.0;
        const MARGIN: f32 = 12.0;
        let blue = Color {
            r: 0.23,
            g: 0.51,
            b: 0.96,
            a: 1.0,
        };
        let white = Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };

        let label = self.t("git.empty.requireSavedFile");
        let pill_w = cx.backend.measure_text(label, 11.0) + 20.0;

        // Anchor near the Init card, then clamp inside the panel so the
        // (often long) localized string never bleeds past the edge.
        let panel_left = panel.origin.x + MARGIN;
        let panel_right = panel.origin.x + EMPTY_STATE_WIDTH - MARGIN;
        let mut pill_left = init_card.origin.x - 6.0;
        if pill_left + pill_w > panel_right {
            pill_left = panel_right - pill_w;
        }
        pill_left = pill_left.max(panel_left);

        let pill_top = init_card.origin.y + init_card.size.y + CARET_H + 2.0;
        let pill = Rect {
            origin: Point2D::new(pill_left, pill_top),
            size: Point2D::new(pill_w, PILL_H),
        };

        // Up-caret toward the Init card, clamped inside the pill.
        let card_center_x = init_card.origin.x + init_card.size.x / 2.0;
        let caret_x = card_center_x.clamp(
            pill_left + CARET_HALF + 2.0,
            pill_left + pill_w - CARET_HALF - 2.0,
        );
        cx.backend.fill_polygon(
            &[
                Point2D::new(caret_x, pill_top - CARET_H),
                Point2D::new(caret_x - CARET_HALF, pill_top + 0.5),
                Point2D::new(caret_x + CARET_HALF, pill_top + 0.5),
            ],
            blue,
        );

        cx.backend.fill_round_rect(pill, 6.0, blue);
        let baseline = pill_top + PILL_H / 2.0 + 4.0;
        self.text(cx, label, pill_left + 10.0, baseline, 11.0, white);
    }
}
