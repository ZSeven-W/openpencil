//! Canvas selection semantics + node-drag gesture for the web host.
//!
//! The pure selection transitions live in
//! `op_editor_core::host_drag_transitions` and the drop-policy /
//! preview pipeline in `op_editor_ui::widgets::drag_flow`; this module
//! is the web platform glue (scene refresh + cache invalidation +
//! layout transitions + layer-panel scrolling).

use super::WidgetHost;
use op_editor_core::host_drag_transitions as core_drag;
use op_editor_core::NodeId;
use op_editor_ui::widgets::drag_flow;
use op_editor_ui::widgets::CanvasNodeDragOverlay;
use op_editor_ui::Rect;

const NODE_DRAG_THRESHOLD_PX: f32 = 2.0;

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct NodeDragState {
    pub(in crate::widget_host) last_screen_x: f32,
    pub(in crate::widget_host) last_screen_y: f32,
    pub(in crate::widget_host) press_screen_x: f32,
    pub(in crate::widget_host) press_screen_y: f32,
    pub(in crate::widget_host) moved: bool,
    pub(in crate::widget_host) total_dx: f64,
    pub(in crate::widget_host) total_dy: f64,
    pub(in crate::widget_host) overlay_bounds: Option<Rect>,
}

impl WidgetHost {
    /// Resolve a canvas press from the rendered root-to-deepest hit
    /// path. Plain click selects the solid-outline primary; a second
    /// press inside 400 ms drills exactly one level to the direct child
    /// under the pointer and enters the primary as the sibling scope.
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
            if hit_path.contains(&editing)
                && self.start_active_image_crop_drag(&editing, &hit_path, x, y)
            {
                return true;
            }
            self.exit_image_crop_edit();
        }
        if resolved.is_double && !text_edit_was_active {
            if resolved.selected_crop_is_deepest && self.enter_selected_image_crop_edit() {
                return true;
            }
            if core_drag::jump_to_deepest_text_edit(&mut self.editor_state, &hit_path) {
                self.editor_state.ui.text_edit_input.touch(self.now_ms);
                self.scroll_layer_panel_selection_into_view(viewport_height);
                self.mark_dirty();
                return true;
            }
            if let Some(secondary) = resolved.targets.secondary_under_pointer {
                core_drag::enter_child_scope(
                    &mut self.editor_state,
                    resolved.targets.primary,
                    secondary,
                );
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
        self.mark_dirty();
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

    pub(in crate::widget_host) fn apply_node_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        let drag = self.node_drag?;
        if !drag.moved
            && (x - drag.press_screen_x).abs() <= NODE_DRAG_THRESHOLD_PX
            && (y - drag.press_screen_y).abs() <= NODE_DRAG_THRESHOLD_PX
        {
            return Some(false);
        }
        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        let total_dx = ((x - drag.press_screen_x) / zoom) as f64;
        let total_dy = ((y - drag.press_screen_y) / zoom) as f64;
        if !drag.moved {
            let result = if let Some(allocator) = self.collab_id_allocator.as_mut() {
                core_drag::activate_node_drag_with_allocator(
                    &mut self.editor_state,
                    allocator,
                    self.alt_held,
                    total_dx,
                    total_dy,
                )
            } else {
                Ok(core_drag::activate_node_drag(
                    &mut self.editor_state,
                    &mut self.next_node_id,
                    self.alt_held,
                    total_dx,
                    total_dy,
                ))
            };
            let activation = match result {
                Ok(activation) => activation,
                Err(error) => {
                    // Abandon the gesture: the alt-clone never landed, so
                    // there is nothing to translate on the following moves.
                    self.node_drag = None;
                    self.show_collab_id_error(error);
                    return Some(true);
                }
            };
            if activation.duplicated {
                self.option_drag_source_ids = activation.option_drag_source_ids;
                // The drag snapshot already advanced the revision before the
                // clone and any flex reorder were authored.
                self.scene_cache.invalidate();
                self.mark_dirty();
            }
            if let Some(d) = self.node_drag.as_mut() {
                d.moved = true;
            }
        }

        if let Some(d) = self.node_drag.as_mut() {
            d.total_dx = total_dx;
            d.total_dy = total_dy;
        }
        let dx = (x - drag.last_screen_x) / zoom;
        let dy = (y - drag.last_screen_y) / zoom;
        if dx == 0.0 && dy == 0.0 {
            return Some(false);
        }

        if let Some(drag) = self.node_drag.as_mut() {
            drag.last_screen_x = x;
            drag.last_screen_y = y;
        }
        if self.editor_state.translate_selected(dx as f64, dy as f64) {
            // Incremental scene patch instead of a full serde reconversion per
            // moved pixel (mirrors the native host). A plain node drag only
            // moves absolute-positioned nodes, so translate just those scene
            // nodes in place and defer the full rebuild to release. Skip the
            // fast path when a reconversion is already pending
            // (`editor_state_dirty`).
            if !self.editor_state_dirty {
                let ids = drag_flow::drag_scene_translate_ids(&self.editor_state);
                let _ = self.layout_scene.translate_nodes(&ids, dx, dy);
                // The scene is now patched away from the last cached build while
                // `scene_cache.last` still holds the pre-drag inputs. Invalidate
                // so a later refresh always rebuilds — otherwise a doc returning
                // to the cached value (undo, or dirty flipping mid-drag) would
                // skip the rebuild and leave this stale patch on screen.
                self.scene_cache.invalidate();
            } else {
                self.mark_dirty();
            }
        } else {
            self.editor_state.editor_ui.active_guides.clear();
        }
        if let Some(drag) = self.node_drag {
            self.apply_live_node_drag_preview(&drag);
        }
        Some(true)
    }

    pub(in crate::widget_host) fn release_node_drag(&mut self) -> bool {
        let Some(drag) = self.node_drag.take() else {
            return false;
        };
        self.refresh_layout_scene();
        let before_scene = self.layout_scene.clone();
        let release_overlay = (self.editor_state.selection_count() == 1)
            .then(|| {
                drag.overlay_bounds
                    .map(|bounds| (self.editor_state.selection.anchor.clone(), bounds))
            })
            .flatten();
        let should_commit_drop = self
            .editor_state
            .editor_ui
            .canvas_drop_indicator
            .as_ref()
            .map(|indicator| indicator.target.is_some())
            .unwrap_or(false)
            || self.editor_state.selection_count() != 1;
        self.editor_state.editor_ui.active_guides.clear();
        self.editor_state.editor_ui.canvas_drop_indicator = None;
        if should_commit_drop {
            // The commit result is deliberately dropped (native does the
            // same in both of its release paths): `commit_node_drag`
            // already invalidated the scene cache and marked dirty when it
            // mutated, the unconditional `mark_dirty()` below covers the
            // no-op drop, and the layout transition is chosen from
            // `should_commit_drop` rather than from whether the tree
            // actually changed.
            let _ = self.commit_node_drag(&drag);
        }
        self.option_drag_source_ids.clear();
        self.mark_dirty();
        if should_commit_drop {
            self.start_layout_transition_from_scene(before_scene);
        } else {
            self.refresh_layout_scene();
            if let Some((node_id, bounds)) = release_overlay {
                self.start_layout_transition_from_bounds(&node_id, bounds);
            }
        }
        true
    }

    fn commit_node_drag(&mut self, drag: &NodeDragState) -> bool {
        if !drag.moved {
            return false;
        }
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

    fn update_node_drag_preview(&mut self, drag: &NodeDragState) {
        drag_flow::refresh_drop_indicator(
            &mut self.editor_state,
            &self.layout_scene,
            drag.total_dx,
            drag.total_dy,
            &self.option_drag_source_ids,
        );
    }

    fn apply_live_node_drag_preview(&mut self, drag: &NodeDragState) {
        if self.editor_state.selection_count() != 1 {
            self.update_node_drag_preview(drag);
            return;
        }
        self.refresh_layout_scene();
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
            if let Some(before_scene) = preview.before_scene {
                self.start_layout_transition_from_scene_excluding(before_scene, &id);
            }
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
}
