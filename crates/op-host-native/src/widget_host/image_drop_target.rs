//! Canvas-side resolution for image files dragged onto the editor.
//!
//! The platform layer knows only "a file is over the window at (x, y)". These
//! helpers turn that into the same answers the pointer paths already produce:
//! the doc point under the cursor, the node that would take the image, and the
//! screen rect to highlight while the file hovers.
//!
//! Every coordinate derivation goes through `canvas_region`, per the host's
//! coordinate invariant — the canvas origin moves with the sidebar.

use super::WidgetHostNative;
use op_editor_core::NodeId;
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// DOC-space point under a LOGICAL window point, or `None` when that
    /// point is not over the canvas (rails, panels, overlays).
    pub fn canvas_doc_point(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<(f64, f64)> {
        if !self.over_canvas(x, y, viewport_w, viewport_h) {
            return None;
        }
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let doc = self
            .editor_state
            .viewport
            .to_document(Point2D::new(x - cx0, y - cy0));
        Some((doc.x as f64, doc.y as f64))
    }

    /// Node an image file dropped at this LOGICAL window point would fill.
    ///
    /// `None` when the point is off-canvas, over dead space, or the editor is
    /// previewing (a running app is not an editing surface).
    pub fn image_drop_target_at(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<NodeId> {
        if self.editor_state.editor_ui.preview.mode {
            return None;
        }
        if !self.over_canvas(x, y, viewport_w, viewport_h) {
            return None;
        }
        // Resolve against the CURRENT tree: a drag can arrive right after an
        // edit, before the next paint would have rebuilt the scene.
        self.refresh_layout_scene();
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let doc_point = self
            .editor_state
            .viewport
            .to_document(Point2D::new(x - cx0, y - cy0));
        let path = self
            .layout_scene
            .node_path_at_doc_point(doc_point, self.editor_state.viewport.zoom)?;
        self.editor_state.resolve_image_drop_target(&path)
    }

    /// SCREEN rect of a node in the LAST RESOLVED scene — the drop-target
    /// ring's geometry, read from the same tree the frame is painting (like
    /// `cursor_over_node`, this does not force a rebuild). `None` when the id
    /// is not in that scene.
    pub fn node_screen_rect(&self, id: &NodeId, viewport_w: f32, viewport_h: f32) -> Option<Rect> {
        let node = self.layout_scene.active_page()?.find(id.as_str())?;
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let vp = &self.editor_state.viewport;
        Some(Rect {
            origin: Point2D::new(
                cx0 + vp.pan_x + node.bounds.origin.x * vp.zoom,
                cy0 + vp.pan_y + node.bounds.origin.y * vp.zoom,
            ),
            size: Point2D::new(node.bounds.size.x * vp.zoom, node.bounds.size.y * vp.zoom),
        })
    }

    /// Record which node a hovering image file would fill. Returns `true`
    /// when the highlight changed and the frame needs a repaint.
    pub fn set_file_drop_target(&mut self, target: Option<NodeId>) -> bool {
        if self.editor_state.editor_ui.file_drop_target == target {
            return false;
        }
        self.editor_state.editor_ui.file_drop_target = target;
        true
    }

    /// Apply a dropped image to `target` as one undoable step, then mark the
    /// document dirty so the scene rebuilds and the save state updates.
    pub fn apply_image_drop(
        &mut self,
        target: &NodeId,
        src: &str,
        original_size: Option<[f32; 2]>,
    ) -> bool {
        if !self.collab_allows_document_mutation_from(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ExternalAssets,
            ),
            op_editor_core::CollabEditSource::Import,
        ) {
            return true;
        }
        if !self
            .editor_state
            .apply_image_drop(target, src, original_size)
        {
            return false;
        }
        self.mark_editor_state_dirty();
        true
    }

    /// Insert a dropped image as a standalone node centred on `centre`
    /// (DOC space), the empty-canvas arm of the same gesture.
    pub fn insert_dropped_image(
        &mut self,
        name: &str,
        src: &str,
        pixel_size: Option<(u32, u32)>,
        centre: (f64, f64),
    ) -> Option<NodeId> {
        if !self.collab_allows_document_mutation_from(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ExternalAssets,
            ),
            op_editor_core::CollabEditSource::Import,
        ) {
            return None;
        }
        let (pixel_width, pixel_height) = pixel_size.unwrap_or((300, 200));
        let id = self.editor_state.insert_image_node_at_doc_point_sized(
            name,
            src,
            pixel_width,
            pixel_height,
            centre,
        )?;
        self.mark_editor_state_dirty();
        Some(id)
    }
}

#[cfg(test)]
#[path = "image_drop_target_tests.rs"]
mod tests;
