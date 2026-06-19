//! Inline clone-wizard view for [`GitPanel`] — a port of the TS
//! `GitPanelCloneForm`. Reached from the empty-state Clone card: the
//! user types a remote URL, picks a destination folder, and the host
//! runs `git clone` on a worker thread. Public-repo clone today; the
//! conditional auth (token / SSH) fields are a follow-up.
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. Geometry is shared by paint + hit-test through
//! [`GitPanel::clone_layout`] so they can't drift.

use crate::widgets::git_panel::{GitPanel, GitPanelHit, BUTTON_H, PAD};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::CloneField;

/// Heading baseline.
const HEADING_BASE: f32 = 30.0;
/// Subheading baseline.
const SUBHEAD_BASE: f32 = 50.0;
/// URL label baseline.
const URL_LABEL_BASE: f32 = 80.0;
/// URL input-box top.
const URL_BOX_TOP: f32 = 88.0;
/// Input-box height (URL + destination).
const INPUT_H: f32 = 32.0;
/// Destination label baseline.
const DEST_LABEL_BASE: f32 = URL_BOX_TOP + INPUT_H + 24.0;
/// Destination input-box top.
const DEST_BOX_TOP: f32 = DEST_LABEL_BASE + 8.0;
/// Error-line baseline (space is reserved whether or not it paints, so
/// the action row never shifts).
const ERROR_BASE: f32 = DEST_BOX_TOP + INPUT_H + 18.0;
/// Cancel / Clone action-row top.
const ACTIONS_TOP: f32 = DEST_BOX_TOP + INPUT_H + 28.0;
/// Width of the destination folder-pick button.
const PICK_W: f32 = 60.0;
/// Gap between the dest input + pick button, and between the actions.
const GAP: f32 = 6.0;

/// The clone view's interactive sub-rects, shared by paint + hit-test.
pub(super) struct CloneLayout {
    pub(super) url_input: Rect,
    pub(super) dest_input: Rect,
    pub(super) dest_pick: Rect,
    pub(super) cancel: Rect,
    pub(super) submit: Rect,
}

impl GitPanel<'_> {
    /// Fixed height of the clone view.
    pub(super) fn clone_view_height(&self) -> f32 {
        ACTIONS_TOP + BUTTON_H + PAD
    }

    /// The clone view's interactive sub-rects.
    pub(super) fn clone_layout(&self, panel: Rect) -> CloneLayout {
        let left = panel.origin.x + PAD;
        let inner_w = panel.size.x - PAD * 2.0;
        let url_input = Rect {
            origin: Point2D::new(left, panel.origin.y + URL_BOX_TOP),
            size: Point2D::new(inner_w, INPUT_H),
        };
        let dest_w = (inner_w - PICK_W - GAP).max(40.0);
        let dest_input = Rect {
            origin: Point2D::new(left, panel.origin.y + DEST_BOX_TOP),
            size: Point2D::new(dest_w, INPUT_H),
        };
        let dest_pick = Rect {
            origin: Point2D::new(left + dest_w + GAP, panel.origin.y + DEST_BOX_TOP),
            size: Point2D::new(PICK_W, INPUT_H),
        };
        let half = (inner_w - GAP) / 2.0;
        let actions_top = panel.origin.y + ACTIONS_TOP;
        let cancel = Rect {
            origin: Point2D::new(left, actions_top),
            size: Point2D::new(half, BUTTON_H),
        };
        let submit = Rect {
            origin: Point2D::new(left + half + GAP, actions_top),
            size: Point2D::new(half, BUTTON_H),
        };
        CloneLayout {
            url_input,
            dest_input,
            dest_pick,
            cancel,
            submit,
        }
    }

    /// `true` when the form carries both a URL + destination and is not
    /// already cloning — gates the Clone button.
    fn clone_can_submit(&self) -> bool {
        self.state.clone_form.as_ref().is_some_and(|f| {
            !f.cloning
                && !f.url_input.text().trim().is_empty()
                && !f.dest_input.text().trim().is_empty()
        })
    }

    /// Paint the inline clone wizard.
    pub(super) fn paint_clone(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let Some(form) = &self.state.clone_form else {
            return;
        };
        let t = self.theme;
        let left = rect.origin.x + PAD;
        let top = rect.origin.y;

        self.text(
            cx,
            self.t("git.wizard.clone.heading"),
            left,
            top + HEADING_BASE,
            14.0,
            t.foreground,
        );
        self.text(
            cx,
            self.t("git.wizard.clone.subheading"),
            left,
            top + SUBHEAD_BASE,
            11.0,
            t.muted_foreground,
        );

        let layout = self.clone_layout(rect);
        // URL field.
        self.text(
            cx,
            self.t("git.wizard.clone.urlLabel"),
            left,
            top + URL_LABEL_BASE,
            11.0,
            t.muted_foreground,
        );
        let url_focused = form.focus == Some(CloneField::Url) && !form.cloning;
        self.paint_clone_input_frame(cx, layout.url_input, url_focused);
        self.paint_text_input_view(
            cx,
            layout.url_input,
            &form.url_input,
            self.t("git.wizard.clone.urlPlaceholder"),
            url_focused,
            12.0,
            10.0,
        );
        // Destination field + folder-pick button.
        self.text(
            cx,
            self.t("git.wizard.clone.destLabel"),
            left,
            top + DEST_LABEL_BASE,
            11.0,
            t.muted_foreground,
        );
        let dest_focused = form.focus == Some(CloneField::Dest) && !form.cloning;
        self.paint_clone_input_frame(cx, layout.dest_input, dest_focused);
        self.paint_text_input_view(
            cx,
            layout.dest_input,
            &form.dest_input,
            self.t("git.wizard.clone.destPlaceholder"),
            dest_focused,
            12.0,
            10.0,
        );
        self.paint_button_with_hit(
            cx,
            layout.dest_pick,
            self.t("git.wizard.clone.destPickButton"),
            !form.cloning,
            false,
            Some(GitPanelHit::CloneDestPick),
        );
        // Error line (validation or a failed `git clone`).
        if let Some(err) = &form.error {
            self.text(cx, err, left, top + ERROR_BASE, 11.0, t.destructive);
        }
        // Cancel / Clone.
        // Cancel stays enabled during a clone — it abandons the job.
        self.paint_button_with_hit(
            cx,
            layout.cancel,
            self.t("git.wizard.clone.cancel"),
            true,
            false,
            Some(GitPanelHit::CloneCancel),
        );
        self.paint_button_with_hit(
            cx,
            layout.submit,
            self.t("git.wizard.clone.submit"),
            self.clone_can_submit(),
            true,
            Some(GitPanelHit::CloneSubmit),
        );
    }

    fn paint_clone_input_frame(&self, cx: &mut PaintCx<'_>, rect: Rect, focused: bool) {
        let t = self.theme;
        cx.backend.fill_round_rect(rect, 6.0, t.card);
        let border = if focused {
            alpha(t.primary, 0.5)
        } else {
            alpha(t.border, 0.7)
        };
        cx.backend.stroke_round_rect(rect, 6.0, border, 1.0);
    }

    /// Map a press inside the clone view onto a [`GitPanelHit`].
    pub(super) fn clone_hit(&self, rect: Rect, point: Point2D) -> Option<GitPanelHit> {
        let layout = self.clone_layout(rect);
        // Cancel is always live — it abandons an in-flight clone.
        if layout.cancel.contains(point) {
            return Some(GitPanelHit::CloneCancel);
        }
        // While a clone runs the rest of the form is locked (paint greys
        // it out) — swallow so a "disabled" control can't act.
        if self.state.clone_form.as_ref().is_some_and(|f| f.cloning) {
            return Some(GitPanelHit::Inside);
        }
        if layout.submit.contains(point) {
            // The Clone button paints disabled until both fields are
            // filled — swallow clicks then so its hit behaviour matches.
            return Some(if self.clone_can_submit() {
                GitPanelHit::CloneSubmit
            } else {
                GitPanelHit::Inside
            });
        }
        if layout.dest_pick.contains(point) {
            return Some(GitPanelHit::CloneDestPick);
        }
        if layout.url_input.contains(point) {
            return Some(GitPanelHit::CloneUrlInput);
        }
        if layout.dest_input.contains(point) {
            return Some(GitPanelHit::CloneDestInput);
        }
        // Inside the panel but not on a target — swallow.
        Some(GitPanelHit::Inside)
    }
}

/// A colour at `factor` of its current alpha (Tailwind `/NN`).
fn alpha(c: Color, factor: f32) -> Color {
    crate::util::alpha(c, factor)
}
