//! Platform-neutral SVG serializer for a layout-resolved scene.
//!
//! Hosts pass the same [`LayoutScene`] they paint on canvas. The
//! serializer emits vector markup for the active page and is usable
//! from both native and wasm builds; host-specific code decides
//! whether to write the string to disk or download it in the browser.

use crate::layout_scene::{
    regular_polygon_points, LayoutScene, NodeKind, SceneGradient, SceneImageFit, SceneNode,
    ScenePage,
};
use crate::{Color, Point2D, Rect};
use std::fmt::Write as _;

mod image_metadata;
mod transform;

use transform::{
    apply_ancestor_clip_bounds, apply_clip_bounds, close_ancestor_clip_groups,
    emit_affine_group_start, emit_ancestor_clip_defs, emit_ancestor_clip_groups_start,
    find_node_with_ancestor_context, node_transform, svg_id, AncestorClip,
};

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
    serialize_svg(
        &page.children,
        page_bounds(page).ok_or("nothing to export")?,
        MARGIN,
        glam::Affine2::IDENTITY,
        &[],
    )
}

/// Serialize one node and its complete subtree, tightly cropped to its
/// painted bounds. The lookup is scoped to the active page.
pub fn serialize_node_svg(scene: &LayoutScene, node_id: &str) -> Result<String, String> {
    let page = scene.active_page().ok_or("no active page")?;
    let (node, ancestor_xform, ancestor_clips) =
        find_node_with_ancestor_context(&page.children, node_id)
            .ok_or_else(|| format!("node {node_id} not found on the active page"))?;
    let mut acc = BoundsAcc::new();
    collect_bounds(node, ancestor_xform, &mut acc);
    let painted_bounds = acc
        .into_rect()
        .ok_or_else(|| format!("node {node_id} paints nothing"))?;
    let bounds = apply_ancestor_clip_bounds(painted_bounds, &ancestor_clips)
        .ok_or_else(|| format!("node {node_id} paints nothing"))?;
    serialize_svg(
        std::slice::from_ref(node),
        bounds,
        0.0,
        ancestor_xform,
        &ancestor_clips,
    )
}

fn serialize_svg(
    nodes: &[SceneNode],
    bounds: Rect,
    margin: f32,
    root_xform: glam::Affine2,
    ancestor_clips: &[AncestorClip],
) -> Result<String, String> {
    let view_x = bounds.origin.x - margin;
    let view_y = bounds.origin.y - margin;
    let view_w = bounds.size.x + margin * 2.0;
    let view_h = bounds.size.y + margin * 2.0;
    if view_w <= 0.0 || view_h <= 0.0 {
        return Err("nothing to export".into());
    }
    let mut svg = String::with_capacity(4096);
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_x} {view_y} {view_w} {view_h}" width="{view_w}" height="{view_h}">"#
    );
    emit_defs(&mut svg, nodes, ancestor_clips);
    emit_ancestor_clip_groups_start(&mut svg, ancestor_clips);
    let wraps_root = root_xform != glam::Affine2::IDENTITY;
    if wraps_root {
        emit_affine_group_start(&mut svg, root_xform);
    }
    for node in nodes {
        emit_node(&mut svg, node);
    }
    if wraps_root {
        svg.push_str("</g>");
    }
    close_ancestor_clip_groups(&mut svg, ancestor_clips);
    svg.push_str("</svg>");
    Ok(svg)
}

fn emit_defs(out: &mut String, nodes: &[SceneNode], ancestor_clips: &[AncestorClip]) {
    let mut defs = String::new();
    emit_ancestor_clip_defs(&mut defs, ancestor_clips);
    for node in nodes {
        emit_node_defs(&mut defs, node);
    }
    if !defs.is_empty() {
        out.push_str("<defs>");
        out.push_str(&defs);
        out.push_str("</defs>");
    }
}

fn emit_node_defs(out: &mut String, n: &SceneNode) {
    let id = svg_id(&n.id);
    if let Some(gradient) = &n.gradient {
        emit_gradient(
            out,
            &format!("gradient-{id}"),
            gradient,
            normalize_rect(n.bounds),
        );
    }
    let clips_image = n.image_src.is_some() && n.corner_radius > 0.0;
    if (n.clip_content || clips_image) && n.bounds.size.x != 0.0 && n.bounds.size.y != 0.0 {
        let r = normalize_rect(n.bounds);
        let _ = write!(
            out,
            r#"<clipPath id="clip-{id}"><rect x="{}" y="{}" width="{}" height="{}" rx="{}"/></clipPath>"#,
            r.origin.x,
            r.origin.y,
            r.size.x,
            r.size.y,
            n.corner_radius.max(0.0),
        );
    }
    if n.image_fit == SceneImageFit::Tile {
        if let Some(src) = n.image_src.as_deref() {
            emit_image_pattern(out, n, src, &id);
        }
    }
    for child in &n.children {
        emit_node_defs(out, child);
    }
}

fn emit_gradient(out: &mut String, id: &str, gradient: &SceneGradient, bounds: Rect) {
    match gradient {
        SceneGradient::Linear {
            angle_deg,
            opacity,
            stops,
        } => {
            let angle = (angle_deg - 90.0).to_radians();
            let cx = bounds.origin.x + bounds.size.x * 0.5;
            let cy = bounds.origin.y + bounds.size.y * 0.5;
            let dx = angle.cos() * bounds.size.x * 0.5;
            let dy = angle.sin() * bounds.size.y * 0.5;
            let _ = write!(
                out,
                r#"<linearGradient id="{id}" gradientUnits="userSpaceOnUse" x1="{}" y1="{}" x2="{}" y2="{}">"#,
                cx - dx,
                cy - dy,
                cx + dx,
                cy + dy,
            );
            emit_stops(out, stops, *opacity);
            out.push_str("</linearGradient>");
        }
        SceneGradient::Radial {
            cx,
            cy,
            radius,
            opacity,
            stops,
        } => {
            let center_x = bounds.origin.x + bounds.size.x * cx.clamp(0.0, 1.0);
            let center_y = bounds.origin.y + bounds.size.y * cy.clamp(0.0, 1.0);
            let outer = (bounds.size.x.max(bounds.size.y) * radius.clamp(0.0, 1.0)).max(0.01);
            let _ = write!(
                out,
                r#"<radialGradient id="{id}" gradientUnits="userSpaceOnUse" cx="{center_x}" cy="{center_y}" r="{outer}">"#
            );
            emit_stops(out, stops, *opacity);
            out.push_str("</radialGradient>");
        }
        SceneGradient::Mesh {
            colors, opacity, ..
        } => {
            if let Some(color) = colors.first() {
                let _ = write!(
                    out,
                    r#"<linearGradient id="{id}"><stop offset="0" stop-color="{}" stop-opacity="{}"/><stop offset="1" stop-color="{}" stop-opacity="{}"/></linearGradient>"#,
                    color_to_rgb(*color),
                    color.a * opacity,
                    color_to_rgb(*color),
                    color.a * opacity
                );
            }
        }
    }
}

fn emit_stops(out: &mut String, stops: &[crate::layout_scene::SceneGradientStop], opacity: f32) {
    // `SceneGradient::opacity` already includes the node's cumulative
    // layer opacity, matching the canvas shader path. Applying
    // `SceneNode::opacity` again here or on an enclosing group would
    // double-dim gradients and descendants.
    for stop in stops {
        let _ = write!(
            out,
            r#"<stop offset="{}" stop-color="{}" stop-opacity="{}"/>"#,
            stop.offset.clamp(0.0, 1.0),
            color_to_rgb(stop.color),
            stop.color.a * opacity.clamp(0.0, 1.0)
        );
    }
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
    let local_xform = parent_xform * node_transform(n);
    if let Some(local_corners) = own_paint_corners(n) {
        for p in local_corners {
            let w = local_xform.transform_point2(p);
            acc.add(w.x, w.y, w.x, w.y);
        }
    }
    if n.clip_content && !n.children.is_empty() && n.bounds.size.x > 0.0 && n.bounds.size.y > 0.0 {
        let mut child_acc = BoundsAcc::new();
        for child in &n.children {
            collect_bounds(child, local_xform, &mut child_acc);
        }
        if let Some(visible) = child_acc
            .into_rect()
            .and_then(|bounds| apply_clip_bounds(bounds, n.bounds, local_xform))
        {
            acc.add(
                visible.origin.x,
                visible.origin.y,
                visible.origin.x + visible.size.x,
                visible.origin.y + visible.size.y,
            );
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
            if n.fill.is_none() && n.gradient.is_none() && n.stroke.is_none() {
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
            if n.fill.is_none()
                && n.gradient.is_none()
                && n.stroke.is_none()
                && n.image_src.is_none()
            {
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
            if n.svg_path.is_some()
                && (n.fill.is_some() || n.gradient.is_some() || n.stroke.is_some())
            {
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
    let local_xform = node_transform(n);
    let needs_g = local_xform != glam::Affine2::IDENTITY;
    if needs_g {
        emit_affine_group_start(out, local_xform);
    }
    match &n.kind {
        NodeKind::Rect | NodeKind::Frame => emit_rect(out, n),
        NodeKind::Ellipse => emit_ellipse(out, n),
        NodeKind::Polygon => emit_polygon(out, n),
        NodeKind::Line => emit_line(out, n),
        NodeKind::Path => emit_path(out, n),
        NodeKind::Text => emit_text(out, n),
        NodeKind::Group => {}
        NodeKind::Other(_) => {
            if n.image_src.is_some() {
                emit_image(out, n);
            } else if n.fill.is_some() || n.gradient.is_some() || n.stroke.is_some() {
                emit_rect(out, n);
            }
        }
    }
    let clips_children = n.clip_content
        && !n.children.is_empty()
        && n.bounds.size.x != 0.0
        && n.bounds.size.y != 0.0;
    if clips_children {
        let _ = write!(out, r#"<g clip-path="url(#clip-{})">"#, svg_id(&n.id));
    }
    for child in &n.children {
        emit_node(out, child);
    }
    if clips_children {
        out.push_str("</g>");
    }
    if needs_g {
        out.push_str("</g>");
    }
}

fn emit_rect(out: &mut String, n: &SceneNode) {
    if n.fill.is_none()
        && n.gradient.is_none()
        && n.stroke.is_none()
        && !matches!(n.kind, NodeKind::Rect)
    {
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
        r#"<rect id="{}" x="{}" y="{}" width="{}" height="{}"{rx}{}/>"#,
        xml_escape(&n.id),
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
        r#"<ellipse id="{}" cx="{cx}" cy="{cy}" rx="{}" ry="{}"{}/>"#,
        xml_escape(&n.id),
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
        r#"<polygon id="{}" points="{point_attr}"{}/>"#,
        xml_escape(&n.id),
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
        r#"<line id="{}" x1="{}" y1="{}" x2="{x2}" y2="{y2}"{}/>"#,
        xml_escape(&n.id),
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
    let _ = write!(out, r#"<g id="{}">"#, xml_escape(&n.id));
    let mut baseline_y = r.origin.y + base_size + 1.0;
    for line in text.split('\n') {
        let _ = write!(
            out,
            r#"<text x="{}" y="{baseline_y}" font-family="{}" font-size="{base_size}"{weight_attr}{fill_attr}{}{}>{}</text>"#,
            r.origin.x,
            xml_escape(if n.font_family.is_empty() {
                "system-ui, sans-serif"
            } else {
                &n.font_family
            }),
            if n.italic {
                r#" font-style="italic""#
            } else {
                ""
            },
            text_decoration_attr(n),
            xml_escape(line),
        );
        baseline_y += line_h;
    }
    out.push_str("</g>");
}

fn emit_path(out: &mut String, n: &SceneNode) {
    let (d, transform) = if let Some(d) = &n.svg_path {
        (d.clone(), svg_path_fit_transform(d, n.bounds))
    } else {
        if n.points.len() < 2 {
            return;
        }
        let mut d = String::with_capacity(n.points.len() * 16);
        let _ = write!(d, "M{} {}", n.points[0].x, n.points[0].y);
        for p in &n.points[1..] {
            let _ = write!(d, " L{} {}", p.x, p.y);
        }
        if n.path_closed {
            d.push_str(" Z");
        }
        (d, String::new())
    };
    let attrs = if n.svg_path.is_none() && !n.path_closed && n.stroke.is_none() {
        let color = n.fill.unwrap_or(Color::BLACK);
        format!(r#" fill="none"{}"#, stroke_attrs(color, 1.5))
    } else {
        fill_stroke_attrs(n)
    };
    let vector_effect = if n.svg_path.is_some() && n.stroke.is_some() {
        r#" vector-effect="non-scaling-stroke""#
    } else {
        ""
    };
    let _ = write!(
        out,
        r#"<path id="{}" d="{}"{transform}{vector_effect}{}/>"#,
        xml_escape(&n.id),
        xml_escape(&d),
        attrs
    );
}

fn svg_path_fit_transform(d: &str, bounds: Rect) -> String {
    let rect = normalize_rect(bounds);
    let Some((source_x, source_y, source_w, source_h)) = op_editor_core::svg_path_data_bounds(d)
    else {
        return format!(
            r#" transform="translate({} {})""#,
            rect.origin.x, rect.origin.y
        );
    };
    let sx = if source_w.abs() > 0.01 {
        rect.size.x / source_w
    } else {
        1.0
    };
    let sy = if source_h.abs() > 0.01 {
        rect.size.y / source_h
    } else {
        1.0
    };
    let tx = rect.origin.x - source_x * sx;
    let ty = rect.origin.y - source_y * sy;
    format!(r#" transform="matrix({sx} 0 0 {sy} {tx} {ty})""#)
}

fn emit_image(out: &mut String, n: &SceneNode) {
    let Some(src) = n.image_src.as_deref() else {
        return;
    };
    let r = normalize_rect(n.bounds);
    let id = svg_id(&n.id);
    let clip = if n.corner_radius > 0.0 {
        format!(r#" clip-path="url(#clip-{id})""#)
    } else {
        String::new()
    };
    if n.image_fit == SceneImageFit::Tile {
        let _ = write!(
            out,
            r#"<rect id="{}" x="{}" y="{}" width="{}" height="{}" fill="url(#image-pattern-{id})" opacity="{}"{clip}/>"#,
            xml_escape(&n.id),
            r.origin.x,
            r.origin.y,
            r.size.x,
            r.size.y,
            n.opacity.clamp(0.0, 1.0),
        );
        return;
    }
    let preserve = match n.image_fit {
        SceneImageFit::Fit => "xMidYMid meet",
        SceneImageFit::Crop | SceneImageFit::Fill => "xMidYMid slice",
        SceneImageFit::Stretch => "none",
        SceneImageFit::Tile => unreachable!("tile images return above"),
    };
    let _ = write!(
        out,
        r#"<image id="{}" x="{}" y="{}" width="{}" height="{}" href="{}" preserveAspectRatio="{}" opacity="{}"{clip}/>"#,
        xml_escape(&n.id),
        r.origin.x,
        r.origin.y,
        r.size.x,
        r.size.y,
        xml_escape(src),
        preserve,
        n.opacity.clamp(0.0, 1.0)
    );
}

fn emit_image_pattern(out: &mut String, n: &SceneNode, src: &str, id: &str) {
    let r = normalize_rect(n.bounds);
    // If source bytes are unavailable (for example an uncached remote URL),
    // fall back to one bounds-sized cell instead of inventing a repeat scale.
    let (tile_w, tile_h) = image_metadata::intrinsic_dimensions(n, src)
        .unwrap_or((r.size.x.max(1.0), r.size.y.max(1.0)));
    let start_x = r.origin.x + (r.size.x - tile_w) * 0.5;
    let start_y = r.origin.y + (r.size.y - tile_h) * 0.5;
    let _ = write!(
        out,
        r#"<pattern id="image-pattern-{id}" patternUnits="userSpaceOnUse" x="{start_x}" y="{start_y}" width="{tile_w}" height="{tile_h}" viewBox="0 0 {tile_w} {tile_h}"><image width="{tile_w}" height="{tile_h}" href="{}" preserveAspectRatio="none"/></pattern>"#,
        xml_escape(src),
    );
}

fn text_decoration_attr(n: &SceneNode) -> &'static str {
    match (n.underline, n.strikethrough) {
        (true, true) => r#" text-decoration="underline line-through""#,
        (true, false) => r#" text-decoration="underline""#,
        (false, true) => r#" text-decoration="line-through""#,
        (false, false) => "",
    }
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
    if n.gradient.is_some() {
        let _ = write!(s, r#" fill="url(#gradient-{})""#, svg_id(&n.id));
    } else if let Some(fill) = n.fill {
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
#[path = "svg_export_tests.rs"]
mod tests;
