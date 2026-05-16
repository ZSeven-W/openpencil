//! Canvas hit-test over a [`LayoutScene`].
//!
//! These are the input-path geometry queries that used to live as
//! `&Document`-bound helpers (`Document::node_at_doc_point`,
//! `Document::nodes_intersecting_doc_rect`). The reorg moves the
//! editor hosts' input dispatch off the derived `Document` snapshot
//! and onto the layout-resolved [`LayoutScene`], so the hit-test
//! reads the same resolved geometry the painter walks.
//!
//! Hit semantics are preserved bit-for-bit from `document/walkers.rs`:
//! top-most-first z-order, per-node rotation inverse-transform, the
//! tighter Ellipse / Polygon / Line geometry, the locked-node
//! body-opts-out-children-stay rule, and the hidden-subtree skip.

use crate::layout_scene::{LayoutScene, SceneNode};
use crate::layout_scene::NodeKind;
use crate::{Point2D, Rect};

impl LayoutScene {
    /// Topmost node id whose geometry contains `point` (doc space)
    /// on the active page. Walks children in reverse z-order so the
    /// most-recently-painted node wins. `None` on dead space.
    ///
    /// `zoom` is the live viewport zoom — it scales the Line stroke
    /// hit slack so a thin line stays clickable at any zoom.
    pub fn node_at_doc_point(&self, point: Point2D, zoom: f32) -> Option<String> {
        let zoom = zoom.max(0.0001);
        let page = self.active_page()?;
        for child in page.children.iter().rev() {
            if let Some(hit) = hit_test_walk(child, point, zoom) {
                return Some(hit);
            }
        }
        None
    }

    /// Top-level node ids on the active page whose aggregate bounds
    /// intersect `rect` (doc space). Backs the marquee rect-select.
    /// Descends only into top-level children — same as the click
    /// hit-test, so the result set selects as a unit.
    pub fn nodes_intersecting_doc_rect(&self, rect: Rect) -> Vec<String> {
        let Some(page) = self.active_page() else {
            return Vec::new();
        };
        let nx = rect.origin.x.min(rect.origin.x + rect.size.x);
        let ny = rect.origin.y.min(rect.origin.y + rect.size.y);
        let nw = rect.size.x.abs();
        let nh = rect.size.y.abs();
        let mut out = Vec::new();
        for child in &page.children {
            let b = child.aggregate_bounds();
            if b.size.x <= 0.0 && b.size.y <= 0.0 {
                continue;
            }
            let bx = b.origin.x.min(b.origin.x + b.size.x);
            let by = b.origin.y.min(b.origin.y + b.size.y);
            let bw = b.size.x.abs();
            let bh = b.size.y.abs();
            // AABB intersection test.
            if bx + bw < nx || nx + nw < bx || by + bh < ny || ny + nh < by {
                continue;
            }
            out.push(child.id.clone());
        }
        out
    }
}

/// Recursive hit-test — returns the topmost id whose geometry
/// contains `point`. When a node carries rotation, the test point
/// is inverse-rotated about the node's pivot BEFORE testing
/// children + self, so the hit area matches the rendered geometry.
fn hit_test_walk(node: &SceneNode, point: Point2D, zoom: f32) -> Option<String> {
    // Hidden nodes skip hit-test entirely — the subtree inherits.
    if node.hidden {
        return None;
    }
    let bounds = node.aggregate_bounds();
    let local = if node.rotation.abs() > f32::EPSILON {
        if let Some(pivot) = rotation_pivot(node, bounds) {
            let dx = point.x - pivot.x;
            let dy = point.y - pivot.y;
            let cos_t = (-node.rotation).cos();
            let sin_t = (-node.rotation).sin();
            Point2D::new(
                pivot.x + dx * cos_t - dy * sin_t,
                pivot.y + dx * sin_t + dy * cos_t,
            )
        } else {
            point
        }
    } else {
        point
    };
    for child in node.children.iter().rev() {
        if let Some(hit) = hit_test_walk(child, local, zoom) {
            return Some(hit);
        }
    }
    // Locked nodes can't be selected via canvas hit, but their
    // children still can — this check runs AFTER the child walk so
    // descendants of a locked Frame remain hittable; only the
    // Frame's own body opts out.
    if node.locked {
        return None;
    }
    if point_in_node(node, local, bounds, zoom) {
        return Some(node.id.clone());
    }
    None
}

/// Rotation pivot for hit-test. Most kinds rotate around the
/// aggregate-bounds center; Lines need a kind-specific path because
/// a Line with a negative-size dimension collapses its aggregate to
/// `Rect::ZERO` (the segment midpoint is still well defined from
/// `bounds`). Returns `None` when no valid pivot exists.
fn rotation_pivot(node: &SceneNode, bounds: Rect) -> Option<Point2D> {
    if matches!(node.kind, NodeKind::Line) {
        let raw = node.bounds;
        if raw.size.x.abs() < f32::EPSILON && raw.size.y.abs() < f32::EPSILON {
            return None;
        }
        return Some(Point2D::new(
            raw.origin.x + raw.size.x / 2.0,
            raw.origin.y + raw.size.y / 2.0,
        ));
    }
    if bounds.size.x > 0.0 && bounds.size.y > 0.0 {
        return Some(Point2D::new(
            bounds.origin.x + bounds.size.x / 2.0,
            bounds.origin.y + bounds.size.y / 2.0,
        ));
    }
    None
}

/// Per-NodeKind hit-test. Frames / Groups / Rects / Text / Other
/// use the axis-aligned bounds; Ellipse / Polygon / Line use tighter
/// geometry so the click area matches what the painter draws.
fn point_in_node(node: &SceneNode, local: Point2D, bounds: Rect, zoom: f32) -> bool {
    // Lines get a dedicated path: horizontal / vertical segments
    // have a zero-dimension bounds rect, and negative-size bounds
    // collapse the aggregate to `Rect::ZERO`, so the Line path reads
    // `node.bounds` directly. The distance-to-segment helper is
    // sign-independent.
    if matches!(node.kind, NodeKind::Line) {
        let raw = node.bounds;
        if raw.size.x.abs() < f32::EPSILON && raw.size.y.abs() < f32::EPSILON {
            return false;
        }
        let from = raw.origin;
        let to = Point2D::new(raw.origin.x + raw.size.x, raw.origin.y + raw.size.y);
        let stroke_half = node.stroke.map(|s| s.width / 2.0).unwrap_or(1.0);
        // 4 screen px of slack regardless of zoom — the point is in
        // doc space, so scale by `1/zoom`.
        let screen_slack = 4.0 / zoom.max(0.0001);
        return distance_point_to_segment(local, from, to) <= stroke_half + screen_slack;
    }
    // Non-line kinds need real positive area on both axes.
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return false;
    }
    let in_bounds = local.x >= bounds.origin.x
        && local.x <= bounds.origin.x + bounds.size.x
        && local.y >= bounds.origin.y
        && local.y <= bounds.origin.y + bounds.size.y;
    if !in_bounds {
        return false;
    }
    let cx = bounds.origin.x + bounds.size.x / 2.0;
    let cy = bounds.origin.y + bounds.size.y / 2.0;
    let rx = (bounds.size.x / 2.0).max(0.0001);
    let ry = (bounds.size.y / 2.0).max(0.0001);
    match node.kind {
        NodeKind::Ellipse => {
            let dx = (local.x - cx) / rx;
            let dy = (local.y - cy) / ry;
            dx * dx + dy * dy <= 1.0
        }
        NodeKind::Polygon => {
            // Triangle vertices: top-center, bottom-left,
            // bottom-right — mirrors the polygon paint path.
            let top = Point2D::new(cx, bounds.origin.y);
            let bl = Point2D::new(bounds.origin.x, bounds.origin.y + bounds.size.y);
            let br = Point2D::new(
                bounds.origin.x + bounds.size.x,
                bounds.origin.y + bounds.size.y,
            );
            point_in_triangle(local, top, bl, br)
        }
        // Frame, Group, Rect, Text, Other, Path — bounds-only hit.
        _ => true,
    }
}

/// Sign-of-cross-product point-in-triangle test.
fn point_in_triangle(p: Point2D, a: Point2D, b: Point2D, c: Point2D) -> bool {
    let s1 = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let s2 = (c.x - b.x) * (p.y - b.y) - (c.y - b.y) * (p.x - b.x);
    let s3 = (a.x - c.x) * (p.y - c.y) - (a.y - c.y) * (p.x - c.x);
    let has_neg = s1 < 0.0 || s2 < 0.0 || s3 < 0.0;
    let has_pos = s1 > 0.0 || s2 > 0.0 || s3 > 0.0;
    !(has_neg && has_pos)
}

/// Shortest distance from `p` to the segment `a`–`b`.
fn distance_point_to_segment(p: Point2D, a: Point2D, b: Point2D) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < f32::EPSILON {
        let pdx = p.x - a.x;
        let pdy = p.y - a.y;
        return (pdx * pdx + pdy * pdy).sqrt();
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0);
    let cx = a.x + t * dx;
    let cy = a.y + t * dy;
    let pdx = p.x - cx;
    let pdy = p.y - cy;
    (pdx * pdx + pdy * pdy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_scene::{LayoutScene, SceneNode, ScenePage};

    fn leaf(id: &str, kind: NodeKind, bounds: Rect) -> SceneNode {
        let mut n = SceneNode::leaf(id, kind);
        n.bounds = bounds;
        n
    }

    fn one_page(children: Vec<SceneNode>) -> LayoutScene {
        LayoutScene {
            pages: vec![ScenePage {
                id: "p".into(),
                name: "P".into(),
                children,
            }],
            active_page_index: 0,
        }
    }

    #[test]
    fn node_at_doc_point_picks_topmost_overlapping_node() {
        let scene = one_page(vec![
            leaf("under", NodeKind::Rect, Rect::xywh(0.0, 0.0, 50.0, 50.0)),
            leaf("over", NodeKind::Rect, Rect::xywh(10.0, 10.0, 50.0, 50.0)),
        ]);
        // Overlap region → the later-painted "over" wins.
        assert_eq!(
            scene.node_at_doc_point(Point2D::new(20.0, 20.0), 1.0).as_deref(),
            Some("over")
        );
        // Only "under" covers (5, 5).
        assert_eq!(
            scene.node_at_doc_point(Point2D::new(5.0, 5.0), 1.0).as_deref(),
            Some("under")
        );
        // Dead space.
        assert!(scene.node_at_doc_point(Point2D::new(200.0, 200.0), 1.0).is_none());
    }

    #[test]
    fn hidden_node_is_not_hit() {
        let mut n = leaf("h", NodeKind::Rect, Rect::xywh(0.0, 0.0, 50.0, 50.0));
        n.hidden = true;
        let scene = one_page(vec![n]);
        assert!(scene.node_at_doc_point(Point2D::new(25.0, 25.0), 1.0).is_none());
    }

    #[test]
    fn locked_node_body_opts_out_but_child_stays_hittable() {
        let child = leaf("c", NodeKind::Rect, Rect::xywh(10.0, 10.0, 10.0, 10.0));
        let mut frame = leaf("f", NodeKind::Frame, Rect::xywh(0.0, 0.0, 100.0, 100.0));
        frame.locked = true;
        frame.children = vec![child];
        let scene = one_page(vec![frame]);
        // Click on the child → child id even though parent is locked.
        assert_eq!(
            scene.node_at_doc_point(Point2D::new(15.0, 15.0), 1.0).as_deref(),
            Some("c")
        );
        // Click on the locked frame's bare body → no hit.
        assert!(scene.node_at_doc_point(Point2D::new(80.0, 80.0), 1.0).is_none());
    }

    #[test]
    fn ellipse_uses_tight_oval_geometry() {
        let scene = one_page(vec![leaf(
            "e",
            NodeKind::Ellipse,
            Rect::xywh(0.0, 0.0, 100.0, 100.0),
        )]);
        // Center → inside the oval.
        assert_eq!(
            scene.node_at_doc_point(Point2D::new(50.0, 50.0), 1.0).as_deref(),
            Some("e")
        );
        // Corner of the bounds → outside the oval.
        assert!(scene.node_at_doc_point(Point2D::new(2.0, 2.0), 1.0).is_none());
    }

    #[test]
    fn line_hit_uses_segment_distance() {
        let mut line = leaf("l", NodeKind::Line, Rect::xywh(0.0, 0.0, 100.0, 0.0));
        line.stroke = Some(crate::layout_scene::SceneStroke {
            color: crate::Color::WHITE,
            width: 1.0,
        });
        let scene = one_page(vec![line]);
        // Right on the horizontal segment.
        assert_eq!(
            scene.node_at_doc_point(Point2D::new(50.0, 0.0), 1.0).as_deref(),
            Some("l")
        );
        // Far from the segment → miss.
        assert!(scene.node_at_doc_point(Point2D::new(50.0, 40.0), 1.0).is_none());
    }

    #[test]
    fn rotated_node_hit_tracks_rendered_geometry() {
        let mut n = leaf("r", NodeKind::Rect, Rect::xywh(0.0, 0.0, 100.0, 20.0));
        n.rotation = std::f32::consts::FRAC_PI_2; // 90°
        let scene = one_page(vec![n]);
        // After a 90° rotation about (50, 10), the painted rect spans
        // roughly x 40..60, y -40..60 — a point at (50, 50) lands
        // inside the rotated body though it's outside authored bounds.
        assert_eq!(
            scene.node_at_doc_point(Point2D::new(50.0, 50.0), 1.0).as_deref(),
            Some("r")
        );
    }

    #[test]
    fn nodes_intersecting_doc_rect_returns_overlapping_top_level_ids() {
        let scene = one_page(vec![
            leaf("a", NodeKind::Rect, Rect::xywh(0.0, 0.0, 30.0, 30.0)),
            leaf("b", NodeKind::Rect, Rect::xywh(100.0, 100.0, 30.0, 30.0)),
        ]);
        let hits = scene.nodes_intersecting_doc_rect(Rect::xywh(10.0, 10.0, 80.0, 80.0));
        assert_eq!(hits, vec!["a".to_string()]);
    }

    #[test]
    fn nodes_intersecting_doc_rect_handles_negative_size() {
        let scene = one_page(vec![leaf(
            "a",
            NodeKind::Rect,
            Rect::xywh(0.0, 0.0, 30.0, 30.0),
        )]);
        // Negative-size marquee still intersects.
        let hits = scene.nodes_intersecting_doc_rect(Rect::xywh(40.0, 40.0, -30.0, -30.0));
        assert_eq!(hits, vec!["a".to_string()]);
    }
}
