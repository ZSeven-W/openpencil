//! Hand-rolled SVG serializer for the active page of a [`LayoutScene`].
//!
//! Walks the same `SceneNode` tree the raster path paints and emits
//! one SVG element per node kind. Geometry is layout-resolved, so the
//! element coordinates are absolute doc-space — only the `viewBox` is
//! offset by the page bounds + margin. Rotation wraps the element (and
//! its children) in a `<g transform="rotate(...)">`, matching how the
//! raster path composes `Canvas::rotate`.

use openpencil_shell_core::document::NodeKind;
use openpencil_shell_core::layout_scene::{LayoutScene, SceneNode};
use openpencil_shell_core::Color;
use std::fmt::Write as _;
use std::path::Path as StdPath;

use super::{MARGIN, TEXT_DEFAULT_FILL, TEXT_DEFAULT_FONT_SIZE};

/// Serialize the scene's active page to an SVG file at `target`.
pub fn export_svg(scene: &LayoutScene, target: &StdPath) -> Result<(), String> {
    let Some(page) = scene.active_page() else {
        return Err("no active page".into());
    };
    let bounds = super::page_bounds(page).ok_or("nothing to export")?;
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
    std::fs::write(target, svg).map_err(|e| e.to_string())?;
    Ok(())
}

fn emit_node(out: &mut String, n: &SceneNode) {
    if n.hidden {
        return;
    }
    // SVG groups inherit transform onto children — same composition
    // model as the raster `paint_node`. Rotation pivots around the
    // node's `aggregate_bounds` centre, matching the raster path.
    let pivot = n.aggregate_bounds();
    let needs_g = n.rotation.abs() > f32::EPSILON
        && (pivot.size.x != 0.0 || pivot.size.y != 0.0);
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
        // Group + tagged kinds (`icon_font`) emit no own element.
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
    let r = normalize(n);
    if r.0.size.x == 0.0 && r.0.size.y == 0.0 {
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
        r.0.origin.x,
        r.0.origin.y,
        r.0.size.x,
        r.0.size.y,
        fill_stroke_attrs(n),
    );
}

fn emit_ellipse(out: &mut String, n: &SceneNode) {
    let (r, _) = normalize(n);
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
    let (r, _) = normalize(n);
    if r.size.x == 0.0 || r.size.y == 0.0 {
        return;
    }
    let cx = r.origin.x + r.size.x * 0.5;
    let top = r.origin.y;
    let left = r.origin.x;
    let right = r.origin.x + r.size.x;
    let bottom = r.origin.y + r.size.y;
    let _ = write!(
        out,
        r#"<polygon points="{cx},{top} {left},{bottom} {right},{bottom}"{}/>"#,
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
    let Some(text) = n.text.as_deref() else { return };
    if text.is_empty() {
        return;
    }
    let (r, _) = normalize(n);
    // Same defaults as the raster `paint_text` + editor canvas so the
    // SVG renders identically to what the user sees on screen.
    let color = n.fill.unwrap_or(TEXT_DEFAULT_FILL);
    let base_size = if n.font_size > 0.0 {
        n.font_size
    } else {
        TEXT_DEFAULT_FONT_SIZE
    };
    let line_h = base_size * 1.35;
    let fill_attr = if color.a < 0.999 {
        format!(r#" fill="{}" fill-opacity="{}""#, color_to_rgb(color), color.a)
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

/// Normalised bounds for `n` (defensive against negative extents) —
/// returns the rect and a unit tuple so callers can `(r, _)`-destructure.
fn normalize(n: &SceneNode) -> (openpencil_shell_core::Rect, ()) {
    let r = n.bounds;
    let x0 = r.origin.x.min(r.origin.x + r.size.x);
    let y0 = r.origin.y.min(r.origin.y + r.size.y);
    (
        openpencil_shell_core::Rect::xywh(x0, y0, r.size.x.abs(), r.size.y.abs()),
        (),
    )
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

/// Stroke `color` + `width` attributes for any element that paints a
/// stroke. Emits `stroke-opacity` when alpha < 1 so semi-transparent
/// strokes round-trip through SVG instead of collapsing to opaque.
fn stroke_attrs(color: Color, width: f32) -> String {
    let mut s = format!(r#" stroke="{}" stroke-width="{}""#, color_to_rgb(color), width);
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
    use crate::export::test_support::{filled_rect, scene_with};

    #[test]
    fn export_svg_writes_rect_element() {
        let scene = scene_with(vec![filled_rect(
            "r1",
            5.0,
            5.0,
            120.0,
            60.0,
            Color { r: 0.2, g: 0.4, b: 0.6, a: 1.0 },
        )]);
        let tmp = std::env::temp_dir().join(format!("op-export-svg-{}.svg", std::process::id()));
        let res = export_svg(&scene, &tmp);
        assert!(res.is_ok(), "export_svg failed: {res:?}");
        let body = std::fs::read_to_string(&tmp).unwrap();
        assert!(body.starts_with("<svg "), "missing svg root: {body}");
        assert!(body.contains("<rect "), "missing rect element: {body}");
        assert!(body.contains(r#"width="120""#), "rect width wrong: {body}");
        assert!(body.ends_with("</svg>"), "missing svg close: {body}");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn export_svg_fails_on_empty_scene() {
        let scene = scene_with(Vec::new());
        let tmp = std::env::temp_dir().join(format!("op-svg-empty-{}.svg", std::process::id()));
        let res = export_svg(&scene, &tmp);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "nothing to export");
    }
}
