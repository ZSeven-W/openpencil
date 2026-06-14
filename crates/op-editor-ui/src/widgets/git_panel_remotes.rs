//! Remotes-section layout + rendering for [`GitPanel`].
//!
//! Split out of `git_panel.rs` to keep that file under the repo's
//! 800-line cap. The Remotes section sits below the Branches list:
//! a one-line remote summary, a URL input + "Set" / "SSH" buttons,
//! and an HTTPS-credential (`username:token`) input + "Login".

use crate::widgets::git_panel::{truncate, GitPanel, GitPanelHit, INPUT_H, PAD, SECTION_GAP};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use jian_core::text_input::{prev_char_boundary, TextInputState};

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
            &self.state.remote_input,
            self.state.remote_focused,
            self.t("git.panel.remotePlaceholder"),
        );
        self.paint_button_with_hit(
            cx,
            layout.set_button,
            self.t("git.panel.set"),
            true,
            false,
            Some(GitPanelHit::SetRemote),
        );
        // "SSH" is a protocol name — the same in every locale.
        self.paint_button_with_hit(
            cx,
            layout.ssh_button,
            "SSH",
            true,
            false,
            Some(GitPanelHit::SetupSshAuth),
        );
        // HTTPS-credential input (token masked) + "Login" button.
        self.paint_masked_section_input(
            cx,
            layout.https_input,
            &self.state.https_input,
            self.state.https_focused,
            self.t("git.panel.httpsPlaceholder"),
        );
        self.paint_button_with_hit(
            cx,
            layout.login_button,
            self.t("git.panel.login"),
            true,
            false,
            Some(GitPanelHit::SetHttpsAuth),
        );
    }

    /// Paint one Remotes-section text input — fill, focus border,
    /// `shown` text (or `placeholder` when empty + unfocused) and a
    /// caret while focused.
    fn paint_section_input(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        input: &TextInputState,
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
        self.paint_text_input_view(cx, rect, input, placeholder, focused, 11.0, 8.0);
    }

    fn paint_masked_section_input(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        input: &TextInputState,
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

        let draft = input.text();
        if draft.is_empty() && !focused {
            self.text(
                cx,
                placeholder,
                rect.origin.x + 8.0,
                rect.origin.y + rect.size.y / 2.0 + 4.0,
                11.0,
                self.theme.muted_foreground,
            );
            return;
        }

        let shown = mask_credential(draft);
        let text_x = rect.origin.x + 8.0;
        let baseline = rect.origin.y + rect.size.y / 2.0 + 4.0;
        cx.backend.save();
        cx.backend.clip_rect(rect);
        if let Some((start, end)) = input.highlight_range() {
            let x0 = cx.backend.measure_text(
                &shown[..masked_byte_for_source_byte(draft, &shown, start)],
                11.0,
            );
            let x1 = cx.backend.measure_text(
                &shown[..masked_byte_for_source_byte(draft, &shown, end)],
                11.0,
            );
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(text_x + x0, rect.origin.y + 6.0),
                    size: Point2D::new((x1 - x0).max(1.0), 16.0),
                },
                3.0,
                self.theme.primary.with_alpha(0.35),
            );
        }
        self.text(cx, &shown, text_x, baseline, 11.0, self.theme.foreground);
        if focused && input.highlight_range().is_none() && input.caret_visible(self.now_ms) {
            let caret_byte = masked_byte_for_source_byte(draft, &shown, input.caret());
            let caret_x = text_x + cx.backend.measure_text(&shown[..caret_byte], 11.0) + 1.0;
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(caret_x, rect.origin.y + 7.0),
                    size: Point2D::new(1.5, 15.0),
                },
                self.theme.foreground,
            );
        }
        cx.backend.restore();
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

fn masked_byte_for_source_byte(source: &str, masked: &str, byte: usize) -> usize {
    let source_byte = prev_char_boundary(source, byte);
    let chars = source[..source_byte].chars().count();
    masked
        .char_indices()
        .nth(chars)
        .map(|(i, _)| i)
        .unwrap_or(masked.len())
}
