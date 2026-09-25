//! Round-trip and sanitization tests for `editorMeta.importedFrom`.

use crate::{
    apply_editor_meta, apply_editor_meta_or_legacy_fallback, extract_editor_meta,
    write_source_with_editor_meta, EditorMeta,
};
use op_editor_core::EditorState;

fn meta_with(origin: Option<&str>) -> EditorMeta {
    EditorMeta {
        imported_from: origin.map(str::to_string),
        ..EditorMeta::default()
    }
}

#[test]
fn the_import_origin_round_trips_through_a_saved_document() {
    let mut out = Vec::new();
    write_source_with_editor_meta(
        &mut out,
        r#"{"version":"1.0.0","children":[]}"#,
        meta_with(Some("https://acme.example/pricing")),
    )
    .expect("write");
    let written = String::from_utf8(out).expect("utf8");
    assert!(
        written.contains(r#""importedFrom":"https://acme.example/pricing""#),
        "{written}"
    );
    let back = extract_editor_meta(&written).expect("meta");
    assert_eq!(
        back.imported_from.as_deref(),
        Some("https://acme.example/pricing")
    );

    // …and through the editor state a reopen installs.
    let mut state = EditorState::new();
    apply_editor_meta(&mut state, back);
    assert_eq!(
        state.editor_ui.home.imported_from.as_deref(),
        Some("https://acme.example/pricing")
    );
    assert_eq!(
        EditorMeta::from_state(&state).imported_from.as_deref(),
        Some("https://acme.example/pricing")
    );
}

#[test]
fn a_save_never_publishes_query_fragment_or_credentials() {
    let json = serde_json::to_string(&meta_with(Some(
        "https://user:pw@acme.example:8443/p?session=abc#top",
    )))
    .expect("json");
    assert!(
        json.contains(r#""importedFrom":"https://acme.example/p""#),
        "{json}"
    );
    for leaked in ["session", "abc", "user", "pw@", "8443", "#top"] {
        assert!(!json.contains(leaked), "{leaked} leaked into {json}");
    }
}

#[test]
fn a_received_origin_is_sanitized_again_and_junk_reads_as_none() {
    let src = r#"{"editorMeta":{"importedFrom":"HTTPS://Acme.Example/a?t=1"}}"#;
    assert_eq!(
        extract_editor_meta(src).and_then(|meta| meta.imported_from),
        Some("https://acme.example/a".to_string())
    );
    for bad in [
        "5",
        "null",
        "{}",
        r#""file:///etc/passwd""#,
        r#""javascript:alert(1)""#,
        r#""""#,
    ] {
        let src = format!(r#"{{"editorMeta":{{"activePageIndex":0,"importedFrom":{bad}}}}}"#);
        let meta = extract_editor_meta(&src).expect("a bad origin never fails the load");
        assert_eq!(meta.imported_from, None, "{bad}");
    }
}

#[test]
fn a_document_without_an_origin_writes_no_key_and_clears_the_state() {
    let json = serde_json::to_string(&meta_with(None)).expect("json");
    assert!(!json.contains("importedFrom"), "{json}");

    let mut state = EditorState::new();
    state.editor_ui.home.imported_from = Some("https://stale.example".into());
    apply_editor_meta_or_legacy_fallback(&mut state, None);
    assert_eq!(state.editor_ui.home.imported_from, None);

    state.editor_ui.home.imported_from = Some("https://stale.example".into());
    apply_editor_meta(&mut state, EditorMeta::default());
    assert_eq!(state.editor_ui.home.imported_from, None);
}
