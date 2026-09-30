//! A scrim authored ABOVE the copy it was meant to sit under.
//!
//! A scrim darkens (or fades) a photo so the text laid over it reads; its one
//! legal slot is between the two — below the copy, above the picture. Because
//! `children[0]` paints topmost, a model that lists the scrim first puts it
//! over the headline instead. Measured on two GLM-5.3-Flash mobile heroes
//! (arena m04/m05, 2026-09-29): `[hero-scrim, hero-copy, hero-illustration]`
//! faded a white headline into a `transparent → $--background` wash, and
//! `[hero-scrim, back, bookmark, hero-title, photo]` dimmed the title and the
//! controls it was built to lift.
//!
//! **Contract, not taste**: a see-through, childless layer that lies over
//! content AND over the photo below that content has only one reading. A
//! modal backdrop, which really is meant to dim content, has no photo as a
//! direct sibling beneath the content it covers, so it is never matched.

use std::collections::HashMap;

use serde_json::Value;

use super::{
    bears_content, children, covered_fraction, is_photo, paints_see_through, rect_of, Rect,
};

/// The scrim must overlap at least this much of a content sibling's box to
/// count as covering it.
const MIN_COVERED_CONTENT: f64 = 0.3;
/// The photo beneath must be at least this much under the scrim — the scrim
/// has to be ON the picture it is meant to tint.
const MIN_SCRIM_ON_PHOTO: f64 = 0.6;
/// A layer must reach at least this opacity somewhere to hide copy. A faint
/// grain or glow wash (`#FFFFFF14`) on top of everything is the composition
/// working as intended and stays where it is.
const MIN_HIDING_ALPHA: f64 = 0.5;

/// Highest opacity any colour of the node's fills reaches. `#RRGGBBAA` reads
/// its alpha; a 6-digit hex or a `$variable` counts as opaque.
fn max_alpha(v: &Value) -> f64 {
    fn alpha(color: &Value) -> f64 {
        match color.as_str().map(str::trim) {
            Some(hex) if hex.len() == 9 && hex.starts_with('#') => {
                u8::from_str_radix(&hex[7..9], 16)
                    .map(|a| f64::from(a) / 255.0)
                    .unwrap_or(1.0)
            }
            _ => 1.0,
        }
    }
    v.get("fill")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|fill| {
            let solid = fill.get("color").map(alpha);
            let stops = fill
                .get("stops")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|stop| stop.get("color").map(alpha));
            solid.into_iter().chain(stops)
        })
        .fold(0.0, f64::max)
}

/// `(scrim_index, target_index)` for the first scrim that sits above content
/// it overlaps while a photo it tints lies below that content. `target_index`
/// is post-detach (`MoveNode` removes first), landing the scrim directly above
/// that photo.
pub(super) fn scrim_over_content(
    kids: &[Value],
    rects: &HashMap<String, Rect>,
) -> Option<(usize, usize)> {
    kids.iter().enumerate().find_map(|(scrim_index, scrim)| {
        if !children(scrim).is_empty()
            || bears_content(scrim)
            || !paints_see_through(scrim)
            || max_alpha(scrim) < MIN_HIDING_ALPHA
        {
            return None;
        }
        let scrim_rect = rect_of(scrim, rects)?;
        let last_covered = kids
            .iter()
            .enumerate()
            .skip(scrim_index + 1)
            .filter(|(_, sib)| bears_content(sib) && !is_photo(sib))
            .filter(|(_, sib)| {
                rect_of(sib, rects)
                    .is_some_and(|r| covered_fraction(r, scrim_rect) >= MIN_COVERED_CONTENT)
            })
            .map(|(index, _)| index)
            .next_back()?;
        let photo_index = kids
            .iter()
            .enumerate()
            .skip(last_covered + 1)
            .find(|(_, sib)| {
                is_photo(sib)
                    && rect_of(sib, rects)
                        .is_some_and(|r| covered_fraction(scrim_rect, r) >= MIN_SCRIM_ON_PHOTO)
            })
            .map(|(index, _)| index)?;
        Some((scrim_index, photo_index - 1))
    })
}

#[cfg(test)]
#[path = "geometry_buried_overlay_scrim_tests.rs"]
mod tests;
