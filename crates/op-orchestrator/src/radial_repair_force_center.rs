//! Lenient (tier-2) concentric repair for authored radial stacks.
//!
//! `authored_radial_patch` (in the parent module) is the strict,
//! high-confidence pass: besides repositioning, it also *resizes* things
//! (fills in missing arc/centre dimensions) and gates on wrapper aspect
//! ratio, arc-to-parent-size ratio, and zero padding to be confident that's
//! safe. Those gates are about whether the ring is worth reshaping
//! automatically — not about whether it's safe to *centre*.
//!
//! Concentricity and front-to-back paint order carry no such wrapper-shape
//! judgment call. Once `radial_layers` has already recognised a genuine
//! track/progress (or segmented-donut) ring — and `radial_layer_order` has
//! derived an unambiguous centre/progress/track order for it — sharing one
//! centre point and painting centre-then-progress-then-track are geometric
//! and structural facts, not guesses, regardless of the wrapper's aspect
//! ratio. Padding doesn't even enter into it: jian positions an explicit
//! `x`/`y` child of a `layout:"none"` frame from the frame's *border* box
//! (taffy's absolute-layout algorithm only adds `border`, never `padding`,
//! to an inset that's already set) — so the wrapper's `padding` value is
//! irrelevant to where a centred child actually renders, and centring
//! always targets the full `width`×`height` box regardless of it.
//!
//! So this module re-centres and reorders whatever `authored_radial_patch`
//! declined to touch for wrapper-shape or padding reasons, without resizing
//! anything it didn't already have a size for. With several centre-content
//! children (a big number plus a caption — the natural timer-ring shape)
//! centring each would stack them on one point, so their authored x/y is
//! kept and only the arcs are re-centred. What still keeps a stack
//! unfixable here (echoed back instead of guessed): a child whose size can
//! be neither read nor estimated at all, a child that doesn't fit inside
//! the wrapper's box no matter how it's centred (an authoring-size bug, not
//! a position bug), or arc diameters too mismatched to plausibly be the
//! same ring (`radial_layers` classifies track vs. progress by sweep angle /
//! naming alone, not by size, so nothing upstream has already vetted that
//! the pairing is size-plausible).

use serde_json::Value;

use super::{
    estimated_subtree_size, has_numeric, is_arc_ellipse, numeric, radial_layers, valid_size,
    MIN_SAFE_ARC_DIAMETER_RATIO,
};

/// `(x, y, width, height)`.
type Placement = (f64, f64, f64, f64);

/// Re-centre every direct arc child of `node` on the wrapper's full box and
/// fix the paint order to centre/progress/track. Centre-content children of
/// a multi-centre stack keep their authored x/y (the author placed the big
/// number above the caption; centring each would stack them on one point).
/// Returns `false` (no-op) when `node` isn't a recognised radial stack, or
/// when the wrapper's own size or any child's size can't be read or
/// estimated — those cases are left for the caller to report rather than
/// guess at.
pub(super) fn force_concentric_radial_stack(node: &mut Value) -> bool {
    let Some(layers) = radial_layers(node) else {
        return false;
    };
    let order = super::layer_order(&layers);
    let Some(placements) = concentric_placements(node) else {
        return false;
    };
    let keep_authored_centres = layers.centres.len() > 1;

    let mut changed = set_if_different(node, "layout", Value::String("none".into()));
    changed |= set_if_different(node, "gap", Value::from(0.0));
    changed |= set_if_different(node, "justifyContent", Value::String("start".into()));
    changed |= set_if_different(node, "alignItems", Value::String("start".into()));

    let kids = node
        .get_mut("children")
        .and_then(Value::as_array_mut)
        .expect("radial_layers already confirmed children exist");
    changed |= order.iter().copied().ne(0..kids.len());

    // Pair each original child with its computed placement before
    // reordering, so the permutation below carries the right placement
    // along with the right child.
    let mut slots: Vec<Option<(Value, Placement)>> = std::mem::take(kids)
        .into_iter()
        .zip(placements)
        .map(Some)
        .collect();
    for index in order {
        let Some((mut child, (x, y, width, height))) = slots.get_mut(index).and_then(Option::take)
        else {
            continue;
        };
        if !keep_authored_centres || is_arc_ellipse(&child) {
            let had_width = has_numeric(&child, "width");
            let had_height = has_numeric(&child, "height");
            changed |= set_if_different(&mut child, "x", Value::from(x));
            changed |= set_if_different(&mut child, "y", Value::from(y));
            if !had_width {
                changed |= set_if_different(&mut child, "width", Value::from(width.round()));
            }
            if !had_height {
                changed |= set_if_different(&mut child, "height", Value::from(height.round()));
            }
        }
        kids.push(child);
    }
    changed
}

/// True when `node` is a recognised radial stack that this lenient pass
/// still can't call safe: the children aren't already in
/// centre/progress/track order, it isn't overlaid (`layout:none`), an arc's
/// authored x/y doesn't match the centred placement, or the placements
/// can't even be computed (unmeasurable wrapper/child size). Centre-content
/// children of a multi-centre stack are exempt from the x/y check — their
/// positions are the author's call; only the arcs must share the centre.
/// `node` not being a radial stack at all is *not* unsafe — there's
/// nothing to check.
pub(super) fn is_still_off_center(node: &Value) -> bool {
    let Some(layers) = radial_layers(node) else {
        return false;
    };
    let order = super::layer_order(&layers);
    let Some(placements) = concentric_placements(node) else {
        return true;
    };
    if node.get("layout").and_then(Value::as_str) != Some("none") {
        return true;
    }
    if order.iter().copied().ne(0..placements.len()) {
        return true;
    }
    let skip_centre_xy = layers.centres.len() > 1;
    let kids = node
        .get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    placements
        .iter()
        .zip(kids)
        .any(|((x, y, _width, _height), child)| {
            if skip_centre_xy && !is_arc_ellipse(child) {
                return false;
            }
            numeric(child, "x") != Some(*x) || numeric(child, "y") != Some(*y)
        })
}

/// The centred `(x, y, width, height)` every direct child of `node` should
/// sit at against the wrapper's full `width`×`height` box. In a
/// multi-centre stack only the arc entries are consumed — centre-content
/// children keep their authored x/y, while their width/height still feed
/// the measurement and fit gates. (Padding is
/// deliberately not subtracted — see the module doc: jian positions an
/// explicit-inset absolute child from the border box, ignoring the
/// parent's padding, so a padding-aware centre would be centring against a
/// box the renderer never actually uses). Indexed in the child array's
/// *original* order — callers that also reorder must carry each placement
/// along with its child through the permutation, not re-look-up by new
/// position. `width`/`height` are the authored size when present, else the
/// best estimate used to compute the centred `x`/`y` — callers only need
/// to persist them when the child had none authored. `None` means there is
/// nothing safe to compute: the wrapper's own size or some child's size
/// can't be read or estimated, a child doesn't fit inside the wrapper's
/// box at all (an authoring-size bug translation can't paper over), or the
/// arcs' diameters are too mismatched to plausibly be the same ring.
fn concentric_placements(node: &Value) -> Option<Vec<Placement>> {
    let box_w = numeric(node, "width")?;
    let box_h = numeric(node, "height")?;
    if !valid_size(box_w, box_h) {
        return None;
    }

    let kids = node.get("children").and_then(Value::as_array)?;
    let mut placements = Vec::with_capacity(kids.len());
    let mut arc_diameters = Vec::new();
    for child in kids {
        let estimate = estimated_subtree_size(child);
        let width = numeric(child, "width").or_else(|| estimate.map(|size| size.0))?;
        let height = numeric(child, "height").or_else(|| estimate.map(|size| size.1))?;
        if !valid_size(width, height) || width > box_w + 0.5 || height > box_h + 0.5 {
            return None;
        }
        if is_arc_ellipse(child) {
            arc_diameters.push(width.max(height));
        }
        let x = ((box_w - width) / 2.0).round();
        let y = ((box_h - height) / 2.0).round();
        placements.push((x, y, width, height));
    }
    let min_arc = arc_diameters.iter().copied().fold(f64::INFINITY, f64::min);
    let max_arc = arc_diameters.iter().copied().fold(0.0, f64::max);
    if max_arc > 0.0 && min_arc / max_arc < MIN_SAFE_ARC_DIAMETER_RATIO {
        return None;
    }
    Some(placements)
}

fn set_if_different(node: &mut Value, key: &str, value: Value) -> bool {
    if node.get(key) == Some(&value) {
        return false;
    }
    node[key] = value;
    true
}

#[cfg(test)]
#[path = "radial_repair_force_center_tests.rs"]
mod tests;
