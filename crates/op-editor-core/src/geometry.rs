//! Bounds + geometry helpers for the editor mutators.
//!
//! `PenNode` does not carry a single `bounds` rect like shell-core's
//! `Node` did — geometry is `x` / `y` on `PenNodeBase` plus per-
//! variant `width` / `height`. This module derives an
//! axis-aligned doc-space rect for any node and unions rects across
//! a selection set.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::walkers::find_node;
use jian_ops_schema::node::PenNode;

/// An axis-aligned document-space rectangle. f64 to line up with the
/// canonical schema's coordinate type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DocRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl DocRect {
    /// A zero rect at the origin — the "no geometry" sentinel.
    pub const ZERO: DocRect = DocRect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };

    /// True when both dimensions are non-positive (no real area).
    pub fn is_empty(&self) -> bool {
        self.w <= 0.0 && self.h <= 0.0
    }
}

/// A node's own axis-aligned rect: `(x, y)` from `base`, `width` /
/// `height` from the variant. Missing coordinates default to 0;
/// missing dimensions to 0 (a container that derives size from
/// children reads as zero-size — callers fall back to the child
/// union via [`aggregate_bounds`]).
pub fn own_bounds(node: &PenNode) -> DocRect {
    let base = node.base();
    DocRect {
        x: base.x.unwrap_or(0.0),
        y: base.y.unwrap_or(0.0),
        w: node.width_px().unwrap_or(0.0),
        h: node.height_px().unwrap_or(0.0),
    }
}

/// Effective bounds: the node's own rect if it has real area, else
/// the union of its children's aggregate bounds. Mirrors shell-core
/// `Node::aggregate_bounds`.
pub fn aggregate_bounds(node: &PenNode) -> DocRect {
    let own = own_bounds(node);
    if own.w > 0.0 || own.h > 0.0 {
        return own;
    }
    let Some(children) = node.children() else {
        return DocRect::ZERO;
    };
    union_of(children.iter().map(aggregate_bounds))
}

/// Union of an iterator of rects, skipping empty ones. `ZERO` when
/// every rect is empty.
pub fn union_of(rects: impl Iterator<Item = DocRect>) -> DocRect {
    let mut iter = rects.filter(|r| !r.is_empty());
    let Some(first) = iter.next() else {
        return DocRect::ZERO;
    };
    let (mut min_x, mut min_y) = (first.x, first.y);
    let (mut max_x, mut max_y) = (first.x + first.w, first.y + first.h);
    for r in iter {
        min_x = min_x.min(r.x);
        min_y = min_y.min(r.y);
        max_x = max_x.max(r.x + r.w);
        max_y = max_y.max(r.y + r.h);
    }
    DocRect {
        x: min_x,
        y: min_y,
        w: max_x - min_x,
        h: max_y - min_y,
    }
}

/// Union of `aggregate_bounds` across the ids that resolve in the
/// forest. `None` when no id resolves to a node with real area.
pub fn union_aggregate_bounds(children: &[PenNode], ids: &[NodeId]) -> Option<DocRect> {
    let rect = union_of(
        ids.iter()
            .filter_map(|id| find_node(children, id))
            .map(aggregate_bounds),
    );
    if rect.is_empty() {
        None
    } else {
        Some(rect)
    }
}
