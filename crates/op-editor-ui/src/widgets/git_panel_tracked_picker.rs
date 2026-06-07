//! Tracked-file picker subview for [`GitPanel`] — a port of the TS
//! `GitPanelTrackedPicker` (`git-panel-tracked-picker.tsx`).
//!
//! Reached from the overflow `…` menu's "切换跟踪文件" entry. Lists the
//! repo's `.op` candidates (host-enumerated into `candidate_files`), lets the
//! user pick one, and binds it as the tracked file (optionally opening it).
//!
//! Rendered as an overflow popover subview (`GitOverflowView::TrackedPicker`),
//! anchored + hit-tested like the remote-settings subview. Geometry is shared
//! between paint + hit-test through the `*_rects` helpers.

use crate::widgets::git_panel::{contains, truncate, GitPanel, GitPanelHit, PAD};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

/// Picker popover width (TS body `w-80` ≈ 320, clamped to the panel).
const TP_W: f32 = 300.0;
/// Inner padding (TS `p-4`).
const TP_PAD: f32 = 12.0;
/// Uppercase header row height.
const TP_HEADER_H: f32 = 16.0;
/// One candidate row (TS bordered card: icon box + title + subtitle).
const TP_ROW_H: f32 = 46.0;
/// Gap between candidate rows (TS `gap-1.5`).
const TP_ROW_GAP: f32 = 6.0;
/// Footer action-bar height.
const TP_FOOTER_H: f32 = 28.0;
/// Gap between the row list and the footer.
const TP_SECTION_GAP: f32 = 10.0;
/// Max candidate rows drawn before the list is clamped.
const TP_MAX_ROWS: usize = 4;
/// Empty-state card height (heading + body + close button).
const TP_EMPTY_H: f32 = 132.0;

impl GitPanel<'_> {
    /// Number of candidate rows the picker will draw (clamped).
    fn picker_rows(&self) -> usize {
        self.state.candidate_files.len().min(TP_MAX_ROWS)
    }

    /// The tracked-file picker popover rect, anchored below the overflow `…`.
    pub(super) fn tracked_picker_panel(&self, panel_rect: Rect) -> Rect {
        let (_, _, overflow_btn) = self.ready_header_buttons(panel_rect);
        let w = TP_W.min(panel_rect.size.x - PAD * 2.0);
        let h = if self.state.candidate_files.is_empty() {
            TP_EMPTY_H
        } else {
            let n = self.picker_rows() as f32;
            TP_PAD * 2.0
                + TP_HEADER_H
                + 8.0
                + n * TP_ROW_H
                + (n - 1.0).max(0.0) * TP_ROW_GAP
                + TP_SECTION_GAP
                + TP_FOOTER_H
        };
        let right = overflow_btn.origin.x + overflow_btn.size.x;
        Rect {
            origin: Point2D::new(right - w, overflow_btn.origin.y + overflow_btn.size.y + 4.0),
            size: Point2D::new(w, h),
        }
    }

    /// One clickable rect per candidate row (list mode).
    pub(super) fn tracked_picker_row_rects(&self, panel_rect: Rect) -> Vec<Rect> {
        let p = self.tracked_picker_panel(panel_rect);
        let left = p.origin.x + TP_PAD;
        let inner_w = p.size.x - TP_PAD * 2.0;
        let first = p.origin.y + TP_PAD + TP_HEADER_H + 8.0;
        (0..self.picker_rows())
            .map(|i| Rect {
                origin: Point2D::new(left, first + i as f32 * (TP_ROW_H + TP_ROW_GAP)),
                size: Point2D::new(inner_w, TP_ROW_H),
            })
            .collect()
    }

    /// `(back, bind, bind_open)` footer button rects (list mode).
    pub(super) fn tracked_picker_footer_rects(&self, panel_rect: Rect) -> (Rect, Rect, Rect) {
        let p = self.tracked_picker_panel(panel_rect);
        let left = p.origin.x + TP_PAD;
        let btn_y = p.origin.y + p.size.y - TP_PAD - TP_FOOTER_H;
        let back = Rect {
            origin: Point2D::new(left, btn_y),
            size: Point2D::new(48.0, TP_FOOTER_H),
        };
        let open_w = 78.0;
        let bind_w = 70.0;
        let open = Rect {
            origin: Point2D::new(p.origin.x + p.size.x - TP_PAD - open_w, btn_y),
            size: Point2D::new(open_w, TP_FOOTER_H),
        };
        let bind = Rect {
            origin: Point2D::new(open.origin.x - 8.0 - bind_w, btn_y),
            size: Point2D::new(bind_w, TP_FOOTER_H),
        };
        (back, bind, open)
    }

    /// Empty-state "Close panel" button rect.
    fn tracked_picker_empty_close(&self, panel_rect: Rect) -> Rect {
        let p = self.tracked_picker_panel(panel_rect);
        Rect {
            origin: Point2D::new(
                p.origin.x + (p.size.x - 96.0) / 2.0,
                p.origin.y + p.size.y - TP_PAD - 26.0,
            ),
            size: Point2D::new(96.0, 26.0),
        }
    }

    /// Paint the tracked-file picker subview.
    pub(super) fn paint_tracked_picker(&self, cx: &mut PaintCx<'_>, panel_rect: Rect) {
        let t = self.theme;
        let p = self.tracked_picker_panel(panel_rect);
        cx.backend.fill_round_rect(p, 8.0, t.popover);
        cx.backend.stroke_round_rect(p, 8.0, t.border, 1.0);

        if self.state.candidate_files.is_empty() {
            self.paint_picker_empty(cx, panel_rect);
            return;
        }

        // Header — "{{count}} .op files in this repo:".
        let header = self
            .t("git.picker.heading")
            .replace("{{count}}", &self.state.candidate_files.len().to_string());
        self.text(
            cx,
            &header,
            p.origin.x + TP_PAD,
            p.origin.y + TP_PAD + 12.0,
            11.0,
            t.muted_foreground,
        );

        let rows = self.tracked_picker_row_rects(panel_rect);
        for (i, (c, row)) in self
            .state
            .candidate_files
            .iter()
            .zip(rows.iter())
            .enumerate()
        {
            let selected = self.state.tracked_picker_selected == Some(i);
            let border = if selected {
                t.primary
            } else {
                alpha(t.border, 0.70)
            };
            let bg = if selected {
                alpha(t.primary, 0.06)
            } else {
                t.card
            };
            cx.backend.fill_round_rect(*row, 8.0, bg);
            self.wash_if_hovered(cx, *row, 8.0, GitPanelHit::TrackedPickerRow(i));
            cx.backend.stroke_round_rect(*row, 8.0, border, 1.0);
            // Leading icon — Check when selected, else FileText.
            let icon = if selected {
                Icon::Check
            } else {
                Icon::FileText
            };
            let icon_color = if selected {
                t.primary
            } else {
                t.muted_foreground
            };
            draw_icon(
                cx.backend,
                icon,
                Point2D::new(
                    row.origin.x + 10.0,
                    row.origin.y + (row.size.y - 14.0) / 2.0,
                ),
                14.0,
                icon_color,
                1.75,
            );
            // Right-aligned milestone label.
            let meta = if c.milestone_count == 0 {
                self.t("git.picker.noHistory").to_string()
            } else {
                self.t("git.picker.milestoneCount")
                    .replace("{{count}}", &c.milestone_count.to_string())
            };
            let meta_w = cx.backend.measure_text(&meta, 10.0);
            let meta_x = row.origin.x + row.size.x - 10.0 - meta_w;
            self.text(
                cx,
                &meta,
                meta_x,
                row.origin.y + 17.0,
                10.0,
                t.muted_foreground,
            );
            // Title — repo-relative path (truncated to space left of the meta).
            let title_x = row.origin.x + 38.0;
            let title_space = (meta_x - title_x - 6.0).max(0.0);
            let title_chars = (title_space / 7.0) as usize;
            self.text(
                cx,
                &truncate(&c.relative_path, title_chars.max(4)),
                title_x,
                row.origin.y + 17.0,
                12.0,
                t.foreground,
            );
            // Subtitle — "{{message}} · {{time}}" when there's a last commit.
            if let Some(msg) = &c.last_commit_message {
                let sub = self
                    .t("git.picker.lastCommit")
                    .replace("{{message}}", msg)
                    .replace("{{time}}", &c.last_commit_time);
                let sub_chars = ((row.size.x - 48.0) / 6.0) as usize;
                self.text(
                    cx,
                    &truncate(&sub, sub_chars.max(4)),
                    title_x,
                    row.origin.y + 34.0,
                    10.0,
                    alpha(t.muted_foreground, 0.80),
                );
            }
        }

        // Footer — back (ghost) + Track / Track-and-open buttons.
        let (back, bind, open) = self.tracked_picker_footer_rects(panel_rect);
        self.text(
            cx,
            self.t("git.picker.back"),
            back.origin.x,
            back.origin.y + back.size.y / 2.0 + 4.0,
            11.0,
            t.foreground,
        );
        let enabled = self.state.tracked_picker_selected.is_some();
        self.paint_button_with_hit(
            cx,
            bind,
            self.t("git.picker.bindButton"),
            enabled,
            false,
            Some(GitPanelHit::TrackedPickerBind),
        );
        self.paint_button_with_hit(
            cx,
            open,
            self.t("git.picker.bindAndOpenButton"),
            enabled,
            true,
            Some(GitPanelHit::TrackedPickerBindOpen),
        );
    }

    /// Paint the empty-state card (no `.op` candidates).
    fn paint_picker_empty(&self, cx: &mut PaintCx<'_>, panel_rect: Rect) {
        let t = self.theme;
        let p = self.tracked_picker_panel(panel_rect);
        let mid = p.origin.x + p.size.x / 2.0;
        let heading = self.t("git.picker.empty.heading");
        let hw = cx.backend.measure_text(heading, 13.0);
        self.text(
            cx,
            heading,
            mid - hw / 2.0,
            p.origin.y + TP_PAD + 22.0,
            13.0,
            t.foreground,
        );
        let body = self.t("git.picker.empty.body");
        let body_chars = ((p.size.x - TP_PAD * 2.0) / 6.0) as usize;
        let body = truncate(body, body_chars.max(8));
        let bw = cx.backend.measure_text(&body, 11.0);
        self.text(
            cx,
            &body,
            mid - bw / 2.0,
            p.origin.y + TP_PAD + 46.0,
            11.0,
            t.muted_foreground,
        );
        let close = self.tracked_picker_empty_close(panel_rect);
        self.paint_button_with_hit(
            cx,
            close,
            self.t("git.picker.empty.close"),
            true,
            false,
            Some(GitPanelHit::TrackedPickerBack),
        );
    }

    /// Hit-test the tracked-file picker subview.
    pub(super) fn tracked_picker_hit(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<GitPanelHit> {
        let p = self.tracked_picker_panel(panel_rect);
        if !contains(p, point) {
            return None;
        }
        if self.state.candidate_files.is_empty() {
            if contains(self.tracked_picker_empty_close(panel_rect), point) {
                return Some(GitPanelHit::TrackedPickerBack);
            }
            return Some(GitPanelHit::Inside);
        }
        let (back, bind, open) = self.tracked_picker_footer_rects(panel_rect);
        if contains(back, point) {
            return Some(GitPanelHit::TrackedPickerBack);
        }
        if self.state.tracked_picker_selected.is_some() {
            if contains(bind, point) {
                return Some(GitPanelHit::TrackedPickerBind);
            }
            if contains(open, point) {
                return Some(GitPanelHit::TrackedPickerBindOpen);
            }
        }
        for (i, row) in self.tracked_picker_row_rects(panel_rect).iter().enumerate() {
            if contains(*row, point) {
                return Some(GitPanelHit::TrackedPickerRow(i));
            }
        }
        Some(GitPanelHit::Inside)
    }
}

/// A colour at `factor` of its current alpha (Tailwind `/NN`).
fn alpha(c: Color, factor: f32) -> Color {
    Color {
        a: c.a * factor,
        ..c
    }
}
