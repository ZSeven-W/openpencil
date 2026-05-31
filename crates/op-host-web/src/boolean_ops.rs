//! Path boolean ops for the web Rust host. Mirrors the native host's
//! Skia-backed implementation so toolbar clicks behave the same in both
//! shells.

use op_editor_core::BooleanOp;
use op_editor_ui::layout_scene::{regular_polygon_points, LayoutScene, NodeKind, SceneNode};
use op_editor_ui::{Point2D, Rect};
use skia_safe::{Matrix, Path as SkPath, PathBuilder, PathOp, Rect as SkRect};

pub struct BooleanResult {
    pub source_ids: Vec<String>,
    pub points: Vec<(f64, f64)>,
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
    let points = extract_points(&acc);
    if points.is_empty() {
        return None;
    }
    Some(BooleanResult {
        source_ids,
        points: points.iter().map(|p| (p.x as f64, p.y as f64)).collect(),
    })
}

fn build_node_path(node: &SceneNode) -> Option<SkPath> {
    let path = match node.kind {
        NodeKind::Frame | NodeKind::Rect => build_rect_path(node.bounds),
        NodeKind::Ellipse => build_oval_path(node.bounds),
        NodeKind::Polygon => {
            build_polyline_path(&regular_polygon_points(node.bounds, node.polygon_sides))
        }
        NodeKind::Path => build_polyline_path(&node.points),
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

fn extract_points(path: &SkPath) -> Vec<Point2D> {
    use skia_safe::PathVerb;
    let mut out = Vec::new();
    for rec in path.iter() {
        let pts = rec.points();
        let pt = match rec.verb() {
            PathVerb::Move | PathVerb::Line => pts.first().copied(),
            PathVerb::Quad | PathVerb::Conic => pts.get(1).copied(),
            PathVerb::Cubic => pts.get(2).copied(),
            PathVerb::Close => None,
        };
        if let Some(p) = pt {
            out.push(Point2D::new(p.x, p.y));
        }
    }
    out
}
