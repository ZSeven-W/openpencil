//! Path boolean ops (Union / Subtract / Intersect / Exclude) for
//! the selection. Backed by skia's built-in `Path::op` so the
//! implementation is short + correct for the polyline path-points
//! model the editor uses today. Mirrors the four shortcuts TS
//! exposes via Paper.js (`use-edit-shortcuts.ts` Ctrl+Alt+U/S/I).
//!
//! Lives in `shell-native` (not shell-core) so shell-core stays
//! skia-free. This module is a pure *computation*: given the
//! layout-resolved `LayoutScene` + the editor's selection set it
//! returns the source path ids + the result polyline. The host
//! commits that result back through an `EditorState` mutator
//! (`replace_paths_with_polyline`) so the canonical tree is never
//! edited directly.

use openpencil_shell_core::document::{BooleanOp, NodeKind};
use openpencil_shell_core::layout_scene::LayoutScene;
use openpencil_shell_core::Point2D;
use skia_safe::{Path as SkPath, PathBuilder, PathOp};

/// Result of a boolean-op computation — the source path ids to
/// remove + the new polyline (doc-space `(x, y)` pairs) to commit.
pub struct BooleanResult {
    pub source_ids: Vec<String>,
    pub points: Vec<(f64, f64)>,
}

/// Compute `op` over the selected Path nodes. `selected` is the
/// editor's selection set (scene-space string ids). Requires 2+
/// Path nodes among the selection; returns `None` when fewer than
/// two paths are selected or the result polyline is empty. The
/// returned `BooleanResult` is the input the host feeds to
/// `EditorState::replace_paths_with_polyline`.
pub fn compute_boolean_op(
    scene: &LayoutScene,
    selected: &[String],
    op: BooleanOp,
) -> Option<BooleanResult> {
    let page = scene.active_page()?;
    let path_ids: Vec<String> = selected
        .iter()
        .filter(|id| {
            page.find(id)
                .map(|n| matches!(n.kind, NodeKind::Path))
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    if path_ids.len() < 2 {
        return None;
    }
    let sk_paths: Vec<SkPath> = path_ids
        .iter()
        .filter_map(|id| page.find(id).map(|n| build_skia_path(&n.points)))
        .collect();
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
        source_ids: path_ids,
        points: result_points
            .iter()
            .map(|p| (p.x as f64, p.y as f64))
            .collect(),
    })
}

fn build_skia_path(points: &[Point2D]) -> SkPath {
    let mut b = PathBuilder::new();
    if let Some(first) = points.first() {
        b.move_to((first.x, first.y));
        for pt in points.iter().skip(1) {
            b.line_to((pt.x, pt.y));
        }
        b.close();
    }
    b.detach()
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
    use openpencil_shell_core::layout_scene::{LayoutScene, SceneNode, ScenePage};
    use openpencil_shell_core::Rect;

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

    #[test]
    fn union_of_two_overlapping_squares_yields_a_polyline() {
        let scene = scene_with_two_squares();
        let sel = vec!["n10".to_string(), "n11".to_string()];
        let r = compute_boolean_op(&scene, &sel, BooleanOp::Union)
            .expect("union computes");
        assert_eq!(r.source_ids.len(), 2);
        assert!(!r.points.is_empty(), "union must yield points");
    }

    #[test]
    fn intersect_keeps_overlap_region() {
        let scene = scene_with_two_squares();
        let sel = vec!["n10".to_string(), "n11".to_string()];
        let r = compute_boolean_op(&scene, &sel, BooleanOp::Intersect)
            .expect("intersect computes");
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
    fn boolean_op_requires_two_path_nodes() {
        let scene = scene_with_two_squares();
        let sel = vec!["n10".to_string()];
        assert!(compute_boolean_op(&scene, &sel, BooleanOp::Union).is_none());
    }

    #[test]
    fn boolean_op_skips_non_path_nodes_in_selection() {
        let mut scene = scene_with_two_squares();
        let mut r = SceneNode::leaf("n12", NodeKind::Rect);
        r.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        scene.pages[0].children.push(r);
        let sel = vec!["n10".to_string(), "n11".to_string(), "n12".to_string()];
        // Still has 2 Path nodes — should succeed; Rect is ignored.
        let res = compute_boolean_op(&scene, &sel, BooleanOp::Union)
            .expect("union computes");
        assert_eq!(res.source_ids.len(), 2);
    }
}
