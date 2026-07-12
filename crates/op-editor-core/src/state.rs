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

use crate::chat_sessions::ChatSessions;
use crate::codegen::CodegenState;
use crate::components::ComponentLibrary;
use crate::editor_ui_state::EditorUiState;
use crate::history::History;
use crate::selection::SelectionState;
use crate::tool::Tool;
use crate::ui_draft::UiDraftState;
use crate::viewport::Viewport;
use jian_ops_schema::node::PenNode;

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
    /// Monotonic local document-content revision. Selection, viewport,
    /// hover, panels, and other editor-only state do not increment it.
    pub revision: u64,
    /// Revision captured by the host after a successful load / save /
    /// save-as / new baseline.
    pub saved_revision: u64,
    /// Monotonic allocator behind `revision`. Never rolled back by
    /// undo/restore, so an edit made after undoing past the save point
    /// can never reuse the saved revision value (false-clean guard).
    pub(crate) revision_counter: u64,
    /// Monotonic count of snapshots pushed onto `history.past`.
    /// Instance-write scopes use the delta to identify newly pushed
    /// snapshots even when the 100-entry deque evicts from the front.
    pub(crate) history_push_count: u64,
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
    /// Wrapped in `ChatSessions` to support multiple parallel tabs;
    /// `Deref`/`DerefMut` forward to the active tab, so all existing
    /// `state.chat.*` call sites compile unchanged.
    pub chat: ChatSessions,
    /// Code-generation sub-state — framework selection, phase,
    /// progress, generated code, and pending-action flags. Read by
    /// the Code panel painter; drained by the host codegen session.
    pub codegen: CodegenState,
    /// Component library — reusable design-system subtrees. Mirrors
    /// shell-core's `Document.components`.
    pub components: ComponentLibrary,
    /// UIKit catalogue — the built-in starter kit plus any imported
    /// kits. Read by the Component-Browser panel and instantiated
    /// from there. Transient (not part of the `.op` file).
    pub ui_kits: Vec<crate::uikit::UIKit>,
    /// Saved theme presets — app-level like `ui_kits` (TS
    /// `theme-preset-store.ts` persists them in localStorage; the
    /// desktop host mirrors them in `theme-presets.json`). Not part
    /// of the `.op` file and not undo history.
    pub theme_presets: Vec<crate::theme_presets::ThemePreset>,
    /// Raised by every preset-list mutation; the desktop host drains
    /// it into a persist (TS `persist()` runs inline on each store
    /// write).
    pub theme_presets_dirty: bool,
    /// Tracks which `plan_idx` "owns" each key that was ADDED to
    /// `doc.state` by `MergeAppState` during this editor session.
    /// Keys pre-existing in the loaded document are not registered
    /// here; they are owned by the file and are never overwritten.
    /// Transient — not serialized, reset on document replace.
    pub app_state_owner: std::collections::BTreeMap<String, usize>,
    /// Virtual child currently swapped into an authored Ref slot by
    /// the synchronous instance-write scope. Lets generic mutators
    /// treat the child as editable after the Ref-level lock gate has
    /// already passed. Never serialized or copied into history.
    pub(crate) instance_write_virtual_anchor: Option<crate::NodeId>,
}

impl EditorState {
    /// Build an editor state around an existing canonical document.
    /// Selection / history / tool / viewport / ui all start fresh —
    /// the transient state is not part of the `.op` file, so a freshly
    /// loaded document always opens with an empty selection, the
    /// Select tool, the identity viewport and page 0 active.
    pub fn from_document(doc: jian_ops_schema::PenDocument) -> Self {
        let components = ComponentLibrary::from_document(&doc);
        Self {
            doc,
            selection: SelectionState::empty(),
            tool: Tool::default(),
            viewport: Viewport::IDENTITY,
            history: History::new(),
            revision: 0,
            saved_revision: 0,
            revision_counter: 0,
            history_push_count: 0,
            clipboard: Vec::new(),
            ui: UiDraftState::new(),
            editor_ui: EditorUiState::new(),
            chat: ChatSessions::default(),
            codegen: CodegenState::default(),
            components,
            ui_kits: crate::uikit::builtin_kits(),
            theme_presets: Vec::new(),
            theme_presets_dirty: false,
            app_state_owner: std::collections::BTreeMap::new(),
            instance_write_virtual_anchor: None,
        }
    }

    /// Build an editor state around an empty single-page document.
    /// `version` is set to the current `.op` format version literal;
    /// `children` is empty (no nodes) and `pages` is left `None` so
    /// the document uses the default single-page fallback.
    pub fn new() -> Self {
        Self::from_document(empty_document())
    }

    /// Replace the live document wholesale — a whole-document sync from an
    /// external client (e.g. a TS app pushing its canvas via REST
    /// `/api/mcp/document`, mirroring `setSyncDocument`). All document-derived
    /// transient state is reset the same way [`from_document`](Self::from_document)
    /// initializes it:
    /// - `selection` cleared (old node IDs may be gone),
    /// - `components` rebuilt from the new tree,
    /// - `history` reset — a remote sync is not an undoable local edit, so undo
    ///   must NOT restore the pre-sync document, and
    /// - `ui` draft state reset — it stashes pre-edit snapshots
    ///   (`pending_pen_history` / `pending_color_history` /
    ///   `pending_text_edit_history`) and in-progress edits keyed by old node
    ///   IDs (`pen_in_progress`, `text_editing`, …). Keeping them would let a
    ///   sync that lands mid-edit commit a STALE pre-sync snapshot and resurrect
    ///   the old document via undo, and would point edits at nodes that no
    ///   longer exist.
    ///
    /// Editor chrome (`editor_ui` settings, `chat`, `clipboard`, `ui_kits`,
    /// viewport, tool) is PRESERVED so a sync never wipes the user's settings —
    /// including the live MCP server config — or their view. Within `editor_ui`,
    /// only the document-derived transient bits (hover/context-menu/collapsed
    /// layers/guides/in-progress property edits, all keyed by old node ids) are
    /// cleared via [`EditorUiState::clear_document_derived`].
    pub fn replace_document(&mut self, doc: jian_ops_schema::PenDocument) {
        self.components = ComponentLibrary::from_document(&doc);
        let old_doc = std::mem::replace(&mut self.doc, doc);
        self.selection = SelectionState::empty();
        self.history = History::new();
        self.revision = 0;
        self.saved_revision = 0;
        self.revision_counter = 0;
        self.history_push_count = 0;
        self.ui = UiDraftState::new();
        self.editor_ui.clear_document_derived();
        self.sync_dirty_flag();
        // Generation-run ownership is doc-scoped; clear on whole-doc replace.
        self.app_state_owner.clear();
        self.instance_write_virtual_anchor = None;
        drop_document_after_replace(old_doc);
    }

    /// Like [`Self::replace_document`], but records the pre-replace
    /// state as an undo step instead of resetting history. Web
    /// live-sync uses this after its first pull: an external write
    /// landing in a live editing session (an AI turn, an MCP client)
    /// is a change the user asked for, so Cmd+Z must be able to take
    /// it back. Draft / document-derived transient state is still
    /// cleared exactly like the plain replace (stale-id safety) — the
    /// undo snapshot is captured through the same
    /// [`snapshot_for_history`](Self::snapshot_for_history) path every
    /// local edit uses, so it can't resurrect mid-edit drafts.
    pub fn replace_document_with_undo(&mut self, doc: jian_ops_schema::PenDocument) {
        let snap = self.snapshot_for_history();
        self.components = ComponentLibrary::from_document(&doc);
        let old_doc = std::mem::replace(&mut self.doc, doc);
        self.selection = SelectionState::empty();
        self.history_push_past(snap);
        self.ui = UiDraftState::new();
        self.editor_ui.clear_document_derived();
        // Generation-run ownership is doc-scoped; clear on whole-doc replace.
        self.app_state_owner.clear();
        self.instance_write_virtual_anchor = None;
        self.sync_dirty_flag();
        drop_document_after_replace(old_doc);
    }

    /// Revision of the live document-content state.
    pub fn document_revision(&self) -> u64 {
        self.revision
    }

    /// Revision captured at the last successful save/load baseline.
    pub fn saved_revision(&self) -> u64 {
        self.saved_revision
    }

    /// True when document content changed since the last saved baseline.
    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    /// Mark the current document revision as saved.
    pub fn mark_saved_revision(&mut self) {
        self.saved_revision = self.revision;
        self.sync_dirty_flag();
    }

    /// Record a successful document-content mutation.
    pub fn mark_document_changed(&mut self) {
        // Allocate from the monotonic counter, NOT `revision + 1`: after
        // save -> undo -> new edit, `revision + 1` would collide with the
        // saved revision and report a clean file over divergent content.
        self.revision_counter = self.revision_counter.saturating_add(1);
        self.revision = self.revision_counter;
        self.sync_dirty_flag();
    }

    pub(crate) fn sync_dirty_flag(&mut self) {
        self.editor_ui.document_dirty = self.is_dirty();
    }
}

fn drop_document_after_replace(doc: jian_ops_schema::PenDocument) {
    if document_max_depth(&doc) <= 512 {
        drop(doc);
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::thread::Builder::new()
            .name("op-doc-drop".into())
            .stack_size(512 * 1024 * 1024)
            .spawn(move || drop(doc));
    }
    #[cfg(target_arch = "wasm32")]
    {
        drop(doc);
    }
}

fn document_max_depth(doc: &jian_ops_schema::PenDocument) -> usize {
    let mut max = 0;
    let mut stack: Vec<(&PenNode, usize)> = doc.children.iter().map(|node| (node, 1)).collect();
    if let Some(pages) = doc.pages.as_ref() {
        for page in pages {
            stack.extend(page.children.iter().map(|node| (node, 1)));
        }
    }
    while let Some((node, depth)) = stack.pop() {
        max = max.max(depth);
        if let Some(children) = node_children(node) {
            stack.extend(children.iter().map(|child| (child, depth + 1)));
        }
    }
    max
}

fn node_children(node: &PenNode) -> Option<&[PenNode]> {
    match node {
        PenNode::Frame(n) => n.children.as_deref(),
        PenNode::Group(n) => n.children.as_deref(),
        PenNode::Rectangle(n) => n.children.as_deref(),
        _ => None,
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
        design_md: None,
        conversion: None,
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

    #[test]
    fn active_text_input_prefers_canvas_text_edit_over_other_focus() {
        let mut s = EditorState::new();
        s.ui.property_focus = Some(crate::ui_draft::PropertyFocus::PositionX);
        s.ui.property_input.set_text("property");
        s.chat.focused = true;
        s.chat.input.set_text("chat");
        s.ui.text_editing = Some(crate::NodeId::new("text"));
        s.ui.text_edit_input.set_text("canvas");

        assert_eq!(
            s.active_text_input().map(|input| input.text()),
            Some("canvas")
        );
    }

    #[test]
    fn active_text_input_mut_updates_the_focused_variable_row_input() {
        let mut s = EditorState::new();
        s.editor_ui.variable_row_focus = Some(crate::editor_ui_state::VariableRowFocus::String(0));
        s.editor_ui.variable_row_input.set_text("row");

        s.active_text_input_mut().unwrap().insert_str("!", 0);

        assert_eq!(s.editor_ui.variable_row_input.text(), "row!");
    }

    #[test]
    fn replace_document_swaps_doc_but_preserves_editor_chrome() {
        let mut s = EditorState::new();
        // Mutate preserved chrome state a remote sync must NOT wipe.
        s.editor_ui.sidebar_open = false;
        s.editor_ui.agent_settings.mcp_server.port = 4321;
        s.viewport.pan_x = 123.0;
        s.viewport.pan_y = 456.0;
        // Document-derived transient editor_ui state keyed by old node ids —
        // must be cleared so it can't point at nodes the sync removed.
        s.editor_ui.hovered_layer_id = Some(crate::NodeId::new("old-1"));
        s.editor_ui
            .collapsed_layers
            .insert(crate::NodeId::new("old-2"));
        s.editor_ui.padding_edit_mode_anchor = "old-3".to_string();
        // Set by a prior Figma import — must not carry into the synced doc.
        s.editor_ui.preserve_authored_geometry = true;

        let mut doc = empty_document();
        doc.name = Some("Synced".to_string());
        s.replace_document(doc);

        // Document swapped; doc-derived transient state reset.
        assert_eq!(s.doc.name.as_deref(), Some("Synced"));
        assert!(s.selection.is_empty());
        assert!(!s.history.can_undo());
        // Settings-level editor chrome preserved across the sync.
        assert!(!s.editor_ui.sidebar_open);
        assert_eq!(s.editor_ui.agent_settings.mcp_server.port, 4321);
        assert_eq!(s.viewport.pan_x, 123.0);
        assert_eq!(s.viewport.pan_y, 456.0);
        // Document-derived editor_ui state cleared (no stale node-id refs).
        assert!(s.editor_ui.hovered_layer_id.is_none());
        assert!(s.editor_ui.collapsed_layers.is_empty());
        assert!(s.editor_ui.padding_edit_mode_anchor.is_empty());
        assert!(!s.editor_ui.preserve_authored_geometry);
    }

    #[test]
    fn replace_document_clears_stale_draft_state_so_undo_cannot_resurrect_old_doc() {
        let mut s = EditorState::new();
        // Simulate a sync that lands mid-edit: a pre-edit snapshot of the OLD
        // document is stashed in the draft state, plus an in-progress edit
        // keyed by an old node id and a live property-input draft.
        let mut pre_sync = empty_document();
        pre_sync.name = Some("PRE-SYNC".to_string());
        s.ui.pending_pen_history = Some(crate::history::EditorSnapshot {
            doc: crate::history_snapshot::SharedDoc::capture(&pre_sync, None),
            selection: SelectionState::empty(),
            active_page_index: 0,
            components: crate::history_snapshot::SharedComponents::default(),
            app_state_owner: std::collections::BTreeMap::new(),
            fill_refs: std::collections::HashMap::new(),
            stroke_refs: std::collections::HashMap::new(),
            revision: s.revision,
        });
        s.ui.pen_in_progress = Some(crate::NodeId::new("n7"));
        s.ui.property_input.set_text("stale");
        s.ui.property_input_draft = "stale".to_string();
        s.editor_ui.variables_theme_rename_axis = Some("Mode".to_string());
        s.editor_ui.variables_variant_rename_value = Some("Dark".to_string());
        s.editor_ui.variables_header_input.set_text("stale header");
        s.editor_ui.variable_row_focus = Some(crate::editor_ui_state::VariableRowFocus::String(0));
        s.editor_ui.variable_row_input.set_text("stale row");

        let mut doc = empty_document();
        doc.name = Some("Synced".to_string());
        s.replace_document(doc);

        // The stale pre-sync snapshot is gone — it can never be committed to
        // history and resurrected via undo.
        assert!(s.ui.pending_pen_history.is_none());
        assert!(s.ui.pending_color_history.is_none());
        assert!(s.ui.pending_text_edit_history.is_none());
        assert!(s.ui.pen_in_progress.is_none());
        assert!(s.ui.property_input.text().is_empty());
        assert!(s.ui.property_input_draft.is_empty());
        assert!(s.editor_ui.variables_theme_rename_axis.is_none());
        assert!(s.editor_ui.variables_variant_rename_value.is_none());
        assert!(s.editor_ui.variables_header_input.text().is_empty());
        assert!(s.editor_ui.variable_row_focus.is_none());
        assert!(s.editor_ui.variable_row_input.text().is_empty());
        assert!(!s.history.can_undo() && !s.history.can_redo());
    }

    #[test]
    fn replace_document_with_undo_makes_external_apply_undoable() {
        let mut s = EditorState::new();
        let mut before = empty_document();
        before.name = Some("BEFORE".to_string());
        s.replace_document(before);
        assert!(!s.history.can_undo());

        let mut after = empty_document();
        after.name = Some("AFTER".to_string());
        s.replace_document_with_undo(after);

        // The external apply landed and is one undo step.
        assert_eq!(s.doc.name.as_deref(), Some("AFTER"));
        assert!(s.history.can_undo());
        assert!(s.undo());
        assert_eq!(s.doc.name.as_deref(), Some("BEFORE"));
        assert!(s.redo());
        assert_eq!(s.doc.name.as_deref(), Some("AFTER"));
    }

    #[test]
    fn replace_document_with_undo_still_clears_stale_draft_state() {
        let mut s = EditorState::new();
        s.ui.pen_in_progress = Some(crate::NodeId::new("n7"));
        s.ui.property_input.set_text("stale");
        s.editor_ui.hovered_layer_id = Some(crate::NodeId::new("old-1"));

        let mut doc = empty_document();
        doc.name = Some("Synced".to_string());
        s.replace_document_with_undo(doc);

        // Same stale-id safety as the plain replace — only the history
        // reset is skipped.
        assert!(s.ui.pen_in_progress.is_none());
        assert!(s.ui.property_input.text().is_empty());
        assert!(s.editor_ui.hovered_layer_id.is_none());
        assert!(s.selection.is_empty());
    }
}
