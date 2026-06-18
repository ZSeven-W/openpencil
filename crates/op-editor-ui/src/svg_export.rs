//! Platform-neutral SVG serializer for a layout-resolved scene.
//!
//! Hosts pass the same [`LayoutScene`] they paint on canvas. The
//! serializer emits vector markup for the active page and is usable
//! from both native and wasm builds; host-specific code decides
//! whether to write the string to disk or download it in the browser.

use crate::layout_scene::{regular_polygon_points, LayoutScene, NodeKind, SceneNode, ScenePage};
use crate::{Color, Point2D, Rect};
use std::fmt::Write as _;

const MARGIN: f32 = 16.0;
const TEXT_DEFAULT_FONT_SIZE: f32 = 13.0;
const TEXT_DEFAULT_FILL: Color = Color {
    r: 0.08,
    g: 0.08,
    b: 0.08,
    a: 1.0,
};

/// Serialize the active page in `scene` as an SVG string.
pub fn serialize_active_page_svg(scene: &LayoutScene) -> Result<String, String> {
    let Some(page) = scene.active_page() else {
        return Err("no active page".into());
    };
    let bounds = page_bounds(page).ok_or("nothing to export")?;
    let view_x = bounds.origin.x - MARGIN;
    let view_y = bounds.origin.y - MARGIN;
    let view_w = bounds.size.x + MARGIN * 2.0;
    let view_h = bounds.size.y + MARGIN * 2.0;
    let mut svg = String::with_capacity(4096);
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_x} {view_y} {view_w} {view_h}" width="{view_w}" height="{view_h}">"#
    );
    for node in &page.children {
        emit_node(&mut svg, node);
    }
    svg.push_str("</svg>");
    Ok(svg)
}

fn page_bounds(page: &ScenePage) -> Option<Rect> {
    let mut acc = BoundsAcc::new();
    for n in &page.children {
        collect_bounds(n, glam::Affine2::IDENTITY, &mut acc);
    }
    acc.into_rect()
}

struct BoundsAcc {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl BoundsAcc {
    fn new() -> Self {
        Self {
            min_x: f32::INFINITY,
            min_y: f32::INFINITY,
            max_x: f32::NEG_INFINITY,
            max_y: f32::NEG_INFINITY,
        }
    }

    fn add(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.min_x = self.min_x.min(x0);
        self.min_y = self.min_y.min(y0);
        self.max_x = self.max_x.max(x1);
        self.max_y = self.max_y.max(y1);
    }

    fn into_rect(self) -> Option<Rect> {
        if !self.min_x.is_finite() {
            return None;
        }
        Some(Rect {
            origin: Point2D::new(self.min_x, self.min_y),
            size: Point2D::new(self.max_x - self.min_x, self.max_y - self.min_y),
        })
    }
}

fn collect_bounds(n: &SceneNode, parent_xform: glam::Affine2, acc: &mut BoundsAcc) {
    if n.hidden {
        return;
    }
    let pivot_rect = n.aggregate_bounds();
    let rotate_self =
        n.rotation.abs() > f32::EPSILON && (pivot_rect.size.x != 0.0 || pivot_rect.size.y != 0.0);
    let local_xform = if rotate_self {
        let pivot = glam::Vec2::new(
            pivot_rect.origin.x + pivot_rect.size.x * 0.5,
            pivot_rect.origin.y + pivot_rect.size.y * 0.5,
        );
        parent_xform
            * glam::Affine2::from_translation(pivot)
            * glam::Affine2::from_angle(n.rotation)
            * glam::Affine2::from_translation(-pivot)
    } else {
        parent_xform
    };
    if let Some(local_corners) = own_paint_corners(n) {
        for p in local_corners {
            let w = local_xform.transform_point2(p);
            acc.add(w.x, w.y, w.x, w.y);
        }
    }
    if n.clip_content && !n.children.is_empty() && n.bounds.size.x > 0.0 && n.bounds.size.y > 0.0 {
        let nr = normalize_rect(n.bounds);
        for (x, y) in [
            (nr.origin.x, nr.origin.y),
            (nr.origin.x + nr.size.x, nr.origin.y),
            (nr.origin.x + nr.size.x, nr.origin.y + nr.size.y),
            (nr.origin.x, nr.origin.y + nr.size.y),
        ] {
            let w = local_xform.transform_point2(glam::Vec2::new(x, y));
            acc.add(w.x, w.y, w.x, w.y);
        }
        return;
    }
    for child in &n.children {
        collect_bounds(child, local_xform, acc);
    }
}

fn own_paint_corners(n: &SceneNode) -> Option<Vec<glam::Vec2>> {
    let stroke_pad = n.stroke.map(|s| s.width * 0.5).unwrap_or(0.0);
    let (x0, y0, x1, y1) = match &n.kind {
        NodeKind::Rect | NodeKind::Ellipse | NodeKind::Polygon | NodeKind::Line => {
            let nr = normalize_rect(n.bounds);
            (
                nr.origin.x,
                nr.origin.y,
                nr.origin.x + nr.size.x,
                nr.origin.y + nr.size.y,
            )
        }
        NodeKind::Frame => {
            if n.fill.is_none() && n.stroke.is_none() {
                return None;
            }
            let nr = normalize_rect(n.bounds);
            (
                nr.origin.x,
                nr.origin.y,
                nr.origin.x + nr.size.x,
                nr.origin.y + nr.size.y,
            )
        }
        NodeKind::Other(tag) if tag == "icon_font" => {
            if n.text.as_ref().is_none_or(|s| s.trim().is_empty()) {
                return None;
            }
            let nr = normalize_rect(n.bounds);
            (
                nr.origin.x,
                nr.origin.y,
                nr.origin.x + nr.size.x,
                nr.origin.y + nr.size.y,
            )
        }
        NodeKind::Other(_) => {
            if n.fill.is_none() && n.stroke.is_none() {
                return None;
            }
            let nr = normalize_rect(n.bounds);
            (
                nr.origin.x,
                nr.origin.y,
                nr.origin.x + nr.size.x,
                nr.origin.y + nr.size.y,
            )
        }
        NodeKind::Text => {
            let has_text = n.text.as_ref().is_some_and(|s| !s.is_empty());
            if !has_text {
                return None;
            }
            let nr = normalize_rect(n.bounds);
            (
                nr.origin.x,
                nr.origin.y,
                nr.origin.x + nr.size.x.max(1.0),
                nr.origin.y + nr.size.y.max(1.0),
            )
        }
        NodeKind::Path => {
            if n.svg_path.is_some() && (n.fill.is_some() || n.stroke.is_some()) {
                let nr = normalize_rect(n.bounds);
                return Some(vec![
                    glam::Vec2::new(nr.origin.x - stroke_pad, nr.origin.y - stroke_pad),
                    glam::Vec2::new(
                        nr.origin.x + nr.size.x + stroke_pad,
                        nr.origin.y - stroke_pad,
                    ),
                    glam::Vec2::new(
                        nr.origin.x + nr.size.x + stroke_pad,
                        nr.origin.y + nr.size.y + stroke_pad,
                    ),
                    glam::Vec2::new(
                        nr.origin.x - stroke_pad,
                        nr.origin.y + nr.size.y + stroke_pad,
                    ),
                ]);
            }
            if n.points.is_empty() {
                return None;
            }
            let mut out = Vec::with_capacity(n.points.len() * 4);
            for p in &n.points {
                out.push(glam::Vec2::new(p.x - stroke_pad, p.y - stroke_pad));
                out.push(glam::Vec2::new(p.x + stroke_pad, p.y - stroke_pad));
                out.push(glam::Vec2::new(p.x - stroke_pad, p.y + stroke_pad));
                out.push(glam::Vec2::new(p.x + stroke_pad, p.y + stroke_pad));
            }
            return Some(out);
        }
        NodeKind::Group => return None,
    };
    if (x1 - x0).abs() == 0.0 && (y1 - y0).abs() == 0.0 {
        return None;
    }
    Some(vec![
        glam::Vec2::new(x0 - stroke_pad, y0 - stroke_pad),
        glam::Vec2::new(x1 + stroke_pad, y0 - stroke_pad),
        glam::Vec2::new(x1 + stroke_pad, y1 + stroke_pad),
        glam::Vec2::new(x0 - stroke_pad, y1 + stroke_pad),
    ])
}

fn emit_node(out: &mut String, n: &SceneNode) {
    if n.hidden {
        return;
    }
    let pivot = n.aggregate_bounds();
    let needs_g = n.rotation.abs() > f32::EPSILON && (pivot.size.x != 0.0 || pivot.size.y != 0.0);
    if needs_g {
        let cx = pivot.origin.x + pivot.size.x * 0.5;
        let cy = pivot.origin.y + pivot.size.y * 0.5;
        let deg = n.rotation.to_degrees();
        let _ = write!(out, r#"<g transform="rotate({deg} {cx} {cy})">"#);
    }
    match &n.kind {
        NodeKind::Rect | NodeKind::Frame => emit_rect(out, n),
        NodeKind::Ellipse => emit_ellipse(out, n),
        NodeKind::Polygon => emit_polygon(out, n),
        NodeKind::Line => emit_line(out, n),
        NodeKind::Path => emit_path(out, n),
        NodeKind::Text => emit_text(out, n),
        NodeKind::Group | NodeKind::Other(_) => {}
    }
    for child in &n.children {
        emit_node(out, child);
    }
    if needs_g {
        out.push_str("</g>");
    }
}

fn emit_rect(out: &mut String, n: &SceneNode) {
    if n.fill.is_none() && n.stroke.is_none() && !matches!(n.kind, NodeKind::Rect) {
        return;
    }
    let r = normalize_rect(n.bounds);
    if r.size.x == 0.0 && r.size.y == 0.0 {
        return;
    }
    let rx = if n.corner_radius > 0.0 {
        format!(r#" rx="{}""#, n.corner_radius)
    } else {
        String::new()
    };
    let _ = write!(
        out,
        r#"<rect x="{}" y="{}" width="{}" height="{}"{rx}{}/>"#,
        r.origin.x,
        r.origin.y,
        r.size.x,
        r.size.y,
        fill_stroke_attrs(n),
    );
}

fn emit_ellipse(out: &mut String, n: &SceneNode) {
    let r = normalize_rect(n.bounds);
    if r.size.x == 0.0 || r.size.y == 0.0 {
        return;
    }
    let cx = r.origin.x + r.size.x * 0.5;
    let cy = r.origin.y + r.size.y * 0.5;
    let _ = write!(
        out,
        r#"<ellipse cx="{cx}" cy="{cy}" rx="{}" ry="{}"{}/>"#,
        r.size.x * 0.5,
        r.size.y * 0.5,
        fill_stroke_attrs(n),
    );
}

fn emit_polygon(out: &mut String, n: &SceneNode) {
    let r = normalize_rect(n.bounds);
    if r.size.x == 0.0 || r.size.y == 0.0 {
        return;
    }
    let points = regular_polygon_points(r, n.polygon_sides);
    let mut point_attr = String::new();
    for (i, p) in points.iter().enumerate() {
        if i > 0 {
            point_attr.push(' ');
        }
        let _ = write!(point_attr, "{},{}", p.x, p.y);
    }
    let _ = write!(
        out,
        r#"<polygon points="{point_attr}"{}/>"#,
        fill_stroke_attrs(n),
    );
}

fn emit_line(out: &mut String, n: &SceneNode) {
    let (color, width) = match n.stroke {
        Some(s) => (s.color, s.width),
        None => (n.fill.unwrap_or(Color::BLACK), 1.5),
    };
    let r = n.bounds;
    let x2 = r.origin.x + r.size.x;
    let y2 = r.origin.y + r.size.y;
    let _ = write!(
        out,
        r#"<line x1="{}" y1="{}" x2="{x2}" y2="{y2}"{}/>"#,
        r.origin.x,
        r.origin.y,
        stroke_attrs(color, width),
    );
}

fn emit_text(out: &mut String, n: &SceneNode) {
    let Some(text) = n.text.as_deref() else {
        return;
    };
    if text.is_empty() {
        return;
    }
    let r = normalize_rect(n.bounds);
    let color = n.fill.unwrap_or(TEXT_DEFAULT_FILL);
    let base_size = if n.font_size > 0.0 {
        n.font_size
    } else {
        TEXT_DEFAULT_FONT_SIZE
    };
    let line_h = base_size * 1.35;
    let fill_attr = if color.a < 0.999 {
        format!(
            r#" fill="{}" fill-opacity="{}""#,
            color_to_rgb(color),
            color.a
        )
    } else {
        format!(r#" fill="{}""#, color_to_rgb(color))
    };
    let weight_attr = if n.font_weight > 0 {
        format!(r#" font-weight="{}""#, n.font_weight)
    } else {
        String::new()
    };
    let mut baseline_y = r.origin.y + base_size + 1.0;
    for line in text.split('\n') {
        let _ = write!(
            out,
            r#"<text x="{}" y="{baseline_y}" font-family="system-ui, sans-serif" font-size="{base_size}"{weight_attr}{fill_attr}>{}</text>"#,
            r.origin.x,
            xml_escape(line),
        );
        baseline_y += line_h;
    }
}

fn emit_path(out: &mut String, n: &SceneNode) {
    if n.points.len() < 2 {
        return;
    }
    let (color, width) = match n.stroke {
        Some(s) => (s.color, s.width),
        None => (n.fill.unwrap_or(Color::BLACK), 1.5),
    };
    let mut d = String::with_capacity(n.points.len() * 16);
    let _ = write!(d, "M{} {}", n.points[0].x, n.points[0].y);
    for p in &n.points[1..] {
        let _ = write!(d, " L{} {}", p.x, p.y);
    }
    let _ = write!(
        out,
        r#"<path d="{d}" fill="none"{}/>"#,
        stroke_attrs(color, width),
    );
}

fn normalize_rect(r: Rect) -> Rect {
    let x0 = r.origin.x.min(r.origin.x + r.size.x);
    let y0 = r.origin.y.min(r.origin.y + r.size.y);
    Rect {
        origin: Point2D::new(x0, y0),
        size: Point2D::new(r.size.x.abs(), r.size.y.abs()),
    }
}

fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

fn fill_stroke_attrs(n: &SceneNode) -> String {
    let mut s = String::new();
    if let Some(fill) = n.fill {
        let _ = write!(s, r#" fill="{}""#, color_to_rgb(fill));
        if fill.a < 0.999 {
            let _ = write!(s, r#" fill-opacity="{}""#, fill.a);
        }
    } else {
        s.push_str(r#" fill="none""#);
    }
    if let Some(stroke) = n.stroke {
        s.push_str(&stroke_attrs(stroke.color, stroke.width));
    }
    s
}

fn stroke_attrs(color: Color, width: f32) -> String {
    let mut s = format!(
        r#" stroke="{}" stroke-width="{}""#,
        color_to_rgb(color),
        width
    );
    if color.a < 0.999 {
        let _ = write!(s, r#" stroke-opacity="{}""#, color.a);
    }
    s
}

fn color_to_rgb(c: Color) -> String {
    format!(
        "rgb({},{},{})",
        (c.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.b.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_with(children: Vec<SceneNode>) -> LayoutScene {
        LayoutScene {
            pages: vec![ScenePage {
                id: "p1".into(),
                name: "Page 1".into(),
                children,
            }],
            active_page_index: 0,
        }
    }

    fn filled_rect(id: &str, x: f32, y: f32, w: f32, h: f32, fill: Color) -> SceneNode {
        let mut n = SceneNode::leaf(id, NodeKind::Rect);
        n.bounds = Rect::xywh(x, y, w, h);
        n.fill = Some(fill);
        n
    }

    #[test]
    fn active_page_svg_contains_vector_markup() {
        let scene = scene_with(vec![filled_rect(
            "r1",
            5.0,
            5.0,
            120.0,
            60.0,
            Color {
                r: 0.2,
                g: 0.4,
                b: 0.6,
                a: 1.0,
            },
        )]);

        let body = serialize_active_page_svg(&scene).expect("svg");

        assert!(body.starts_with("<svg "), "missing svg root: {body}");
        assert!(body.contains("<rect "), "missing rect element: {body}");
        assert!(body.contains(r#"width="120""#), "rect width wrong: {body}");
        assert!(body.ends_with("</svg>"), "missing svg close: {body}");
    }

    #[test]
    fn active_page_svg_fails_on_empty_scene() {
        let scene = scene_with(Vec::new());
        let res = serialize_active_page_svg(&scene);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "nothing to export");
    }
}
