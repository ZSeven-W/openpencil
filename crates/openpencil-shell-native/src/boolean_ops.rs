//! Path boolean ops (Union / Subtract / Intersect / Exclude) for
//! the selection. Backed by skia's built-in `Path::op` so the
//! implementation is short + correct for the polyline `Node.points`
//! model the editor uses today. Mirrors the four shortcuts TS
//! exposes via Paper.js (`use-edit-shortcuts.ts` Ctrl+Alt+U/S/I).
//!
//! Lives in `shell-native` (not shell-core) so shell-core stays
//! skia-free. This module is a pure *computation*: given the derived
//! paint `Document` it returns the source path ids + the result
//! polyline. The host commits that result back through an
//! `EditorState` mutator (`replace_paths_with_polyline`) so the
//! canonical tree is never edited directly.

use openpencil_shell_core::document::{BooleanOp, Document, NodeId, NodeKind};
use openpencil_shell_core::Point2D;
use skia_safe::{Path as SkPath, PathBuilder, PathOp};

/// Result of a boolean-op computation — the source path ids to
/// remove + the new polyline (doc-space `(x, y)` pairs) to commit.
pub struct BooleanResult {
    pub source_ids: Vec<NodeId>,
    pub points: Vec<(f64, f64)>,
}

/// Compute `op` over the active selection's Path nodes. Requires 2+
/// Path nodes in the active selection set; returns `None` when fewer
/// than two paths are selected or the result polyline is empty. The
/// returned `BooleanResult` is the input the host feeds to
/// `EditorState::replace_paths_with_polyline`.
pub fn compute_boolean_op(doc: &Document, op: BooleanOp) -> Option<BooleanResult> {
    let path_ids: Vec<NodeId> = doc
        .selected_set
        .iter()
        .cloned()
        .filter(|id| {
            doc.active_page()
                .and_then(|p| p.find(id))
                .map(|n| matches!(n.kind, NodeKind::Path))
                .unwrap_or(false)
        })
        .collect();
    if path_ids.len() < 2 {
        return None;
    }
    let sk_paths: Vec<SkPath> = path_ids
        .iter()
        .filter_map(|id| {
            doc.active_page()
                .and_then(|p| p.find(id))
                .map(|n| build_skia_path(&n.points))
        })
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
    use openpencil_shell_core::document::{Node, NodeId, NodeKind};
    use openpencil_shell_core::Rect;

    fn doc_with_two_squares() -> Document {
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        let mut a = Node::leaf("n10", NodeKind::Path, "a");
        a.points = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(20.0, 0.0),
            Point2D::new(20.0, 20.0),
            Point2D::new(0.0, 20.0),
        ];
        a.bounds = Rect::xywh(0.0, 0.0, 20.0, 20.0);
        let mut b = Node::leaf("n11", NodeKind::Path, "b");
        b.points = vec![
            Point2D::new(10.0, 10.0),
            Point2D::new(30.0, 10.0),
            Point2D::new(30.0, 30.0),
            Point2D::new(10.0, 30.0),
        ];
        b.bounds = Rect::xywh(10.0, 10.0, 20.0, 20.0);
        page.children.push(a);
        page.children.push(b);
        doc.selected_set = vec![NodeId::new("n10"), NodeId::new("n11")];
        doc.selected = NodeId::new("n11");
        doc
    }

    #[test]
    fn union_of_two_overlapping_squares_yields_a_polyline() {
        let doc = doc_with_two_squares();
        let r = compute_boolean_op(&doc, BooleanOp::Union).expect("union computes");
        assert_eq!(r.source_ids.len(), 2);
        assert!(!r.points.is_empty(), "union must yield points");
    }

    #[test]
    fn intersect_keeps_overlap_region() {
        let doc = doc_with_two_squares();
        let r =
            compute_boolean_op(&doc, BooleanOp::Intersect).expect("intersect computes");
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
        let mut doc = doc_with_two_squares();
        doc.selected_set = vec![NodeId::new("n10")];
        doc.selected = NodeId::new("n10");
        assert!(compute_boolean_op(&doc, BooleanOp::Union).is_none());
    }

    #[test]
    fn boolean_op_skips_non_path_nodes_in_selection() {
        let mut doc = doc_with_two_squares();
        let page = doc.pages.get_mut(0).unwrap();
        let mut r = Node::leaf("n12", NodeKind::Rect, "r");
        r.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        page.children.push(r);
        doc.selected_set.push(NodeId::new("n12"));
        // Still has 2 Path nodes — should succeed; Rect is ignored.
        let res =
            compute_boolean_op(&doc, BooleanOp::Union).expect("union computes");
        assert_eq!(res.source_ids.len(), 2);
    }
}
