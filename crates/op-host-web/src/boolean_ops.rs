//! Path boolean ops for the web Rust host. Mirrors the native host's
//! Skia-backed implementation so toolbar clicks behave the same in both
//! shells.

use op_editor_core::BooleanOp;
use op_editor_ui::layout_scene::{regular_polygon_points, LayoutScene, NodeKind, SceneNode};
use op_editor_ui::{Point2D, Rect};
use skia_safe::{ContourMeasureIter, Matrix, Path as SkPath, PathBuilder, PathOp, Rect as SkRect};

pub struct BooleanResult {
    pub source_ids: Vec<String>,
    /// One closed polyline per result contour (doc-space `(x, y)`).
    /// Multiple contours encode holes / disjoint regions; the committer
    /// emits them as a compound even-odd path.
    pub contours: Vec<Vec<(f64, f64)>>,
}

pub fn compute_boolean_op(
    scene: &LayoutScene,
    selected: &[String],
    op: BooleanOp,
) -> Option<BooleanResult> {
    let page = scene.active_page()?;
    let mut source_ids = Vec::with_capacity(selected.len());
    let mut sk_paths = Vec::with_capacity(selected.len());
    for id in selected {
        let node = page.find(id)?;
        let path = build_node_path(node)?;
        source_ids.push(id.clone());
        sk_paths.push(path);
    }
    if sk_paths.len() < 2 {
        return None;
    }
    let pop = match op {
        BooleanOp::Union => PathOp::Union,
        BooleanOp::Subtract => PathOp::Difference,
        BooleanOp::Intersect => PathOp::Intersect,
        BooleanOp::Exclude => PathOp::XOR,
    };
    let mut acc = sk_paths[0].clone();
    for p in &sk_paths[1..] {
        acc = acc.op(p, pop)?;
    }
    let result_contours = flatten_path(&acc);
    if result_contours.is_empty() {
        return None;
    }
    Some(BooleanResult {
        source_ids,
        contours: result_contours
            .iter()
            .map(|c| c.iter().map(|p| (p.x as f64, p.y as f64)).collect())
            .collect(),
    })
}

fn build_node_path(node: &SceneNode) -> Option<SkPath> {
    let path = match node.kind {
        NodeKind::Frame | NodeKind::Rect => build_rect_path(node.bounds),
        NodeKind::Ellipse => build_oval_path(node.bounds),
        NodeKind::Polygon => {
            build_polyline_path(&regular_polygon_points(node.bounds, node.polygon_sides))
        }
        // Prefer the compound `d` so a boolean op chained onto a previous
        // boolean result (which is `d`-only) round-trips correctly.
        NodeKind::Path => node
            .svg_path
            .as_deref()
            .and_then(|d| build_svg_path(d, node.bounds))
            .or_else(|| build_polyline_path(&node.points)),
        NodeKind::Line => {
            if node.points.len() >= 2 {
                build_polyline_path(&node.points)
            } else {
                build_polyline_path(&[
                    node.bounds.origin,
                    Point2D::new(
                        node.bounds.origin.x + node.bounds.size.x,
                        node.bounds.origin.y + node.bounds.size.y,
                    ),
                ])
            }
        }
        NodeKind::Group | NodeKind::Text | NodeKind::Other(_) => None,
    }?;
    Some(apply_node_rotation(path, node))
}

fn build_rect_path(bounds: Rect) -> Option<SkPath> {
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    let mut b = PathBuilder::new();
    b.add_rect(
        SkRect::from_xywh(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.x,
            bounds.size.y,
        ),
        None,
        None,
    );
    Some(b.detach())
}

fn build_oval_path(bounds: Rect) -> Option<SkPath> {
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    let mut b = PathBuilder::new();
    b.add_oval(
        SkRect::from_xywh(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.x,
            bounds.size.y,
        ),
        None,
        None,
    );
    Some(b.detach())
}

/// Parse a node's compound SVG `d` (node-local coords) into a doc-space
/// skia path fitted to `bounds`; ≥2 subpaths → even-odd so chained ops
/// see holes. Mirrors the native host's `build_svg_path`.
fn build_svg_path(d: &str, bounds: Rect) -> Option<SkPath> {
    let parsed = skia_safe::utils::parse_path::from_svg(d)?;
    if parsed.is_empty() {
        return None;
    }
    let mut path = fit_path_to_rect(&parsed, bounds);
    if d.bytes().filter(|c| *c == b'Z' || *c == b'z').count() >= 2 {
        path.set_fill_type(skia_safe::PathFillType::EvenOdd);
    }
    Some(path)
}

/// Scale + translate `path` so its tight bounds map onto `rect` — a copy
/// of the backend's `fit_path_to_rect`.
fn fit_path_to_rect(path: &SkPath, rect: Rect) -> SkPath {
    let bounds = path.compute_tight_bounds();
    if !bounds.is_finite()
        || !rect.size.x.is_finite()
        || !rect.size.y.is_finite()
        || rect.size.x <= 0.0
        || rect.size.y <= 0.0
    {
        let mut matrix = Matrix::new_identity();
        matrix.set_translate((rect.origin.x, rect.origin.y));
        return path.with_transform(&matrix);
    }
    let sx = if bounds.width().abs() > 0.01 {
        rect.size.x / bounds.width()
    } else {
        1.0
    };
    let sy = if bounds.height().abs() > 0.01 {
        rect.size.y / bounds.height()
    } else {
        1.0
    };
    let tx = rect.origin.x - bounds.left() * sx;
    let ty = rect.origin.y - bounds.top() * sy;
    let mut matrix = Matrix::new_identity();
    matrix.set_scale_translate((sx, sy), (tx, ty));
    path.with_transform(&matrix)
}

fn build_polyline_path(points: &[Point2D]) -> Option<SkPath> {
    if points.len() < 2 {
        return None;
    }
    let mut b = PathBuilder::new();
    if let Some(first) = points.first() {
        b.move_to((first.x, first.y));
        for pt in points.iter().skip(1) {
            b.line_to((pt.x, pt.y));
        }
        b.close();
    }
    Some(b.detach())
}

fn apply_node_rotation(path: SkPath, node: &SceneNode) -> SkPath {
    if node.rotation.abs() <= f32::EPSILON {
        return path;
    }
    let b = node.aggregate_bounds();
    let pivot = skia_safe::Point::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
    let mut matrix = Matrix::new_identity();
    matrix.set_rotate(node.rotation.to_degrees(), Some(pivot));
    path.with_transform(&matrix)
}

/// Flatten the boolean result into one closed polyline per contour.
/// `ContourMeasureIter` splits subpaths (so separate contours don't fuse
/// into a self-crossing loop) and `pos_tan` arc-length sampling turns
/// conic arcs into smooth segments instead of collapsing each curve to a
/// single endpoint. Mirrors the native host's `flatten_path`.
fn flatten_path(path: &SkPath) -> Vec<Vec<Point2D>> {
    const STEP: f32 = 1.0; // doc-px arc length between samples
    const MAX_PER_CONTOUR: usize = 8192; // runaway-path guard
    let mut contours = Vec::new();
    for cm in ContourMeasureIter::new(path, false, None) {
        let len = cm.length();
        if len <= 0.0 {
            continue;
        }
        let n = ((len / STEP).ceil() as usize).clamp(1, MAX_PER_CONTOUR);
        let count = if cm.is_closed() { n } else { n + 1 };
        let mut poly: Vec<Point2D> = Vec::with_capacity(count);
        for i in 0..count {
            let d = len * (i as f32) / (n as f32);
            if let Some((p, _tan)) = cm.pos_tan(d) {
                push_simplified(&mut poly, Point2D::new(p.x, p.y));
            }
        }
        if poly.len() >= 2 {
            contours.push(poly);
        }
    }
    contours
}

/// Drop a sample that is within `EPS_DIST` of the segment spanned by the
/// previous two points (collapses collinear runs on straight edges).
fn push_simplified(poly: &mut Vec<Point2D>, p: Point2D) {
    const EPS_DIST: f32 = 0.08; // px: max perpendicular deviation
    let n = poly.len();
    if n >= 2 {
        let a = poly[n - 2];
        let b = poly[n - 1];
        let dx = p.x - a.x;
        let dy = p.y - a.y;
        let seg = (dx * dx + dy * dy).sqrt();
        if seg > 1e-6 {
            let dist = ((b.x - a.x) * dy - (b.y - a.y) * dx).abs() / seg;
            if dist < EPS_DIST {
                poly[n - 1] = p;
                return;
            }
        }
    }
    poly.push(p);
}
