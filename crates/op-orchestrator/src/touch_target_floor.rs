//! Raise undersized, painted mobile controls to the 48px touch-target floor.
//!
//! Detection is based on jian's resolved rectangles. The authored shape is
//! intentionally narrow: a painted horizontal/vertical frame with one or two
//! text/icon leaves, so cards and arbitrary content rows are not resized.

use super::*;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use std::collections::HashSet;

const TOUCH_TARGET_FLOOR: f64 = 44.0;
const TOUCH_TARGET_HEIGHT: i32 = 48;
const MIN_TOUCH_TARGET_WIDTH: f64 = 120.0;

/// Apply the touch-target repair to one mobile-screen root.
pub(crate) fn repair_touch_target_floor(sink: &mut dyn DocSink, root_id: &str) -> usize {
    if super::root_design_form(sink.state(), root_id) != DesignForm::MobileScreen {
        return 0;
    }

    let rects = super::resolved_rects(sink.state());
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
    collect_touch_target_floor_fixes(&root, &rects, &bottom_nav_ids, &mut cmds);

    let mut applied = 0;
    for cmd in cmds {
        if sink.apply(cmd) {
            applied += 1;
        }
    }
    applied
}

/// Collect height and, when needed, cross-axis alignment repairs from the
/// current resolved layout.
pub(super) fn collect_touch_target_floor_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    bottom_nav_ids: &HashSet<String>,
    cmds: &mut Vec<EditorCommand>,
) {
    let mut ancestors = Vec::new();
    collect_in_tree(v, rects, bottom_nav_ids, cmds, &mut ancestors);
}

fn collect_in_tree<'a>(
    v: &'a Value,
    rects: &HashMap<String, Rect>,
    bottom_nav_ids: &HashSet<String>,
    cmds: &mut Vec<EditorCommand>,
    ancestors: &mut Vec<&'a Value>,
) {
    if is_touch_target_offender(v, rects, bottom_nav_ids, ancestors) {
        let id = v.get("id").and_then(Value::as_str).unwrap_or_default();
        cmds.push(EditorCommand::UpdateNode {
            node_id: NodeId::new(id.to_string()),
            x: None,
            y: None,
            width: None,
            height: Some(TOUCH_TARGET_HEIGHT),
            name: None,
            fill_hex: None,
            page_id: None,
        });
        if v.get("alignItems").is_none_or(Value::is_null) {
            cmds.push(EditorCommand::SetNodeLayoutProp {
                node_id: NodeId::new(id.to_string()),
                property: "alignItems".to_string(),
                value: LayoutPropValue::Keyword("center".to_string()),
            });
        }
    }

    ancestors.push(v);
    for child in children(v) {
        collect_in_tree(child, rects, bottom_nav_ids, cmds, ancestors);
    }
    ancestors.pop();
}

fn is_touch_target_offender(
    v: &Value,
    rects: &HashMap<String, Rect>,
    bottom_nav_ids: &HashSet<String>,
    ancestors: &[&Value],
) -> bool {
    if v.get("type").and_then(Value::as_str) != Some("frame")
        || !matches!(layout_str(v), Some("horizontal" | "vertical"))
        || !has_visible_solid_fill_or_stroke(v)
        || !has_hugging_height(v)
    {
        return false;
    }

    let Some(id) = v.get("id").and_then(Value::as_str) else {
        return false;
    };
    let Some(rect) = rects.get(id) else {
        return false;
    };
    if !rect.w.is_finite()
        || !rect.h.is_finite()
        || rect.w < MIN_TOUCH_TARGET_WIDTH
        || rect.h >= TOUCH_TARGET_FLOOR
    {
        return false;
    }

    let kids = children(v);
    if !(1..=2).contains(&kids.len())
        || !kids.iter().all(|child| {
            matches!(
                child.get("type").and_then(Value::as_str),
                Some("text" | "icon_font")
            ) && children(child).is_empty()
        })
    {
        return false;
    }

    if is_protected_node(v, bottom_nav_ids)
        || ancestors.iter().any(|ancestor| {
            crate::cleanup::is_status_bar_from_json(ancestor)
                || is_protected_node(ancestor, bottom_nav_ids)
                || layout_str(ancestor) == Some("none")
        })
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

fn has_hugging_height(v: &Value) -> bool {
    match v.get("height") {
        None | Some(Value::Null) => true,
        Some(Value::String(height)) => height == "fit_content",
        _ => false,
    }
}

fn has_visible_solid_fill_or_stroke(v: &Value) -> bool {
    let fill_is_visible = |fill: &Value| visible_solid_fill(fill);
    let own_fill = v
        .get("fill")
        .and_then(Value::as_array)
        .is_some_and(|fills| fills.iter().any(fill_is_visible));
    let stroke_fill = v
        .get("stroke")
        .and_then(Value::as_object)
        .and_then(|stroke| stroke.get("fill"))
        .and_then(Value::as_array)
        .is_some_and(|fills| fills.iter().any(fill_is_visible));
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
    op_util::hex_color::parse_hex_rgba8(color, op_util::hex_color::HexOptions::LENIENT)
        .is_none_or(|rgba| rgba[3] != 0)
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
#[path = "touch_target_floor_tests.rs"]
mod tests;
