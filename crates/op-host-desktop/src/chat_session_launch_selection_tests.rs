//! Selection-biased direct-modify launch tests for builtin / ACP routing.

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

fn state_with_selected_card() -> EditorState {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "screen",
        "Food App Home",
        vec![frame("popular-card", "Bella Napoli Pizzeria", Vec::new())],
    ));
    state.set_single_selection(op_editor_core::NodeId::new("popular-card"));
    state
}

fn state_with_selected_header() -> EditorState {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(frame(
        "screen",
        "Client Dashboard",
        vec![frame("header", "Header", Vec::new())],
    ));
    state.set_single_selection(op_editor_core::NodeId::new("header"));
    state
}

#[test]
fn blank_starter_canvas_never_launches_modify() {
    let state = EditorState::starter();

    assert!(state.selection.is_empty(), "fresh starter has no selection");
    assert!(
        !should_launch_direct_modify(&state, "Luxury webapp for managing barbershop clients"),
        "fresh blank starter canvas has no selected target or real content to modify"
    );
}

#[test]
fn real_content_modify_request_still_launches_modify() {
    let state = state_with_selected_header();

    assert!(
        should_launch_direct_modify(&state, "change the header color to blue"),
        "selected real content + clear edit request should still update in place"
    );
}

#[test]
fn selection_with_keywordless_instruction_launches_direct_modify() {
    let state = state_with_selected_card();

    assert!(
        should_launch_direct_modify(&state, "给它加一个边框"),
        "selected existing design + keyword-less edit wording should update in place"
    );
}

#[test]
fn selection_does_not_hijack_whole_new_screen_or_chat() {
    let state = state_with_selected_card();

    assert!(
        !should_launch_direct_modify(&state, "重新画一个首页"),
        "whole-screen draw request must keep the new-design route"
    );
    assert!(
        !should_launch_direct_modify(&state, "这是什么字体"),
        "plain chat questions must not become direct modify requests"
    );
}

#[test]
fn no_selection_modify_keyword_behavior_is_unchanged() {
    let mut state = state_with_selected_card();
    state.clear_selection();

    assert!(
        should_launch_direct_modify(&state, "修改成饺子"),
        "explicit modify wording should still launch direct modify without a selection"
    );
}
