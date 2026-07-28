//! Canvas selection semantics (one-level drill on double-click) + the
//! node-drag release commit (auto-layout reorder / cross-container
//! reparenting).
//!
//! The pure selection transitions live in
//! `op_editor_core::host_drag_transitions` and the drop-policy /
//! preview pipeline in `op_editor_ui::widgets::drag_flow`; this module
//! is the native platform glue (scene refresh + cache invalidation +
//! layout transitions + layer-panel scrolling + the hover-probe cache).
//!
//! TS sources: `skia-interaction.ts:1182-1256` (`handleDragEnd`),
//! `:1262-1294` (double-click enter-group) and
//! `op_editor_core::selection_resolve`.

use super::{NodeDragState, WidgetHostNative};
use op_editor_core::host_drag_transitions as core_drag;
use op_editor_core::NodeId;
use op_editor_ui::widgets::drag_flow;
use op_editor_ui::widgets::CanvasNodeDragOverlay;

impl WidgetHostNative {
    /// Select-tool press over a resolved root-to-deepest hit path.
    /// A plain click selects the current level's primary node; a
    /// double-click drills exactly one level to the child under the
    /// pointer. The primary then becomes the entered sibling scope.
    /// This is the same hierarchy Pencil exposes with a solid primary
    /// hover outline and dashed direct-child guides.
    pub(in crate::widget_host) fn apply_canvas_node_press(
        &mut self,
        hit_path: Vec<NodeId>,
        x: f32,
        y: f32,
        text_edit_was_active: bool,
        viewport_height: f32,
    ) -> bool {
        let Some(resolved) = core_drag::resolve_canvas_press(
            &mut self.editor_state,
            &hit_path,
            self.now_ms,
            self.shift_held,
        ) else {
            return false;
        };
        if let Some(editing) = self.editor_state.editor_ui.image_crop_editing.clone() {
            // A crop can be selected directly from the Layer panel while its
            // ancestors remain the canvas depth resolver's primary target.
            // The rendered hit path is authoritative: any descendant hit
            // inside the editing node should pan that node's bitmap.
            if hit_path.contains(&editing)
                && self.start_active_image_crop_drag(&editing, &hit_path, x, y)
            {
                return true;
            }
            // A press on another node exits the dedicated crop editor, then
            // continues through ordinary selection/drag routing.
            self.exit_image_crop_edit();
        }
        if resolved.is_double && !text_edit_was_active {
            if resolved.selected_crop_is_deepest && self.enter_selected_image_crop_edit() {
                return true;
            }
            if let Some(secondary) = resolved.targets.secondary_under_pointer {
                core_drag::enter_child_scope(
                    &mut self.editor_state,
                    resolved.targets.primary,
                    secondary,
                );
                // The native 3 px probe cache would otherwise retain the old
                // level until the mouse moved again.
                self.last_hover_probe = None;
                self.scroll_layer_panel_selection_into_view(viewport_height);
                self.mark_dirty();
                return true;
            }
            if self
                .editor_state
                .start_text_edit(resolved.targets.primary.clone())
            {
                self.editor_state.ui.text_edit_input.touch(self.now_ms);
                self.mark_dirty();
                return true;
            }
        }
        let should_start_drag = core_drag::apply_canvas_press_selection(
            &mut self.editor_state,
            resolved.primary,
            self.shift_held,
            &hit_path,
        );
        self.scroll_layer_panel_selection_into_view(viewport_height);
        if should_start_drag {
            self.start_node_drag(x, y);
        }
        // Native deliberately does not `mark_dirty()` on a plain selection
        // press (web does): the desktop runner already repaints off this
        // `true` return and the selection wash needs no scene rebuild.
        true
    }

    /// Open a fresh node-drag gesture, snapshotting history so the whole
    /// drag undoes as one step.
    pub(in crate::widget_host) fn start_node_drag(&mut self, x: f32, y: f32) {
        self.editor_state.commit_history();
        self.option_drag_source_ids.clear();
        self.node_drag = Some(NodeDragState {
            last_screen_x: x,
            last_screen_y: y,
            press_screen_x: x,
            press_screen_y: y,
            moved: false,
            total_dx: 0.0,
            total_dy: 0.0,
            overlay_bounds: None,
        });
    }

    pub(in crate::widget_host) fn update_node_drag_preview(&mut self, drag: &NodeDragState) {
        drag_flow::refresh_drop_indicator(
            &mut self.editor_state,
            &self.layout_scene,
            drag.total_dx,
            drag.total_dy,
            &self.option_drag_source_ids,
        );
    }

    pub(in crate::widget_host) fn apply_live_node_drag_preview(&mut self, drag: &NodeDragState) {
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::NodeMove) {
            return;
        }
        if self.editor_state.selection_count() != 1 {
            self.update_node_drag_preview(drag);
            return;
        }
        self.refresh_layout_scene();
        let before_scene = self.layout_scene.clone();
        let id = self.editor_state.selection.anchor.clone();
        let Some(preview) = drag_flow::apply_live_drag_preview(
            &mut self.editor_state,
            &self.layout_scene,
            &id,
            drag.total_dx,
            drag.total_dy,
            &self.option_drag_source_ids,
        ) else {
            return;
        };
        if preview.mutated {
            // Drag history advances the document revision when the gesture
            // starts, before this live reorder mutates the tree. Invalidate the
            // revision-keyed scene cache so sibling reflow observes the new
            // order instead of reusing the pre-mutation scene.
            self.scene_cache.invalidate();
            self.mark_dirty();
            self.start_layout_transition_from_scene_excluding(before_scene, &id);
        }
        if let Some(active_drag) = self.node_drag.as_mut() {
            active_drag.overlay_bounds = preview.overlay_bounds;
        }
    }

    pub(in crate::widget_host) fn node_drag_overlay_for_paint(
        &self,
    ) -> Option<CanvasNodeDragOverlay> {
        let drag = self.node_drag?;
        let overlay_bounds = drag.overlay_bounds?;
        let node_id = self.editor_state.selection.anchor.clone();
        Some(CanvasNodeDragOverlay {
            node_id: node_id.as_str().to_string(),
            target_origin_doc: overlay_bounds.origin,
        })
    }

    /// Node-drag release commit: a node dropped into another container
    /// reparents there; a node dropped outside every container becomes
    /// a page root; a child dropped within an auto-layout parent
    /// re-inserts at the midpoint-derived sibling index. Free-layout
    /// children were already translated live.
    pub(in crate::widget_host) fn commit_node_drag(&mut self, drag: &NodeDragState) -> bool {
        if !drag.moved {
            return false;
        }
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::NodeMove) {
            return false;
        }
        // Free-node translations were committed live into the doc;
        // re-derive the scene so the policy reads dropped positions.
        self.refresh_layout_scene();
        let mutated = drag_flow::commit_node_drag(
            &mut self.editor_state,
            &self.layout_scene,
            drag.total_dx,
            drag.total_dy,
            &self.option_drag_source_ids,
        );
        if mutated {
            // The drag snapshot already consumed this gesture's document
            // revision, so the final tree mutation must invalidate the scene
            // cache explicitly.
            self.scene_cache.invalidate();
            self.mark_dirty();
        }
        mutated
    }
}
