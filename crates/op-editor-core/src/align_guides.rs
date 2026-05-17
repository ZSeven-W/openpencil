//! Smart-guide (alignment) geometry for node dragging.
//!
//! While a node is dragged, an edge or centre of the moving node that
//! lines up with an edge/centre of another node should surface a
//! guide line and snap the drag onto it — the behaviour TS exposes
//! via `apps/web` smart guides.
//!
//! This module is pure geometry: it takes the moving node's AABB plus
//! the other nodes' AABBs and returns the guide segments to paint and
//! the snap offset to apply. The host wires it into the drag handler;
//! the canvas painter draws the returned guides. Keeping it free of
//! `EditorState` makes the alignment logic fully unit-testable.

/// A doc-space axis-aligned bounding box: `(x, y, w, h)`.
pub type Aabb = (f64, f64, f64, f64);

/// An axis-aligned guide line to paint during a drag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignmentGuide {
    /// `true` → a vertical line at a constant x; `false` → horizontal
    /// at a constant y.
    pub vertical: bool,
    /// The line's fixed coordinate (doc-space x for vertical guides,
    /// y for horizontal).
    pub pos: f64,
    /// Segment extent along the other axis — start..end so the line
    /// is drawn only across the aligned nodes, not the whole canvas.
    pub start: f64,
    pub end: f64,
}

/// Outcome of an alignment pass: the guides to draw plus the snap
/// offset to add to the moving node so it locks onto the alignment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AlignmentResult {
    pub guides: Vec<AlignmentGuide>,
    pub snap_dx: f64,
    pub snap_dy: f64,
}

/// The three alignment lines of a rect on one axis: near edge,
/// centre, far edge. For x that is `(left, center_x, right)`.
fn axis_lines(origin: f64, extent: f64) -> [f64; 3] {
    [origin, origin + extent / 2.0, origin + extent]
}

/// Best single-axis match: the moving-line / other-line pair with the
/// smallest gap within `threshold`. Returns `(snap, other_line)` —
/// `snap` is added to the moving rect, `other_line` is the guide's
/// fixed coordinate.
fn best_axis_match(
    moving_lines: [f64; 3],
    other_lines: &[[f64; 3]],
    threshold: f64,
) -> Option<(f64, f64)> {
    let mut best: Option<(f64, f64)> = None; // snap, guide line
    let mut best_gap = f64::INFINITY;
    for others in other_lines {
        for &m in &moving_lines {
            for &o in others {
                let gap = (o - m).abs();
                // `gap < best_gap` (strict) keeps the FIRST candidate
                // on a tie — moving's near edge is checked before its
                // centre / far edge, so an exact-tie prefers the more
                // intuitive edge-to-edge alignment.
                if gap <= threshold && gap < best_gap {
                    best_gap = gap;
                    best = Some((o - m, o));
                }
            }
        }
    }
    best
}

/// Compute alignment guides between `moving` and each rect in
/// `others`. An edge or centre of `moving` within `threshold`
/// doc-px of an edge/centre of any other rect yields a guide line +
/// a snap offset that locks `moving` exactly onto it. The closest
/// candidate per axis wins; the two axes are resolved independently.
pub fn compute_alignment_guides(moving: Aabb, others: &[Aabb], threshold: f64) -> AlignmentResult {
    let (mx, my, mw, mh) = moving;
    let mut result = AlignmentResult::default();
    if others.is_empty() || threshold <= 0.0 {
        return result;
    }
    let moving_x = axis_lines(mx, mw);
    let moving_y = axis_lines(my, mh);
    let others_x: Vec<[f64; 3]> = others
        .iter()
        .map(|&(x, _, w, _)| axis_lines(x, w))
        .collect();
    let others_y: Vec<[f64; 3]> = others
        .iter()
        .map(|&(_, y, _, h)| axis_lines(y, h))
        .collect();

    // Vertical guide (x alignment).
    if let Some((snap_dx, line_x)) = best_axis_match(moving_x, &others_x, threshold) {
        result.snap_dx = snap_dx;
        // Span the guide across the moving node + every other node
        // whose y-range it crosses, so the line reads as "these are
        // aligned" rather than a full-canvas ruler.
        let mut top = my;
        let mut bottom = my + mh;
        for &(_, y, _, h) in others {
            top = top.min(y);
            bottom = bottom.max(y + h);
        }
        result.guides.push(AlignmentGuide {
            vertical: true,
            pos: line_x,
            start: top,
            end: bottom,
        });
    }
    // Horizontal guide (y alignment).
    if let Some((snap_dy, line_y)) = best_axis_match(moving_y, &others_y, threshold) {
        result.snap_dy = snap_dy;
        let mut left = mx;
        let mut right = mx + mw;
        for &(x, _, w, _) in others {
            left = left.min(x);
            right = right.max(x + w);
        }
        result.guides.push(AlignmentGuide {
            vertical: false,
            pos: line_y,
            start: left,
            end: right,
        });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_others_yields_no_guides() {
        let r = compute_alignment_guides((0.0, 0.0, 10.0, 10.0), &[], 5.0);
        assert!(r.guides.is_empty());
        assert_eq!((r.snap_dx, r.snap_dy), (0.0, 0.0));
    }

    #[test]
    fn left_edges_within_threshold_snap_and_guide() {
        // Moving left edge at x=3, other left edge at x=0 → gap 3 ≤ 5.
        let r = compute_alignment_guides((3.0, 100.0, 20.0, 20.0), &[(0.0, 0.0, 40.0, 40.0)], 5.0);
        // Snap moves the node left by 3 so the left edges coincide.
        assert!((r.snap_dx - (-3.0)).abs() < 1e-9);
        let v = r
            .guides
            .iter()
            .find(|g| g.vertical)
            .expect("vertical guide");
        assert!((v.pos - 0.0).abs() < 1e-9);
    }

    #[test]
    fn centers_align_on_both_axes() {
        // Moving 20×20 centred at (10,10); other 40×40 centred at
        // (12,12) → centre gap (2,2) ≤ 4.
        let r = compute_alignment_guides((0.0, 0.0, 20.0, 20.0), &[(-8.0, -8.0, 40.0, 40.0)], 4.0);
        // Moving centre is (10,10); other centre is (12,12).
        assert!((r.snap_dx - 2.0).abs() < 1e-9);
        assert!((r.snap_dy - 2.0).abs() < 1e-9);
        assert_eq!(r.guides.len(), 2);
    }

    #[test]
    fn gap_beyond_threshold_does_not_snap() {
        let r =
            compute_alignment_guides((100.0, 100.0, 10.0, 10.0), &[(0.0, 0.0, 10.0, 10.0)], 5.0);
        assert!(r.guides.is_empty());
        assert_eq!((r.snap_dx, r.snap_dy), (0.0, 0.0));
    }

    #[test]
    fn far_candidate_is_ignored_near_one_snaps() {
        // One other is far out of threshold range and contributes
        // nothing; the near one's left edge (x=2) is a gap-2 match
        // for the moving node's left edge (x=0).
        let r = compute_alignment_guides(
            (0.0, 0.0, 10.0, 10.0),
            &[(500.0, 500.0, 10.0, 10.0), (2.0, 0.0, 10.0, 10.0)],
            5.0,
        );
        // Closest x alignment is moving-left(0) ↔ other-left(2): +2.
        assert!((r.snap_dx - 2.0).abs() < 1e-9);
        assert!(r.guides.iter().any(|g| g.vertical));
    }

    #[test]
    fn vertical_guide_spans_aligned_nodes() {
        // Moving at y=100..120, other at y=0..40 → the vertical guide
        // segment should cover y 0..120.
        let r = compute_alignment_guides((0.0, 100.0, 20.0, 20.0), &[(0.0, 0.0, 20.0, 40.0)], 5.0);
        let v = r
            .guides
            .iter()
            .find(|g| g.vertical)
            .expect("vertical guide");
        assert!((v.start - 0.0).abs() < 1e-9);
        assert!((v.end - 120.0).abs() < 1e-9);
    }
}
