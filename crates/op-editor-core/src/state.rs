//! `EditorState` — OpenPencil's editor runtime state, built on the
//! canonical `.op` document model.
//!
//! This is the strangler-reorg replacement for
//! `openpencil-shell-core::document::Document`. The crucial change
//! (spec §5.2): the node tree is no longer a private `Vec<Page>` — it
//! is `jian_ops_schema::PenDocument`, the one canonical document
//! model. `PenDocument` already carries pages, the root `children`
//! tree, `variables` and `themes`, so there is no second document
//! model anywhere in the editor.
//!
//! `EditorState` wraps that canonical document with the *editor*
//! state that is not part of a `.op` file:
//!
//!   - `selection` — selected node id(s) + anchor (page-scoped).
//!   - `tool` — the active canvas tool.
//!   - `viewport` — infinite-canvas pan / zoom.
//!   - `history` — undo / redo stacks (each entry clones `doc`).
//!   - `ui` — transient draft buffers, focus, the active page index,
//!     and the rebuilt-on-load variable/theme caches.
//!
//! ## Variable / theme split (spec §5.2)
//!
//! shell-core's `VariableTable` mixed persisted data with transient
//! editor state. Here that split is explicit:
//!
//!   - **Persisted** `variables` + `themes` → `EditorState.doc`
//!     (`PenDocument::variables` / `PenDocument::themes`). They
//!     serialize with the `.op` file. They are NOT duplicated on
//!     `EditorState`.
//!   - **Transient** `active_theme` selection + `fill_refs` /
//!     `stroke_refs` resolution caches → `EditorState.ui.variables`
//!     (`UiDraftState::variables`). They are rebuilt on load and
//!     never serialized.
//!
//! ## Scope of this task (4.4)
//!
//! Types only. The mutator `impl`s (`translate_selected`,
//! `delete_selected`, undo/redo push, …) are Task 4.5. A minimal
//! constructor is provided so the type is usable + testable.

use crate::chat::ChatState;
use crate::components::ComponentLibrary;
use crate::editor_ui_state::EditorUiState;
use crate::history::History;
use crate::selection::SelectionState;
use crate::tool::Tool;
use crate::ui_draft::UiDraftState;
use crate::viewport::Viewport;

/// The editor's runtime state — the canonical document plus the
/// editor-only state layered on top of it.
#[derive(Debug, Clone)]
pub struct EditorState {
    /// The canonical `.op` document — node tree + pages + variables +
    /// themes. The single source of truth for everything that
    /// serializes to a file.
    pub doc: jian_ops_schema::PenDocument,
    /// Selected node id(s) + anchor. Page-scoped (the active page
    /// index lives on `ui`).
    pub selection: SelectionState,
    /// The active canvas tool.
    pub tool: Tool,
    /// Infinite-canvas pan / zoom.
    pub viewport: Viewport,
    /// Undo / redo stacks.
    pub history: History,
    /// Cross-action clipboard buffer. Copy / cut fill it; paste
    /// drains it (clones, so repeated paste works). Not part of the
    /// `.op` file — transient editor state.
    pub clipboard: Vec<jian_ops_schema::node::PenNode>,
    /// Transient UI state — draft buffers, focus, active page index,
    /// rebuilt-on-load variable/theme caches.
    pub ui: UiDraftState,
    /// Editor-UI overlay + panel state — the widget-layer toggles, hover
    /// targets, menu / modal open flags and panel metrics. With this
    /// + `chat` + `components`, `EditorState` is a complete state
    ///   superset of shell-core's `Document` (Phase 6 Task 6.1a).
    pub editor_ui: EditorUiState,
    /// AI chat sub-state — message transcript, input draft, panel
    /// anchor, model catalog. Mirrors shell-core's `Document.chat`.
    pub chat: ChatState,
    /// Component library — reusable design-system subtrees. Mirrors
    /// shell-core's `Document.components`.
    pub components: ComponentLibrary,
}

impl EditorState {
    /// Build an editor state around an existing canonical document.
    /// Selection / history / tool / viewport / ui all start fresh —
    /// the transient state is not part of the `.op` file, so a freshly
    /// loaded document always opens with an empty selection, the
    /// Select tool, the identity viewport and page 0 active.
    pub fn from_document(doc: jian_ops_schema::PenDocument) -> Self {
        Self {
            doc,
            selection: SelectionState::empty(),
            tool: Tool::default(),
            viewport: Viewport::IDENTITY,
            history: History::new(),
            clipboard: Vec::new(),
            ui: UiDraftState::new(),
            editor_ui: EditorUiState::new(),
            chat: ChatState::default(),
            components: ComponentLibrary::default(),
        }
    }

    /// Build an editor state around an empty single-page document.
    /// `version` is set to the current `.op` format version literal;
    /// `children` is empty (no nodes) and `pages` is left `None` so
    /// the document uses the default single-page fallback.
    pub fn new() -> Self {
        Self::from_document(empty_document())
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

/// An empty canonical document — `version` set, no nodes, no pages.
fn empty_document() -> jian_ops_schema::PenDocument {
    jian_ops_schema::PenDocument {
        version: "0.8.0".to_string(),
        name: None,
        themes: None,
        variables: None,
        pages: None,
        children: Vec::new(),
        format_version: None,
        id: None,
        app: None,
        routes: None,
        state: None,
        lifecycle: None,
        logic_modules: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_empty_and_quiescent() {
        let s = EditorState::new();
        assert!(s.doc.children.is_empty());
        assert!(s.doc.pages.is_none());
        // Persisted variables/themes live on `doc`, not duplicated.
        assert!(s.doc.variables.is_none());
        assert!(s.doc.themes.is_none());
        assert!(s.selection.is_empty());
        assert_eq!(s.tool, Tool::Select);
        assert_eq!(s.viewport, Viewport::IDENTITY);
        assert!(!s.history.can_undo());
        assert_eq!(s.ui.active_page_index, 0);
    }

    #[test]
    fn from_document_keeps_the_node_tree_but_resets_editor_state() {
        let mut doc = empty_document();
        doc.name = Some("Loaded".to_string());
        let s = EditorState::from_document(doc);
        assert_eq!(s.doc.name.as_deref(), Some("Loaded"));
        // Transient editor state always starts fresh.
        assert!(s.selection.is_empty());
        assert_eq!(s.tool, Tool::Select);
    }

    #[test]
    fn new_state_carries_editor_ui_chat_and_components() {
        let s = EditorState::new();
        // Editor-UI defaults: sidebar open, dark theme, no menus open.
        assert!(s.editor_ui.sidebar_open);
        assert!(!s.editor_ui.file_menu_open);
        assert!(!s.editor_ui.agent_settings_open);
        // Chat starts empty + idle.
        assert!(s.chat.messages.is_empty());
        assert!(s.chat.pending_send.is_none());
        // Component library starts empty.
        assert!(s.components.is_empty());
    }

    #[test]
    fn default_matches_new() {
        let a = EditorState::default();
        let b = EditorState::new();
        assert_eq!(a.doc, b.doc);
        assert_eq!(a.tool, b.tool);
    }
}
