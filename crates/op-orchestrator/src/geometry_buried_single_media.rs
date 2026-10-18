//! A single full-viewport image can bury a pure-geometry indicator row too.
//! Keep this narrower than the generic content/card rules and reuse the same
//! static media shape and measured glyph proof as the multi-page lane repair.
use super::*;
use jian_ops_schema::style::PenEffect;

fn tree_composites_statically(node: &Value) -> bool {
    media_node_composites_statically(node) && children(node).iter().all(tree_composites_statically)
}

// An already-front control is not crossed or moved. Its literal outer shadow
// must not block restoring dots behind the image. Unknown/dynamic effects and
// shadows on the viewport, image or crossed overlays are still rejected.
fn front_overlay_effects_are_static(node: &Value) -> bool {
    let effects_ok = media_has_no_effects(node)
        || node
            .get("effects")
            .and_then(Value::as_array)
            .is_some_and(|effects| {
                effects.iter().all(|effect| {
                    let Ok(PenEffect::Shadow(shadow)) = serde_json::from_value(effect.clone())
                    else {
                        return false;
                    };
                    shadow.inner != Some(true)
                        && [shadow.offset_x, shadow.offset_y, shadow.blur, shadow.spread]
                            .iter()
                            .all(|value| value.is_finite())
                        && shadow.blur >= 0.0
                        && op_util::hex_color::parse_hex_rgba8(
                            &shadow.color,
                            op_util::hex_color::HexOptions::LENIENT,
                        )
                        .is_some()
                })
            });
    effects_ok && children(node).iter().all(front_overlay_effects_are_static)
}

pub(super) fn sink(
    state: Option<&EditorState>,
    stack: &Value,
    kids: &[Value],
    rects: &HashMap<String, Rect>,
) -> Option<(usize, usize)> {
    let viewport = rect_of(stack, rects)?;
    let photo = crate::clipped_media_strip::single_media_page(stack, viewport.w)?;
    if !tree_composites_statically(stack)
        || !media_has_no_effects(stack)
        || !media_lane_composites_normally(photo)
        || !media_page_is_opaque(photo, state)
        || covered_fraction(viewport, rect_of(photo, rects)?) < 0.999
        || kids
            .iter()
            .filter(|child| !std::ptr::eq(*child, photo))
            .any(|child| overlay_has_large_opaque_surface(child, viewport, rects))
    {
        return None;
    }
    let index = kids.iter().position(|child| std::ptr::eq(child, photo))?;
    let target = kids
        .iter()
        .enumerate()
        .skip(index + 1)
        .rev()
        .find(|(_, overlay)| covered_overlay_glyph(overlay, viewport, rects))?
        .0;
    if !kids[..index].iter().all(front_overlay_effects_are_static)
        || !kids[index + 1..=target]
            .iter()
            .all(media_lane_composites_normally)
    {
        return None;
    }
    Some((index, target))
}

#[cfg(test)]
#[path = "geometry_buried_single_media_tests.rs"]
mod tests;
