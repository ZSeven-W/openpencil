//! No-repo onboarding empty state for [`GitPanel`].
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. When the open document has no git history yet the
//! panel paints a centred clock, a heading, the Init / Open / Clone
//! cards and an "optional" note (TS parity with the TS
//! `git-panel-empty-state`). The Init card is disabled until the
//! document has a saved path; a hover hint explains why.

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
/// Top offset (from the panel's top edge) of the card row — `pt-8`
/// (32) + clock (48) + `gap-5` (20) + heading line (~16) + `gap-5`.
const EMPTY_CARDS_TOP: f32 = 136.0;
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
    /// repo, and not in the diff / merge / clone-wizard views. (The
    /// clone wizard opens *from* this state, so it must take over.)
    pub(super) fn is_empty_state(&self) -> bool {
        !self.state.loading
            && !self.state.in_repo
            && self.state.diff.is_none()
            && self.state.merge_resolve.is_none()
            && self.state.clone_form.is_none()
    }

    /// The empty-state "Init" card rect (index 0) — the host uses it
    /// for the disabled-Init not-allowed cursor. `None` outside the
    /// empty state. `rect` is the painted body ([`git_panel_rect`]).
    pub fn empty_init_card_rect(&self, rect: Rect) -> Option<Rect> {
        self.is_empty_state()
            .then(|| self.empty_state_rects(rect)[0])
    }

    /// Index of the empty-state card under `point` (`0` Init / `1` Open
    /// / `2` Clone), or `None`. Drives `git_panel.empty_hovered_card`
    /// (the per-card hover effect + the disabled-Init hint). `rect` is
    /// the painted body ([`git_panel_rect`]).
    pub fn empty_card_at(&self, rect: Rect, point: Point2D) -> Option<u8> {
        if !self.is_empty_state() {
            return None;
        }
        self.empty_state_rects(rect)
            .iter()
            .position(|r| r.contains(point))
            .map(|i| i as u8)
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

    /// Paint the no-repo onboarding empty state — a faithful port of
    /// the TS `GitPanelEmptyState`: a gradient clock badge, heading,
    /// the Init / Open / Clone cards (with per-card hover lift/recolor)
    /// and the optional-note, plus the disabled-Init hover hint.
    pub(super) fn paint_empty_state(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let t = self.theme;
        let center_x = rect.origin.x + EMPTY_STATE_WIDTH / 2.0;

        // Clock badge — TS `rounded-2xl bg-gradient-to-br
        // from-muted/60 to-muted/20 ring-1 ring-inset ring-border/60`.
        let box_y = rect.origin.y + 32.0;
        let box_rect = Rect {
            origin: Point2D::new(center_x - EMPTY_ICON_BOX / 2.0, box_y),
            size: Point2D::new(EMPTY_ICON_BOX, EMPTY_ICON_BOX),
        };
        cx.backend.fill_round_rect_linear_gradient(
            box_rect,
            16.0,
            &[(0.0, alpha(t.muted, 0.60)), (1.0, alpha(t.muted, 0.20))],
            135.0,
            1.0,
        );
        cx.backend
            .stroke_round_rect(box_rect, 16.0, alpha(t.border, 0.60), 1.0);
        let hist = 22.0;
        draw_icon(
            cx.backend,
            Icon::History,
            Point2D::new(center_x - hist / 2.0, box_y + (EMPTY_ICON_BOX - hist) / 2.0),
            hist,
            t.muted_foreground,
            1.5,
        );

        // Heading — TS `text-[13px] font-medium text-foreground`.
        self.text_centered(
            cx,
            self.t("git.empty.heading"),
            center_x,
            rect.origin.y + 113.0,
            13.0,
            t.foreground,
        );

        // Init / Open / Clone cards.
        let cards = self.empty_state_rects(rect);
        for (i, (icon, label_key, desc_key)) in EMPTY_CARDS.iter().enumerate() {
            // Init (index 0) is disabled until the doc has a saved path.
            let enabled = i != 0 || self.state.has_saved_file;
            let hovered = self.state.empty_hovered_card == Some(i as u8);
            self.paint_empty_card(
                cx,
                cards[i],
                *icon,
                self.t(label_key),
                self.t(desc_key),
                enabled,
                hovered,
            );
        }

        // Footer note — TS `text-[11px] text-muted-foreground/80`.
        // Painted before the hint so the pill reads on top of it.
        self.text_centered(
            cx,
            self.t("git.empty.optional"),
            center_x,
            rect.origin.y + 271.0,
            11.0,
            alpha(t.muted_foreground, 0.80),
        );

        // Disabled-Init hint — shown only while the cursor hovers the
        // greyed Init card (TS `<Tooltip side="bottom">`).
        if !self.state.has_saved_file && self.state.empty_hovered_card == Some(0) {
            self.paint_disabled_init_hint(cx, rect, cards[0]);
        }
    }

    /// One onboarding card — a port of the TS `EmptyStateCard`: rounded
    /// body + icon box + label + description. `enabled == false` renders
    /// the TS `opacity-50` disabled look; `hovered` drives the TS
    /// `hover:` / `group-hover:` lift + recolor.
    #[allow(clippy::too_many_arguments)]
    fn paint_empty_card(
        &self,
        cx: &mut PaintCx<'_>,
        card: Rect,
        icon: Icon,
        label: &str,
        desc: &str,
        enabled: bool,
        hovered: bool,
    ) {
        let t = self.theme;
        // Whole-card dim for the disabled state (TS `opacity-50`).
        let o = if enabled { 1.0 } else { 0.5 };
        let active = enabled && hovered;
        // TS `hover:-translate-y-px` — the card lifts 1px on hover.
        let card = if active {
            Rect {
                origin: Point2D::new(card.origin.x, card.origin.y - 1.0),
                size: card.size,
            }
        } else {
            card
        };

        // Body — hover `bg-accent/30` over `bg-card`, else `bg-card`.
        let body = if active {
            over(t.card, alpha(t.accent, 0.30))
        } else {
            t.card
        };
        cx.backend.fill_round_rect(card, 12.0, alpha(body, o));
        // Border — hover `border-primary/40`, else `border-border/70`
        // (enabled) / `border-border/60` (disabled).
        let border = if active {
            alpha(t.primary, 0.40)
        } else if enabled {
            alpha(t.border, 0.70)
        } else {
            alpha(t.border, 0.60)
        };
        cx.backend
            .stroke_round_rect(card, 12.0, alpha(border, o), 1.0);

        let card_cx = card.origin.x + card.size.x / 2.0;
        let ib = EMPTY_CARD_ICON_BOX;
        let ib_rect = Rect {
            origin: Point2D::new(card_cx - ib / 2.0, card.origin.y + 14.0),
            size: Point2D::new(ib, ib),
        };
        // Icon box — hover `bg-primary/10`, else `bg-muted/60`
        // (enabled) / `bg-muted/50` (disabled).
        let ib_fill = if active {
            alpha(t.primary, 0.10)
        } else if enabled {
            alpha(t.muted, 0.60)
        } else {
            alpha(t.muted, 0.50)
        };
        // TS icon box is `rounded-lg` (8px), inside the `rounded-xl` card.
        cx.backend.fill_round_rect(ib_rect, 8.0, alpha(ib_fill, o));
        // Icon — hover `text-primary`, else `text-foreground/80`
        // (enabled) / `text-muted-foreground` (disabled).
        let icon_color = if active {
            t.primary
        } else if enabled {
            alpha(t.foreground, 0.80)
        } else {
            t.muted_foreground
        };
        let isz = 18.0;
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(card_cx - isz / 2.0, ib_rect.origin.y + (ib - isz) / 2.0),
            isz,
            alpha(icon_color, o),
            1.75,
        );
        // Label (semibold, `text-foreground`) + description
        // (`text-muted-foreground`).
        self.text_centered(
            cx,
            label,
            card_cx,
            card.origin.y + 66.0,
            11.0,
            alpha(t.foreground, o),
        );
        self.text_centered(
            cx,
            desc,
            card_cx,
            card.origin.y + 80.0,
            9.0,
            alpha(t.muted_foreground, o),
        );
    }

    /// The "save first" hint below the disabled Init card — a port of
    /// the TS `<Tooltip side="bottom">` (`bg-primary
    /// text-primary-foreground rounded-md px-3 text-xs`) with a small
    /// up-arrow. Clamped to stay inside the panel so the localized
    /// string never bleeds out.
    fn paint_disabled_init_hint(&self, cx: &mut PaintCx<'_>, panel: Rect, init_card: Rect) {
        const PILL_H: f32 = 24.0;
        const CARET_H: f32 = 6.0;
        const CARET_HALF: f32 = 6.0;
        const MARGIN: f32 = 12.0;
        let bg = self.theme.primary;
        let fg = self.theme.primary_foreground;

        let label = self.t("git.empty.requireSavedFile");
        // TS `px-3` (12) each side around a `text-xs` (12 px) label.
        let pill_w = cx.backend.measure_text(label, 12.0) + 24.0;

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

        // Up-arrow toward the Init card, clamped inside the pill.
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
            bg,
        );

        cx.backend.fill_round_rect(pill, 6.0, bg);
        let baseline = pill_top + PILL_H / 2.0 + 4.0;
        self.text(cx, label, pill_left + 12.0, baseline, 12.0, fg);
    }
}

/// A colour at `factor` of its current alpha — the Rust analogue of a
/// Tailwind `/NN` opacity modifier (e.g. `bg-muted/60`).
fn alpha(c: Color, factor: f32) -> Color {
    crate::util::alpha(c, factor)
}

/// `top` (a translucent colour) composited over the opaque `base`,
/// yielding an opaque colour — so a `bg-accent/30`-over-`bg-card` hover
/// fill paints in a single pass.
fn over(base: Color, top: Color) -> Color {
    crate::util::over(base, top)
}
