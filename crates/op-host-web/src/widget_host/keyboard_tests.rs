use super::WidgetHost;
use op_editor_core::{EditorState, NodeId};
use op_editor_ui::{KeyCode, KeyEvent, KeyLocation, KeyState, KeyValue, Modifiers};

fn seed_text_edit(host: &mut WidgetHost, content: &str) {
    let doc = jian_ops_schema::load_str(&format!(
        r##"{{"version":"0.8.0","children":[
          {{"type":"text","id":"t1","name":"Title","x":0,"y":0,"width":100,"height":24,
           "content":{content:?},"font_size":16,"fills":[{{"type":"solid","color":"#111827"}}]}}
        ]}}"##
    ))
    .expect("fixture JSON parses")
    .value;
    host.editor_state = EditorState::from_document(doc);
    host.editor_state.set_single_selection(NodeId::new("t1"));
    assert!(host.editor_state.start_text_edit(NodeId::new("t1")));
}

#[test]
fn apply_key_unhandled_event_reports_no_change() {
    let mut host = WidgetHost::new();
    host.editor_state_dirty = false;

    let event = KeyEvent {
        key: KeyValue::Char('a'),
        code: KeyCode::KeyA,
        location: KeyLocation::Standard,
        modifiers: Modifiers::empty(),
        state: KeyState::Pressed,
        repeat: false,
        is_composing: false,
    };

    assert!(!host.apply_key(&event));
    assert!(!host.editor_state_dirty);
}

#[test]
fn text_edit_horizontal_arrows_move_caret_and_consume_event() {
    let mut host = WidgetHost::new();
    seed_text_edit(&mut host, "hi");
    host.editor_state_dirty = false;

    assert!(host.apply_text_edit_caret(false));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 1);
    assert!(host.editor_state_dirty);

    host.editor_state_dirty = false;
    assert!(host.apply_text_edit_caret(true));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 2);
    assert!(host.editor_state_dirty);

    host.editor_state_dirty = false;
    assert!(host.apply_text_edit_caret(true));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 2);

    assert!(host.editor_state.text_edit_commit());
    assert!(!host.apply_text_edit_caret(true));
}

#[test]
fn text_edit_vertical_arrows_move_caret_and_consume_event() {
    let mut host = WidgetHost::new();
    seed_text_edit(&mut host, "hello\nworld");
    assert!(host.editor_state.text_edit_set_caret(11, false, 0));
    host.editor_state_dirty = false;

    assert!(host.apply_text_edit_vertical(false));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 5);
    assert!(host.editor_state_dirty);

    host.editor_state_dirty = false;
    assert!(host.apply_text_edit_vertical(true));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 11);
    assert!(host.editor_state_dirty);

    assert!(host.editor_state.text_edit_commit());
    assert!(!host.apply_text_edit_vertical(true));
}

#[test]
fn text_edit_line_edge_jumps_within_current_line() {
    let mut host = WidgetHost::new();
    seed_text_edit(&mut host, "hello\nworld");
    assert!(host.editor_state.text_edit_set_caret(8, false, 0));
    host.editor_state_dirty = false;

    assert!(host.apply_text_edit_line_edge(false));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 6);
    assert!(host.editor_state_dirty);

    host.editor_state_dirty = false;
    assert!(host.apply_text_edit_line_edge(true));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 11);
    assert!(host.editor_state_dirty);

    assert!(host.editor_state.text_edit_commit());
    assert!(!host.apply_text_edit_line_edge(true));
}

#[test]
fn text_edit_enter_inserts_newline_instead_of_committing() {
    let mut host = WidgetHost::new();
    seed_text_edit(&mut host, "hello\nworld");
    host.editor_state_dirty = false;

    assert!(host.apply_send());
    assert_eq!(
        host.editor_state.ui.text_editing,
        Some(NodeId::new("t1")),
        "Enter must keep the text edit session open"
    );
    assert_eq!(
        host.editor_state.text_edit_content(),
        Some("hello\nworld\n")
    );
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 12);
    assert!(host.editor_state_dirty);

    assert!(host.apply_text('!'));
    assert_eq!(
        host.editor_state.text_edit_content(),
        Some("hello\nworld\n!")
    );
}

#[test]
fn text_edit_delete_removes_character_after_caret() {
    let mut host = WidgetHost::new();
    seed_text_edit(&mut host, "abcd");
    assert!(host.editor_state.text_edit_set_caret(1, false, 0));
    host.editor_state_dirty = false;

    assert!(host.apply_delete());

    assert_eq!(host.editor_state.text_edit_content(), Some("acd"));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 1);
    assert!(host.editor_state_dirty);
}
