//! Card inner padding — a painted, rounded card laid out horizontally or
//! vertically whose content sits FLUSH against the card edge (no authored
//! padding) gets the standard card inset `padding: [12, 16]` (vertical 12,
//! horizontal 16).
//!
//! Measured on the round-2 fitness screen: an "Exercise Row" card (rounded,
//! solid-filled horizontal frame; children = thumbnail image + info frame +
//! duration text) carried no padding, so the thumbnail and the trailing text
//! touched the card's left/right edges. The fix is paint-level, not geometry:
//! the card is the right size, it simply offers no inner breathing room.
//!
//! Deliberate exclusions (each a shape with its own contract):
//! - status bar / bottom tab bar subtrees (chrome has its own passes),
//! - frames inside a `layout: "none"` parent (absolute stacks own their
//!   children's placement; padding would shift a pinned layout),
//! - frames whose only children are frames (structural wrappers, not cards),
//! - image-only cards (a single full-bleed thumbnail is intentional),
//! - chips (resolved width < 120 AND height < 44 — `touch-target-floor` and
//!   the chip rules own them),
//! - frames that already carry any non-zero padding.
//!
//! Runs in the cleanup pre-pass right after `touch-target-floor`, checkpointed
//! under `card-inner-padding` in the Layout category.

use super::*;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use std::collections::HashSet;

/// Card inset applied by this repair: vertical 12, horizontal 16 — the
/// schema's two-value `[vertical, horizontal]` padding form.
const CARD_PADDING: [f64; 2] = [12.0, 16.0];
/// Below this radius a painted frame reads as a flat band, not a card.
const MIN_CARD_CORNER_RADIUS: f64 = 8.0;
/// Chips have their own sizing rules: a resolved box narrower than 120px AND
/// shorter than the 44px touch floor is a chip, not a card.
const CHIP_MAX_WIDTH: f64 = 120.0;
const CHIP_MAX_HEIGHT: f64 = 44.0;

/// Apply the card-inner-padding repair to one root. Returns the number of
/// edits the sink accepted.
pub(crate) fn repair_card_inner_padding(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let rects = resolved_rects(sink.state());
    let (root, bottom_nav_ids) = {
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.to_string()),
        ) else {
            return 0;
        };
        let bottom_nav_ids = bottom_nav_protected_ids(root);
        let Ok(value) = serde_json::to_value(root) else {
            return 0;
        };
        (value, bottom_nav_ids)
    };

    let mut cmds = Vec::new();
    collect_card_inner_padding_fixes(&root, &rects, &bottom_nav_ids, &mut cmds);

    let mut applied = 0;
    for cmd in cmds {
        if sink.apply(cmd) {
            applied += 1;
        }
    }
    applied
}

/// Collect a `padding: [12, 16]` repair for every painted, unpadded card in
/// the tree.
pub(super) fn collect_card_inner_padding_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    bottom_nav_ids: &HashSet<String>,
    cmds: &mut Vec<EditorCommand>,
) {
    let mut ancestors = Vec::new();
    collect_in_tree(v, rects, bottom_nav_ids, &mut ancestors, cmds);
}

fn collect_in_tree<'a>(
    v: &'a Value,
    rects: &HashMap<String, Rect>,
    bottom_nav_ids: &HashSet<String>,
    ancestors: &mut Vec<&'a Value>,
    cmds: &mut Vec<EditorCommand>,
) {
    if is_card_inner_padding_offender(v, rects, bottom_nav_ids, ancestors) {
        let id = v.get("id").and_then(Value::as_str).unwrap_or_default();
        cmds.push(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(id.to_string()),
            property: "padding".to_string(),
            value: LayoutPropValue::NumberArray(CARD_PADDING.to_vec()),
        });
    }

    ancestors.push(v);
    for child in children(v) {
        collect_in_tree(child, rects, bottom_nav_ids, ancestors, cmds);
    }
    ancestors.pop();
}

fn is_card_inner_padding_offender(
    v: &Value,
    rects: &HashMap<String, Rect>,
    bottom_nav_ids: &HashSet<String>,
    ancestors: &[&Value],
) -> bool {
    if v.get("type").and_then(Value::as_str) != Some("frame")
        || !matches!(layout_str(v), Some("horizontal" | "vertical"))
        || !has_visible_solid_fill_or_stroke(v)
        || corner_radius_min(v).is_none_or(|radius| radius < MIN_CARD_CORNER_RADIUS)
    {
        return false;
    }

    // Padding must be absent or zero on every side. An expression-backed
    // padding (`None` from the tolerant reader) is intent this pass cannot
    // evaluate, so it stays untouched; so does any non-zero side.
    let Some(sides) = numeric_padding_sides(v) else {
        return false;
    };
    if sides.iter().any(|side| *side != 0.0) {
        return false;
    }

    let kids = children(v);
    // A full-bleed thumbnail card is deliberate: a single image child is
    // meant to reach the painted edge.
    if kids.len() == 1 && kids[0].get("type").and_then(Value::as_str) == Some("image") {
        return false;
    }
    // The card must hold actual content at its edge: at least one direct
    // text / image / icon LEAF. A frame whose only children are frames is a
    // structural wrapper, not a card.
    let has_edge_content = kids.iter().any(|child| {
        matches!(
            child.get("type").and_then(Value::as_str),
            Some("text" | "image" | "icon_font")
        ) && children(child).is_empty()
    });
    if !has_edge_content {
        return false;
    }

    // Chips have their own sizing rules: resolved width < 120 AND height < 44.
    if let Some(rect) = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
    {
        if rect.w < CHIP_MAX_WIDTH && rect.h < CHIP_MAX_HEIGHT {
            return false;
        }
    }

    // Chrome subtrees have their own passes, and a `layout: "none"` parent
    // owns its children's placement — padding would shift a pinned layout.
    if is_protected_node(v, bottom_nav_ids)
        || ancestors.iter().any(|ancestor| {
            crate::cleanup::is_status_bar_from_json(ancestor)
                || is_protected_node(ancestor, bottom_nav_ids)
        })
        || ancestors
            .last()
            .is_some_and(|parent| layout_str(parent) == Some("none"))
    {
        return false;
    }

    true
}

fn is_protected_node(v: &Value, bottom_nav_ids: &HashSet<String>) -> bool {
    crate::cleanup::is_status_bar_from_json(v)
        || v.get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| bottom_nav_ids.contains(id))
}

fn corner_radius_min(v: &Value) -> Option<f64> {
    match v.get("cornerRadius") {
        Some(Value::Array(values)) if !values.is_empty() => values
            .iter()
            .map(number_value)
            .collect::<Option<Vec<_>>>()
            .and_then(|values| values.into_iter().reduce(f64::min)),
        Some(value) => number_value(value),
        None => None,
    }
}

fn has_visible_solid_fill_or_stroke(v: &Value) -> bool {
    let own_fill = v
        .get("fill")
        .and_then(Value::as_array)
        .is_some_and(|fills| fills.iter().any(visible_solid_fill));
    let stroke_fill = v
        .get("stroke")
        .and_then(Value::as_object)
        .and_then(|stroke| stroke.get("fill"))
        .and_then(Value::as_array)
        .is_some_and(|fills| fills.iter().any(visible_solid_fill));
    own_fill || stroke_fill
}

fn visible_solid_fill(fill: &Value) -> bool {
    if fill.get("type").and_then(Value::as_str) != Some("solid")
        || fill
            .get("opacity")
            .and_then(number_value)
            .is_some_and(|opacity| opacity == 0.0)
    {
        return false;
    }
    let Some(color) = fill.get("color").and_then(Value::as_str) else {
        return false;
    };
    if color.eq_ignore_ascii_case("transparent") {
        return false;
    }
    if color.starts_with('$') {
        return true;
    }
    op_util::hex_color::parse_hex_rgba8(color, op_util::hex_color::HexOptions::LENIENT)
        .is_some_and(|rgba| rgba[3] != 0)
}

fn number_value(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
}

/// Reuse the cleanup chrome predicates and protect the whole matched
/// bottom-tab subtree, including unnamed structural rows nested inside it.
fn bottom_nav_protected_ids(root: &PenNode) -> HashSet<String> {
    let mut ids = HashSet::new();
    let Some(children) = root.children() else {
        return ids;
    };
    let last_index = children.len().saturating_sub(1);
    for (index, child) in children.iter().enumerate() {
        if (index == last_index && crate::cleanup::is_trailing_bottom_nav_section(child))
            || crate::cleanup::is_bottom_nav_section(child)
        {
            collect_subtree_ids(child, &mut ids);
        } else {
            collect_named_bottom_nav_ids(child, &mut ids);
        }
    }
    ids
}

fn collect_named_bottom_nav_ids(node: &PenNode, ids: &mut HashSet<String>) {
    if crate::cleanup::is_bottom_nav_section(node) {
        collect_subtree_ids(node, ids);
        return;
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_named_bottom_nav_ids(child, ids);
        }
    }
}

fn collect_subtree_ids(node: &PenNode, ids: &mut HashSet<String>) {
    ids.insert(node.id_str().to_string());
    if let Some(children) = node.children() {
        for child in children {
            collect_subtree_ids(child, ids);
        }
    }
}

#[cfg(test)]
#[path = "card_inner_padding_tests.rs"]
mod tests;
