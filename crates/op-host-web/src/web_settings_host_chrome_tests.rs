use super::{fingerprint, host_owns_chat_from_query};
use op_editor_core::EditorState;

#[test]
fn host_chat_is_an_explicit_embedded_capability() {
    assert!(host_owns_chat_from_query("?embed=vscode&chat=host"));
    for query in [
        "",
        "?chat=host",
        "?embed=vscode",
        "?embed=vscode&chat=local",
    ] {
        assert!(!host_owns_chat_from_query(query), "{query}");
    }
    let mut state = EditorState::new();
    let before = fingerprint(&state);
    state.editor_ui.host_owns_chat = true;
    assert_eq!(
        fingerprint(&state),
        before,
        "the host hint must not persist"
    );
}
