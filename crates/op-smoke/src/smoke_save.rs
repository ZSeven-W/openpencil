//! Benchmark outputs use the production serializer, including work identity.

pub(crate) fn bytes(
    state: &op_editor_core::EditorState,
) -> Result<Vec<u8>, op_host_services::doc_io::DocIoError> {
    op_host_services::doc_io::canonical_document_bytes(
        &state.doc,
        op_pen_loader::EditorMeta::from_state(state),
    )
}

#[cfg(test)]
mod tests {
    use op_editor_core::{EditorState, HomeFamily};

    #[test]
    fn output_keeps_declared_purpose_and_omits_the_private_brief() {
        let mut state = EditorState::starter();
        state.editor_ui.home.work_family = Some(HomeFamily::KnowledgeCards);
        let bytes = super::bytes(&state).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["editorMeta"]["workFamily"], "knowledge");
        let loaded = op_host_services::doc_io::load_editor_state_from_source(
            std::str::from_utf8(&bytes).unwrap(),
            op_editor_core::Locale::ZhCn,
        )
        .unwrap();
        assert_eq!(
            loaded.editor_ui.home.work_family,
            Some(HomeFamily::KnowledgeCards)
        );
        assert_eq!(loaded.doc, state.doc);
        assert!(value["editorMeta"].get("shareRecipe").is_none());
    }
}
