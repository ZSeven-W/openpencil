//! Code-tab placeholder for the PropertyPanel. Renders a few JSX-
//! style lines describing the selected node — full code-gen pipe
//! lands later alongside the TS parity work.

use crate::theme::Theme;
use crate::widgets::property_panel::NodeSnapshot;
use crate::widgets::PaintCx;
use crate::{Point2D, TextLayout};

pub fn paint_code_placeholder(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    x: f32,
    y: f32,
    w: f32,
) {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    let muted = jian_core::scene::Color::rgba(
        ch(theme.muted_foreground.r),
        ch(theme.muted_foreground.g),
        ch(theme.muted_foreground.b),
        ch(theme.muted_foreground.a),
    );
    let fg = jian_core::scene::Color::rgba(
        ch(theme.foreground.r),
        ch(theme.foreground.g),
        ch(theme.foreground.b),
        ch(theme.foreground.a),
    );
    let hint = TextLayout::single_run(
        "// Generated code preview",
        "system-ui",
        12.0,
        muted,
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&hint, Point2D::new(x + 16.0, y + 24.0));
    let lines = [
        format!("<{} ", snapshot.kind_variant.label()),
        format!("  x={} y={}", snapshot.x, snapshot.y),
        format!("  w={} h={}", snapshot.width, snapshot.height),
        "/>".to_string(),
    ];
    let mut line_y = y + 52.0;
    for line in &lines {
        let layout = TextLayout::single_run(line, "monospace", 13.0, fg, Point2D::new(0.0, 0.0));
        cx.backend
            .draw_text(&layout, Point2D::new(x + 16.0, line_y));
        line_y += 22.0;
    }
    let _ = w;
}
