//! SSH-keys subview for [`GitPanel`] — a port of the TS `GitPanelSshKeys`
//! list view (`git-panel-ssh-keys.tsx`).
//!
//! Reached from the overflow `…` menu's "SSH 密钥" entry. Lists the stored
//! SSH keys (host-enumerated into `ssh_keys`) with an empty state, and offers
//! "导入现有密钥" + "生成新密钥". Rendered as an overflow popover subview
//! (`GitOverflowView::SshKeys`), anchored + hit-tested like remote-settings.

use crate::widgets::git_panel::{truncate, GitPanel, GitPanelHit, PAD};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

const SSH_W: f32 = 300.0;
const SSH_PAD: f32 = 12.0;
const SSH_BACK_H: f32 = 24.0;
const SSH_HEADING_H: f32 = 16.0;
const SSH_KEY_ROW_H: f32 = 24.0;
const SSH_BTN_H: f32 = 28.0;
const SSH_MAX_KEYS: usize = 5;

impl GitPanel<'_> {
    fn ssh_rows(&self) -> usize {
        self.state.ssh_keys.len().min(SSH_MAX_KEYS)
    }

    /// The SSH-keys subview popover rect, anchored below the overflow `…`.
    pub(super) fn ssh_keys_panel(&self, panel_rect: Rect) -> Rect {
        let (_, _, overflow_btn) = self.ready_header_buttons(panel_rect);
        let w = SSH_W.min(panel_rect.size.x - PAD * 2.0);
        let body = if self.state.ssh_keys.is_empty() {
            22.0
        } else {
            self.ssh_rows() as f32 * SSH_KEY_ROW_H
        };
        let h = SSH_PAD * 2.0 + SSH_BACK_H + 4.0 + SSH_HEADING_H + 8.0 + body + 10.0 + SSH_BTN_H;
        let right = overflow_btn.origin.x + overflow_btn.size.x;
        Rect {
            origin: Point2D::new(right - w, overflow_btn.origin.y + overflow_btn.size.y + 4.0),
            size: Point2D::new(w, h),
        }
    }

    /// `(import, generate)` footer button rects.
    pub(super) fn ssh_keys_footer_rects(&self, panel_rect: Rect) -> (Rect, Rect) {
        let p = self.ssh_keys_panel(panel_rect);
        let btn_y = p.origin.y + p.size.y - SSH_PAD - SSH_BTN_H;
        let gen_w = 100.0;
        let imp_w = 108.0;
        let generate = Rect {
            origin: Point2D::new(p.origin.x + p.size.x - SSH_PAD - gen_w, btn_y),
            size: Point2D::new(gen_w, SSH_BTN_H),
        };
        let import = Rect {
            origin: Point2D::new(generate.origin.x - 8.0 - imp_w, btn_y),
            size: Point2D::new(imp_w, SSH_BTN_H),
        };
        (import, generate)
    }

    /// Paint the SSH-keys subview.
    pub(super) fn paint_ssh_keys(&self, cx: &mut PaintCx<'_>, panel_rect: Rect) {
        let t = self.theme;
        let p = self.ssh_keys_panel(panel_rect);
        cx.backend.fill_round_rect(p, 8.0, t.popover);
        cx.backend.stroke_round_rect(p, 8.0, t.border, 1.0);
        let left = p.origin.x + SSH_PAD;
        // ‹ Back header.
        let back_y = p.origin.y + SSH_PAD;
        draw_icon(
            cx.backend,
            Icon::ChevronLeft,
            Point2D::new(left, back_y + (SSH_BACK_H - 13.0) / 2.0),
            13.0,
            t.muted_foreground,
            1.75,
        );
        self.text(
            cx,
            self.t("git.ssh.back"),
            left + 18.0,
            back_y + SSH_BACK_H / 2.0 + 4.0,
            12.0,
            t.foreground,
        );
        // Heading.
        let heading_y = back_y + SSH_BACK_H + 4.0;
        self.text(
            cx,
            self.t("git.ssh.heading"),
            left,
            heading_y + 12.0,
            11.0,
            t.muted_foreground,
        );
        // Body — empty hint or key list.
        let body_top = heading_y + SSH_HEADING_H + 8.0;
        if self.state.ssh_keys.is_empty() {
            self.text(
                cx,
                self.t("git.ssh.emptyList"),
                left,
                body_top + 12.0,
                11.0,
                alpha(t.muted_foreground, 0.85),
            );
        } else {
            let inner_w = p.size.x - SSH_PAD * 2.0;
            for (i, name) in self.state.ssh_keys.iter().take(SSH_MAX_KEYS).enumerate() {
                let y = body_top + i as f32 * SSH_KEY_ROW_H;
                draw_icon(
                    cx.backend,
                    Icon::Key,
                    Point2D::new(left, y + (SSH_KEY_ROW_H - 13.0) / 2.0),
                    13.0,
                    t.muted_foreground,
                    1.5,
                );
                let chars = ((inner_w - 22.0) / 7.0) as usize;
                self.text(
                    cx,
                    &truncate(name, chars.max(6)),
                    left + 22.0,
                    y + SSH_KEY_ROW_H / 2.0 + 4.0,
                    12.0,
                    t.foreground,
                );
            }
        }
        // Footer — 导入现有密钥 (outline) + 生成新密钥 (primary).
        let (import, generate) = self.ssh_keys_footer_rects(panel_rect);
        self.paint_button_with_hit(
            cx,
            import,
            self.t("git.ssh.importAction"),
            true,
            false,
            Some(GitPanelHit::SshImportKey),
        );
        self.paint_button_with_hit(
            cx,
            generate,
            self.t("git.ssh.generateAction"),
            true,
            true,
            Some(GitPanelHit::SshGenerateKey),
        );
    }

    /// Hit-test the SSH-keys subview.
    pub(super) fn ssh_keys_hit(&self, panel_rect: Rect, point: Point2D) -> Option<GitPanelHit> {
        let p = self.ssh_keys_panel(panel_rect);
        if !p.contains(point) {
            return None;
        }
        // ‹ Back row.
        let back = Rect {
            origin: Point2D::new(p.origin.x + SSH_PAD, p.origin.y + SSH_PAD),
            size: Point2D::new(p.size.x - SSH_PAD * 2.0, SSH_BACK_H),
        };
        if back.contains(point) {
            return Some(GitPanelHit::OverflowBack);
        }
        let (import, generate) = self.ssh_keys_footer_rects(panel_rect);
        if import.contains(point) {
            return Some(GitPanelHit::SshImportKey);
        }
        if generate.contains(point) {
            return Some(GitPanelHit::SshGenerateKey);
        }
        Some(GitPanelHit::Inside)
    }
}

/// A colour at `factor` of its current alpha (Tailwind `/NN`).
fn alpha(c: Color, factor: f32) -> Color {
    crate::util::alpha(c, factor)
}
