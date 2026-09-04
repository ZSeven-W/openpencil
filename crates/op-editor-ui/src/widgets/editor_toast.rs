//! The editor's transient notice banner.
//!
//! One line of text and a dismiss cross, floating over the canvas. It exists
//! for the things the editor does to a user's document that need saying when
//! no panel is listening — see `op_editor_core::editor_toast` for why the slot
//! is single and time-driven.
//!
//! ## Placement: top-centre of the canvas, not bottom
//!
//! The bottom band of the canvas is the busiest chrome in the editor: the
//! vertical Toolbar column hugs the bottom-left, the minimized AI-chat bar
//! docks to the bottom edge on whichever side its anchor picked, the StatusBar
//! sits bottom-right, and the post-import diagnostics card claims the same
//! bottom-right corner. A bottom-centre banner would collide with at least one
//! of them at some viewport width.
//!
//! The top band holds exactly one floating surface — the [`AlignToolbar`],
//! which appears only on a multi-selection — so the toast is placed there and
//! *stacks under* the align toolbar when it is showing rather than fighting it
//! for the same pixels. Nothing else occupies that strip, and it is where the
//! eye already is after a document-level change.
//!
//! [`AlignToolbar`]: crate::widgets::AlignToolbar

use op_editor_core::editor_toast::{EditorToastLevel, EditorToastState};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::EditorState;

use crate::theme::Theme;
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{text_metrics, PaintCx};
use crate::{Point2D, Rect, TextLayout};

/// Banner height. Sized for one line of 12 pt chrome text with the same
/// optical padding the tooltip uses, scaled up.
pub const TOAST_HEIGHT: f32 = 34.0;
pub const TOAST_FONT_SIZE: f32 = 12.0;
pub const TOAST_RADIUS: f32 = 8.0;
/// Padding inside the leading edge, before the message.
const PAD_X: f32 = 14.0;
/// Gap between the message and the dismiss cross.
const DISMISS_GAP: f32 = 12.0;
/// Side of the square dismiss hit area. Larger than the 12 px glyph so the
/// cross is comfortably clickable at any pointer precision.
const DISMISS_HIT: f32 = 24.0;
const DISMISS_GLYPH: f32 = 12.0;
/// Distance from the canvas's top edge, matching the align toolbar's own.
const TOP_INSET: f32 = 16.0;
/// Gap kept below the align toolbar when both are up.
const STACK_GAP: f32 = 8.0;
/// Left-edge clearance for the vertical Toolbar column — the same reserve the
/// align toolbar keeps, so neither floating surface ever covers the tools.
const VERTICAL_TOOLBAR_RESERVE: f32 = 56.0;
/// Widest the banner may grow, however long the sentence is.
const MAX_WIDTH: f32 = 520.0;
/// Narrowest it may shrink to before it is not worth painting at all.
const MIN_WIDTH: f32 = 180.0;

/// What a press landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorToastHit {
    /// The dismiss cross.
    Dismiss,
    /// Inside the banner but on no control — consumed, so a press aimed at the
    /// banner never falls through to the canvas behind it.
    Inside,
    /// Outside — NOT consumed. The banner is non-modal: it never takes a press
    /// the user aimed somewhere else.
    Outside,
}

/// The banner, resolved against a live editor state.
pub struct EditorToast<'a> {
    theme: Theme,
    ui: &'a EditorUiState,
    toast: &'a EditorToastState,
}

impl<'a> EditorToast<'a> {
    /// `None` whenever nothing should paint: empty slot, or the toast's
    /// lifetime has run out at `now_ms`.
    pub fn for_editor(state: &'a EditorState, now_ms: u64) -> Option<Self> {
        let ui = &state.editor_ui;
        let toast = ui.visible_toast(now_ms)?;
        Some(Self {
            theme: theme_for(ui),
            ui,
            toast,
        })
    }

    /// The localized sentence, with the toast's arguments interpolated.
    ///
    /// A key with no locale entry falls back to the key itself rather than to
    /// an empty banner: a silent blank box is a worse bug report than a raw
    /// key, and it is the only signal a missing translation would give.
    pub fn message(&self) -> String {
        match op_i18n::translate_dynamic(self.ui.effective_locale(), &self.toast.i18n_key) {
            Some(template) => op_i18n::interpolate(template, &self.toast.arg_pairs()),
            None => self.toast.i18n_key.clone(),
        }
    }

    pub const fn level(&self) -> EditorToastLevel {
        self.toast.level
    }

    /// Banner width for this message, measured in the family it paints with.
    ///
    /// Measured family-blind the box is born narrower than its own text, which
    /// is the bug the tooltip already documents.
    pub fn width(&self, cx: &mut PaintCx<'_>) -> f32 {
        let text = text_metrics::measure_chrome(cx.backend, &self.message(), TOAST_FONT_SIZE);
        (text + PAD_X * 2.0 + DISMISS_GAP + DISMISS_HIT).clamp(MIN_WIDTH, MAX_WIDTH)
    }

    /// Place a `width`-wide banner in the canvas region.
    ///
    /// `align_toolbar_visible` pushes it below that toolbar instead of under
    /// it — the two are the only floating surfaces in this strip, and stacking
    /// is the whole reason the toast lives at the top (see the module docs).
    ///
    /// `None` when the canvas cannot hold the banner between the tool column
    /// and its right edge; a clipped banner with stale hit-test geometry is
    /// worse than no banner, which is the same rule the align toolbar follows.
    pub fn rect_in_canvas(canvas: Rect, width: f32, align_toolbar_visible: bool) -> Option<Rect> {
        let min_x = canvas.origin.x + VERTICAL_TOOLBAR_RESERVE;
        let max_x = canvas.origin.x + canvas.size.x - width;
        if max_x < min_x {
            return None;
        }
        let centred = canvas.origin.x + (canvas.size.x - width) / 2.0;
        let x = centred.clamp(min_x, max_x);
        let mut y = canvas.origin.y + TOP_INSET;
        if align_toolbar_visible {
            y += crate::widgets::align_toolbar::ALIGN_TOOLBAR_HEIGHT + STACK_GAP;
        }
        if y + TOAST_HEIGHT > canvas.origin.y + canvas.size.y {
            return None;
        }
        Some(Rect::xywh(x, y, width, TOAST_HEIGHT))
    }

    /// The dismiss cross's hit area, right-aligned inside the banner.
    pub fn dismiss_rect(rect: Rect) -> Rect {
        Rect::xywh(
            rect.origin.x + rect.size.x - PAD_X - DISMISS_HIT,
            rect.origin.y + (TOAST_HEIGHT - DISMISS_HIT) / 2.0,
            DISMISS_HIT,
            DISMISS_HIT,
        )
    }

    /// Route a point. Paint and hit-test derive from the same rects, so a
    /// press lands where the cross is drawn.
    pub fn hit_test(rect: Rect, point: Point2D) -> EditorToastHit {
        if !rect.contains(point) {
            return EditorToastHit::Outside;
        }
        if Self::dismiss_rect(rect).contains(point) {
            return EditorToastHit::Dismiss;
        }
        EditorToastHit::Inside
    }

    /// Surface + message + dismiss cross.
    pub fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let theme = &self.theme;
        // `popover` rather than `card`: the banner floats over the canvas
        // rather than sitting in the chrome, and popover is the surface every
        // other floating notice in the editor already uses.
        cx.backend
            .fill_round_rect(rect, TOAST_RADIUS, theme.popover);
        // The level shows in the border, not the fill. A fully tinted banner
        // reads as an error state; a tinted edge marks urgency while keeping
        // the sentence on the same readable surface as every other notice.
        let border = match self.level() {
            EditorToastLevel::Info => theme.border,
            EditorToastLevel::Warn => theme.status_warning,
        };
        cx.backend
            .stroke_round_rect(rect, TOAST_RADIUS, border, 1.0);

        let baseline_y = jian_widgets::centered_text_baseline_y(rect, TOAST_FONT_SIZE);
        let text = TextLayout::single_run(
            &self.message(),
            "system-ui",
            TOAST_FONT_SIZE,
            theme.popover_foreground.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&text, Point2D::new(rect.origin.x + PAD_X, baseline_y));

        let dismiss = Self::dismiss_rect(rect);
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(
                dismiss.origin.x + (DISMISS_HIT - DISMISS_GLYPH) / 2.0,
                dismiss.origin.y + (DISMISS_HIT - DISMISS_GLYPH) / 2.0,
            ),
            DISMISS_GLYPH,
            theme.muted_foreground,
            1.4,
        );
    }
}
