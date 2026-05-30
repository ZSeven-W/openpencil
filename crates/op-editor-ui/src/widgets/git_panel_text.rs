//! GitPanel text-drawing helpers (footer + one-line text + the
//! `Color` -> jian conversion), split out of `git_panel.rs` to keep
//! it under the 800-line cap.

use super::git_panel::GitPanel;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, TextLayout};

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
        let layout =
            TextLayout::single_run(s, "system-ui", size, to_jian(color), Point2D::new(0.0, 0.0));
        cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
    }
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
