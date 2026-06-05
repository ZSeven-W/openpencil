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
use skia_safe::{ContourMeasureIter, Matrix, Path as SkPath, PathBuilder, PathOp, Rect as SkRect};

/// Result of a boolean-op computation — the source shape ids to remove +
/// the result contours. One closed polyline per contour (doc-space
/// `(x, y)` pairs); multiple contours encode holes / disjoint regions,
/// which the committer turns into a compound even-odd path.
pub struct BooleanResult {
    pub source_ids: Vec<String>,
    pub contours: Vec<Vec<(f64, f64)>>,
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
        // A Path may carry geometry either as a compound SVG `d`
        // (boolean results, SVG imports) or as flat anchor points (pen
        // tool). Prefer the `d` so a boolean op chained onto a previous
        // boolean result — which is `d`-only — round-trips correctly.
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

/// Parse a node's compound SVG `d` (node-local coords) into a doc-space
/// skia path, fitted to `bounds` exactly as the renderer paints it.
/// Compound `d`s with ≥2 subpaths (Subtract / Exclude holes) are tagged
/// even-odd so a chained `Path::op` sees the hole.
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
/// of the backend's `fit_path_to_rect` (the boolean layer can't reach the
/// private backend fn), so boolean input geometry matches what is painted.
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
///
/// `ContourMeasureIter` walks the result path one subpath at a time, so
/// separate contours never fuse into a single self-crossing loop (the
/// old `extract_points` concatenated them — hence the XOR "bowtie").
/// Arc-length sampling via `pos_tan` turns the oval's conic arcs into
/// smooth line segments rather than collapsing each curve to a single
/// endpoint (the old "ellipse becomes a triangle" bug). Nearly-collinear
/// samples are dropped so straight edges stay crisp.
fn flatten_path(path: &SkPath) -> Vec<Vec<Point2D>> {
    const STEP: f32 = 1.0; // doc-px arc length between samples
    const MAX_PER_CONTOUR: usize = 8192; // runaway-path guard
    let mut contours = Vec::new();
    // force_closed=false: respect the contour's own open/closed flag.
    for cm in ContourMeasureIter::new(path, false, None) {
        let len = cm.length();
        if len <= 0.0 {
            continue;
        }
        let n = ((len / STEP).ceil() as usize).clamp(1, MAX_PER_CONTOUR);
        // A closed contour stops one sample short of `len` — that sample
        // duplicates the start, and the emitted path closes the seam.
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

/// Append `p`, but when it is within `EPS_DIST` of the segment spanned by
/// the last two points, replace the middle point instead — collapsing
/// collinear runs (the straight edges of rects/polygons) without touching
/// curved spans, where consecutive samples deviate measurably.
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
        assert!(!r.contours.is_empty(), "union must yield a contour");
        assert!(r.contours[0].len() >= 4, "union outline needs ≥4 points");
    }

    #[test]
    fn intersect_keeps_overlap_region() {
        let scene = scene_with_two_squares();
        let sel = vec!["n10".to_string(), "n11".to_string()];
        let r = compute_boolean_op(&scene, &sel, BooleanOp::Intersect).expect("intersect computes");
        // Intersection covers the 10..20 × 10..20 square.
        let xs = || r.contours.iter().flatten().map(|p| p.0);
        let min_x = xs().fold(f64::INFINITY, f64::min);
        let max_x = xs().fold(f64::NEG_INFINITY, f64::max);
        assert!((max_x - min_x - 10.0).abs() < 0.5);
    }

    #[test]
    fn subtract_accepts_two_rectangles() {
        let scene = scene_with_two_rectangles();
        let sel = vec!["n10".to_string(), "n11".to_string()];
        let r = compute_boolean_op(&scene, &sel, BooleanOp::Subtract).expect("subtract computes");
        assert_eq!(r.source_ids, sel);
        assert!(!r.contours.is_empty(), "subtract must yield a contour");
        let xs = || r.contours.iter().flatten().map(|p| p.0);
        let min_x = xs().fold(f64::INFINITY, f64::min);
        let max_x = xs().fold(f64::NEG_INFINITY, f64::max);
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

    /// A boolean result is committed as a `d`-only Path (no anchors → the
    /// SceneNode has empty `points`). Chaining another op onto it must
    /// still work by parsing `svg_path` — this would return `None` before
    /// the svg_path branch in `build_node_path`.
    #[test]
    fn boolean_op_accepts_svg_path_only_nodes() {
        let mut a = SceneNode::leaf("n10", NodeKind::Path);
        a.svg_path = Some("M 0 0 L 20 0 L 20 20 L 0 20 Z".into());
        a.bounds = Rect::xywh(0.0, 0.0, 20.0, 20.0);
        assert!(a.points.is_empty(), "svg-path node has no flat points");
        let mut b = SceneNode::leaf("n11", NodeKind::Path);
        b.svg_path = Some("M 0 0 L 20 0 L 20 20 L 0 20 Z".into());
        b.bounds = Rect::xywh(10.0, 10.0, 20.0, 20.0);
        let scene = LayoutScene {
            pages: vec![ScenePage {
                id: "p".into(),
                name: "P".into(),
                children: vec![a, b],
            }],
            active_page_index: 0,
        };
        let sel = vec!["n10".to_string(), "n11".to_string()];
        let r = compute_boolean_op(&scene, &sel, BooleanOp::Union)
            .expect("union of two svg-path-only nodes computes");
        assert!(
            !r.contours.is_empty(),
            "chained boolean op must yield a contour"
        );
    }
}
