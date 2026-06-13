//! Floating IME preedit (composition) overlay.
//!
//! TS/Electron renders CJK preedit inline inside DOM inputs; the
//! Rust shell's inputs paint their drafts themselves, so in-flight
//! composition would otherwise be invisible until commit. This
//! overlay paints the composition string in a small bubble anchored
//! under the focused input (or at a bottom-center fallback), with
//! the IME's highlighted segment underlined — composition is always
//! visible, and the host anchors the OS candidate window to the same
//! rect via `set_ime_cursor_area`.
//!
//! Documented divergence from TS: the preedit paints as a floating
//! bubble rather than inline at the caret; inline rendering would
//! need per-input paint surgery across ten input surfaces.

use crate::widgets::property_panel_inputs::to_jian_color;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout, Theme};

const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 6.0;
const FONT_SIZE: f32 = 13.0;
const RADIUS: f32 = 6.0;

/// Paint the preedit bubble. `anchor` is the focused input's rect
/// (bubble sits just below its bottom-left corner); `None` falls
/// back to bottom-center of the viewport, above the status bar.
pub fn paint_ime_preedit(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    viewport: Rect,
    anchor: Option<Rect>,
    text: &str,
    highlight: Option<(usize, usize)>,
) {
    if text.is_empty() {
        return;
    }
    let text_w = cx.backend.measure_text(text, FONT_SIZE);
    let w = text_w + PAD_X * 2.0;
    let h = FONT_SIZE + PAD_Y * 2.0;
    let (x, y) = match anchor {
        Some(a) => {
            // Below the input; clamp into the viewport.
            let x = a.origin.x.clamp(
                viewport.origin.x,
                (viewport.origin.x + viewport.size.x - w).max(viewport.origin.x),
            );
            let mut y = a.origin.y + a.size.y + 4.0;
            if y + h > viewport.origin.y + viewport.size.y {
                y = (a.origin.y - h - 4.0).max(viewport.origin.y);
            }
            (x, y)
        }
        None => (
            viewport.origin.x + (viewport.size.x - w) / 2.0,
            viewport.origin.y + viewport.size.y - h - 56.0,
        ),
    };
    let rect = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, h),
    };
    cx.backend.fill_round_rect(rect, RADIUS, theme.card);
    cx.backend
        .stroke_round_rect(rect, RADIUS, theme.border, 1.0);
    let origin = Point2D::new(x + PAD_X, y + PAD_Y);
    cx.backend.draw_text(
        &TextLayout::single_run(
            text,
            "system-ui",
            FONT_SIZE,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        ),
        origin,
    );
    // Underline: the IME-highlighted segment when reported, else the
    // whole composition (the macOS convention for raw preedit).
    let (from, to) = match highlight {
        Some((a, b)) if a < b && b <= text.len() => (a, b),
        _ => (0, text.len()),
    };
    let pre_w = if from > 0 {
        cx.backend.measure_text(&text[..from], FONT_SIZE)
    } else {
        0.0
    };
    let seg_w = cx.backend.measure_text(&text[from..to], FONT_SIZE);
    if seg_w > 0.0 {
        let uy = y + h - 3.0;
        cx.backend.stroke_line(
            Point2D::new(x + PAD_X + pre_w, uy),
            Point2D::new(x + PAD_X + pre_w + seg_w, uy),
            underline_color(theme),
            1.5,
        );
    }
}

fn underline_color(theme: &Theme) -> Color {
    // Primary-tinted underline reads as "active composition".
    theme.primary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct CountingBackend {
        ops: usize,
    }

    impl crate::RenderBackend for CountingBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {
            self.ops += 1;
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {
            self.ops += 1;
        }
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {
            self.ops += 1;
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {
            self.ops += 1;
        }
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {
            self.ops += 1;
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {
            self.ops += 1;
        }
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn empty_preedit_paints_nothing() {
        let mut backend = CountingBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let theme = Theme::dark();

        paint_ime_preedit(
            &mut cx,
            &theme,
            Rect::xywh(0.0, 0.0, 320.0, 200.0),
            None,
            "",
            None,
        );

        assert_eq!(backend.ops, 0);
    }
}
