//! Serialize the same static icon/widget visuals used by the design canvas.
//!
//! Their scene kinds are degraded rectangles/text for layout. Serializing
//! those kinds directly leaks select values and turns icon glyphs into boxes.

use super::{color_to_rgb, normalize_rect, stroke_attrs, svg_id, xml_escape};
use crate::layout_scene::{NodeKind, SceneNode};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use std::fmt::Write as _;

pub(super) fn has_widget_visual(node: &SceneNode) -> bool {
    node.widget.as_ref().is_some_and(|widget| {
        matches!(
            widget.kind.as_str(),
            "switch"
                | "checkbox"
                | "slider"
                | "progress"
                | "select"
                | "radio_group"
                | "text_input"
                | "text_area"
                | "number_input"
                | "tabs"
        ) && node.bounds.size.x > 0.0
            && node.bounds.size.y > 0.0
    })
}

pub(super) fn emit_visual(out: &mut String, node: &SceneNode) -> bool {
    let is_icon = matches!(&node.kind, NodeKind::Other(tag) if tag == "icon_font");
    if !is_icon && !has_widget_visual(node) {
        return false;
    }
    let mut backend = VisualBackend::new(&node.id);
    let painted = if is_icon {
        crate::widgets::icons::paint_icon_font_node(
            &mut backend,
            &node.font_family,
            node.text.as_deref().unwrap_or(""),
            normalize_rect(node.bounds),
            node.fill,
        );
        true
    } else {
        crate::widgets::canvas_viewport_widget::paint_widget_visual(
            &mut crate::widgets::PaintCx {
                backend: &mut backend,
            },
            node,
            normalize_rect(node.bounds),
            1.0,
        )
    };
    if painted {
        backend.close_groups(0);
        let _ = write!(
            out,
            r#"<g id="{}">{}</g>"#,
            xml_escape(&node.id),
            backend.markup
        );
    }
    painted
}

struct VisualBackend {
    markup: String,
    id: String,
    clip_index: usize,
    groups: usize,
    saved: Vec<usize>,
}

impl VisualBackend {
    fn new(id: &str) -> Self {
        Self {
            markup: String::new(),
            id: svg_id(id),
            clip_index: 0,
            groups: 0,
            saved: Vec::new(),
        }
    }

    fn close_groups(&mut self, count: usize) {
        while self.groups > count {
            self.markup.push_str("</g>");
            self.groups -= 1;
        }
    }

    fn rect(&mut self, r: Rect, radius: f32, attrs: &str) {
        let _ = write!(
            self.markup,
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}"{attrs}/>"#,
            r.origin.x,
            r.origin.y,
            r.size.x,
            r.size.y,
            radius.max(0.0)
        );
    }

    fn path(&mut self, d: &str, origin: Point2D, scale: f32, attrs: &str) {
        let _ = write!(
            self.markup,
            r#"<path d="{}" transform="translate({} {}) scale({scale})"{attrs}/>"#,
            xml_escape(d),
            origin.x,
            origin.y
        );
    }
}

fn fill(color: Color) -> String {
    format!(
        r#" fill="{}" fill-opacity="{}""#,
        color_to_rgb(color),
        color.a
    )
}

impl RenderBackend for VisualBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, r: Rect, color: Color) {
        self.rect(r, 0.0, &fill(color));
    }
    fn stroke_rect(&mut self, r: Rect, color: Color, width: f32) {
        self.stroke_round_rect(r, 0.0, color, width);
    }
    fn fill_round_rect(&mut self, r: Rect, radius: f32, color: Color) {
        self.rect(r, radius, &fill(color));
    }
    fn stroke_round_rect(&mut self, r: Rect, radius: f32, color: Color, width: f32) {
        self.rect(
            r,
            radius,
            &format!(r#" fill="none"{}"#, stroke_attrs(color, width)),
        );
    }
    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
        let _ = write!(
            self.markup,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke-linecap="round"{}/>"#,
            from.x,
            from.y,
            to.x,
            to.y,
            stroke_attrs(color, width)
        );
    }
    fn stroke_svg_path(&mut self, d: &str, origin: Point2D, size: f32, color: Color, width: f32) {
        let scale = size / 24.0;
        if scale <= 0.0 {
            return;
        }
        self.path(
            d,
            origin,
            scale,
            &format!(
                r#" fill="none" stroke-linecap="round" stroke-linejoin="round"{}"#,
                stroke_attrs(color, width / scale)
            ),
        );
    }
    fn fill_svg_path(&mut self, d: &str, origin: Point2D, size: f32, viewbox: f32, color: Color) {
        if viewbox > 0.0 {
            self.path(d, origin, size / viewbox, &fill(color));
        }
    }
    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        for run in layout.runs() {
            let _ = write!(
                self.markup,
                r#"<text x="{}" y="{}" font-family="{}" font-size="{}" font-weight="{}"{}>{}</text>"#,
                origin.x + run.origin.x,
                origin.y + run.origin.y,
                xml_escape(if run.font_family.is_empty() {
                    "system-ui, sans-serif"
                } else {
                    &run.font_family
                }),
                run.font_size,
                run.font_weight,
                fill(Color::rgba_u8(
                    run.color.r(),
                    run.color.g(),
                    run.color.b(),
                    f32::from(run.color.a()) / 255.0
                )),
                xml_escape(&run.content)
            );
        }
    }
    fn save(&mut self) {
        self.saved.push(self.groups);
    }
    fn restore(&mut self) {
        if let Some(count) = self.saved.pop() {
            self.close_groups(count);
        }
    }
    fn clip_rect(&mut self, r: Rect) {
        let id = format!("visual-clip-{}-{}", self.id, self.clip_index);
        self.clip_index += 1;
        let _ = write!(
            self.markup,
            r#"<defs><clipPath id="{id}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath></defs><g clip-path="url(#{id})">"#,
            r.origin.x, r.origin.y, r.size.x, r.size.y
        );
        self.groups += 1;
    }
    fn translate(&mut self, offset: Point2D) {
        let _ = write!(
            self.markup,
            r#"<g transform="translate({} {})">"#,
            offset.x, offset.y
        );
        self.groups += 1;
    }
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}
