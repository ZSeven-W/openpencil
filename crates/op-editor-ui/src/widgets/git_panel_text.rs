//! GitPanel text-drawing helpers (footer + one-line text + the
//! `Color` -> jian conversion), split out of `git_panel.rs` to keep
//! it under the 800-line cap.

use super::git_panel::GitPanel;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, TextLayout};
use jian_widgets::{Density, Tokens};

impl GitPanel<'_> {
    /// Paint the menu hint at the panel foot.
    pub(super) fn footer(&self, cx: &mut PaintCx<'_>, left: f32, y: f32) {
        self.text(
            cx,
            self.t("git.panel.footer"),
            left,
            y,
            10.0,
            self.theme.muted_foreground,
        );
    }

    /// Draw one line of text.
    pub(super) fn text(
        &self,
        cx: &mut PaintCx<'_>,
        s: &str,
        x: f32,
        baseline_y: f32,
        size: f32,
        color: Color,
    ) {
        let layout = TextLayout::single_run(
            s,
            "system-ui",
            size,
            (color).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
    }

    pub(super) fn widget_tokens(&self) -> Tokens {
        Tokens {
            background: self.theme.background,
            foreground: self.theme.foreground,
            card: self.theme.card,
            card_foreground: self.theme.card_foreground,
            popover: self.theme.popover,
            popover_foreground: self.theme.popover_foreground,
            primary: self.theme.primary,
            primary_foreground: self.theme.primary_foreground,
            muted: self.theme.muted,
            muted_foreground: self.theme.muted_foreground,
            border: self.theme.border,
            accent: self.theme.accent,
            accent_foreground: self.theme.accent_foreground,
            destructive: self.theme.destructive,
            button_hover: self.theme.button_hover,
            row_selected: self.theme.row_selected,
            row_selected_primary: self.theme.row_selected_primary,
            density: Density::Desktop,
        }
    }
}
