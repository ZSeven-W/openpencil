//! Buried-overlay repair — a `layout:none` overlay painted over by an opaque
//! sibling that sits EARLIER in the child array.
//!
//! The canvas paints absolute-stack siblings in reverse index order
//! (`canvas_viewport_paint_mask::paint_child_siblings`, `.enumerate().rev()`),
//! so **`children[0]` is topmost**. `skills/phases/agent/design-agent.md`
//! states this to the model — "Put badges, labels, controls, scrims, and other
//! overlays BEFORE the full-bleed image/background they must cover; repair a
//! hidden overlay with `M(overlayId, stackId, 0)`" — but it is the one
//! convention that is inverted from every other tool a model has seen, and a
//! model that authors the base first and the overlays after gets a page whose
//! controls silently vanish under the artwork.
//!
//! Measured on `0808-k3-2.op`'s 星图 screen: a `layout:none` starfield
//! container holding `[circle, compass label, zoom control, gyro pill]`. The
//! 297px circle is `children[0]` — topmost — with an opaque radial gradient,
//! so it paints over all three controls. Only the slivers falling outside the
//! circle survived; the gyro pill showed its icon and half a glyph and nothing
//! else, which is what the user saw and reported as "cut in half".
//!
//! **Why this is contract, not taste.** A node carrying an icon or a label,
//! given an explicit position and its own shadow, and then painted over by an
//! opaque sibling, is not a composition — the author cannot see it. The
//! deliberate version of "a later sibling is covered" is the DECK: a back
//! layer peeking behind a front card. The corpus requires those back layers to
//! be decorative and EMPTY ("NEVER text/icon/content children",
//! `layout.md`), and the ring/gauge stacks are `ellipse`s with no children at
//! all. So "the buried node bears content" is exactly the line between the
//! two, and it is the only gate this pass needs beyond the geometry.
//!
//! Repair: move the buried overlay ahead of whatever covers it — the same
//! `M(overlayId, stackId, 0)` the corpus prescribes, expressed as
//! `EditorCommand::MoveNode { index: Some(0) }`.
//!
//! Two image-specific shapes sit beside it, both measured on GLM-5.3-Flash
//! pages whose generated photos never reached the render: an image hidden by
//! an EMPTY backdrop plate (moved just ahead of the plate), and a content card
//! painted opaque over a scrimmed photo (its fill cleared — the scrim is the
//! author's own proof the photo was meant to show).

use std::collections::HashMap;

use op_editor_core::{EditorCommand, NodeId};
use serde_json::Value;

use super::{children, layout_str, Rect};

/// The covering sibling must hide at least this fraction of the overlay's own
/// area. A corner badge deliberately half-tucked behind a card edge is a
/// composition; a control with three quarters of itself gone is not.
const MIN_BURIED_FRACTION: f64 = 0.6;

/// An OVERLAY is a small thing placed ON a large surface: a badge, a label, a
/// control. It must be at most this fraction of the area of whatever covers it.
///
/// This is the gate that separates the two shapes, and `bears_content` alone is
/// not enough for it. A deck's peeked back card is a PEER of the front card —
/// near-identical size — and the measured `0724-1-gm-2` deck puts an example
/// sentence on that back layer, so "carries content" is true of both. Their
/// areas are not: the gyro pill is 3.6% of the starfield circle it vanished
/// under, while the back card is 92% of the front card that peeks over it.
/// Comparable size means peers in a stack, and their order is a composition.
const OVERLAY_MAX_AREA_RATIO: f64 = 0.5;

/// Does this node paint an opaque surface — something that can actually hide
/// what is behind it? A fill array with any entry counts; a translucent hex
/// (8-digit with a low alpha) does not.
fn paints_opaque(v: &Value) -> bool {
    let Some(first) = v
        .get("fill")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
    else {
        return false;
    };
    match first.get("type").and_then(Value::as_str) {
        Some("linear_gradient" | "radial_gradient" | "mesh_gradient" | "image") => true,
        Some("solid") => first
            .get("color")
            .and_then(Value::as_str)
            .map(|color| {
                // `#RRGGBBAA` — treat anything under ~0.8 alpha as see-through.
                let hex = color.trim();
                if hex.len() != 9 || !hex.starts_with('#') {
                    return true; // token or 6-digit hex: opaque
                }
                u8::from_str_radix(&hex[7..9], 16)
                    .map(|a| a >= 0xCC)
                    .unwrap_or(true)
            })
            .unwrap_or(false),
        _ => false,
    }
}

/// Does this subtree carry something a reader is meant to SEE — a glyph, an
/// icon, an image? The deck's decorative back layers and the ring stacks'
/// bare ellipses deliberately carry nothing.
fn bears_content(v: &Value) -> bool {
    if matches!(
        v.get("type").and_then(Value::as_str),
        Some("text" | "icon_font" | "image")
    ) {
        return true;
    }
    children(v).iter().any(bears_content)
}

/// Fraction of `overlay` hidden by `cover`.
fn covered_fraction(overlay: &Rect, cover: &Rect) -> f64 {
    let w = (overlay.x + overlay.w).min(cover.x + cover.w) - overlay.x.max(cover.x);
    let h = (overlay.y + overlay.h).min(cover.y + cover.h) - overlay.y.max(cover.y);
    if w <= 0.0 || h <= 0.0 {
        return 0.0;
    }
    let area = overlay.w * overlay.h;
    if area <= 0.0 {
        return 0.0;
    }
    (w * h) / area
}

fn rect_of<'a>(v: &Value, rects: &'a HashMap<String, Rect>) -> Option<&'a Rect> {
    v.get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
}

/// The index of the first EMPTY opaque plate that hides an image — the offset
/// "backdrop" a model authors after (i.e. over) the photo it was meant to sit
/// behind. Measured on a GLM-5.3-Flash brewery page: a 744×636 cream
/// `visual-backdrop` rectangle at a lower index than the 768×636 process photo
/// painted a blank panel where the generated image should be.
///
/// The size gate above cannot see this — image and plate are peers in size —
/// but the deck exception it protects does not apply either: a deck's FRONT
/// card carries content, while this cover carries nothing at all. An empty
/// plate over a picture hides the only thing in the stack worth seeing.
fn image_plate_cover(kids: &[Value], index: usize, rects: &HashMap<String, Rect>) -> Option<usize> {
    let image = &kids[index];
    if image.get("type").and_then(Value::as_str) != Some("image") {
        return None;
    }
    let image_rect = rect_of(image, rects)?;
    kids[..index].iter().position(|cover| {
        paints_opaque(cover)
            && !bears_content(cover)
            && rect_of(cover, rects)
                .is_some_and(|c| covered_fraction(image_rect, c) >= MIN_BURIED_FRACTION)
    })
}

/// Is this node a SCRIM — a fill that deliberately lets what is behind it
/// through? Every colour in its first fill must carry a low alpha
/// (`#RRGGBBAA` under ~0.8), whether a solid or each stop of a gradient.
fn is_translucent_scrim(v: &Value) -> bool {
    let Some(first) = v
        .get("fill")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
    else {
        return false;
    };
    let see_through = |color: &Value| {
        color.as_str().is_some_and(|hex| {
            let hex = hex.trim();
            hex.len() == 9
                && hex.starts_with('#')
                && u8::from_str_radix(&hex[7..9], 16).is_ok_and(|a| a < 0xCC)
        })
    };
    match first.get("type").and_then(Value::as_str) {
        Some("solid") => first.get("color").is_some_and(see_through),
        Some("linear_gradient" | "radial_gradient") => first
            .get("stops")
            .and_then(Value::as_array)
            .is_some_and(|stops| {
                !stops.is_empty()
                    && stops
                        .iter()
                        .all(|stop| stop.get("color").is_some_and(see_through))
            }),
        _ => false,
    }
}

fn bears_image(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("image") || children(v).iter().any(bears_image)
}

/// A content card painted opaque over a scrimmed image. Measured on a
/// GLM-5.3-Flash EV page hero: `[scroll-hint, hero-content ($--card, the
/// headline + CTAs), hero-scrim (a #…4D gradient), hero-parallax-bg (the car
/// photo)]`. The scrim exists only to darken the photo under the text, which
/// proves the author meant the photo to show; the opaque card fill above both
/// hides the scrim AND the photo, so the render is a flat panel. Clearing the
/// card's fill turns it back into the text layer the scrim was built for.
fn collect_scrimmed_image_cover_fixes(
    kids: &[Value],
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    for (image_index, image) in kids.iter().enumerate() {
        if !bears_image(image) {
            continue;
        }
        let Some(image_rect) = rect_of(image, rects) else {
            continue;
        };
        let covers = |node: &Value| {
            rect_of(node, rects)
                .is_some_and(|r| covered_fraction(image_rect, r) >= MIN_BURIED_FRACTION)
        };
        let Some(scrim_index) = kids[..image_index]
            .iter()
            .position(|node| is_translucent_scrim(node) && covers(node))
        else {
            continue;
        };
        for card in &kids[..scrim_index] {
            if !(paints_opaque(card) && bears_content(card) && !bears_image(card) && covers(card)) {
                continue;
            }
            let Some(card_id) = card.get("id").and_then(Value::as_str) else {
                continue;
            };
            cmds.push(EditorCommand::PatchNodeData {
                node_id: NodeId::new(card_id.to_string()),
                patch_json: r#"{"fill":[]}"#.to_string(),
                page_id: None,
            });
        }
    }
}

/// Emit a `MoveNode` for every content-bearing overlay buried under an opaque
/// earlier sibling of the same `layout:none` stack: small overlays go to index
/// 0, an image hidden by an empty plate goes just ahead of that plate (so any
/// badge already above both stays on top).
pub(super) fn collect_buried_overlay_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    if layout_str(v) == Some("none") {
        let kids = children(v);
        collect_scrimmed_image_cover_fixes(kids, rects, cmds);
        // Later index = painted EARLIER = further back. Walk from the back
        // forward so the rescued overlays keep their relative order once each
        // lands at index 0.
        for (index, overlay) in kids.iter().enumerate().rev() {
            if index == 0 || !bears_content(overlay) {
                continue;
            }
            let Some(overlay_rect) = rect_of(overlay, rects) else {
                continue;
            };
            let overlay_area = overlay_rect.w * overlay_rect.h;
            let buried = kids[..index].iter().any(|cover| {
                paints_opaque(cover)
                    && rect_of(cover, rects).is_some_and(|c| {
                        covered_fraction(overlay_rect, c) >= MIN_BURIED_FRACTION
                            && overlay_area <= c.w * c.h * OVERLAY_MAX_AREA_RATIO
                    })
            });
            let target_index = if buried {
                0
            } else if let Some(plate_index) = image_plate_cover(kids, index, rects) {
                plate_index
            } else {
                continue;
            };
            let (Some(stack_id), Some(overlay_id)) = (
                v.get("id").and_then(Value::as_str),
                overlay.get("id").and_then(Value::as_str),
            ) else {
                continue;
            };
            cmds.push(EditorCommand::MoveNode {
                node_id: NodeId::new(overlay_id.to_string()),
                target_parent: NodeId::new(stack_id.to_string()),
                page_id: None,
                index: Some(target_index),
            });
        }
    }
    for c in children(v) {
        collect_buried_overlay_fixes(c, rects, cmds);
    }
}

#[cfg(test)]
#[path = "geometry_buried_overlay_tests.rs"]
mod tests;
