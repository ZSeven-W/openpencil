//! Interactive merge-conflict-resolution view for [`GitPanel`].
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. When a branch merge conflicts entirely in `.op`
//! files this view replaces the panel body: each conflicting
//! PenNode gets an Ours / Theirs choice, then "Apply" re-runs the
//! merge with those choices and completes it.

use crate::widgets::git_panel::{truncate, GitPanel, GitPanelHit, BUTTON_H, FOOTER_H, PAD};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

/// Title baseline within the resolution view.
const TITLE_BASELINE: f32 = 30.0;
/// Subtitle baseline.
const SUBTITLE_BASELINE: f32 = 50.0;
/// Divider y.
const DIVIDER_Y: f32 = 60.0;
/// Top of the first conflict row.
const ROWS_TOP: f32 = 70.0;
/// Conflict-row height.
const ROW_H: f32 = 26.0;
/// Ours / Theirs choice-button width + gap.
const CHOICE_W: f32 = 46.0;
const CHOICE_GAP: f32 = 6.0;
/// Gap from the last row to the Apply / Cancel buttons.
const ACTION_GAP: f32 = 14.0;
/// Most conflict rows the view shows — extras keep their default
/// (ours) and are noted in the footer.
const MAX_ROWS: usize = 12;

/// The interactive sub-rects of the resolution view.
pub(super) struct ResolveLayout {
    /// Per shown conflict row — `(ours button, theirs button)`.
    pub(super) rows: Vec<(Rect, Rect)>,
    /// The "Apply" button.
    pub(super) apply: Rect,
    /// The "Cancel" button.
    pub(super) cancel: Rect,
}

impl GitPanel<'_> {
    /// Conflict rows actually shown (the total, capped at `MAX_ROWS`).
    fn resolve_rows_shown(&self) -> usize {
        self.state
            .merge_resolve
            .as_ref()
            .map(|m| m.total().min(MAX_ROWS))
            .unwrap_or(0)
    }

    /// Fixed height of the resolution view for the current conflict
    /// count.
    pub(super) fn resolve_view_height(&self) -> f32 {
        let rows = self.resolve_rows_shown() as f32;
        ROWS_TOP + rows * ROW_H + ACTION_GAP + BUTTON_H + FOOTER_H + PAD
    }

    /// The resolution view's interactive sub-rects.
    pub(super) fn resolve_layout(&self, panel: Rect) -> ResolveLayout {
        let left = panel.origin.x + PAD;
        let inner_w = panel.size.x - PAD * 2.0;
        let shown = self.resolve_rows_shown();
        let rows = (0..shown)
            .map(|i| {
                let row_top = panel.origin.y + ROWS_TOP + i as f32 * ROW_H;
                let btn_h = ROW_H - 6.0;
                let ours = Rect {
                    origin: Point2D::new(left, row_top + 3.0),
                    size: Point2D::new(CHOICE_W, btn_h),
                };
                let theirs = Rect {
                    origin: Point2D::new(left + CHOICE_W + CHOICE_GAP, row_top + 3.0),
                    size: Point2D::new(CHOICE_W, btn_h),
                };
                (ours, theirs)
            })
            .collect();
        let actions_top = panel.origin.y + ROWS_TOP + shown as f32 * ROW_H + ACTION_GAP;
        let half = (inner_w - CHOICE_GAP) / 2.0;
        let apply = Rect {
            origin: Point2D::new(left, actions_top),
            size: Point2D::new(half, BUTTON_H),
        };
        let cancel = Rect {
            origin: Point2D::new(left + half + CHOICE_GAP, actions_top),
            size: Point2D::new(half, BUTTON_H),
        };
        ResolveLayout {
            rows,
            apply,
            cancel,
        }
    }

    /// Paint the merge-conflict-resolution view.
    pub(super) fn paint_resolve(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let Some(merge) = &self.state.merge_resolve else {
            return;
        };
        let left = rect.origin.x + PAD;
        let top = rect.origin.y;

        self.text(
            cx,
            &truncate(
                &self
                    .t("git.panel.resolveTitle")
                    .replace("{{branch}}", &merge.branch),
                56,
            ),
            left,
            top + TITLE_BASELINE,
            14.0,
            self.theme.foreground,
        );
        let total = merge.total();
        self.text(
            cx,
            &self
                .t("git.panel.resolveSubtitle")
                .replace("{{count}}", &total.to_string()),
            left,
            top + SUBTITLE_BASELINE,
            11.0,
            self.theme.muted_foreground,
        );
        self.divider(cx, left, top + DIVIDER_Y, rect.size.x);

        let layout = self.resolve_layout(rect);
        let rows = merge.rows();
        for (i, (ours_rect, theirs_rect)) in layout.rows.iter().enumerate() {
            let row = rows[i];
            // Ours / Theirs choice buttons — the picked side is the
            // accent (primary) one; a structural conflict greys out
            // Theirs since it can only resolve to Ours.
            self.paint_button_with_hit(
                cx,
                *ours_rect,
                self.t("git.panel.ours"),
                true,
                !row.take_theirs,
                Some(GitPanelHit::MergeChoiceOurs(i)),
            );
            self.paint_button_with_hit(
                cx,
                *theirs_rect,
                self.t("git.panel.theirs"),
                row.theirs_allowed,
                row.take_theirs,
                Some(GitPanelHit::MergeChoiceTheirs(i)),
            );
            let label_x = theirs_rect.origin.x + theirs_rect.size.x + 10.0;
            let baseline = ours_rect.origin.y + ours_rect.size.y / 2.0 + 4.0;
            self.text(
                cx,
                &truncate(&format!("{}  ·  {}", row.label, row.kind), 56),
                label_x,
                baseline,
                12.0,
                self.theme.foreground,
            );
        }

        // Apply / Cancel.
        self.paint_button_with_hit(
            cx,
            layout.apply,
            self.t("git.panel.applyMerge"),
            true,
            true,
            Some(GitPanelHit::ApplyMergeResolution),
        );
        self.paint_button_with_hit(
            cx,
            layout.cancel,
            self.t("common.cancel"),
            true,
            false,
            Some(GitPanelHit::CancelMergeResolution),
        );

        // Footer — note any conflicts beyond the shown cap.
        let footer = if total > MAX_ROWS {
            self.t("git.panel.resolveFooterMore")
                .replace("{{count}}", &(total - MAX_ROWS).to_string())
        } else {
            self.t("git.panel.resolveFooter").to_string()
        };
        self.text(
            cx,
            &footer,
            left,
            top + self.height() - PAD,
            10.0,
            self.theme.muted_foreground,
        );
    }
}
