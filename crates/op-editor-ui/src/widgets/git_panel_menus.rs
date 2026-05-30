//! Ready-state header popovers for [`GitPanel`] — the branch-picker
//! dropdown (`⎇ <branch> ▾`) and the overflow `…` menu.
//!
//! A port of the TS `GitPanelBranchPicker` (list mode) + the
//! `GitPanelHeader` overflow popover. Split out of `git_panel_ready.rs`
//! to keep both files under the repo's 800-line cap.
//!
//! These paint as overlays ON TOP of the ready view and are hit-tested
//! BEFORE it (so an open popover captures the click). Geometry is
//! shared between paint + hit-test through the `*_rects` helpers so the
//! pure-geometry hit-test agrees with paint.

use crate::widgets::git_panel::{contains, truncate, GitPanel, GitPanelHit, PAD};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::GitOverflowView;

/// Dropdown row height (TS menu item ≈ 28 px).
const MENU_ROW_H: f32 = 28.0;
/// Inner padding around a popover's row stack.
const MENU_PAD: f32 = 4.0;
/// Most branches the picker lists (matches the classic section cap).
const MENU_MAX_BRANCHES: usize = 8;
/// Branch-name truncation inside the dropdown.
const BRANCH_NAME_MAX: usize = 26;
/// Overflow-menu width (TS `w-56` ≈ 224 px, clamped to the panel).
const OVERFLOW_W: f32 = 208.0;
/// Remote-settings subview width (TS `w-[300px]`, clamped).
const RS_W: f32 = 280.0;
/// Inner padding inside the remote-settings subview.
const RS_PAD: f32 = 10.0;
/// Input / button row height inside the subview.
const RS_ROW_H: f32 = 28.0;
/// The `‹ Back` header-row height.
const RS_BACK_H: f32 = 24.0;
/// Width of the "Set" / "Login" buttons at an input row's right edge.
const RS_BTN_W: f32 = 52.0;
/// Gap between an input and its trailing button.
const RS_GAP: f32 = 8.0;

/// One overflow-menu entry — an icon, a label key, and the
/// [`GitPanelHit`] it dispatches.
struct OverflowItem {
    icon: Icon,
    label_key: &'static str,
    hit: GitPanelHit,
    /// A `›` submenu affordance (the entry opens a subview).
    submenu: bool,
}

impl GitPanel<'_> {
    /// The overflow menu's entries, top to bottom — a port of the TS
    /// header popover. Only the remote-settings / SSH-keys subviews are
    /// wired today; the entries map to existing git actions.
    fn overflow_items(&self) -> [OverflowItem; 2] {
        [
            OverflowItem {
                icon: Icon::Settings2,
                label_key: "git.header.overflowRemoteSettings",
                hit: GitPanelHit::OverflowRemoteSettings,
                submenu: true,
            },
            OverflowItem {
                icon: Icon::Lock,
                label_key: "git.header.overflowSshKeys",
                hit: GitPanelHit::OverflowSshKeys,
                submenu: false,
            },
        ]
    }

    // ── Branch picker ────────────────────────────────────────────────

    /// The branch-picker dropdown rect, anchored below the branch
    /// button. At least one row tall (a placeholder when no branches).
    pub(super) fn branch_picker_panel(&self, panel_rect: Rect) -> Rect {
        let btn = self.ready_branch_rect(panel_rect);
        let rows = self.state.branches.len().clamp(1, MENU_MAX_BRANCHES);
        let w = (panel_rect.size.x - PAD * 2.0).clamp(180.0, 260.0);
        let h = MENU_PAD * 2.0 + rows as f32 * MENU_ROW_H;
        Rect {
            origin: Point2D::new(btn.origin.x, btn.origin.y + btn.size.y + 4.0),
            size: Point2D::new(w, h),
        }
    }

    /// One clickable rect per listed branch.
    pub(super) fn branch_picker_row_rects(&self, panel_rect: Rect) -> Vec<Rect> {
        let panel = self.branch_picker_panel(panel_rect);
        (0..self.state.branches.len().min(MENU_MAX_BRANCHES))
            .map(|i| Rect {
                origin: Point2D::new(
                    panel.origin.x + MENU_PAD,
                    panel.origin.y + MENU_PAD + i as f32 * MENU_ROW_H,
                ),
                size: Point2D::new(panel.size.x - MENU_PAD * 2.0, MENU_ROW_H),
            })
            .collect()
    }

    /// Paint the branch-picker dropdown.
    pub(super) fn paint_branch_picker(&self, cx: &mut PaintCx<'_>, panel_rect: Rect) {
        let t = self.theme;
        let panel = self.branch_picker_panel(panel_rect);
        cx.backend.fill_round_rect(panel, 8.0, t.popover);
        cx.backend.stroke_round_rect(panel, 8.0, t.border, 1.0);
        if self.state.branches.is_empty() {
            self.text(
                cx,
                self.t("git.branch.listHeading"),
                panel.origin.x + 12.0,
                panel.origin.y + MENU_PAD + 18.0,
                12.0,
                t.muted_foreground,
            );
            return;
        }
        let rows = self.branch_picker_row_rects(panel_rect);
        for (i, row) in rows.iter().enumerate() {
            let is_current = self.state.branches.get(i) == self.state.branch.as_ref();
            let name = truncate(
                self.state.branches.get(i).map(String::as_str).unwrap_or(""),
                BRANCH_NAME_MAX,
            );
            self.text(
                cx,
                &name,
                row.origin.x + 10.0,
                row.origin.y + row.size.y / 2.0 + 4.0,
                12.0,
                if is_current { t.primary } else { t.foreground },
            );
            if is_current {
                draw_icon(
                    cx.backend,
                    Icon::Check,
                    Point2D::new(
                        row.origin.x + row.size.x - 22.0,
                        row.origin.y + (row.size.y - 14.0) / 2.0,
                    ),
                    14.0,
                    t.primary,
                    1.6,
                );
            } else {
                // A non-current branch shows a merge affordance on the
                // right edge (TS branch-row merge button).
                let merge = GitPanel::branch_merge_button(*row);
                draw_icon(
                    cx.backend,
                    Icon::GitBranch,
                    Point2D::new(merge.origin.x, merge.origin.y),
                    14.0,
                    t.muted_foreground,
                    1.5,
                );
            }
        }
    }

    /// Hit-test the branch-picker dropdown. `None` when the point is
    /// outside the popover (the caller then closes it + falls through).
    pub(super) fn branch_picker_hit(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<GitPanelHit> {
        let panel = self.branch_picker_panel(panel_rect);
        if !contains(panel, point) {
            return None;
        }
        for (i, row) in self.branch_picker_row_rects(panel_rect).iter().enumerate() {
            if !contains(*row, point) {
                continue;
            }
            let is_current = self.state.branches.get(i) == self.state.branch.as_ref();
            if is_current {
                return Some(GitPanelHit::Inside);
            }
            if contains(GitPanel::branch_merge_button(*row), point) {
                return Some(GitPanelHit::MergeBranch(i));
            }
            return Some(GitPanelHit::SwitchBranch(i));
        }
        Some(GitPanelHit::Inside)
    }

    // ── Overflow menu ────────────────────────────────────────────────

    /// The overflow `…` menu rect, anchored below the overflow button
    /// and right-aligned to the panel edge.
    pub(super) fn overflow_panel(&self, panel_rect: Rect) -> Rect {
        let (_, _, overflow_btn) = self.ready_header_buttons(panel_rect);
        let items = self.overflow_items().len();
        let w = OVERFLOW_W.min(panel_rect.size.x - PAD * 2.0);
        let h = MENU_PAD * 2.0 + items as f32 * MENU_ROW_H;
        let right = overflow_btn.origin.x + overflow_btn.size.x;
        Rect {
            origin: Point2D::new(right - w, overflow_btn.origin.y + overflow_btn.size.y + 4.0),
            size: Point2D::new(w, h),
        }
    }

    /// One clickable rect per overflow-menu entry.
    pub(super) fn overflow_row_rects(&self, panel_rect: Rect) -> Vec<Rect> {
        let panel = self.overflow_panel(panel_rect);
        (0..self.overflow_items().len())
            .map(|i| Rect {
                origin: Point2D::new(
                    panel.origin.x + MENU_PAD,
                    panel.origin.y + MENU_PAD + i as f32 * MENU_ROW_H,
                ),
                size: Point2D::new(panel.size.x - MENU_PAD * 2.0, MENU_ROW_H),
            })
            .collect()
    }

    /// Paint the overflow `…` menu.
    pub(super) fn paint_overflow_menu(&self, cx: &mut PaintCx<'_>, panel_rect: Rect) {
        let t = self.theme;
        let panel = self.overflow_panel(panel_rect);
        cx.backend.fill_round_rect(panel, 8.0, t.popover);
        cx.backend.stroke_round_rect(panel, 8.0, t.border, 1.0);
        let rows = self.overflow_row_rects(panel_rect);
        for (item, row) in self.overflow_items().iter().zip(rows.iter()) {
            draw_icon(
                cx.backend,
                item.icon,
                Point2D::new(row.origin.x + 8.0, row.origin.y + (row.size.y - 14.0) / 2.0),
                14.0,
                t.muted_foreground,
                1.5,
            );
            self.text(
                cx,
                self.t(item.label_key),
                row.origin.x + 30.0,
                row.origin.y + row.size.y / 2.0 + 4.0,
                12.0,
                t.foreground,
            );
            if item.submenu {
                draw_icon(
                    cx.backend,
                    Icon::ChevronRight,
                    Point2D::new(
                        row.origin.x + row.size.x - 18.0,
                        row.origin.y + (row.size.y - 12.0) / 2.0,
                    ),
                    12.0,
                    alpha(t.muted_foreground, 0.70),
                    1.5,
                );
            }
        }
    }

    /// Hit-test the overflow menu. `None` when the point is outside the
    /// popover (the caller then closes it + falls through).
    pub(super) fn overflow_hit(&self, panel_rect: Rect, point: Point2D) -> Option<GitPanelHit> {
        let panel = self.overflow_panel(panel_rect);
        if !contains(panel, point) {
            return None;
        }
        for (item, row) in self
            .overflow_items()
            .iter()
            .zip(self.overflow_row_rects(panel_rect).iter())
        {
            if contains(*row, point) {
                return Some(item.hit);
            }
        }
        Some(GitPanelHit::Inside)
    }

    /// Paint whichever overflow view is active — the menu or a subview.
    pub(super) fn paint_overflow(&self, cx: &mut PaintCx<'_>, panel_rect: Rect) {
        match self.state.overflow_view {
            GitOverflowView::Menu => self.paint_overflow_menu(cx, panel_rect),
            GitOverflowView::RemoteSettings => self.paint_remote_settings(cx, panel_rect),
        }
    }

    /// Hit-test whichever overflow view is active.
    pub(super) fn overflow_hit_dispatch(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<GitPanelHit> {
        match self.state.overflow_view {
            GitOverflowView::Menu => self.overflow_hit(panel_rect, point),
            GitOverflowView::RemoteSettings => self.remote_settings_hit(panel_rect, point),
        }
    }

    // ── Remote-settings subview ──────────────────────────────────────

    /// The remote-settings subview popover rect.
    pub(super) fn remote_settings_panel(&self, panel_rect: Rect) -> Rect {
        let (_, _, overflow_btn) = self.ready_header_buttons(panel_rect);
        let w = RS_W.min(panel_rect.size.x - PAD * 2.0);
        let h = RS_PAD * 2.0 + RS_BACK_H + 8.0 + RS_ROW_H + 8.0 + RS_ROW_H;
        let right = overflow_btn.origin.x + overflow_btn.size.x;
        Rect {
            origin: Point2D::new(right - w, overflow_btn.origin.y + overflow_btn.size.y + 4.0),
            size: Point2D::new(w, h),
        }
    }

    /// The subview's interactive sub-rects: `(back, url_input, set,
    /// https_input, login)`. Shared by paint + hit-test (+ tests).
    pub(super) fn remote_settings_rects(&self, panel_rect: Rect) -> (Rect, Rect, Rect, Rect, Rect) {
        let p = self.remote_settings_panel(panel_rect);
        let left = p.origin.x + RS_PAD;
        let inner_w = p.size.x - RS_PAD * 2.0;
        let back = Rect {
            origin: Point2D::new(left, p.origin.y + RS_PAD),
            size: Point2D::new(inner_w, RS_BACK_H),
        };
        let url_top = back.origin.y + RS_BACK_H + 8.0;
        let field_w = inner_w - RS_BTN_W - RS_GAP;
        let url_input = Rect {
            origin: Point2D::new(left, url_top),
            size: Point2D::new(field_w, RS_ROW_H),
        };
        let set = Rect {
            origin: Point2D::new(left + field_w + RS_GAP, url_top),
            size: Point2D::new(RS_BTN_W, RS_ROW_H),
        };
        let cred_top = url_top + RS_ROW_H + 8.0;
        let https_input = Rect {
            origin: Point2D::new(left, cred_top),
            size: Point2D::new(field_w, RS_ROW_H),
        };
        let login = Rect {
            origin: Point2D::new(left + field_w + RS_GAP, cred_top),
            size: Point2D::new(RS_BTN_W, RS_ROW_H),
        };
        (back, url_input, set, https_input, login)
    }

    /// Paint the remote-settings subview.
    pub(super) fn paint_remote_settings(&self, cx: &mut PaintCx<'_>, panel_rect: Rect) {
        let t = self.theme;
        let p = self.remote_settings_panel(panel_rect);
        cx.backend.fill_round_rect(p, 8.0, t.popover);
        cx.backend.stroke_round_rect(p, 8.0, t.border, 1.0);
        let (back, url_input, set, https_input, login) = self.remote_settings_rects(panel_rect);
        // ‹ Back header row.
        draw_icon(
            cx.backend,
            Icon::ChevronLeft,
            Point2D::new(back.origin.x, back.origin.y + (back.size.y - 14.0) / 2.0),
            14.0,
            t.muted_foreground,
            1.5,
        );
        self.text(
            cx,
            self.t("git.remote.settingsHeading"),
            back.origin.x + 20.0,
            back.origin.y + back.size.y / 2.0 + 4.0,
            12.0,
            t.foreground,
        );
        // Remote-URL field + "Save".
        self.paint_menu_input(
            cx,
            url_input,
            &self.state.remote_draft,
            self.t("git.remote.urlPlaceholder"),
            self.state.remote_focused,
        );
        self.paint_button(
            cx,
            set,
            self.t("git.remote.saveButton"),
            !self.state.remote_draft.trim().is_empty(),
            true,
        );
        // HTTPS-credential field + "Login".
        self.paint_menu_input(
            cx,
            https_input,
            &self.state.https_draft,
            self.t("git.panel.httpsPlaceholder"),
            self.state.https_focused,
        );
        self.paint_button(
            cx,
            login,
            self.t("git.panel.login"),
            !self.state.https_draft.trim().is_empty(),
            false,
        );
    }

    /// Hit-test the remote-settings subview.
    pub(super) fn remote_settings_hit(
        &self,
        panel_rect: Rect,
        point: Point2D,
    ) -> Option<GitPanelHit> {
        let p = self.remote_settings_panel(panel_rect);
        if !contains(p, point) {
            return None;
        }
        let (back, url_input, set, https_input, login) = self.remote_settings_rects(panel_rect);
        if contains(back, point) {
            return Some(GitPanelHit::OverflowBack);
        }
        if contains(url_input, point) {
            return Some(GitPanelHit::RemoteInput);
        }
        if contains(set, point) {
            return Some(GitPanelHit::SetRemote);
        }
        if contains(https_input, point) {
            return Some(GitPanelHit::HttpsInput);
        }
        if contains(login, point) {
            return Some(GitPanelHit::SetHttpsAuth);
        }
        Some(GitPanelHit::Inside)
    }

    /// A simple popover input box — rounded field + draft / placeholder
    /// text + a blink-free caret bar when focused.
    fn paint_menu_input(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        value: &str,
        placeholder: &str,
        focused: bool,
    ) {
        let t = self.theme;
        cx.backend.fill_round_rect(rect, 6.0, t.muted);
        let border = if focused {
            alpha(t.primary, 0.50)
        } else {
            alpha(t.border, 0.70)
        };
        cx.backend.stroke_round_rect(rect, 6.0, border, 1.0);
        let baseline = rect.origin.y + rect.size.y / 2.0 + 4.0;
        if value.is_empty() && !focused {
            self.text(
                cx,
                placeholder,
                rect.origin.x + 8.0,
                baseline,
                11.0,
                alpha(t.muted_foreground, 0.70),
            );
        } else {
            let shown = truncate(value, 30);
            let shown = if focused { format!("{shown}|") } else { shown };
            self.text(
                cx,
                &shown,
                rect.origin.x + 8.0,
                baseline,
                11.0,
                t.foreground,
            );
        }
    }
}

/// A colour at `factor` of its current alpha (Tailwind `/NN`).
fn alpha(c: Color, factor: f32) -> Color {
    Color {
        a: c.a * factor,
        ..c
    }
}
