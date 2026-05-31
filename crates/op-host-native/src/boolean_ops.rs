//! Path boolean ops (Union / Subtract / Intersect / Exclude) for
//! the selection. Backed by skia's built-in `Path::op` so the
//! implementation is short + correct for the layout-resolved shape
//! model the editor uses today. Mirrors the four shortcuts TS
//! exposes via Paper.js (`use-edit-shortcuts.ts` Ctrl+Alt+U/S/I).
//!
//! Lives in `shell-native` (not shell-core) so shell-core stays
//! skia-free. This module is a pure *computation*: given the
//! layout-resolved `LayoutScene` + the editor's selection set it
//! returns the source shape ids + the result polyline. The host
//! commits that result back through an `EditorState` mutator
//! (`replace_paths_with_polyline`) so the canonical tree is never
//! edited directly.

use op_editor_core::BooleanOp;
use op_editor_ui::layout_scene::{regular_polygon_points, LayoutScene, NodeKind, SceneNode};
use op_editor_ui::{Point2D, Rect};
use skia_safe::{Matrix, Path as SkPath, PathBuilder, PathOp, Rect as SkRect};

/// Result of a boolean-op computation — the source shape ids to
/// remove + the new polyline (doc-space `(x, y)` pairs) to commit.
pub struct BooleanResult {
    pub source_ids: Vec<String>,
    pub points: Vec<(f64, f64)>,
}

/// Compute `op` over the selected boolean-compatible shape nodes.
/// `selected` is the editor's selection set (scene-space string ids).
/// Requires every selected node to be a supported shape, and at least
/// two operands; returns `None` when the selection is unsupported or
/// the result polyline is empty. The
/// returned `BooleanResult` is the input the host feeds to
/// `EditorState::replace_paths_with_polyline`.
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
    let result_points = extract_points(&acc);
    if result_points.is_empty() {
        return None;
    }
    Some(BooleanResult {
        source_ids,
        points: result_points
            .iter()
            .map(|p| (p.x as f64, p.y as f64))
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

/// Walk the result Path and yield a flat polyline. Curves (Quad /
/// Conic / Cubic) degrade to their endpoint (TS Paper.js also emits
/// curve segments here; full handle support arrives with the
/// anchor-with-handles model).
fn extract_points(path: &SkPath) -> Vec<Point2D> {
    use skia_safe::PathVerb;
    let mut out = Vec::new();
    for rec in path.iter() {
        let pts = rec.points();
        let pt = match rec.verb() {
            // Move + Line use a single point in pts[0] (legacy
            // skia-bindings PathIter behavior — see path_iter.rs).
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

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_ui::layout_scene::{LayoutScene, SceneNode, ScenePage};
    use op_editor_ui::Rect;

    /// A two-overlapping-square `LayoutScene` — both top-level Path
    /// nodes plus an optional extra node for the non-path test.
    fn scene_with_two_squares() -> LayoutScene {
        let mut a = SceneNode::leaf("n10", NodeKind::Path);
        a.points = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(20.0, 0.0),
            Point2D::new(20.0, 20.0),
            Point2D::new(0.0, 20.0),
        ];
        a.bounds = Rect::xywh(0.0, 0.0, 20.0, 20.0);
        let mut b = SceneNode::leaf("n11", NodeKind::Path);
        b.points = vec![
            Point2D::new(10.0, 10.0),
            Point2D::new(30.0, 10.0),
            Point2D::new(30.0, 30.0),
            Point2D::new(10.0, 30.0),
        ];
        b.bounds = Rect::xywh(10.0, 10.0, 20.0, 20.0);
        LayoutScene {
            pages: vec![ScenePage {
                id: "p".into(),
                name: "P".into(),
                children: vec![a, b],
            }],
            active_page_index: 0,
        }
    }

    fn scene_with_two_rectangles() -> LayoutScene {
        let mut a = SceneNode::leaf("n10", NodeKind::Rect);
        a.bounds = Rect::xywh(0.0, 0.0, 20.0, 20.0);
        let mut b = SceneNode::leaf("n11", NodeKind::Rect);
        b.bounds = Rect::xywh(10.0, 0.0, 20.0, 20.0);
        LayoutScene {
            pages: vec![ScenePage {
                id: "p".into(),
                name: "P".into(),
                children: vec![a, b],
            }],
            active_page_index: 0,
        }
    }

    #[test]
    fn union_of_two_overlapping_squares_yields_a_polyline() {
        let scene = scene_with_two_squares();
        let sel = vec!["n10".to_string(), "n11".to_string()];
        let r = compute_boolean_op(&scene, &sel, BooleanOp::Union).expect("union computes");
        assert_eq!(r.source_ids.len(), 2);
        assert!(!r.points.is_empty(), "union must yield points");
    }

    #[test]
    fn intersect_keeps_overlap_region() {
        let scene = scene_with_two_squares();
        let sel = vec!["n10".to_string(), "n11".to_string()];
        let r = compute_boolean_op(&scene, &sel, BooleanOp::Intersect).expect("intersect computes");
        // Intersection covers the 10..20 × 10..20 square.
        let min_x = r.points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let max_x = r
            .points
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((max_x - min_x - 10.0).abs() < 0.5);
    }

    #[test]
    fn subtract_accepts_two_rectangles() {
        let scene = scene_with_two_rectangles();
        let sel = vec!["n10".to_string(), "n11".to_string()];
        let r = compute_boolean_op(&scene, &sel, BooleanOp::Subtract).expect("subtract computes");
        assert_eq!(r.source_ids, sel);
        assert!(!r.points.is_empty(), "subtract must yield points");
        let min_x = r.points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let max_x = r
            .points
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((min_x - 0.0).abs() < 0.5);
        assert!((max_x - 10.0).abs() < 0.5);
    }

    #[test]
    fn boolean_op_requires_two_path_nodes() {
        let scene = scene_with_two_squares();
        let sel = vec!["n10".to_string()];
        assert!(compute_boolean_op(&scene, &sel, BooleanOp::Union).is_none());
    }

    #[test]
    fn boolean_op_rejects_unsupported_nodes_in_selection() {
        let mut scene = scene_with_two_squares();
        let mut text = SceneNode::leaf("n12", NodeKind::Text);
        text.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        scene.pages[0].children.push(text);
        let sel = vec!["n10".to_string(), "n11".to_string(), "n12".to_string()];
        assert!(compute_boolean_op(&scene, &sel, BooleanOp::Union).is_none());
    }
}
