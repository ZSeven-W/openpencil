//! Unit tests for `chat_session_launch` — sibling file per the
//! 800-line-per-file ceiling (mirrors `chat_session_launch_selection_tests.rs`).

use super::*;
use op_editor_core::pen_node_ext::PenNodeExt;

fn frame(
    id: &str,
    name: &str,
    children: Vec<jian_ops_schema::node::PenNode>,
) -> jian_ops_schema::node::PenNode {
    let mut node: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": id,
        "name": name,
        "width": 390,
        "height": 120,
        "children": []
    }))
    .expect("frame fixture");
    if let Some(kids) = node.children_mut() {
        *kids = children;
    }
    node
}

#[test]
fn clear_fresh_starter_frame_bumps_document_revision() {
    let mut state = EditorState::new();
    // Install the exact blank starter frame the design classifier
    // recognizes (id "n10", name "Frame", 1200x800, white fill).
    let starter: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "n10",
        "name": "Frame",
        "x": 0,
        "y": 0,
        "width": 1200,
        "height": 800,
        "fill": [{ "type": "solid", "color": "#ffffff" }],
        "children": []
    }))
    .expect("starter frame fixture");
    state.active_children_mut().clear();
    state.active_children_mut().push(starter);
    let revision_before = state.document_revision();

    assert!(
        clear_fresh_starter_frame_for_design(&mut state),
        "the blank starter frame must be recognized and cleared"
    );
    assert!(
        state.active_children().is_empty(),
        "the starter Frame row must be gone after the clear"
    );
    // Regression: the raw `active_children_mut().clear()` must bump the
    // revision, or the layer-panel row cache (keyed on
    // `document_revision()`) keeps painting the deleted "Frame" row.
    assert_ne!(
        state.document_revision(),
        revision_before,
        "clearing the starter frame must advance document_revision"
    );
}

#[test]
fn builtin_design_keyword_with_existing_target_prefers_modify_route() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "screen",
        "Food App Home",
        vec![frame("popular-card", "Bella Napoli Pizzeria", Vec::new())],
    ));
    state.set_single_selection(op_editor_core::NodeId::new("popular-card"));

    assert!(
            should_launch_direct_modify(&state, "修改成饺子"),
            "selected existing design + modify wording should update in place, not start a new orchestrator design"
        );
}
