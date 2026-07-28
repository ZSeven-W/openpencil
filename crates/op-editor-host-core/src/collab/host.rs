use op_editor_core::{
    DocumentInstallError, DocumentInstallReport, EditOrigin, EditorState, PenDocument,
};

/// Minimal host surface needed by the collaboration actor.
///
/// Native implementations invalidate paint caches in the install method.
/// Headless and web tests can use the direct [`EditorState`] implementation.
pub trait CollaborationEditorHost {
    fn editor_state(&self) -> &EditorState;

    fn editor_state_mut(&mut self) -> &mut EditorState;

    fn install_collaboration_document(
        &mut self,
        document: PenDocument,
        origin: EditOrigin,
    ) -> Result<DocumentInstallReport, DocumentInstallError>;
}

impl CollaborationEditorHost for EditorState {
    fn editor_state(&self) -> &EditorState {
        self
    }

    fn editor_state_mut(&mut self) -> &mut EditorState {
        self
    }

    fn install_collaboration_document(
        &mut self,
        document: PenDocument,
        origin: EditOrigin,
    ) -> Result<DocumentInstallReport, DocumentInstallError> {
        self.install_verified_document(document, origin)
    }
}
