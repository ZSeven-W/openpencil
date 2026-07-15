use super::*;
use crate::walkers::find_node;
use jian_ops_schema::node::container::{AlignItems, JustifyContent};

const HISTORY_DOC: &str = r##"{
  "version":"1.0.0",
  "children":[
    {"type":"frame","id":"master","name":"Master","reusable":true,
     "x":0,"y":0,"width":100,"height":100,"layout":"vertical",
     "fill":[{"type":"solid","color":"#222222"}],"children":[]},
    {"type":"ref","id":"inst1","ref":"master","x":120,"y":0}
  ]
}"##;

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(HISTORY_DOC)
        .expect("fixture parses")
        .value;
    let mut state = EditorState::from_document(doc);
    state.set_single_selection(NodeId::new("inst1"));
    state
}

fn resolved_instance_alignment(
    state: &EditorState,
) -> (Option<JustifyContent>, Option<AlignItems>) {
    let node = find_node(state.active_children(), &NodeId::new("inst1")).expect("instance");
    let display = resolve_instance_display_node(&state.doc, node).expect("display");
    let PenNode::Frame(frame) = display else {
        panic!("frame display");
    };
    (frame.container.justify_content, frame.container.align_items)
}

fn apply_layout(state: &mut EditorState, node_id: NodeId, property: &str, value: &str) {
    assert!(state.apply(crate::EditorCommand::SetNodeLayoutProp {
        node_id,
        property: property.to_string(),
        value: crate::LayoutPropValue::Keyword(value.to_string()),
    }));
}

fn apply_compound_alignment(state: &mut EditorState) {
    apply_instance_override(state, &NodeId::new("inst1"), |state| {
        let id = state.selection.anchor.clone();
        state.commit_history();
        apply_layout(state, id.clone(), "justifyContent", "center");
        state.commit_history();
        apply_layout(state, id, "alignItems", "end");
    });
}

#[test]
fn history_pushed_inside_scope_is_repaired_to_hold_the_ref() {
    let mut state = state();
    apply_instance_override(&mut state, &NodeId::new("inst1"), |state| {
        state.commit_history();
        state.set_selected_color(true, "#00ff00")
    });
    let snapshot = state.history.past.back().expect("history entry pushed");
    let document = snapshot.doc.materialize();
    let node = find_node(&document.children, &NodeId::new("inst1")).expect("instance in snapshot");
    assert!(
        matches!(node, PenNode::Ref(_)),
        "scope snapshot repaired — undo must restore a Ref, not the display node"
    );
}

#[test]
fn scope_repairs_each_history_snapshot_to_its_own_display_state() {
    let mut state = state();
    apply_compound_alignment(&mut state);

    assert_eq!(state.history.past.len(), 2);
    assert_eq!(
        resolved_instance_alignment(&state),
        (Some(JustifyContent::Center), Some(AlignItems::End))
    );
    assert!(state.undo());
    assert_eq!(
        resolved_instance_alignment(&state),
        (Some(JustifyContent::Center), None)
    );
    assert!(state.undo());
    assert_eq!(resolved_instance_alignment(&state), (None, None));
    assert!(!state.undo());

    assert!(state.redo());
    assert_eq!(
        resolved_instance_alignment(&state),
        (Some(JustifyContent::Center), None)
    );
    assert!(state.redo());
    assert_eq!(
        resolved_instance_alignment(&state),
        (Some(JustifyContent::Center), Some(AlignItems::End))
    );
    assert!(!state.redo());
}

#[test]
fn scope_repairs_new_snapshots_when_history_is_at_capacity() {
    let mut state = state();
    for _ in 0..crate::HISTORY_CAP {
        state.commit_history();
    }
    assert_eq!(state.history.past.len(), crate::HISTORY_CAP);

    apply_compound_alignment(&mut state);

    assert_eq!(state.history.past.len(), crate::HISTORY_CAP);
    assert!(state.undo());
    assert_eq!(
        resolved_instance_alignment(&state),
        (Some(JustifyContent::Center), None)
    );
    assert!(state.undo());
    assert_eq!(resolved_instance_alignment(&state), (None, None));
    assert!(state.redo());
    assert_eq!(
        resolved_instance_alignment(&state),
        (Some(JustifyContent::Center), None)
    );
    assert!(state.redo());
    assert_eq!(
        resolved_instance_alignment(&state),
        (Some(JustifyContent::Center), Some(AlignItems::End))
    );
}

#[test]
fn scope_repairs_pending_history_to_its_captured_display_state() {
    let mut state = state();
    let id = NodeId::new("inst1");
    let scope = state.begin_instance_write(&id).expect("instance scope");
    apply_layout(&mut state, id.clone(), "justifyContent", "center");
    state.ui.pending_color_history = Some(state.snapshot_for_history());
    apply_layout(&mut state, id.clone(), "alignItems", "end");
    assert!(state.finish_instance_write(scope));

    let pending = state
        .ui
        .pending_color_history
        .as_ref()
        .expect("pending snapshot retained");
    let document = pending.doc.materialize();
    let node = find_node(&document.children, &id).expect("instance in pending snapshot");
    let display = resolve_instance_display_node(&document, node).expect("pending display resolves");
    let PenNode::Frame(frame) = display else {
        panic!("frame display");
    };
    assert_eq!(
        frame.container.justify_content,
        Some(JustifyContent::Center)
    );
    assert_eq!(
        frame.container.align_items, None,
        "pending snapshot must not inherit the later live write"
    );
}
