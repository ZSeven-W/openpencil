//! Test-only fixture builders for the editor mutator tests.
//!
//! The ported mutator tests need `PenNode` fixtures the same way
//! shell-core's tests used `Node::leaf` / `Node::with_children`.
//! These helpers build canonical-schema nodes with the geometry the
//! tests assert on.

#![cfg(test)]

use crate::state::EditorState;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::node::{
    ContainerProps, FrameNode, GroupNode, PenNodeBase, RectangleNode, TextContent, TextNode,
};
use jian_ops_schema::sizing::SizingBehavior;

/// A rectangle leaf at `(x, y)` sized `w × h`.
pub fn rect(id: &str, name: &str, x: f64, y: f64, w: f64, h: f64) -> PenNode {
    PenNode::Rectangle(RectangleNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.to_string()),
            x: Some(x),
            y: Some(y),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Number(w)),
            height: Some(SizingBehavior::Number(h)),
            ..Default::default()
        },
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// A text leaf at `(x, y)` sized `w × h` with `content`.
pub fn text(id: &str, name: &str, x: f64, y: f64, w: f64, h: f64, content: &str) -> PenNode {
    PenNode::Text(TextNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.to_string()),
            x: Some(x),
            y: Some(y),
            ..Default::default()
        },
        width: Some(SizingBehavior::Number(w)),
        height: Some(SizingBehavior::Number(h)),
        content: TextContent::Plain(content.to_string()),
        font_family: None,
        font_size: None,
        font_weight: None,
        font_style: None,
        letter_spacing: None,
        line_height: None,
        text_align: None,
        text_align_vertical: None,
        text_growth: None,
        underline: None,
        strikethrough: None,
        fill: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// A frame container at `(x, y)` sized `w × h` with `children`.
pub fn frame(
    id: &str,
    name: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    children: Vec<PenNode>,
) -> PenNode {
    PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.to_string()),
            x: Some(x),
            y: Some(y),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Number(w)),
            height: Some(SizingBehavior::Number(h)),
            ..Default::default()
        },
        children: Some(children),
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// An ellipse leaf at `(x, y)` sized `w × h` with no arc geometry.
pub fn ellipse(id: &str, name: &str, x: f64, y: f64, w: f64, h: f64) -> PenNode {
    use jian_ops_schema::node::EllipseNode;
    PenNode::Ellipse(EllipseNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.to_string()),
            x: Some(x),
            y: Some(y),
            ..Default::default()
        },
        width: Some(SizingBehavior::Number(w)),
        height: Some(SizingBehavior::Number(h)),
        corner_radius: None,
        inner_radius: None,
        start_angle: None,
        sweep_angle: None,
        fill: None,
        stroke: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// A bounds-less group container holding `children` — size derives
/// from the children.
pub fn group(id: &str, name: &str, children: Vec<PenNode>) -> PenNode {
    PenNode::Group(GroupNode {
        base: PenNodeBase {
            id: id.to_string(),
            name: Some(name.to_string()),
            ..Default::default()
        },
        container: ContainerProps::default(),
        children: Some(children),
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// An `EditorState` over a single-page document with `roots` as the
/// top-level node tree.
pub fn state_with(roots: Vec<PenNode>) -> EditorState {
    let mut s = EditorState::new();
    s.doc.children = roots;
    s
}

/// The shell-core `Document::sample()` shape: a bounded Frame `n10`
/// containing a Text `n11` and a Group `n12` (`n13` rect + `n14`
/// text). Selection anchors on `n11`.
pub fn sample() -> EditorState {
    let title = text("n11", "Title", 60.0, 60.0, 240.0, 28.0, "Hello OpenPencil");
    let button_rect = rect("n13", "Button background", 60.0, 130.0, 180.0, 36.0);
    let button_text = text("n14", "Click me", 76.0, 152.0, 160.0, 16.0, "Click me");
    let button = group("n12", "Button", vec![button_rect, button_text]);
    let f = frame(
        "n10",
        "Frame",
        40.0,
        40.0,
        360.0,
        240.0,
        vec![title, button],
    );
    let mut s = state_with(vec![f]);
    s.set_single_selection(crate::node_id::NodeId::new("n11"));
    s
}
