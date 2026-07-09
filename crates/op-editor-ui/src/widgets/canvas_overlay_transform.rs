//! Root→node overlay transform chains for editor chrome.
//!
//! The canvas paints a node inside its ancestors' save/scale/rotate
//! stack, but editor overlays (e.g. the hover outline) paint at page
//! level — outside every node transform. This module carries the
//! chain of per-node flip/rotation transforms from the page root down
//! to a node so overlays can replay it and land exactly on the
//! rendered geometry.
//!
//! Pivots are SCREEN-space (viewport transform already applied), so a
//! chain is only valid for the pan/zoom it was built with.

use crate::widgets::PaintCx;
use crate::Point2D;

/// One node's paint transform — flip scale first, then rotation, both
/// about `pivot` (screen space), matching the order `paint_node_inner`
/// applies to the node's own paint.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct OverlayTransform {
    pub(crate) rotation: f32,
    pub(crate) flip_x: bool,
    pub(crate) flip_y: bool,
    pub(crate) pivot: Point2D,
}

/// Push the chain onto the paint backend (one `save` + the scale /
/// rotate sequence, root first). Returns whether a `save` was pushed —
/// the caller must `restore` after painting when true.
pub(crate) fn replay_on_backend(cx: &mut PaintCx<'_>, transforms: &[OverlayTransform]) -> bool {
    if transforms.is_empty() {
        return false;
    }
    cx.backend.save();
    for t in transforms {
        if t.flip_x || t.flip_y {
            cx.backend.scale(
                Point2D::new(
                    if t.flip_x { -1.0 } else { 1.0 },
                    if t.flip_y { -1.0 } else { 1.0 },
                ),
                t.pivot,
            );
        }
        if t.rotation.abs() > f32::EPSILON {
            cx.backend.rotate(t.rotation, t.pivot);
        }
    }
    true
}
