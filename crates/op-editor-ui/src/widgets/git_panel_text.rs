//! GitPanel text-drawing helpers (footer + one-line text + the
//! `Color` -> jian conversion), split out of `git_panel.rs` to keep
//! it under the 800-line cap.

use super::git_panel::GitPanel;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, TextLayout};
use jian_core::text_input::TextInputState;
use jian_widgets::components::text_input::TextInputView;
use jian_widgets::Tokens;

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
        crate::widgets::button::tokens_from_theme(&self.theme)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_text_input_view(
        &self,
        cx: &mut PaintCx<'_>,
        rect: crate::Rect,
        input: &TextInputState,
        placeholder: &str,
        focused: bool,
        font_size: f32,
        pad_x: f32,
    ) {
        self.paint_text_input_view_masked(
            cx,
            rect,
            input,
            placeholder,
            focused,
            font_size,
            pad_x,
            None,
        );
    }

    /// Like [`Self::paint_text_input_view`] but renders glyphs through a `mask`
    /// transform (e.g. the HTTPS credential's `user:••••`); the caret /
    /// selection still track the source buffer via the unified `TextInputView`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_text_input_view_masked(
        &self,
        cx: &mut PaintCx<'_>,
        rect: crate::Rect,
        input: &TextInputState,
        placeholder: &str,
        focused: bool,
        font_size: f32,
        pad_x: f32,
        mask: Option<&dyn Fn(&str) -> String>,
    ) {
        let placeholder = if focused { "" } else { placeholder };
        TextInputView {
            state: input,
            placeholder,
            focused,
            font_size,
            now_ms: self.now_ms,
            pad_x,
            baseline_delta_y: 0.0,
            mask,
        }
        .paint(cx.backend, rect, &self.widget_tokens());
    }
}
