//! `EditorState` + derived-`LayoutScene` plumbing on
//! `WidgetHostNative` — the lazy scene refresh, layout-transition
//! starts, dirty bookkeeping, and whole-state replacement / import.
//!
//! Split out of the `widget_host.rs` spine to keep it under the repo's
//! 800-line cap.

use super::*;

impl WidgetHostNative {
    /// Rebuild the layout-resolved `LayoutScene` from `editor_state`
    /// if `editor_state_dirty` is set; clear the flag. Cheap no-op
    /// when the scene is already current. The input hit-test + the
    /// paint pass both call this before reading `layout_scene`.
    pub(in crate::widget_host) fn refresh_layout_scene(&mut self) {
        // A runtime font import/removal advances the font-registry generation
        // without dirtying `editor_state`, so watch it directly — otherwise
        // the early-out below skips the rebuild and the open document keeps
        // its stale fallback-font layout. Reuses the same generation the
        // scene cache measures against, so the two stay consistent.
        let font_generation = jian_skia::font_generation();
        let font_changed = font_generation != self.layout_scene_font_generation;
        if self.editor_state_dirty || font_changed {
            let active_page_index = self
                .editor_state
                .ui
                .active_page_index
                .min(self.editor_state.page_count().saturating_sub(1));
            let active_page_changed = active_page_index != self.layout_scene.active_page_index;
            if active_page_changed {
                // A page switch builds a disjoint render tree. Release the
                // previous transition + scene before the loader allocates the
                // new payload and scene so both page trees do not overlap at
                // the switch's peak. Same-page document/font rebuilds retain
                // the old scene until the replacement is ready.
                self.layout_transition = None;
                drop(std::mem::take(&mut self.layout_scene));
            }
            // Only re-derive when the scene inputs (doc / theme / active page /
            // font generation) actually changed — most `editor_state_dirty`
            // marks (hover, scroll, selection, caret drafts, chat streaming)
            // leave them identical, and the scene carries no editor state, so
            // the rebuild would be a no-op.
            if let Some(scene) = self.scene_cache.maybe_rebuild(&self.editor_state) {
                self.layout_scene = scene;
                // A rebuilt scene invalidates the pan bitmap cache
                // (covers the font-generation path, which bypasses
                // `mark_dirty`).
                self.drop_pan_cache();
            }
            self.editor_state_dirty = false;
            self.layout_scene_font_generation = font_generation;
        }
    }

    pub(in crate::widget_host) fn start_layout_transition_from_scene(
        &mut self,
        before: op_editor_ui::layout_scene::LayoutScene,
    ) {
        self.refresh_layout_scene();
        self.layout_transition =
            bookkeeping::transition_between(&before, &self.layout_scene, self.now_ms);
    }

    pub(in crate::widget_host) fn start_layout_transition_from_scene_excluding(
        &mut self,
        before: op_editor_ui::layout_scene::LayoutScene,
        excluded_id: &op_editor_core::NodeId,
    ) {
        self.refresh_layout_scene();
        self.layout_transition = bookkeeping::transition_between_excluding(
            &before,
            &self.layout_scene,
            self.now_ms,
            excluded_id,
        );
    }

    pub(in crate::widget_host) fn start_layout_transition_from_bounds(
        &mut self,
        node_id: &op_editor_core::NodeId,
        bounds: Rect,
    ) {
        self.layout_transition =
            bookkeeping::transition_from_single_bounds(node_id, bounds, self.now_ms);
    }

    /// The layout-resolved render scene for the live `EditorState`.
    /// Rebuilt on demand when the state changed since the last
    /// derive. The `CanvasViewport` paint + the host's canvas
    /// hit-test both read through this.
    pub fn layout_scene(&mut self) -> &op_editor_ui::layout_scene::LayoutScene {
        self.refresh_layout_scene();
        &self.layout_scene
    }

    /// Borrow the canonical state mutably together with the scene snapshot
    /// resolved immediately before that borrow. Background enrichers may use
    /// it to derive one coherent pre-mutation validation view; it must not be
    /// treated as current after a state mutation, and callers must mark the
    /// host dirty when they mutate the state.
    pub fn editor_state_mut_and_layout_scene(
        &mut self,
    ) -> (
        &mut op_editor_core::EditorState,
        &op_editor_ui::layout_scene::LayoutScene,
    ) {
        self.refresh_layout_scene();
        (&mut self.editor_state, &self.layout_scene)
    }

    /// Mark `editor_state` as mutated so the next `refresh_layout_scene()`
    /// re-derives the render scene. Call after any direct mutation of
    /// `self.editor_state`.
    pub(in crate::widget_host) fn mark_dirty(&mut self) {
        self.editor_state_dirty = true;
        // A mutated document / UI invalidates the pan bitmap cache —
        // a blitted frame must never show stale content.
        self.drop_pan_cache();
    }

    /// Test-only: flag the render scene stale after a test mutated
    /// `editor_state` directly through `editor_state_mut()`.
    #[cfg(test)]
    pub(in crate::widget_host) fn mark_paint_dirty_for_test(&mut self) {
        self.editor_state_dirty = true;
        self.drop_pan_cache();
    }

    /// Borrow the canonical-model editor state — the host's single
    /// source of truth.
    pub fn editor_state(&self) -> &op_editor_core::EditorState {
        &self.editor_state
    }

    /// Mutable borrow of the canonical-model editor state. Callers
    /// that mutate through this MUST call `mark_editor_state_dirty()`
    /// afterwards, else the paint snapshot goes stale.
    pub fn editor_state_mut(&mut self) -> &mut op_editor_core::EditorState {
        &mut self.editor_state
    }

    /// Public dirty-flag — desktop-side code that mutates
    /// `editor_state` through `editor_state_mut()` (settings I/O,
    /// `.op` load, chat streaming, model discovery) calls this so the
    /// next paint re-derives the snapshot.
    pub fn mark_editor_state_dirty(&mut self) {
        self.editor_state_dirty = true;
    }

    /// Switch all collaboration-supported creation paths to one
    /// owner-assigned namespace, resuming above ids already in the document.
    pub fn enable_collaboration_ids(
        &mut self,
        namespace: op_editor_core::PeerNamespace,
    ) -> Result<(), op_editor_core::IdAllocError> {
        self.collab_id_allocator = Some(
            op_editor_core::DocumentIdAllocator::namespaced_for_document(
                &self.editor_state.doc,
                namespace,
            )?,
        );
        Ok(())
    }

    /// Return to the unchanged standalone `n{counter}` allocation policy.
    pub fn disable_collaboration_ids(&mut self) {
        self.collab_id_allocator = None;
        if let Ok(next) = op_editor_core::next_sequential_counter(&self.editor_state.doc) {
            self.next_node_id = self.next_node_id.max(next);
        }
    }

    pub fn collaboration_id_next_counter(&self) -> Option<u64> {
        self.collab_id_allocator
            .as_ref()
            .map(op_editor_core::DocumentIdAllocator::next_counter)
    }

    /// Atomically install an already-verified collaboration document.
    ///
    /// Protocol validation and canonical-hash verification happen before this
    /// host seam. The editor performs its own neutral identity validation
    /// before swapping the document, so a rejected install leaves both the
    /// editor and every host cache untouched.
    pub fn install_collaboration_document(
        &mut self,
        document: jian_ops_schema::PenDocument,
        origin: op_editor_core::EditOrigin,
    ) -> Result<op_editor_core::DocumentInstallReport, op_editor_core::DocumentInstallError> {
        let report = self
            .editor_state
            .install_verified_document(document, origin)?;

        // A full snapshot supersedes the previous document lifetime. Remote
        // commits and replay stay within the same lifetime so epoch-guarded
        // async work is not discarded after every collaboration operation.
        if origin == op_editor_core::EditOrigin::Snapshot {
            self.document_epoch = self.document_epoch.wrapping_add(1);
            self.force_rotate_layer_panel_owner();
            if let Some(op_editor_core::DocumentIdAllocator::Namespaced(allocator)) =
                self.collab_id_allocator.as_ref()
            {
                let namespace = allocator.namespace().clone();
                let allocator = op_editor_core::DocumentIdAllocator::namespaced_for_document(
                    &self.editor_state.doc,
                    namespace.clone(),
                )
                .unwrap_or_else(|_| {
                    // A snapshot may legitimately contain this peer's
                    // final u64 id. Keep the session readable and make the
                    // next creation fail with typed exhaustion instead of
                    // rejecting an otherwise valid authoritative snapshot.
                    op_editor_core::DocumentIdAllocator::namespaced(namespace, u64::MAX)
                });
                self.collab_id_allocator = Some(allocator);
            }
        }

        self.layout_transition = None;
        self.scene_cache.invalidate();
        self.editor_state_dirty = true;
        self.drop_pan_cache();
        Ok(report)
    }

    /// The current document epoch — bumped on every whole-document
    /// replacement (Open / New / import), never on save or in-place
    /// edit. Async work captures this at dispatch and re-checks it
    /// before applying, so a result decoded for a since-replaced
    /// document is dropped. See [`Self::document_epoch`] field docs.
    pub fn document_epoch(&self) -> u64 {
        self.document_epoch
    }

    /// Replace the whole editor state (Open / New) and bump the
    /// document epoch. Use this instead of assigning through
    /// `editor_state_mut()` whenever a fresh document supersedes the
    /// current one, so epoch-guarded async work can detect the swap.
    /// (`install_imported_state` is the import-specific analogue and
    /// bumps the epoch itself.)
    pub fn replace_editor_state(&mut self, state: op_editor_core::EditorState) -> bool {
        if !self.collab_allows_user_action(op_editor_core::CollabGateAction::ReplaceDocument) {
            return false;
        }
        self.editor_state = state;
        self.document_epoch = self.document_epoch.wrapping_add(1);
        self.scene_cache.invalidate();
        self.editor_state_dirty = true;
        true
    }

    /// Install a Figma-imported editor state. The worker only parses
    /// into canonical data; layout scene construction stays on the
    /// normal host path so the worker never touches Skia / FontMgr.
    pub fn install_imported_state(&mut self, state: op_editor_core::EditorState) -> bool {
        self.install_imported_state_with_drop_hook(state, || {})
    }

    /// Import-specific replacement with a callback that runs after the old
    /// state and scene finish dropping on the background worker. Desktop uses
    /// this to schedule allocator pressure relief at the correct lifetime
    /// boundary without blocking the UI thread; other native callers keep the
    /// no-op callback above.
    pub fn install_imported_state_with_drop_hook<F>(
        &mut self,
        mut state: op_editor_core::EditorState,
        after_drop: F,
    ) -> bool
    where
        F: FnOnce() + Send + 'static,
    {
        if !self.collab_allows_document_mutation_from(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ExternalAssets,
            ),
            op_editor_core::CollabEditSource::Import,
        ) {
            return false;
        }
        let imported_document_dirty = state.editor_ui.document_dirty;
        let mut preserved = self.editor_state.editor_ui.clone();
        preserved.figma_import_in_progress = false;
        preserved.file_name_display = state.editor_ui.file_name_display.take();
        preserved.preserve_authored_geometry = state.editor_ui.preserve_authored_geometry;
        // Dirty/saved state belongs to the incoming document. The rest of the
        // live shell UI is intentionally retained, but inheriting this flag
        // from the replaced editor would make a saved import appear dirty (or
        // an unsaved import appear clean).
        preserved.document_dirty = imported_document_dirty;
        // The imported document replaces the previous one, so an in-flight
        // clone wizard belongs to a document that no longer exists — drop
        // it. The host's `poll_git_clone_job` then abandons the job (it
        // only binds while a `cloning` form is live). Without this the
        // clone could bind a repo onto the freshly-imported untitled
        // document, which the path-based origin check can't catch (both
        // documents are untitled → the same `None` path).
        preserved.git_panel.clone_form = None;
        state.editor_ui = preserved;

        let old_state = std::mem::replace(&mut self.editor_state, state);
        // Whole-document replacement — bump the epoch so any async
        // work captured against the previous document (e.g. a pending
        // clipboard paste decode) is dropped instead of applied here.
        self.document_epoch = self.document_epoch.wrapping_add(1);
        let old_scene = std::mem::take(&mut self.layout_scene);
        // Take-once slot so the drop work runs exactly once: on the worker
        // when the spawn succeeds, inline (blocking the UI thread briefly)
        // when thread creation fails under FD/memory pressure.
        let work: Box<dyn FnOnce() + Send> = Box::new(move || {
            drop(old_state);
            drop(old_scene);
            after_drop();
        });
        let work = std::sync::Arc::new(std::sync::Mutex::new(Some(work)));
        let worker_slot = std::sync::Arc::clone(&work);
        let spawned = std::thread::Builder::new()
            .name("op-import-drop".into())
            .spawn(move || {
                if let Some(f) = worker_slot.lock().unwrap_or_else(|p| p.into_inner()).take() {
                    f();
                }
            });
        if let Err(err) = spawned {
            eprintln!("[widget-host] failed to spawn op-import-drop worker: {err}");
            if let Some(f) = work.lock().unwrap_or_else(|p| p.into_inner()).take() {
                f();
            }
        }

        // The imported document restarts at revision 0 / page 0, so its
        // LayerPanel row-model-cache key aliases the replaced document's.
        // Rotate the owner here — the single funnel for the Figma-import path
        // (figma_import_session) — so the next owned paint resolve rebuilds
        // instead of serving the previous document's cached rows.
        self.force_rotate_layer_panel_owner();

        // The scene was just taken (left empty) and is rebuilt lazily on the next
        // `refresh_layout_scene`. Invalidate the build cache so that rebuild is
        // NOT skipped even if the imported document happens to match the last
        // build's inputs — otherwise the canvas would stay blank.
        self.scene_cache.invalidate();
        self.editor_state_dirty = true;
        self.arm_missing_fonts_detection();
        true
    }
}

impl op_editor_host_core::collab::CollaborationEditorHost for WidgetHostNative {
    fn editor_state(&self) -> &op_editor_core::EditorState {
        WidgetHostNative::editor_state(self)
    }

    fn editor_state_mut(&mut self) -> &mut op_editor_core::EditorState {
        WidgetHostNative::editor_state_mut(self)
    }

    fn install_collaboration_document(
        &mut self,
        document: jian_ops_schema::PenDocument,
        origin: op_editor_core::EditOrigin,
    ) -> Result<op_editor_core::DocumentInstallReport, op_editor_core::DocumentInstallError> {
        WidgetHostNative::install_collaboration_document(self, document, origin)
    }
}
