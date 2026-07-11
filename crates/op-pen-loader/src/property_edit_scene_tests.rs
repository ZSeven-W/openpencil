//! End-to-end property-edit probes: panel-level mutators → doc → scene.
//! One test per reported dead-end class (icon color, stroke width, …) so a
//! break anywhere in write→rebuild→scene shows up as a failing layer.

use op_editor_core::{EditorState, NodeId, PropertyFocus};

fn state_with(nodes: serde_json::Value) -> EditorState {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": nodes,
    }))
    .expect("doc");
    EditorState::from_document(doc)
}

fn scene_node<'a>(
    scene: &'a jian_scene::layout_scene::LayoutScene,
    id: &str,
) -> &'a jian_scene::layout_scene::SceneNode {
    fn find<'a>(
        nodes: &'a [jian_scene::layout_scene::SceneNode],
        id: &str,
    ) -> Option<&'a jian_scene::layout_scene::SceneNode> {
        for n in nodes {
            if n.id == id {
                return Some(n);
            }
            if let Some(hit) = find(&n.children, id) {
                return Some(hit);
            }
        }
        None
    }
    scene
        .pages
        .iter()
        .find_map(|page| find(&page.children, id))
        .expect("scene node")
}

#[test]
fn icon_font_fill_edit_reaches_the_scene() {
    let mut state = state_with(serde_json::json!([{
        "type": "frame", "id": "root", "name": "Root", "width": 200, "height": 200,
        "children": [{
            "type": "icon_font", "id": "ic", "name": "home", "iconFontName": "home",
            "width": 24, "height": 24,
            "fill": [{ "type": "solid", "color": "#111111ff" }]
        }]
    }]));
    state.selection.anchor = NodeId::new("ic");
    assert!(
        state.set_selected_color(true, "#ff0000"),
        "icon fill write must land"
    );
    let scene = crate::editor_state_to_layout_scene(&state);
    let icon = scene_node(&scene, "ic");
    let fill = icon.fill.expect("icon scene fill present");
    assert!(
        fill.r > 0.78 && fill.g < 0.24 && fill.b < 0.24,
        "icon scene fill must be the edited red, got {fill:?}"
    );
}

#[test]
fn stroke_width_edit_reaches_the_scene_for_uniform_and_per_side() {
    // Uniform-thickness frame stroke.
    let mut state = state_with(serde_json::json!([{
        "type": "frame", "id": "card", "name": "Card", "width": 200, "height": 100,
        "stroke": { "align": "inside", "thickness": 1,
                     "fill": [{ "type": "solid", "color": "#333333ff" }] }
    }]));
    state.selection.anchor = NodeId::new("card");
    assert!(
        state.commit_property_edit(PropertyFocus::StrokeWidth, 4.0),
        "uniform stroke-width write must land"
    );
    let scene = crate::editor_state_to_layout_scene(&state);
    let card = scene_node(&scene, "card");
    let stroke = card.stroke.as_ref().expect("scene stroke present");
    assert!(
        (stroke.width - 4.0).abs() < 0.01,
        "scene stroke width must be 4, got {}",
        stroke.width
    );

    // Per-side thickness ({bottom:1} divider style, the generated-card shape).
    let mut state = state_with(serde_json::json!([{
        "type": "frame", "id": "row", "name": "Row", "width": 200, "height": 40,
        "stroke": { "align": "inside", "thickness": { "bottom": 1 },
                     "fill": [{ "type": "solid", "color": "#333333ff" }] }
    }]));
    state.selection.anchor = NodeId::new("row");
    assert!(
        state.commit_property_edit(PropertyFocus::StrokeWidth, 3.0),
        "per-side stroke-width write must land"
    );
    let scene = crate::editor_state_to_layout_scene(&state);
    let row = scene_node(&scene, "row");
    let stroke = row.stroke.as_ref().expect("scene stroke present");
    assert!(
        stroke.width >= 2.99 || stroke.sides.is_some(),
        "per-side stroke width edit must be visible in the scene: {stroke:?}"
    );
}

#[test]
fn stroke_color_edit_reaches_the_scene() {
    let mut state = state_with(serde_json::json!([{
        "type": "frame", "id": "card", "name": "Card", "width": 200, "height": 100,
        "stroke": { "align": "inside", "thickness": 1,
                     "fill": [{ "type": "solid", "color": "#333333ff" }] }
    }]));
    state.selection.anchor = NodeId::new("card");
    assert!(state.set_selected_color(false, "#00ff00"));
    let scene = crate::editor_state_to_layout_scene(&state);
    let stroke = scene_node(&scene, "card").stroke.as_ref().expect("stroke");
    assert!(
        stroke.color.g > 0.78 && stroke.color.r < 0.24,
        "scene stroke color must be the edited green: {:?}",
        stroke.color
    );
}

#[test]
fn instance_icon_child_fill_edit_reaches_the_scene() {
    let mut state = state_with(serde_json::json!([
        {
            "type": "frame", "id": "button", "name": "Button", "reusable": true,
            "width": 120, "height": 40,
            "children": [{
                "type": "icon_font", "id": "button-icon", "name": "home",
                "iconFontName": "home", "width": 24, "height": 24,
                "fill": [{ "type": "solid", "color": "#111111ff" }]
            }]
        },
        { "type": "ref", "id": "button-instance", "ref": "button", "x": 200, "y": 80 }
    ]));
    state.selection.anchor = NodeId::new("button-instance__button-icon");
    assert!(
        state.set_selected_color(true, "#ff0000"),
        "virtual instance child fill write must land"
    );
    let scene = crate::editor_state_to_layout_scene(&state);
    let icon = scene_node(&scene, "button-instance__button-icon");
    let fill = icon.fill.expect("instance icon scene fill present");
    assert!(
        fill.r > 0.78 && fill.g < 0.24 && fill.b < 0.24,
        "instance icon scene fill must be the edited red, got {fill:?}"
    );
}
