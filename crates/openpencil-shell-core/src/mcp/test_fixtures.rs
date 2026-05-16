//! Test-only `EditorState` fixture builders for the MCP test modules.
//!
//! `op_editor_core::test_support` is `#[cfg(test)]`-private to that
//! crate, so the MCP test modules can't reach it. This module mirrors
//! the handful of canonical-schema node builders the ported MCP tests
//! need, plus an `EditorState`-based `sample()` fixture matching the
//! shape shell-core's old `Document::sample()` produced.

#![cfg(test)]

use std::collections::BTreeMap;

use op_editor_core::EditorState;
use jian_ops_schema::node::{
    ContainerProps, FrameNode, GroupNode, PenNode, PenNodeBase, RectangleNode,
    TextContent, TextNode,
};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::variable::{
    VariableDefinition, VariableKind, VariableScalar, VariableValue,
};

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

/// A text leaf at `(x, y)` sized `w × h`.
pub fn text(
    id: &str,
    name: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    content: &str,
) -> PenNode {
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

/// A bounds-less group container holding `children`.
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
    let f = frame("n10", "Frame", 40.0, 40.0, 360.0, 240.0, vec![title, button]);
    let mut s = state_with(vec![f]);
    s.set_single_selection(op_editor_core::NodeId::new("n11"));
    s
}

/// Insert a scalar variable into the document's variable table.
pub fn add_variable(
    state: &mut EditorState,
    name: &str,
    kind: VariableKind,
    scalar: VariableScalar,
) {
    state
        .doc
        .variables
        .get_or_insert_with(BTreeMap::new)
        .insert(
            name.to_string(),
            VariableDefinition {
                kind,
                value: VariableValue::Scalar(scalar),
            },
        );
}

/// Declare a theme axis with the given ordered value list.
pub fn add_theme_axis(state: &mut EditorState, axis: &str, values: &[&str]) {
    state.doc.themes.get_or_insert_with(BTreeMap::new).insert(
        axis.to_string(),
        values.iter().map(|v| v.to_string()).collect(),
    );
}
