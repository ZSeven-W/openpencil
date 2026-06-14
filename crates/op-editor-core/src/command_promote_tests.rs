//! Tests for the semantic-upgrade migration command
//! (`EditorCommand::PromoteLegacyWidgets`).

use crate::command::EditorCommand;
use crate::state::EditorState;
use crate::walkers::find_node;
use crate::NodeId;
use jian_ops_schema::node::PenNode;

fn id(raw: &str) -> NodeId {
    NodeId::new(raw)
}

/// A single-page document whose only top-level node is an explicitly
/// marked `frame role="input"` carrying a two-way `bind:value` binding
/// and a muted-grey placeholder text child — exactly the shape the
/// promote pass turns into a `text_input`.
fn state_with_marked_input_frame() -> EditorState {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{
          "version": "1.1",
          "formatVersion": "1.1",
          "children": [
            {
              "type": "frame",
              "id": "f1",
              "role": "input",
              "bindings": { "bind:value": "$state.email" },
              "fill": [{ "type": "solid", "color": "#1E1E1E" }],
              "children": [
                {
                  "type": "text",
                  "id": "t1",
                  "content": "Enter email",
                  "fill": [{ "type": "solid", "color": "#9A9A9A" }]
                }
              ]
            }
          ]
        }"##,
    )
    .expect("valid PenDocument fixture");
    EditorState::from_document(doc)
}

#[test]
fn promote_legacy_widgets_upgrades_marked_input_frame_to_text_input() {
    let mut state = state_with_marked_input_frame();

    // Before: the marked node is still a plain Frame.
    assert!(matches!(
        find_node(state.active_children(), &id("f1")),
        Some(PenNode::Frame(_))
    ));

    // The dedicated method reports the promotion count + notes.
    let result = state.promote_legacy_widgets();
    assert_eq!(result.count(), 1);
    assert!(result.changed());
    assert_eq!(result.notes[0].node_id, "f1");
    assert_eq!(result.notes[0].from_role, "input");
    assert_eq!(result.notes[0].to, "text_input");

    // After: the frame is now a first-class TextInput widget node.
    let node = find_node(state.active_children(), &id("f1")).expect("node f1");
    let PenNode::TextInput(ti) = node else {
        panic!("expected f1 to be promoted to a TextInput node");
    };
    // The two-way binding is carried over verbatim.
    assert!(ti
        .bindings
        .as_ref()
        .map(|b| b.contains_key("bind:value"))
        .unwrap_or(false));
    // The muted-grey text child becomes the placeholder; no value.
    assert_eq!(ti.placeholder.as_deref(), Some("Enter email"));
    assert!(ti.value.is_none());
    // The node type now carries the semantics — `role` is cleared.
    assert!(ti.base.role.is_none());
    // Style is carried over from the frame container.
    assert!(ti.fill.is_some());
}

#[test]
fn promote_legacy_widgets_command_is_a_single_undoable_step() {
    let mut state = state_with_marked_input_frame();
    assert!(!state.history.can_undo());

    // Apply via the command surface; it reports changed-or-not.
    assert!(state.apply(EditorCommand::PromoteLegacyWidgets));

    // Promotion landed.
    assert!(matches!(
        find_node(state.active_children(), &id("f1")),
        Some(PenNode::TextInput(_))
    ));
    // Exactly one undo entry was pushed — the whole rewrite is one step.
    assert!(state.history.can_undo());

    // Undo restores the original frame.
    assert!(state.undo());
    assert!(matches!(
        find_node(state.active_children(), &id("f1")),
        Some(PenNode::Frame(_))
    ));
    assert!(!state.history.can_undo());
}

#[test]
fn promote_legacy_widgets_is_a_no_op_when_nothing_is_marked() {
    // A frame with no explicit role marker must NOT be promoted, and a
    // zero-promotion run must NOT push onto the undo stack.
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r#"{
          "version": "1.1",
          "formatVersion": "1.1",
          "children": [
            { "type": "frame", "id": "f1", "name": "Input looking thing", "children": [] }
          ]
        }"#,
    )
    .expect("valid PenDocument fixture");
    let mut state = EditorState::from_document(doc);

    // The command reports no change.
    assert!(!state.apply(EditorCommand::PromoteLegacyWidgets));
    // The frame is untouched.
    assert!(matches!(
        find_node(state.active_children(), &id("f1")),
        Some(PenNode::Frame(_))
    ));
    // No no-op snapshot was pushed onto the undo stack.
    assert!(!state.history.can_undo());
}
