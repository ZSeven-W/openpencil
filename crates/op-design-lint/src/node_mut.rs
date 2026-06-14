//! Mutable node field accessors — the write-side counterpart to
//! [`crate::node_util`]'s read accessors.
//!
//! `apply_fixes` needs to write typed jian fields back onto a `PenNode`;
//! this module isolates every `&mut PenNode` mutation so `fixes.rs` stays
//! declarative. Each accessor matches the `PenNode` enum and writes the
//! typed field. Accessors that target a field a node kind lacks (e.g.
//! `set_corner_radius` on a `Text`) are silent no-ops returning `false` —
//! `apply_fixes` only routes issues a real detector emitted, so the kind
//! always matches in practice, but the defensive no-op keeps the
//! `FixReport` total-counting honest.

use jian_ops_schema::node::{CornerRadius, Padding, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::PenStroke;
use serde_json::Value;

/// Write a uniform `corner_radius` onto a Frame/Group/Rectangle.
/// Returns whether the field was written.
pub fn set_corner_radius(node: &mut PenNode, radius: f32) -> bool {
    let value = CornerRadius::Uniform(radius as f64);
    match node {
        PenNode::Frame(n) => n.container.corner_radius = Some(value),
        PenNode::Group(n) => n.container.corner_radius = Some(value),
        PenNode::Rectangle(n) => n.container.corner_radius = Some(value),
        _ => return false,
    }
    true
}

/// Write `base.rotation` (degrees). Applies to every node kind.
pub fn set_rotation(node: &mut PenNode, rotation: f64) -> bool {
    base_mut(node).rotation = Some(rotation);
    true
}

/// Clear `effects` on a node kind that carries them (Frame/Group/Rectangle/
/// Path/Text). Returns whether the node kind has an `effects` field.
pub fn clear_effects(node: &mut PenNode) -> bool {
    match node {
        PenNode::Frame(n) => n.container.effects = None,
        PenNode::Group(n) => n.container.effects = None,
        PenNode::Rectangle(n) => n.container.effects = None,
        PenNode::Path(n) => n.effects = None,
        PenNode::Text(n) => n.effects = None,
        _ => return false,
    }
    true
}

/// Write a uniform `padding` onto a Frame/Group/Rectangle.
pub fn set_padding_uniform(node: &mut PenNode, padding: f32) -> bool {
    set_padding(node, Padding::Uniform(padding as f64))
}

/// Write a `[left, top, right, bottom]` `padding` onto a Frame/Group/Rectangle.
pub fn set_padding_ltrb(node: &mut PenNode, ltrb: [f32; 4]) -> bool {
    set_padding(
        node,
        Padding::LtrB([
            ltrb[0] as f64,
            ltrb[1] as f64,
            ltrb[2] as f64,
            ltrb[3] as f64,
        ]),
    )
}

fn set_padding(node: &mut PenNode, value: Padding) -> bool {
    match node {
        PenNode::Frame(n) => n.container.padding = Some(value),
        PenNode::Group(n) => n.container.padding = Some(value),
        PenNode::Rectangle(n) => n.container.padding = Some(value),
        _ => return false,
    }
    true
}

/// Deserialize a `{thickness, fill, …}` JSON object into a [`PenStroke`] and
/// write it onto a Frame/Group/Rectangle/Path. Returns `false` if the JSON
/// does not deserialize as a `PenStroke` or the node kind lacks a stroke.
pub fn set_stroke_from_json(node: &mut PenNode, suggested: &Value) -> bool {
    let stroke: PenStroke = match serde_json::from_value(suggested.clone()) {
        Ok(s) => s,
        Err(_) => return false,
    };
    match node {
        PenNode::Frame(n) => n.container.stroke = Some(stroke),
        PenNode::Group(n) => n.container.stroke = Some(stroke),
        PenNode::Rectangle(n) => n.container.stroke = Some(stroke),
        PenNode::Path(n) => n.stroke = Some(stroke),
        _ => return false,
    }
    true
}

/// Write `font_size` onto a Text node.
pub fn set_font_size(node: &mut PenNode, font_size: f32) -> bool {
    match node {
        PenNode::Text(n) => {
            n.font_size = Some(font_size as f64);
            true
        }
        _ => false,
    }
}

/// Flip a Text node's `height` sizing to the `fit_content` keyword — the
/// fix for `text-explicit-height`.
pub fn set_text_height_fit_content(node: &mut PenNode) -> bool {
    match node {
        PenNode::Text(n) => {
            n.height = Some(SizingBehavior::Keyword(SizingKeyword::FitContent));
            true
        }
        _ => false,
    }
}

/// Remove the child with `node_id` from `parent`'s subtree. Walks the
/// `children` of every container (Frame/Group/Rectangle), `retain`s out the
/// id, and recurses into surviving children. Returns `true` once a node was
/// removed.
pub fn detach_child(parent: &mut PenNode, node_id: &str) -> bool {
    let kids = match children_mut(parent) {
        Some(k) => k,
        None => return false,
    };
    let before = kids.len();
    kids.retain(|child| crate::node_util::node_id(child) != node_id);
    if kids.len() != before {
        return true;
    }
    for child in kids.iter_mut() {
        if detach_child(child, node_id) {
            return true;
        }
    }
    false
}

/// Mutable borrow of a container's `children` vec. `None` for non-container
/// kinds and for containers whose `children` field is unset.
fn children_mut(node: &mut PenNode) -> Option<&mut Vec<PenNode>> {
    match node {
        PenNode::Frame(n) => n.children.as_mut(),
        PenNode::Group(n) => n.children.as_mut(),
        PenNode::Rectangle(n) => n.children.as_mut(),
        _ => None,
    }
}

/// Mutable borrow of any node kind's shared [`PenNodeBase`].
fn base_mut(node: &mut PenNode) -> &mut jian_ops_schema::node::PenNodeBase {
    match node {
        PenNode::Frame(n) => &mut n.base,
        PenNode::Group(n) => &mut n.base,
        PenNode::Rectangle(n) => &mut n.base,
        PenNode::Ellipse(n) => &mut n.base,
        PenNode::Line(n) => &mut n.base,
        PenNode::Polygon(n) => &mut n.base,
        PenNode::Path(n) => &mut n.base,
        PenNode::Text(n) => &mut n.base,
        PenNode::TextInput(n) => &mut n.base,
        PenNode::Image(n) => &mut n.base,
        PenNode::IconFont(n) => &mut n.base,
        PenNode::TextArea(n) => &mut n.base,
        PenNode::Select(n) => &mut n.base,
        PenNode::Switch(n) => &mut n.base,
        PenNode::Checkbox(n) => &mut n.base,
        PenNode::Slider(n) => &mut n.base,
        PenNode::RadioGroup(n) => &mut n.base,
        PenNode::NumberInput(n) => &mut n.base,
        PenNode::Progress(n) => &mut n.base,
        PenNode::Tabs(n) => &mut n.base,
        PenNode::Ref(n) => &mut n.base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    #[test]
    fn set_corner_radius_writes_uniform() {
        let mut n = node(json!({"type": "frame", "id": "f"}));
        assert!(set_corner_radius(&mut n, 8.0));
        match n {
            PenNode::Frame(f) => {
                assert!(
                    matches!(f.container.corner_radius, Some(CornerRadius::Uniform(r)) if r == 8.0)
                );
            }
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn set_corner_radius_on_text_is_noop() {
        let mut n = node(json!({"type": "text", "id": "t", "content": "hi"}));
        assert!(!set_corner_radius(&mut n, 8.0));
    }

    #[test]
    fn set_rotation_writes_base_rotation() {
        let mut n = node(json!({"type": "frame", "id": "f", "rotation": 12}));
        assert!(set_rotation(&mut n, 0.0));
        match n {
            PenNode::Frame(f) => assert_eq!(f.base.rotation, Some(0.0)),
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn clear_effects_sets_none() {
        let mut n = node(json!({
            "type": "frame", "id": "f",
            "effects": [{"type": "shadow", "offsetX": 0, "offsetY": 2, "blur": 4, "spread": 0, "color": "#000"}]
        }));
        assert!(clear_effects(&mut n));
        match n {
            PenNode::Frame(f) => assert!(f.container.effects.is_none()),
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn clear_effects_on_text() {
        let mut n = node(json!({
            "type": "text", "id": "t", "content": "hi",
            "effects": [{"type": "blur", "radius": 4}]
        }));
        assert!(clear_effects(&mut n));
        match n {
            PenNode::Text(t) => assert!(t.effects.is_none()),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn set_padding_uniform_writes_uniform_variant() {
        let mut n = node(json!({"type": "frame", "id": "f"}));
        assert!(set_padding_uniform(&mut n, 16.0));
        match n {
            PenNode::Frame(f) => {
                assert!(matches!(f.container.padding, Some(Padding::Uniform(p)) if p == 16.0));
            }
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn set_padding_ltrb_writes_ltrb_variant() {
        let mut n = node(json!({"type": "frame", "id": "f"}));
        assert!(set_padding_ltrb(&mut n, [0.0, 16.0, 0.0, 16.0]));
        match n {
            PenNode::Frame(f) => {
                assert!(matches!(
                    f.container.padding,
                    Some(Padding::LtrB([0.0, 16.0, 0.0, 16.0]))
                ));
            }
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn set_stroke_from_json_deserializes_object() {
        let mut n = node(json!({"type": "frame", "id": "f"}));
        let suggested = json!({"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]});
        assert!(set_stroke_from_json(&mut n, &suggested));
        match n {
            PenNode::Frame(f) => {
                let stroke = f.container.stroke.expect("stroke set");
                assert!(stroke.fill.is_some());
            }
            _ => panic!("expected frame"),
        }
    }

    #[test]
    fn set_stroke_from_json_rejects_bad_json() {
        let mut n = node(json!({"type": "frame", "id": "f"}));
        assert!(!set_stroke_from_json(&mut n, &json!(5)));
    }

    #[test]
    fn set_font_size_writes_text_font_size() {
        let mut n = node(json!({"type": "text", "id": "t", "content": "hi"}));
        assert!(set_font_size(&mut n, 14.0));
        match n {
            PenNode::Text(t) => assert_eq!(t.font_size, Some(14.0)),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn set_text_height_fit_content_flips_sizing() {
        let mut n = node(json!({"type": "text", "id": "t", "content": "hi", "height": 40}));
        assert!(set_text_height_fit_content(&mut n));
        match n {
            PenNode::Text(t) => assert!(matches!(
                t.height,
                Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
            )),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn detach_child_removes_direct_child() {
        let mut parent = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "text", "id": "keep", "content": "a"},
                {"type": "path", "id": "drop"}
            ]
        }));
        assert!(detach_child(&mut parent, "drop"));
        assert_eq!(crate::node_util::children(&parent).len(), 1);
        assert!(!detach_child(&mut parent, "drop"));
    }

    #[test]
    fn detach_child_recurses_into_descendants() {
        let mut parent = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "mid", "children": [
                    {"type": "path", "id": "deep"}
                ]}
            ]
        }));
        assert!(detach_child(&mut parent, "deep"));
        let mid = &crate::node_util::children(&parent)[0];
        assert_eq!(crate::node_util::children(mid).len(), 0);
    }
}
