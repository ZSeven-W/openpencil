use jian_ops_schema::node::PenNode;
use op_editor_core::{
    apply_instance_override, resolve_instance_display_node_for_anchor, EditorCommand, EditorState,
    LayoutPropValue, NodeId,
};

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"frame","id":"master","name":"Master","reusable":true,
           "x":0,"y":0,"width":100,"height":100,"children":[
             {"type":"rectangle","id":"surface","name":"Surface",
              "width":80,"height":32,"cornerRadius":4}
           ]},
          {"type":"ref","id":"inst","ref":"master","x":120,"y":0}
        ]}"##,
    )
    .expect("fixture parses")
    .value;
    let mut state = EditorState::from_document(doc);
    state.set_single_selection(NodeId::new("inst__surface"));
    state
}

fn resolved_surface(state: &EditorState) -> (f64, Option<f64>) {
    let display =
        resolve_instance_display_node_for_anchor(&state.doc, &NodeId::new("inst__surface"))
            .expect("virtual instance child resolves");
    assert!(matches!(display, PenNode::Rectangle(_)));
    let value = serde_json::to_value(display).expect("display serializes");
    (
        value
            .get("cornerRadius")
            .and_then(serde_json::Value::as_f64)
            .expect("corner radius"),
        value.get("opacity").and_then(serde_json::Value::as_f64),
    )
}

#[test]
fn virtual_child_scope_preserves_each_history_state() {
    let mut state = state();
    let child = NodeId::new("inst__surface");
    apply_instance_override(&mut state, &child, |state| {
        state.commit_history();
        assert!(state.apply(EditorCommand::SetNodeCornerRadius {
            node_id: child.clone(),
            radius: 8.0,
        }));
        state.commit_history();
        assert!(state.apply(EditorCommand::SetNodeLayoutProp {
            node_id: child.clone(),
            property: "opacity".to_string(),
            value: LayoutPropValue::Number(0.5),
        }));
    })
    .expect("virtual child write scope");

    assert_eq!(resolved_surface(&state), (8.0, Some(0.5)));
    assert!(state.undo());
    assert_eq!(resolved_surface(&state), (8.0, None));
    assert!(state.undo());
    assert_eq!(resolved_surface(&state), (4.0, None));

    assert!(state.redo());
    assert_eq!(resolved_surface(&state), (8.0, None));
    assert!(state.redo());
    assert_eq!(resolved_surface(&state), (8.0, Some(0.5)));
}
