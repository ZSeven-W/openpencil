//! Document / node ingestion entry points on the web `WidgetHost` —
//! the host-side consumers of the browser file-IO glue
//! (`crate::dom_io`). Ports of the native host's
//! `install_imported_state` (minus the background drop thread —
//! wasm32 has no threads) and `paste_figma_nodes`
//! (`op_host_native::widget_host`). Gated behind `codegen` with the
//! rest of the document-pipeline deps (`jian-ops-schema`).

use super::WidgetHost;

impl WidgetHost {
    /// Public viewport-fit wrapper — mirrors the native host's
    /// `fit_content_to_viewport` (`viewport_fit.rs`). The browser
    /// file-IO glue calls it after a document swap so the loaded
    /// content frames within the canvas.
    pub fn fit_content_to_viewport(&mut self, viewport_w: f32, viewport_h: f32) {
        self.zoom_to_fit(viewport_w, viewport_h);
    }

    /// Replace the editor state with an ingested document (Figma
    /// import) while preserving the live chrome state — port of the
    /// native `install_imported_state`. The whole live `editor_ui` is
    /// carried over (theme / locale / agent settings / open panels);
    /// the import-progress flag clears and the imported state's
    /// `file_name_display` + `preserve_authored_geometry` win.
    pub fn install_ingested_state(&mut self, mut state: op_editor_core::EditorState) {
        let mut preserved = self.editor_state.editor_ui.clone();
        preserved.figma_import_in_progress = false;
        preserved.file_name_display = state.editor_ui.file_name_display.take();
        preserved.preserve_authored_geometry = state.editor_ui.preserve_authored_geometry;
        state.editor_ui = preserved;
        self.editor_state = state;
        self.editor_state_dirty = true;
    }

    /// Insert nodes parsed from the Figma clipboard, centred on the
    /// viewport, with fresh ids, batched undo, and the pasted roots
    /// selected — mirrors TS `use-figma-paste.ts:67-100` and the
    /// native `paste_figma_nodes`.
    pub fn paste_figma_nodes(
        &mut self,
        nodes: Vec<jian_ops_schema::node::PenNode>,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        use op_editor_core::PenNodeExt;
        if nodes.is_empty() {
            return false;
        }
        // Union of the incoming roots' own bounds — the paste centres
        // this box on the canvas viewport centre.
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for node in &nodes {
            let b = op_editor_core::own_bounds(node);
            min_x = min_x.min(b.x);
            min_y = min_y.min(b.y);
            max_x = max_x.max(b.x + b.w);
            max_y = max_y.max(b.y + b.h);
        }
        if min_x > max_x {
            min_x = 0.0;
            min_y = 0.0;
            max_x = 0.0;
            max_y = 0.0;
        }
        let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_local = op_editor_ui::Point2D::new(cw / 2.0, ch / 2.0);
        let centre = self.editor_state.viewport.to_document(canvas_local);
        let dx = centre.x as f64 - (min_x + max_x) / 2.0;
        let dy = centre.y as f64 - (min_y + max_y) / 2.0;

        let snap = self.editor_state.snapshot_for_history();
        let mut taken = self.editor_state.collect_node_ids();
        let mut new_ids = Vec::with_capacity(nodes.len());
        for node in &nodes {
            let mut clone = op_editor_core::walkers::deep_clone_with_new_ids(
                node,
                &mut self.next_node_id,
                &mut taken,
            );
            op_editor_core::walkers::translate_subtree(&mut clone, dx, dy);
            new_ids.push(op_editor_core::NodeId::new(clone.base().id.clone()));
            self.editor_state.active_children_mut().push(clone);
        }
        if let Some(anchor) = new_ids.first().cloned() {
            self.editor_state.set_single_selection(anchor);
            for id in new_ids.into_iter().skip(1) {
                self.editor_state.toggle_selection(id);
            }
        }
        self.editor_state.history_push_past(snap);
        self.mark_dirty();
        true
    }
}
