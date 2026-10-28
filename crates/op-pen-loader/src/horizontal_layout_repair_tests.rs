//! Explicit horizontal layout must resolve stale generated coordinates.

use jian_scene::layout_scene::{LayoutScene, SceneNode};
use op_editor_core::EditorState;
use serde_json::{json, Value};

fn state(root: Value) -> EditorState {
    let doc = serde_json::from_value(json!({"version": "1.0", "children": [root]}))
        .expect("fixture document");
    EditorState::from_document(doc)
}

fn node<'a>(scene: &'a LayoutScene, id: &str) -> &'a SceneNode {
    scene.pages[0].find(id).expect("scene node")
}

fn assert_position(scene: &LayoutScene, id: &str, x: f32, y: f32) {
    let bounds = node(scene, id).bounds;
    assert!(
        (bounds.origin.x - x).abs() < 0.01,
        "{id}: {bounds:?}; expected x={x}"
    );
    assert!(
        (bounds.origin.y - y).abs() < 0.01,
        "{id}: {bounds:?}; expected y={y}"
    );
}

#[test]
fn non_start_rows_apply_justify_padding_gap_and_cross_alignment() {
    for (justify, first_x, second_x) in [
        ("center", 220.0, 340.0),
        ("end", 320.0, 440.0),
        ("space_between", 120.0, 440.0),
        ("space_around", 170.0, 390.0),
    ] {
        let state = state(json!({
            "type": "frame", "id": "row", "x": 100, "y": 50,
            "width": 420, "height": 80, "layout": "horizontal",
            "justifyContent": justify, "alignItems": "center", "padding": [10, 20], "gap": 20,
            "children": [
                {"type": "rectangle", "id": "first", "x": 7, "y": 3, "width": 100, "height": 20},
                {"type": "rectangle", "id": "second", "x": 9, "y": 5, "width": 60, "height": 30}
            ]
        }));
        let authored = serde_json::to_value(&state.doc).expect("authored document");
        let scene = crate::editor_state_to_layout_scene(&state);
        assert_position(&scene, "row", 100.0, 50.0);
        assert_position(&scene, "first", first_x, 80.0);
        assert_position(&scene, "second", second_x, 75.0);
        assert_eq!(
            serde_json::to_value(&state.doc).expect("unchanged document"),
            authored
        );
    }
}

#[test]
fn empty_and_single_child_rows_have_finite_stable_geometry() {
    for (justify, single_x) in [
        ("center", 180.0),
        ("end", 360.0),
        ("space_between", 0.0),
        ("space_around", 180.0),
    ] {
        for children in [
            json!([]),
            json!([{"type": "rectangle", "id": "single", "x": 0, "y": 0, "width": 40, "height": 20}]),
        ] {
            let has_child = !children.as_array().expect("children").is_empty();
            let state = state(json!({
                "type": "frame", "id": "row", "width": 400, "height": 44,
                "layout": "horizontal", "justifyContent": justify, "alignItems": "center",
                "gap": 12, "children": children
            }));
            let scene = crate::editor_state_to_layout_scene(&state);
            assert_position(&scene, "row", 0.0, 0.0);
            assert_eq!(node(&scene, "row").bounds.size.x, 400.0);
            assert_eq!(node(&scene, "row").bounds.size.y, 44.0);
            if has_child {
                assert_position(&scene, "single", single_x, 12.0);
            }
        }
    }
}

#[test]
fn overflowing_rows_keep_the_minimum_gap_without_overlapping_children() {
    for justify in ["center", "end", "space_between", "space_around"] {
        let state = state(json!({
            "type": "frame", "id": "row", "width": 100, "height": 20,
            "layout": "horizontal", "justifyContent": justify, "gap": 10,
            "children": [
                {"type": "rectangle", "id": "first", "x": 0, "y": 0, "width": 80, "height": 12},
                {"type": "rectangle", "id": "second", "x": 0, "y": 0, "width": 60, "height": 12}
            ]
        }));
        let scene = crate::editor_state_to_layout_scene(&state);
        assert_position(&scene, "first", 0.0, 0.0);
        assert_position(&scene, "second", 90.0, 0.0);
        assert_eq!(node(&scene, "row").bounds.size.x, 100.0);
    }
}

#[test]
fn absolute_overlays_and_constrained_children_do_not_consume_distribution_space() {
    let state = state(json!({
        "type": "frame", "id": "row", "width": 400, "height": 40,
        "layout": "horizontal", "justifyContent": "space_between",
        "children": [
            {"type": "rectangle", "id": "first", "x": 0, "y": 0, "width": 80, "height": 12},
            {"type": "rectangle", "id": "overlay", "role": "overlay", "x": 21, "y": 7,
             "width": 350, "height": 10},
            {"type": "rectangle", "id": "anchored", "x": 81, "y": 9, "width": 300, "height": 10,
             "constraints": {"h": "left", "v": "top"}},
            {"type": "rectangle", "id": "second", "x": 0, "y": 0, "width": 40, "height": 12}
        ]
    }));
    let scene = crate::editor_state_to_layout_scene(&state);
    assert_position(&scene, "first", 0.0, 0.0);
    assert_position(&scene, "second", 360.0, 0.0);
    assert_position(&scene, "overlay", 21.0, 7.0);
    assert_position(&scene, "anchored", 81.0, 9.0);
}

#[test]
fn row_uses_repaired_nested_width_and_translates_every_descendant() {
    let state = state(json!({
        "type": "frame", "id": "row", "width": 400, "height": 60,
        "layout": "horizontal", "justifyContent": "space_between", "alignItems": "center",
        "children": [
            {"type": "frame", "id": "brand", "x": 200, "y": 0, "width": 60, "height": 30,
             "layout": "horizontal", "gap": 10, "children": [
                {"type": "rectangle", "id": "wordmark", "x": 0, "y": 0, "width": 100, "height": 10},
                {"type": "rectangle", "id": "version", "x": 0, "y": 0, "width": 40, "height": 10}
             ]},
            {"type": "frame", "id": "switch", "x": 0, "y": 0, "width": 40, "height": 20,
             "layout": "none", "children": [
                {"type": "rectangle", "id": "icon", "x": 3, "y": 5, "width": 8, "height": 8}
             ]}
        ]
    }));
    let scene = crate::editor_state_to_layout_scene(&state);
    assert_eq!(node(&scene, "brand").bounds.size.x, 150.0);
    assert_position(&scene, "brand", 0.0, 15.0);
    assert_position(&scene, "wordmark", 0.0, 15.0);
    assert_position(&scene, "version", 110.0, 15.0);
    assert_position(&scene, "switch", 360.0, 20.0);
    assert_position(&scene, "icon", 363.0, 25.0);
}

#[test]
fn row_keeps_already_solved_fill_container_width() {
    let state = state(json!({
        "type": "frame", "id": "row", "width": 400, "height": 44,
        "layout": "horizontal", "justifyContent": "space_between", "alignItems": "center", "gap": 12,
        "children": [
            {"type": "rectangle", "id": "fill", "width": "fill_container", "height": 20},
            {"type": "rectangle", "id": "switch", "width": 40, "height": 20}
        ]
    }));
    let scene = crate::editor_state_to_layout_scene(&state);
    assert_eq!(node(&scene, "fill").bounds.size.x, 348.0);
    assert_position(&scene, "fill", 0.0, 12.0);
    assert_position(&scene, "switch", 360.0, 12.0);
}

#[test]
fn stale_coordinates_do_not_make_a_fill_child_consume_the_entire_row() {
    let state = state(json!({
        "type": "frame", "id": "row", "width": 400, "height": 44,
        "layout": "horizontal", "justifyContent": "space_between", "alignItems": "center",
        "children": [
            {"type": "rectangle", "id": "fill", "x": 0, "y": 0,
             "width": "fill_container", "height": 20},
            {"type": "rectangle", "id": "switch", "x": 0, "y": 0, "width": 40, "height": 20}
        ]
    }));
    let authored = serde_json::to_value(&state.doc).expect("authored document");
    let scene = crate::editor_state_to_layout_scene(&state);
    assert_eq!(node(&scene, "fill").bounds.size.x, 360.0);
    assert_position(&scene, "fill", 0.0, 12.0);
    assert_position(&scene, "switch", 360.0, 12.0);
    assert_eq!(
        serde_json::to_value(&state.doc).expect("unchanged document"),
        authored
    );
}

#[test]
fn stale_fill_wrapper_measures_nested_text_at_its_remaining_width() {
    let state = state(json!({
        "type": "frame", "id": "row", "width": 400, "height": 300,
        "layout": "horizontal", "justifyContent": "space_between",
        "children": [
            {"type": "frame", "id": "fill", "x": 0, "y": 0,
             "width": "fill_container", "height": "fit_content", "layout": "vertical",
             "children": [{"type": "text", "id": "copy", "width": "fill_container",
                 "height": "fit_content", "textGrowth": "fixed-width", "fontSize": 16,
                 "content": "A longer description wraps within the remaining row width. ".repeat(12)}]},
            {"type": "rectangle", "id": "switch", "x": 0, "y": 0, "width": 40, "height": 20}
        ]
    }));
    let scene = crate::editor_state_to_layout_scene(&state);
    let fill = node(&scene, "fill").bounds;
    let copy = node(&scene, "copy").bounds;
    assert_eq!(fill.size.x, 360.0);
    assert_eq!(copy.size.x, 360.0);
    assert!(
        copy.size.y > 20.0,
        "the text must wrap into multiple lines: {copy:?}"
    );
    assert!(
        fill.size.y >= copy.size.y,
        "the wrapper must contain measured text: {fill:?}, {copy:?}"
    );
    assert_position(&scene, "switch", 360.0, 0.0);
}

#[test]
fn preserve_geometry_mode_bypasses_horizontal_reflow() {
    let state = state(json!({
        "type": "frame", "id": "row", "width": 400, "height": 44,
        "layout": "horizontal", "justifyContent": "space_between",
        "children": [
            {"type": "rectangle", "id": "first", "x": 37, "y": 8, "width": 80, "height": 12},
            {"type": "rectangle", "id": "second", "x": 93, "y": 9, "width": 40, "height": 12}
        ]
    }));
    let scene = crate::pen_document_to_layout_scene_with_geometry_mode(
        &state.doc,
        &std::collections::BTreeMap::new(),
        0,
        true,
    );
    assert_position(&scene, "first", 37.0, 8.0);
    assert_position(&scene, "second", 93.0, 9.0);
}

#[test]
fn hug_height_row_resolves_its_height_before_aligning_stale_children() {
    for (align, short_y) in [("center", 8.0), ("end", 16.0)] {
        let state = state(json!({
            "type": "frame", "id": "row", "width": 400, "height": "fit_content",
            "layout": "horizontal", "justifyContent": "space_between", "alignItems": align,
            "children": [
                {"type": "rectangle", "id": "short", "x": 0, "y": 0, "width": 120, "height": 24},
                {"type": "rectangle", "id": "tall", "x": 0, "y": 0, "width": 40, "height": 40}
            ]
        }));
        let scene = crate::editor_state_to_layout_scene(&state);
        assert_eq!(node(&scene, "row").bounds.size.y, 40.0);
        assert_position(&scene, "short", 0.0, short_y);
        assert_position(&scene, "tall", 360.0, 0.0);
    }
}
