//! Tests for the 改这一页 scope: the prompt names exactly one board, and
//! the fence keeps every other board as it was.

use super::*;

fn deck() -> EditorState {
    let source = r#"{ "version": "1.0.0", "children": [
        { "type": "frame", "id": "s1", "name": "封面", "x": 0, "y": 0, "width": 1920, "height": 1080,
          "children": [ { "type": "text", "id": "t1", "content": "好想法" } ] },
        { "type": "frame", "id": "s2", "name": "价值", "x": 2000, "y": 0, "width": 1920, "height": 1080,
          "children": [ { "type": "text", "id": "t2", "content": "省时间" } ] },
        { "type": "frame", "id": "s3", "name": "流程", "x": 4000, "y": 0, "width": 1920, "height": 1080,
          "children": [ { "type": "text", "id": "t3", "content": "三步" } ] }
    ] }"#;
    let document = jian_ops_schema::load_str(source)
        .expect("parse fixture")
        .value;
    EditorState::from_document(document)
}

fn target(id: &str, index: usize) -> PageEditTarget {
    PageEditTarget {
        board_id: id.into(),
        index,
    }
}

fn text_of(state: &EditorState, board: &str) -> String {
    let node = state
        .active_children()
        .iter()
        .find(|node| node.id_str() == board)
        .expect("board");
    serde_json::to_string(node).expect("serialize")
}

#[test]
fn the_prompt_names_the_bound_board_and_keeps_the_instruction() {
    let state = deck();
    let prompt = page_edit_prompt(&state, &target("s2", 1), "换成暖色").expect("board exists");
    assert!(prompt.contains("page 2"), "{prompt}");
    assert!(prompt.contains("\"价值\""), "{prompt}");
    assert!(prompt.contains("`s2`"), "{prompt}");
    assert!(prompt.ends_with("INSTRUCTION:\n换成暖色"), "{prompt}");
    assert!(
        !prompt.contains("s1") && !prompt.contains("s3"),
        "no other board is named"
    );
}

#[test]
fn a_vanished_board_is_not_named() {
    let state = deck();
    assert!(page_edit_prompt(&state, &target("gone", 0), "改一下").is_none());
}

#[test]
fn edits_inside_the_target_survive_and_other_boards_revert() {
    let mut state = deck();
    let before = state.active_children().to_vec();
    let s1_before = text_of(&state, "s1");
    let s3_before = text_of(&state, "s3");
    // The "turn": edits the target, scribbles on s1, deletes s3, adds a
    // brand-new top-level board.
    {
        let children = state.active_children_mut();
        let edited = jian_ops_schema::load_str(
            r##"{ "version": "1.0.0", "children": [
                { "type": "frame", "id": "s2", "name": "价值", "x": 2000, "y": 0, "width": 1920,
                  "height": 1080, "fill": [{ "type": "solid", "color": "#FF8A3D" }], "children": [] },
                { "type": "frame", "id": "s1", "name": "封面（改）", "x": 0, "y": 0, "width": 10,
                  "height": 10, "children": [] },
                { "type": "frame", "id": "new", "x": 9000, "y": 0, "width": 100, "height": 100,
                  "children": [] }
            ] }"##,
        )
        .expect("parse edit")
        .value
        .children;
        *children = edited;
    }
    let s2_after_turn = text_of(&state, "s2");
    let revision = state.document_revision();

    assert!(restore_other_boards(&mut state, &before, "s2"));
    let ids: Vec<&str> = state.active_children().iter().map(|n| n.id_str()).collect();
    assert_eq!(ids, ["s1", "s2", "s3"], "original order, no stray board");
    assert_eq!(text_of(&state, "s1"), s1_before);
    assert_eq!(text_of(&state, "s3"), s3_before);
    assert_eq!(
        text_of(&state, "s2"),
        s2_after_turn,
        "the target keeps its edit"
    );
    assert!(
        state.document_revision() > revision,
        "the revert is a real change"
    );
}

#[test]
fn a_well_behaved_turn_is_left_alone() {
    let mut state = deck();
    let before = state.active_children().to_vec();
    if let Some(PenNode::Frame(frame)) = state
        .active_children_mut()
        .iter_mut()
        .find(|node| node.id_str() == "s2")
    {
        frame.base.name = Some("价值（暖色）".into());
    }
    let revision = state.document_revision();
    assert!(!restore_other_boards(&mut state, &before, "s2"));
    assert_eq!(state.document_revision(), revision);
}
