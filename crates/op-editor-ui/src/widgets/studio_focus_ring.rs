//! The Studio surfaces' keyboard focus ring.
//!
//! Home and the generation workspace are custom-painted, so they have no
//! platform focus indicator; this is theirs. It is a focus-VISIBLE ring:
//! the hosts only set a keyboard focus from Tab / Shift+Tab and drop it on
//! any pointer press, so the ring never shows for mouse users.

use crate::{Color, Rect, RenderBackend};

/// Gap between the focused target's edge and the ring.
pub const FOCUS_RING_OFFSET: f32 = 2.0;
/// Ring stroke width.
pub const FOCUS_RING_WIDTH: f32 = 2.0;

/// Stroke the focus ring `FOCUS_RING_OFFSET` px outside `rect`, with the
/// target's own `radius` grown by the same offset so the ring stays
/// concentric with rounded targets.
pub fn paint_focus_ring(backend: &mut dyn RenderBackend, rect: Rect, radius: f32, color: Color) {
    let ring = Rect::xywh(
        rect.origin.x - FOCUS_RING_OFFSET,
        rect.origin.y - FOCUS_RING_OFFSET,
        rect.size.x + FOCUS_RING_OFFSET * 2.0,
        rect.size.y + FOCUS_RING_OFFSET * 2.0,
    );
    backend.stroke_round_rect(ring, radius + FOCUS_RING_OFFSET, color, FOCUS_RING_WIDTH);
}
