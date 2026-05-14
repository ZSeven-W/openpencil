//! Path boolean ops (Union / Subtract / Intersect / Exclude) for
//! the selection. Backed by skia's built-in `Path::op` so the
//! implementation is short + correct for the polyline `Node.points`
//! model the editor uses today. Mirrors the four shortcuts TS
//! exposes via Paper.js (`use-edit-shortcuts.ts` Ctrl+Alt+U/S/I).
//!
//! Lives in `shell-native` (not shell-core) so shell-core stays
//! skia-free; the widget host dispatches the keyboard shortcut and
//! calls `apply_boolean_op`.

use openpencil_shell_core::document::{BooleanOp, Document, Node, NodeId, NodeKind};
use openpencil_shell_core::Point2D;
use skia_safe::{Path as SkPath, PathBuilder, PathOp};

/// Apply `op` to the selection. Requires 2+ Path nodes in the
/// active selection set; otherwise returns false (no-op, no
/// history). On success: removes the source paths, inserts one new
/// Path node holding the result polyline, selects it, pushes a
/// single history entry.
pub fn apply_boolean_op(doc: &mut Document, op: BooleanOp, next_id: &mut u64) -> bool {
    let active = doc.active_page_index;
    let path_ids: Vec<NodeId> = doc
        .selected_set
        .iter()
        .copied()
        .filter(|id| {
            doc.active_page()
                .and_then(|p| p.find(*id))
                .map(|n| matches!(n.kind, NodeKind::Path))
                .unwrap_or(false)
        })
        .collect();
    if path_ids.len() < 2 {
        return false;
    }
    let sk_paths: Vec<SkPath> = path_ids
        .iter()
        .filter_map(|id| {
            doc.active_page()
                .and_then(|p| p.find(*id))
                .map(|n| build_skia_path(&n.points))
        })
        .collect();
    if sk_paths.len() < 2 {
        return false;
    }
    let pop = match op {
        BooleanOp::Union => PathOp::Union,
        BooleanOp::Subtract => PathOp::Difference,
        BooleanOp::Intersect => PathOp::Intersect,
        BooleanOp::Exclude => PathOp::XOR,
    };
    let mut acc = sk_paths[0].clone();
    for p in &sk_paths[1..] {
        acc = match acc.op(p, pop) {
            Some(r) => r,
            None => return false,
        };
    }
    let result_points = extract_points(&acc);
    if result_points.is_empty() {
        return false;
    }
    let pre = doc.snapshot_for_history();
    // Mint a fresh node id past the high-water mark.
    let safe = match doc.max_node_id().checked_add(1) {
        Some(v) => v,
        None => return false,
    };
    let raw = (*next_id).max(safe);
    *next_id = match raw.checked_add(1) {
        Some(v) => v,
        None => return false,
    };
    let id = NodeId::new(raw);
    let mut new_node = Node::leaf(raw, NodeKind::Path, "Boolean");
    // Seed stroke from the first source so the result has visible ink
    // (the editor's default Path has no stroke until one is assigned).
    if let Some(src) = doc.active_page().and_then(|p| p.find(path_ids[0])) {
        new_node.fill = src.fill;
        new_node.stroke = src.stroke;
    }
    new_node.points = result_points;
    let (origin, size) = bbox_of(&new_node.points);
    new_node.bounds = openpencil_shell_core::Rect { origin, size };
    let id_set: std::collections::HashSet<NodeId> = path_ids.iter().copied().collect();
    if let Some(page) = doc.pages.get_mut(active) {
        // Recursive removal — sources may be nested inside groups /
        // frames (codex CONCERN: `retain` only on top-level left
        // originals behind + duplicated the result). Walk every
        // children Vec depth-first and drop any node whose id is in
        // the source set.
        remove_nodes_recursively(&mut page.children, &id_set);
        page.children.push(new_node);
    }
    doc.selected_set.clear();
    doc.selected_set.push(id);
    doc.selected = id;
    doc.history_push_past(pre);
    true
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

fn remove_nodes_recursively(
    children: &mut Vec<openpencil_shell_core::document::Node>,
    targets: &std::collections::HashSet<NodeId>,
) {
    children.retain(|n| !targets.contains(&n.id));
    for child in children.iter_mut() {
        remove_nodes_recursively(&mut child.children, targets);
    }
}

fn bbox_of(points: &[Point2D]) -> (Point2D, Point2D) {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in points {
        if p.x < min_x { min_x = p.x; }
        if p.y < min_y { min_y = p.y; }
        if p.x > max_x { max_x = p.x; }
        if p.y > max_y { max_y = p.y; }
    }
    if min_x.is_finite() {
        (
            Point2D::new(min_x, min_y),
            Point2D::new(max_x - min_x, max_y - min_y),
        )
    } else {
        (Point2D::new(0.0, 0.0), Point2D::new(0.0, 0.0))
    }
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
        let mut a = Node::leaf(10, NodeKind::Path, "a");
        a.points = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(20.0, 0.0),
            Point2D::new(20.0, 20.0),
            Point2D::new(0.0, 20.0),
        ];
        a.bounds = Rect::xywh(0.0, 0.0, 20.0, 20.0);
        let mut b = Node::leaf(11, NodeKind::Path, "b");
        b.points = vec![
            Point2D::new(10.0, 10.0),
            Point2D::new(30.0, 10.0),
            Point2D::new(30.0, 30.0),
            Point2D::new(10.0, 30.0),
        ];
        b.bounds = Rect::xywh(10.0, 10.0, 20.0, 20.0);
        page.children.push(a);
        page.children.push(b);
        doc.selected_set = vec![NodeId::new(10), NodeId::new(11)];
        doc.selected = NodeId::new(11);
        doc
    }

    #[test]
    fn union_of_two_overlapping_squares_collapses_to_one_path() {
        let mut doc = doc_with_two_squares();
        let mut next = 100u64;
        assert!(apply_boolean_op(&mut doc, BooleanOp::Union, &mut next));
        // Original two paths gone; one new path remains.
        let page = doc.active_page().unwrap();
        assert_eq!(page.children.len(), 1);
        let result = &page.children[0];
        assert!(matches!(result.kind, NodeKind::Path));
        assert!(!result.points.is_empty(), "union must yield points");
        // History pushed exactly once.
        assert_eq!(doc.history.past.len(), 1);
    }

    #[test]
    fn intersect_keeps_overlap_region() {
        let mut doc = doc_with_two_squares();
        let mut next = 100u64;
        assert!(apply_boolean_op(&mut doc, BooleanOp::Intersect, &mut next));
        let page = doc.active_page().unwrap();
        assert_eq!(page.children.len(), 1);
        let r = &page.children[0];
        // Intersection covers the 10..20 × 10..20 square — bounds match.
        assert!((r.bounds.size.x - 10.0).abs() < 0.5);
        assert!((r.bounds.size.y - 10.0).abs() < 0.5);
    }

    #[test]
    fn boolean_op_requires_two_path_nodes() {
        let mut doc = doc_with_two_squares();
        // Drop selection to one.
        doc.selected_set = vec![NodeId::new(10)];
        doc.selected = NodeId::new(10);
        let mut next = 100u64;
        assert!(!apply_boolean_op(&mut doc, BooleanOp::Union, &mut next));
        // Page still has both paths; nothing committed.
        assert_eq!(doc.active_page().unwrap().children.len(), 2);
        assert_eq!(doc.history.past.len(), 0);
    }

    #[test]
    fn boolean_op_removes_nested_paths_not_just_top_level() {
        // Codex BLOCK: when source paths live inside a group/frame,
        // the previous top-level-only `retain` left them in the
        // group + appended the result at top level — duplication.
        // Fix removes from anywhere in the children tree.
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        let mut a = Node::leaf(10, NodeKind::Path, "a");
        a.points = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(20.0, 0.0),
            Point2D::new(20.0, 20.0),
            Point2D::new(0.0, 20.0),
        ];
        a.bounds = Rect::xywh(0.0, 0.0, 20.0, 20.0);
        let mut b = Node::leaf(11, NodeKind::Path, "b");
        b.points = vec![
            Point2D::new(10.0, 10.0),
            Point2D::new(30.0, 10.0),
            Point2D::new(30.0, 30.0),
            Point2D::new(10.0, 30.0),
        ];
        b.bounds = Rect::xywh(10.0, 10.0, 20.0, 20.0);
        // Wrap both paths inside a Group.
        let group = Node::with_children(99, NodeKind::Group, "g", vec![a, b]);
        page.children.push(group);
        doc.selected_set = vec![NodeId::new(10), NodeId::new(11)];
        doc.selected = NodeId::new(11);
        let mut next = 100u64;
        assert!(apply_boolean_op(&mut doc, BooleanOp::Union, &mut next));
        // Group still exists but is now empty of paths; result Path
        // lives at top level — total page.children = 2 (empty group + result).
        let page = doc.active_page().unwrap();
        assert_eq!(page.children.len(), 2);
        let group_after = &page.children[0];
        assert!(matches!(group_after.kind, NodeKind::Group));
        assert!(
            group_after.children.is_empty(),
            "source paths must be removed from their group, not duplicated"
        );
    }

    #[test]
    fn boolean_op_skips_non_path_nodes_in_selection() {
        let mut doc = doc_with_two_squares();
        // Add a Rect (non-Path) to the selection mix.
        let page = doc.pages.get_mut(0).unwrap();
        let mut r = Node::leaf(12, NodeKind::Rect, "r");
        r.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        page.children.push(r);
        doc.selected_set.push(NodeId::new(12));
        let mut next = 100u64;
        // Still has 2 Path nodes — should succeed; Rect is ignored.
        assert!(apply_boolean_op(&mut doc, BooleanOp::Union, &mut next));
        // The Rect survives (only Path sources are removed); result
        // Path is added; total = 1 result + 1 surviving rect = 2.
        assert_eq!(doc.active_page().unwrap().children.len(), 2);
    }
}
