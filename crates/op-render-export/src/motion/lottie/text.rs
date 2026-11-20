//! Capture the shared text layout as glyph outlines, using native font metrics.
use op_editor_ui::{
    layout_scene::SceneNode, Color, Point2D, Rect, RenderBackend, TextBaselineRequest, TextLayout,
};
use op_host_native::NativeBackend;
use std::fmt::Write;

pub(super) fn markup(node: &SceneNode, native: &mut NativeBackend) -> Option<String> {
    let mut backend = TextPaths {
        native,
        markup: String::new(),
        complete: true,
    };
    op_editor_ui::widgets::paint_text_node(
        &mut op_editor_ui::widgets::PaintCx {
            backend: &mut backend,
        },
        node,
        node.bounds,
        1.0,
        &None,
    );
    backend.complete.then_some(backend.markup)
}

struct TextPaths<'a> {
    native: &'a mut NativeBackend,
    markup: String,
    complete: bool,
}

impl TextPaths<'_> {
    fn path(&mut self, path: skia_safe::Path, color: Color, width: Option<f32>) {
        let c = format!(
            "rgb({},{},{})",
            (color.r * 255.0).round(),
            (color.g * 255.0).round(),
            (color.b * 255.0).round()
        );
        let style = width
            .map(|w| {
                format!(
                    r#"fill="none" stroke="{c}" stroke-width="{w}" stroke-opacity="{}""#,
                    color.a
                )
            })
            .unwrap_or_else(|| format!(r#"fill="{c}" fill-opacity="{}""#, color.a));
        let _ = write!(self.markup, r#"<path d="{}" {style}/>"#, path.to_svg());
    }
}

impl RenderBackend for TextPaths<'_> {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, r: Rect, c: Color) {
        self.path(
            skia_safe::Path::rect(
                skia_safe::Rect::from_xywh(r.origin.x, r.origin.y, r.size.x, r.size.y),
                None,
            ),
            c,
            None,
        );
    }
    fn stroke_rect(&mut self, r: Rect, c: Color, w: f32) {
        self.path(
            skia_safe::Path::rect(
                skia_safe::Rect::from_xywh(r.origin.x, r.origin.y, r.size.x, r.size.y),
                None,
            ),
            c,
            Some(w),
        );
    }
    fn fill_round_rect(&mut self, r: Rect, _radius: f32, c: Color) {
        self.fill_rect(r, c);
    }
    fn stroke_round_rect(&mut self, r: Rect, _radius: f32, c: Color, w: f32) {
        self.stroke_rect(r, c, w);
    }
    fn stroke_line(&mut self, a: Point2D, b: Point2D, c: Color, w: f32) {
        let mut path = skia_safe::PathBuilder::new();
        path.move_to((a.x, a.y));
        path.line_to((b.x, b.y));
        self.path(path.detach(), c, Some(w));
    }
    fn stroke_svg_path(&mut self, _d: &str, _p: Point2D, _s: f32, _c: Color, _w: f32) {
        self.complete = false;
    }
    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        if let Some(paths) = self.native.outline_text(layout, origin) {
            for (path, paint) in paths {
                let c = paint.color4f();
                let c = Color {
                    r: c.r,
                    g: c.g,
                    b: c.b,
                    a: c.a,
                };
                self.path(path.clone(), c, None);
                if paint.style() == skia_safe::PaintStyle::StrokeAndFill {
                    self.path(path, c, Some(paint.stroke_width()));
                }
            }
        } else {
            self.complete = false;
        }
    }
    fn clip_rect(&mut self, _r: Rect) {
        self.complete = false;
    }
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _p: Point2D) {
        self.complete = false;
    }
    fn resize(&mut self, _w: u32, _h: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
    fn measure_text(&mut self, t: &str, s: f32) -> f32 {
        self.native.measure_text(t, s)
    }
    fn measure_text_weighted(&mut self, t: &str, s: f32, w: u16) -> f32 {
        self.native.measure_text_weighted(t, s, w)
    }
    fn text_ascent(&mut self, s: f32, w: u16) -> f32 {
        self.native.text_ascent(s, w)
    }
    fn text_ascent_family(&mut self, s: f32, f: &str, w: u16) -> f32 {
        self.native.text_ascent_family(s, f, w)
    }
    fn text_first_baseline(&mut self, r: &TextBaselineRequest<'_>) -> f32 {
        self.native.text_first_baseline(r)
    }
    fn measure_text_styled(&mut self, t: &str, s: f32, w: u16, i: bool) -> f32 {
        self.native.measure_text_styled(t, s, w, i)
    }
    fn measure_text_family(&mut self, t: &str, s: f32, f: &str) -> f32 {
        self.native.measure_text_family(t, s, f)
    }
    fn measure_text_family_styled(&mut self, t: &str, s: f32, f: &str, w: u16, i: bool) -> f32 {
        self.native.measure_text_family_styled(t, s, f, w, i)
    }
}
