use super::*;

#[test]
fn scoped_copy_and_image_changes_undo_and_redo_as_one_complete_result() {
    let source = r#"{"version":"1.0.0","children":[
      {"type":"frame","id":"poster","width":1080,"height":1440,"children":[
        {"type":"text","id":"title","content":"夜航咖啡节","fontSize":72},
        {"type":"text","id":"date","content":"11月14日19:00","fontSize":24},
        {"type":"image","id":"photo","src":"data:image/png;base64,ORIGINAL","width":200,"height":200}
      ]},
      {"type":"frame","id":"other","width":1080,"height":1440,"children":[
        {"type":"text","id":"protected","content":"其他页保持不变"}
      ]}
    ]}"#;
    let mut state = EditorState::from_document(jian_ops_schema::load_str(source).unwrap().value);
    let before = state.doc.clone();
    let nodes = vec![
        (
            "null".into(),
            serde_json::json!({"type":"text","id":"title","content":"月下咖啡节"}),
        ),
        (
            "null".into(),
            serde_json::json!({"type":"text","id":"date","content":"11月15日20:00"}),
        ),
        (
            "null".into(),
            serde_json::json!({"type":"image","id":"photo","src":"data:image/png;base64,REPLACEMENT"}),
        ),
        (
            "null".into(),
            serde_json::json!({"type":"text","id":"protected","content":"must not apply"}),
        ),
    ];
    let (count, mutated) = apply_design_modification(&mut state, &nodes, &["poster".into()]);
    assert_eq!((count, mutated), (3, true));
    let after = state.doc.clone();
    assert_eq!(state.active_children()[1], before.children[1]);
    assert_eq!(state.history.past.len(), 1);
    assert!(state.undo());
    assert_eq!(
        state.doc, before,
        "one undo restores copy, image and all geometry"
    );
    let future = state.history.future.clone();
    assert_eq!(
        apply_design_modification(&mut state, &nodes, &["missing".into()]),
        (0, false)
    );
    assert_eq!(
        state.history.future, future,
        "rejected writes preserve redo"
    );
    assert!(state.redo());
    assert_eq!(state.doc, after);
}
