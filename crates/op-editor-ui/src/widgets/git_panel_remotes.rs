//! Remotes-section layout + rendering for [`GitPanel`].
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. The Remotes section sits below the Branches list:
//! a one-line remote summary, a URL input + "Set" / "SSH" buttons,
//! and an HTTPS-credential (`username:token`) input + "Login".

use crate::widgets::git_panel::{truncate, GitPanel, INPUT_H, PAD, SECTION_GAP};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

/// Label baseline offset within the Remotes block.
const LABEL_OFF: f32 = 8.0;
/// Summary-line baseline offset within the block.
const SUMMARY_OFF: f32 = 22.0;
/// URL-input-row top offset within the block.
const INPUT_OFF: f32 = 30.0;
/// Gap between the URL row and the HTTPS-credential row.
const CRED_GAP: f32 = 8.0;
/// Width of each button at an input row's right edge.
const SET_BTN_W: f32 = 52.0;
/// Gap between an input and the buttons.
const SET_GAP: f32 = 8.0;

/// The interactive sub-rects of the Remotes section.
pub(super) struct RemotesLayout {
    /// The remote-URL input box.
    pub(super) input: Rect,
    /// The "Set" (origin) button.
    pub(super) set_button: Rect,
    /// The "SSH" (set up SSH auth) button.
    pub(super) ssh_button: Rect,
    /// The HTTPS-credential (`username:token`) input box.
    pub(super) https_input: Rect,
    /// The "Login" (store HTTPS credential) button.
    pub(super) login_button: Rect,
}

impl GitPanel<'_> {
    /// Total height of the Remotes section — a fixed block (label +
    /// summary + URL row + HTTPS-credential row).
    pub(super) fn remotes_block_height(&self) -> f32 {
        SECTION_GAP + INPUT_OFF + INPUT_H + CRED_GAP + INPUT_H
    }

    /// The Remotes section's interactive sub-rects.
    pub(super) fn remotes_layout(&self, panel: Rect) -> RemotesLayout {
        let top = panel.origin.y + self.remotes_section_top();
        let left = panel.origin.x + PAD;
        let inner_w = panel.size.x - PAD * 2.0;
        let url_top = top + SECTION_GAP + INPUT_OFF;
        let cred_top = url_top + INPUT_H + CRED_GAP;
        // URL row: field + the "Set" and "SSH" buttons.
        let input = Rect {
            origin: Point2D::new(left, url_top),
            size: Point2D::new(inner_w - 2.0 * (SET_BTN_W + SET_GAP), INPUT_H),
        };
        let url_button = |from_right: f32| Rect {
            origin: Point2D::new(
                left + inner_w - (from_right + 1.0) * SET_BTN_W - from_right * SET_GAP,
                url_top,
            ),
            size: Point2D::new(SET_BTN_W, INPUT_H),
        };
        // Credential row: field + the "Login" button.
        let https_input = Rect {
            origin: Point2D::new(left, cred_top),
            size: Point2D::new(inner_w - SET_BTN_W - SET_GAP, INPUT_H),
        };
        let login_button = Rect {
            origin: Point2D::new(left + inner_w - SET_BTN_W, cred_top),
            size: Point2D::new(SET_BTN_W, INPUT_H),
        };
        RemotesLayout {
            input,
            set_button: url_button(1.0),
            ssh_button: url_button(0.0),
            https_input,
            login_button,
        }
    }

    /// Paint the Remotes section into the panel.
    pub(super) fn paint_remotes(&self, cx: &mut PaintCx<'_>, panel: Rect) {
        let top = panel.origin.y + self.remotes_section_top();
        let left = panel.origin.x + PAD;

        self.text(
            cx,
            self.t("git.panel.remotes"),
            left,
            top + SECTION_GAP + LABEL_OFF,
            12.0,
            self.theme.muted_foreground,
        );

        // One-line summary of the configured remote(s).
        let (summary, summary_color) = match self.state.remotes.first() {
            Some(first) if self.state.remotes.len() > 1 => (
                self.t("git.panel.remotesMore")
                    .replace("{{name}}", &truncate(first, 38))
                    .replace("{{count}}", &(self.state.remotes.len() - 1).to_string()),
                self.theme.foreground,
            ),
            Some(first) => (truncate(first, 52), self.theme.foreground),
            None => (
                self.t("git.panel.noRemote").to_string(),
                self.theme.muted_foreground,
            ),
        };
        self.text(
            cx,
            &summary,
            left,
            top + SECTION_GAP + SUMMARY_OFF,
            11.0,
            summary_color,
        );

        let layout = self.remotes_layout(panel);
        // URL input + "Set" / "SSH" buttons.
        self.paint_section_input(
            cx,
            layout.input,
            &self.state.remote_draft,
            self.state.remote_focused,
            self.t("git.panel.remotePlaceholder"),
        );
        self.paint_button(cx, layout.set_button, self.t("git.panel.set"), true, false);
        // "SSH" is a protocol name — the same in every locale.
        self.paint_button(cx, layout.ssh_button, "SSH", true, false);
        // HTTPS-credential input (token masked) + "Login" button.
        self.paint_section_input(
            cx,
            layout.https_input,
            &mask_credential(&self.state.https_draft),
            self.state.https_focused,
            self.t("git.panel.httpsPlaceholder"),
        );
        self.paint_button(
            cx,
            layout.login_button,
            self.t("git.panel.login"),
            true,
            false,
        );
    }

    /// Paint one Remotes-section text input — fill, focus border,
    /// `shown` text (or `placeholder` when empty + unfocused) and a
    /// caret while focused.
    fn paint_section_input(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        shown: &str,
        focused: bool,
        placeholder: &str,
    ) {
        cx.backend.fill_round_rect(rect, 6.0, self.theme.muted);
        let border = if focused {
            self.theme.primary
        } else {
            self.theme.border
        };
        cx.backend.stroke_round_rect(rect, 6.0, border, 1.0);
        let text_x = rect.origin.x + 8.0;
        let baseline = rect.origin.y + rect.size.y / 2.0 + 4.0;
        if shown.is_empty() && !focused {
            self.text(
                cx,
                placeholder,
                text_x,
                baseline,
                11.0,
                self.theme.muted_foreground,
            );
        } else {
            let line = if focused {
                format!("{shown}|")
            } else {
                shown.to_string()
            };
            self.text(
                cx,
                &truncate(&line, 56),
                text_x,
                baseline,
                11.0,
                self.theme.foreground,
            );
        }
    }
}

/// Mask the token half of a `username:token` draft — the username
/// stays visible, the token renders as bullets so a shoulder-surfer
/// cannot read it. A draft with no `:` yet (still typing the name)
/// is shown verbatim.
fn mask_credential(draft: &str) -> String {
    match draft.split_once(':') {
        Some((user, token)) => format!("{user}:{}", "•".repeat(token.chars().count())),
        None => draft.to_string(),
    }
}
