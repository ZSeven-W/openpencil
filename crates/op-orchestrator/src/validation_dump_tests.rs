//! Tests for `validation_dump` — split out to honour the 800-line ceiling.
//!
//! Loaded via `#[path = "validation_dump_tests.rs"] mod tests;` in
//! `validation_dump.rs`, so this stays nested as `validation_dump::tests`.

use super::*;
use jian_ops_schema::node::container::{ContainerProps, CornerRadius, Padding};
use jian_ops_schema::node::text::{TextContent, TextNode};
use jian_ops_schema::node::{AlignItems, FrameNode, JustifyContent, LayoutMode, PenNodeBase};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{PenFill, PenStroke, SolidFillBody, StrokeThickness};
use op_editor_core::EditorState;

// ── helper: build an EditorState with given page children ────────────────

fn state_with_nodes(nodes: Vec<PenNode>) -> EditorState {
    let mut state = EditorState::new();
    state.doc.children = nodes;
    state
}

fn make_frame(id: &str, children: Option<Vec<PenNode>>) -> PenNode {
    PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: id.to_string(),
            ..Default::default()
        },
        container: ContainerProps::default(),
        children,
        image_search_query: None,
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

fn make_text(id: &str, content: &str) -> PenNode {
    PenNode::Text(TextNode {
        base: PenNodeBase {
            id: id.to_string(),
            ..Default::default()
        },
        width: None,
        height: None,
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

// ── count_nodes_in_active_page ────────────────────────────────────────────

#[test]
fn count_empty_page_is_zero() {
    let state = state_with_nodes(vec![]);
    assert_eq!(count_nodes_in_active_page(&state), 0);
}

#[test]
fn count_flat_nodes() {
    let state = state_with_nodes(vec![
        make_frame("f1", None),
        make_frame("f2", None),
        make_text("t1", "hello"),
    ]);
    assert_eq!(count_nodes_in_active_page(&state), 3);
}

#[test]
fn count_nested_nodes() {
    // root frame → 2 children → 1 grandchild each = 1 + 2 + 2 = 5 total
    let child1 = make_frame("c1", Some(vec![make_text("gc1", "a")]));
    let child2 = make_frame("c2", Some(vec![make_text("gc2", "b")]));
    let root = make_frame("root", Some(vec![child1, child2]));
    let state = state_with_nodes(vec![root]);
    assert_eq!(count_nodes_in_active_page(&state), 5);
}

// ── build_node_tree_dump ──────────────────────────────────────────────────

/// Basic id + type fields are present in correct order.
#[test]
fn dump_id_and_type_present() {
    let state = state_with_nodes(vec![make_frame("frame-abc", None)]);
    let dump = build_node_tree_dump(&state);
    // First prop is id="frame-abc", second is type=frame
    assert!(
        dump.contains("id=\"frame-abc\""),
        "missing id in dump: {dump}"
    );
    assert!(dump.contains("type=frame"), "missing type in dump: {dump}");
    // id comes BEFORE type in the line
    let id_pos = dump.find("id=\"frame-abc\"").unwrap();
    let type_pos = dump.find("type=frame").unwrap();
    assert!(id_pos < type_pos, "id should precede type");
}

/// Root nodes have zero indentation; children have 2-space indent.
#[test]
fn dump_indentation() {
    let child = make_text("child-1", "Hello");
    let root = make_frame("root-1", Some(vec![child]));
    let state = state_with_nodes(vec![root]);
    let dump = build_node_tree_dump(&state);
    let mut lines = dump.lines();
    let root_line = lines.next().expect("root line");
    let child_line = lines.next().expect("child line");
    assert!(
        !root_line.starts_with(' '),
        "root should not be indented: {root_line}"
    );
    assert!(
        child_line.starts_with("  "),
        "child should be indented with 2 spaces: {child_line}"
    );
}

/// Width/height as literal numbers format without trailing .0.
#[test]
fn dump_width_height_number() {
    let mut state = state_with_nodes(vec![]);
    state.doc.children = vec![PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: "f".to_string(),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Number(390.0)),
            height: Some(SizingBehavior::Number(844.0)),
            ..Default::default()
        },
        children: None,
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    let dump = build_node_tree_dump(&state);
    assert!(dump.contains("w=390"), "expected w=390 in {dump}");
    assert!(dump.contains("h=844"), "expected h=844 in {dump}");
}

/// Keywords (fit_content / fill_container) are JSON-string quoted.
#[test]
fn dump_width_keyword_quoted() {
    let mut state = state_with_nodes(vec![]);
    state.doc.children = vec![PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: "f".to_string(),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Keyword(SizingKeyword::FitContent)),
            height: Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)),
            ..Default::default()
        },
        children: None,
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    let dump = build_node_tree_dump(&state);
    assert!(
        dump.contains("w=\"fit_content\""),
        "expected w=\"fit_content\" in {dump}"
    );
    assert!(
        dump.contains("h=\"fill_container\""),
        "expected h=\"fill_container\" in {dump}"
    );
}

/// Layout / gap / padding / justify / align present when set.
#[test]
fn dump_layout_props() {
    let mut state = state_with_nodes(vec![]);
    state.doc.children = vec![PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: "f".to_string(),
            ..Default::default()
        },
        container: ContainerProps {
            layout: Some(LayoutMode::Horizontal),
            gap: Some(jian_ops_schema::node::base::NumberOrExpression::Number(
                12.0,
            )),
            padding: Some(Padding::XY([0.0, 24.0])),
            justify_content: Some(JustifyContent::SpaceBetween),
            align_items: Some(AlignItems::Center),
            ..Default::default()
        },
        children: None,
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    let dump = build_node_tree_dump(&state);
    assert!(
        dump.contains("layout=horizontal"),
        "expected layout in {dump}"
    );
    assert!(dump.contains("gap=12"), "expected gap in {dump}");
    assert!(dump.contains("pad=[0,24]"), "expected pad in {dump}");
    assert!(
        dump.contains("justify=space_between"),
        "expected justify in {dump}"
    );
    assert!(dump.contains("align=center"), "expected align in {dump}");
}

/// Corner radius is present when set.
#[test]
fn dump_corner_radius() {
    let mut state = state_with_nodes(vec![]);
    state.doc.children = vec![PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: "f".to_string(),
            ..Default::default()
        },
        container: ContainerProps {
            corner_radius: Some(CornerRadius::Uniform(8.0)),
            ..Default::default()
        },
        children: None,
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    let dump = build_node_tree_dump(&state);
    assert!(dump.contains("cr=8"), "expected cr=8 in {dump}");
}

/// Fill colour is present.
#[test]
fn dump_fill_color() {
    let mut state = state_with_nodes(vec![]);
    state.doc.children = vec![PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: "f".to_string(),
            ..Default::default()
        },
        container: ContainerProps {
            fill: Some(vec![PenFill::Solid(SolidFillBody {
                color: "#FF0000".to_string(),
                explain: None,
                opacity: None,
                blend_mode: None,
            })]),
            ..Default::default()
        },
        children: None,
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    let dump = build_node_tree_dump(&state);
    assert!(dump.contains("fill=\"#FF0000\""), "expected fill in {dump}");
}

/// Stroke colour + width are present.
#[test]
fn dump_stroke() {
    let mut state = state_with_nodes(vec![]);
    state.doc.children = vec![PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: "f".to_string(),
            ..Default::default()
        },
        container: ContainerProps {
            stroke: Some(PenStroke {
                thickness: StrokeThickness::Uniform(2.0),
                align: None,
                join: None,
                cap: None,
                dash_pattern: None,
                dash_offset: None,
                fill: Some(vec![PenFill::Solid(SolidFillBody {
                    color: "#0000FF".to_string(),
                    explain: None,
                    opacity: None,
                    blend_mode: None,
                })]),
            }),
            ..Default::default()
        },
        children: None,
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    let dump = build_node_tree_dump(&state);
    assert!(
        dump.contains("stroke=\"#0000FF\" strokeW=2"),
        "expected stroke in {dump}"
    );
}

/// Text node emits fontSize, fontWeight, lineHeight, textGrowth, textAlign, text fields.
#[test]
fn dump_text_fields() {
    use jian_ops_schema::node::{FontWeight, TextAlign, TextGrowth};
    let mut state = state_with_nodes(vec![]);
    state.doc.children = vec![PenNode::Text(TextNode {
        base: PenNodeBase {
            id: "t1".to_string(),
            ..Default::default()
        },
        width: None,
        height: None,
        content: TextContent::Plain("Hello World".to_string()),
        font_family: None,
        font_size: Some(24.0),
        font_weight: Some(FontWeight::Number(700)),
        font_style: None,
        letter_spacing: None,
        line_height: Some(1.2),
        text_align: Some(TextAlign::Center),
        text_align_vertical: None,
        text_growth: Some(TextGrowth::FixedWidth),
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
    })];
    let dump = build_node_tree_dump(&state);
    assert!(dump.contains("type=text"), "missing type=text in {dump}");
    assert!(dump.contains("fontSize=24"), "missing fontSize in {dump}");
    assert!(
        dump.contains("fontWeight=700"),
        "missing fontWeight in {dump}"
    );
    assert!(
        dump.contains("lineHeight=1.2"),
        "missing lineHeight in {dump}"
    );
    assert!(
        dump.contains("textGrowth=fixed-width"),
        "missing textGrowth in {dump}"
    );
    assert!(
        dump.contains("textAlign=center"),
        "missing textAlign in {dump}"
    );
    assert!(
        dump.contains("text=\"Hello World\""),
        "missing text in {dump}"
    );
}

/// Text content is truncated at 30 characters.
#[test]
fn dump_text_content_truncated_at_30() {
    let long = "A".repeat(50);
    let state = state_with_nodes(vec![make_text("t1", &long)]);
    let dump = build_node_tree_dump(&state);
    // The text= field should contain exactly 30 'A' chars
    let expected_content = "A".repeat(30);
    assert!(
        dump.contains(&format!("text=\"{expected_content}\"")),
        "expected 30-char truncation in {dump}"
    );
}

/// Exact dump string for a simple two-node tree (regression fixture).
#[test]
fn dump_exact_format_simple_tree() {
    // Root frame id="root" type=frame w=390
    // Child text id="lbl" type=text text="Hello"
    let mut state = state_with_nodes(vec![]);
    let child = make_text("lbl", "Hello");
    state.doc.children = vec![PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: "root".to_string(),
            ..Default::default()
        },
        container: ContainerProps {
            width: Some(SizingBehavior::Number(390.0)),
            ..Default::default()
        },
        children: Some(vec![child]),
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    let dump = build_node_tree_dump(&state);
    let expected = "id=\"root\" type=frame w=390\n  id=\"lbl\" type=text text=\"Hello\"";
    assert_eq!(
        dump, expected,
        "dump did not match expected format.\nGot:\n{dump}\nExpected:\n{expected}"
    );
}

/// Opacity is omitted when it equals 1.0 (default), present when different.
#[test]
fn dump_opacity_omitted_at_1() {
    use jian_ops_schema::node::base::NumberOrExpression;
    let mut state = state_with_nodes(vec![]);
    // opacity = 1.0 → omitted
    let base_opaque = PenNodeBase {
        id: "f1".to_string(),
        opacity: Some(NumberOrExpression::Number(1.0)),
        ..Default::default()
    };
    state.doc.children = vec![PenNode::Frame(FrameNode {
        base: base_opaque,
        container: ContainerProps::default(),
        children: None,
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    let dump = build_node_tree_dump(&state);
    assert!(
        !dump.contains("opacity"),
        "opacity=1 should be omitted in {dump}"
    );

    // opacity = 0.5 → present
    let base_half = PenNodeBase {
        id: "f2".to_string(),
        opacity: Some(NumberOrExpression::Number(0.5)),
        ..Default::default()
    };
    state.doc.children = vec![PenNode::Frame(FrameNode {
        base: base_half,
        container: ContainerProps::default(),
        children: None,
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })];
    let dump2 = build_node_tree_dump(&state);
    assert!(
        dump2.contains("opacity=0.5"),
        "opacity=0.5 should appear in {dump2}"
    );
}
