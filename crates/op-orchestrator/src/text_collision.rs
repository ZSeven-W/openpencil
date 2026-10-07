//! Cross-subtree TEXT-vs-TEXT collision detector.
//!
//! `collect_sibling_jam_diagnostics` (the existing overlap/jam detector)
//! only compares DIRECT siblings inside one flex row/column, and it
//! deliberately steps over `layout:none` parents — a `layout:none` card
//! stack is usually an intentional deck where the CARDS overlap on
//! purpose (front card hides the back one). That gate is correct for
//! boxes, but it also makes the sibling-jam detector structurally blind
//! to the failure this module catches: a weak model authoring a
//! `layout:none` deck routinely lands a LEAF text node from one card
//! directly on top of a leaf text node from another card. The two text
//! nodes are cousins, not siblings, and neither one's ancestor chain sets
//! `layout:none` at the SAME level the jam detector inspects, so the jam
//! pass never sees the pair at all.
//!
//! The contract here is deliberately flatter than the box-overlap one:
//! Different reading copy landing on the same pixels is a defect. A proven
//! same-copy offset treatment is one text effect, not two competing messages;
//! the shared text-effect proof scopes that exception to one tiny wrapper.
//! Detect-only: which reading text should move belongs to the model.
//!
//! The other carve-out is about INK rather than
//! intent: a block set at watermark opacity contributes no ink to obscure
//! with, so the premise "neither is readable" never holds for it. See
//! [`WATERMARK_MAX_OPACITY`].
//!
//! Measured root cause (0724-1-gm-2.op, "今日词卡" stacked word-card
//! preview): the front card's Chinese meaning resolves to y 708–728 and
//! the back card's example sentence resolves to y 705–722 (68% of the
//! smaller block's area) because both cards anchor their text near the
//! bottom of a shared `layout:none` container without either one
//! accounting for the other card's text block.

use std::collections::HashMap;

use serde_json::Value;

use super::{children, Rect};

/// Both axes must show more than this many px of overlap before a pair
/// counts as colliding — guards against sub-pixel rounding touches
/// between adjacent (non-overlapping) flow text.
const MIN_AXIS_OVERLAP_PX: f64 = 4.0;

/// The overlapping area must cover more than this fraction of the
/// SMALLER text block's own area — guards against a long line merely
/// clipping the corner of a short label it doesn't meaningfully collide
/// with.
const MIN_OVERLAP_AREA_RATIO: f64 = 0.25;

/// One detected TEXT-vs-TEXT collision. Deliberately a structured payload
/// (id + name + resolved rect for each side, plus the measured overlap)
/// rather than a pre-formatted string — this is `geometry_validation`'s
/// first detector with a real category/shape, laying the ground for
/// `loop_blocker_ledger` to eventually classify it directly instead of
/// pattern-matching message text.
#[derive(Debug)]
pub(crate) struct TextCollision {
    pub(crate) a_id: String,
    pub(crate) a_name: String,
    pub(crate) a_rect: Rect,
    pub(crate) b_id: String,
    pub(crate) b_name: String,
    pub(crate) b_rect: Rect,
    pub(crate) overlap_x: f64,
    pub(crate) overlap_y: f64,
    /// Overlap area as a fraction of the SMALLER of the two text blocks'
    /// own areas (0.0–1.0+; values are already known > MIN_OVERLAP_AREA_RATIO).
    pub(crate) overlap_area_ratio: f64,
}

/// At or below this alpha a text block is a WATERMARK: ink so faint it
/// cannot obscure anything, and cannot itself be obscured in any way a
/// reader would notice.
///
/// This is the one exception to the module's "two text blocks on the same
/// pixels is never intent" premise, and it is a fact about rendering rather
/// than a taste call — the premise assumes both blocks contribute ink.
///
/// Measured on the shipped `lecture-deck-light` / `pitch-deck-dark` scene
/// templates (human-authored and audited): each slide sets a giant page
/// numeral — `fontSize` 460–480 at `opacity` 0.06–0.07 — behind the title
/// block, the standard editorial deck watermark. Every one of those decks'
/// 8 reported collisions was a title crossing that numeral, and none of them
/// is a defect. A genuinely translucent-but-readable label sits far above
/// this line (0.4+), so the gate cannot swallow a real collision.
const WATERMARK_MAX_OPACITY: f64 = 0.15;

fn is_watermark(v: &Value) -> bool {
    v.get("opacity")
        .and_then(Value::as_f64)
        .is_some_and(|opacity| opacity <= WATERMARK_MAX_OPACITY)
}

fn is_visible_nonempty_text(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("text")
        && v.get("visible").and_then(Value::as_bool) != Some(false)
        && !is_watermark(v)
        && crate::text_effect_overlay::content(v).is_some_and(|c| !c.trim().is_empty())
}

/// Depth-first collection of every visible, non-empty text LEAF under `v`
/// that has a positive-area resolved rect. Document order, so pairwise
/// comparison downstream is deterministic.
fn collect_text_leaves<'a>(
    v: &'a Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<(&'a str, &'a str, Rect)>,
) {
    if is_visible_nonempty_text(v) {
        if let Some(id) = v.get("id").and_then(Value::as_str) {
            if let Some(rect) = rects.get(id) {
                if rect.w > 0.0 && rect.h > 0.0 {
                    let name = v.get("name").and_then(Value::as_str).unwrap_or("text");
                    out.push((id, name, *rect));
                }
            }
        }
    }
    // Text is a leaf in this schema; recursing defensively costs nothing
    // and matches the `bears_text` convention elsewhere in this module.
    for c in children(v) {
        collect_text_leaves(c, rects, out);
    }
}

/// All TEXT-vs-TEXT collisions inside `root`'s subtree (root included),
/// scoped to that one root the same way the caller scopes every other
/// per-root geometry diagnostic.
pub(crate) fn collect_text_collisions(
    root: &Value,
    rects: &HashMap<String, Rect>,
) -> Vec<TextCollision> {
    let mut leaves = Vec::new();
    collect_text_leaves(root, rects, &mut leaves);
    let effects = crate::text_effect_overlay::effect_pairs(root);
    let mut out = Vec::new();
    for i in 0..leaves.len() {
        let (a_id, a_name, a) = leaves[i];
        for &(b_id, b_name, b) in &leaves[i + 1..] {
            let key = if a_id < b_id {
                (a_id.into(), b_id.into())
            } else {
                (b_id.into(), a_id.into())
            };
            if effects.contains(&key) {
                continue;
            }
            let overlap_x = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
            let overlap_y = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
            if overlap_x <= MIN_AXIS_OVERLAP_PX || overlap_y <= MIN_AXIS_OVERLAP_PX {
                continue;
            }
            let smaller_area = (a.w * a.h).min(b.w * b.h);
            if smaller_area <= 0.0 {
                continue;
            }
            let overlap_area_ratio = (overlap_x * overlap_y) / smaller_area;
            if overlap_area_ratio <= MIN_OVERLAP_AREA_RATIO {
                continue;
            }
            out.push(TextCollision {
                a_id: a_id.to_string(),
                a_name: a_name.to_string(),
                a_rect: a,
                b_id: b_id.to_string(),
                b_name: b_name.to_string(),
                b_rect: b,
                overlap_x,
                overlap_y,
                overlap_area_ratio,
            });
        }
    }
    out
}

fn format_collision(c: &TextCollision) -> String {
    format!(
        "{} ({}, resolved at {}x{} @{},{}) and {} ({}, resolved at {}x{} @{},{}): TEXT leaves \
         OVERLAP by {}x{}px (~{}% of the smaller block's area) — two separate text blocks are \
         rendering on the same pixels; give each its own non-overlapping position (or \
         hide/remove one) so neither is unreadable",
        c.a_name,
        c.a_id,
        c.a_rect.w.round(),
        c.a_rect.h.round(),
        c.a_rect.x.round(),
        c.a_rect.y.round(),
        c.b_name,
        c.b_id,
        c.b_rect.w.round(),
        c.b_rect.h.round(),
        c.b_rect.x.round(),
        c.b_rect.y.round(),
        c.overlap_x.round(),
        c.overlap_y.round(),
        (c.overlap_area_ratio * 100.0).round()
    )
}

/// Detect + format text collisions under `root`, appending to `out` and
/// respecting the shared `MAX_DIAGNOSTICS` budget the same way every
/// other collector in this module does.
pub(crate) fn push_text_collision_diagnostics(
    root: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    for collision in collect_text_collisions(root, rects) {
        if out.len() >= super::MAX_DIAGNOSTICS {
            return;
        }
        out.push(format_collision(&collision));
    }
}

#[cfg(test)]
#[path = "text_collision_tests.rs"]
mod tests;
