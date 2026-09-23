//! Lane ownership between the two ring repairs.
//!
//! Two passes can fix a ring authored wrong, and they must never both act on
//! the same one. This module holds the single predicate that divides them, so
//! the boundary is stated once instead of being re-derived (and drifting) on
//! each side.

use serde_json::Value;

use super::{
    children, has_nonzero_padding, is_arc_ellipse, near_square, numeric, radial_layer_order,
    MAX_SAFE_ARC_TO_PARENT_RATIO, MIN_SAFE_ARC_TO_PARENT_RATIO,
};

/// True when `v` is ITSELF the ring's wrapper — the shape this module repairs
/// by converting the parent in place (`layout:none` + concentric children).
///
/// The gates mirror [`radial_stack_repair`]'s own acceptance conditions, read
/// off the authored tree: zero padding, a recognised radial layer order (one
/// or several centre-content children), and arcs that fill most of the
/// parent box. A parent that fails them
/// is a general-purpose container (a padded card, a section with its own
/// heading, a KPI tile) that merely HOLDS a ring, and converting it would
/// absolutely-position its unrelated content — so this module declines it and
/// `ring_repair` extracts the arcs into a dedicated wrapper instead. Exactly
/// one pass owns any given ring; this predicate is the seam between them.
pub(crate) fn parent_is_dedicated_ring_wrapper(v: &Value) -> bool {
    if has_nonzero_padding(v) || radial_layer_order(v).is_none() {
        return false;
    }
    let kids = children(v);
    let max_arc = kids
        .iter()
        .filter(|child| is_arc_ellipse(child))
        .filter_map(|child| {
            let width = numeric(child, "width").or_else(|| numeric(child, "height"))?;
            let height = numeric(child, "height").or_else(|| numeric(child, "width"))?;
            near_square(width, height).then(|| width.max(height))
        })
        .fold(0.0, f64::max);
    if max_arc <= 0.0 {
        return false;
    }
    // With no authored parent box the wrapper gets SIZED from its arcs, so it
    // is a dedicated ring wrapper by construction.
    let (Some(parent_w), Some(parent_h)) = (numeric(v, "width"), numeric(v, "height")) else {
        return true;
    };
    let parent_min = parent_w.min(parent_h);
    parent_min > 0.0
        && (MIN_SAFE_ARC_TO_PARENT_RATIO..=MAX_SAFE_ARC_TO_PARENT_RATIO)
            .contains(&(max_arc / parent_min))
}
