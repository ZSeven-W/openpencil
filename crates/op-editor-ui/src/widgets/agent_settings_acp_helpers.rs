//! Small paint/text helpers for the ACP agent-settings section, split out of
//! `agent_settings_acp.rs` to keep that file under the 800-line cap.

use crate::widgets::PaintCx;
use crate::{Color, Point2D, TextLayout};

pub(super) fn draw_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, y: f32) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y));
}

pub(super) fn ellipsize(cx: &mut PaintCx<'_>, value: &str, max_w: f32, size: f32) -> String {
    if cx.backend.measure_text(value, size) <= max_w {
        return value.to_string();
    }
    let mut out = value.to_string();
    while !out.is_empty() && cx.backend.measure_text(&format!("{out}..."), size) > max_w {
        out.pop();
    }
    format!("{out}...")
}
