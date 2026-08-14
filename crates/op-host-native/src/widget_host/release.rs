//! Mouse-release handlers on `WidgetHostNative`.
//!
//! Split out of `input.rs` to respect the per-file line cap. Like the
//! press / cursor-move ladders these are strictly ordered: the first
//! live gesture claims the release, so the `take()` order below decides
//! which drag gets committed. `apply_release_with_viewport` is the full
//! path (marquee + layer-drop + chat snap need the viewport);
//! `apply_release` is the viewport-less variant that drops those.

use super::{DragState, WidgetHostNative};
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// Mouse-release — ends active drag; chat-panel snaps corner.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        if let Some(consumed) = self.release_agent_settings_touch_gesture() {
            let pressed_released = self.release_pressed_feedback();
            return consumed || pressed_released;
        }
        if let Some(consumed) = self.release_touch_panel_gesture() {
            let pressed_released = self.release_pressed_feedback();
            return consumed || pressed_released;
        }
        let pressed_released = self.release_pressed_feedback();
        if self.screen_switcher_release() {
            return true;
        }
        if self.preview_switcher_release() {
            return true;
        }
        // Presenting a deck: the toolbar's own release first, then the
        // board's click-to-advance. Both precede the runtime release below,
        // which a presentation never arms.
        if self.slideshow_toolbar_release() {
            return true;
        }
        if self.slideshow_board_release() {
            return true;
        }
        // Live preview drag → pointer Up into the runtime.
        if self.preview_dispatch_release() {
            return true;
        }
        // The rail's slides tab — a row click frames its board, a row
        // drag reorders the deck. Both resolve here rather than on press
        // so a press that turned out to be a drag is not also a
        // navigation.
        if self.slides_panel_release(viewport_w, viewport_h) {
            return true;
        }
        // Pen owns the release while authoring (TS onMouseUp).
        if self.apply_pen_release() {
            return true;
        }
        // Drop color-picker drag.
        if self.editor_state.ui.color_picker.is_some() {
            self.editor_state.color_picker_set_drag(None);
            self.mark_dirty();
        }
        if self.editor_state.editor_ui.agent_settings_drag.is_some() {
            self.editor_state.editor_ui.agent_settings_drag = None;
            self.mark_dirty();
        }
        if self.panel_resize.take().is_some() {
            return true;
        }
        if self.variables_resize.take().is_some() {
            return true;
        }
        if self.chat_resize.take().is_some() {
            return true;
        }
        if self.rotate_drag.take().is_some() {
            return true;
        }
        if self.handle_drag.take().is_some() {
            return true;
        }
        if self.create_drag.take().is_some() {
            // Switch back to Select for immediate shape refinement.
            self.editor_state.tool = op_editor_core::Tool::Select;
            self.mark_dirty();
            return true;
        }
        if self.finish_image_crop_drag() {
            return true;
        }
        if let Some(drag) = self.node_drag.take() {
            // Drag ended — drop the transient smart-guide lines, then
            // run the drop policy (auto-layout reorder / reparent).
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
            self.mark_dirty();
            return true;
        }
        if self.code_selection_drag.take().is_some() {
            return true;
        }
        if self.image_input_selection_drag.take().is_some() {
            return true;
        }
        if self.chat_input_selection_drag.take().is_some() {
            return true;
        }
        if self.chat_text_selection_drag.take().is_some() {
            return true;
        }
        if self.text_edit_selection_drag.take().is_some() {
            return true;
        }
        if let Some(drag) = self.path_anchor_drag.take() {
            // Push history only when the anchor actually moved.
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
                return true;
            }
            return false;
        }
        if let Some(drag) = self.arc_handle_drag.take() {
            // Commit history only when the arc actually changed.
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
                return true;
            }
            return false;
        }
        if self.design_md_drag.take().is_some() {
            // Position was updated live; release only ends the drag.
            return true;
        }
        if self.component_browser_drag.take().is_some() {
            return true;
        }
        if self.icon_picker_drag.take().is_some() {
            return true;
        }
        if self.image_adjustment_drag.take().is_some() {
            return true;
        }
        if self.effect_radius_drag.take().is_some() {
            return true;
        }
        if let Some(m) = self.marquee_drag.take() {
            self.commit_marquee_selection(m, viewport_w, viewport_h);
            return true;
        }
        if let Some(d) = self.layer_drag.take() {
            return self.commit_layer_drag(d, viewport_w, viewport_h);
        }
        if let Some(d) = self.chat_drag.take() {
            // Snap using the live expanded/collapsed panel size.
            let (panel_w, panel_h) = self.ai_chat_size();
            let center = Point2D::new(d.pos_x + panel_w / 2.0, d.pos_y + panel_h / 2.0);
            let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
            self.editor_state.chat.anchor =
                op_editor_core::ChatAnchor::nearest(center, cx0, cy0, cw, ch);
            self.editor_state.chat.panel_position = None;
            self.mark_dirty();
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging || pressed_released
    }

    /// Viewport-less release variant — drops viewport-bound drags.
    /// Begin a canvas pan directly (middle-mouse press) — bypasses
    /// the tool branch; the shared cursor-move / release paths drive
    /// and end it like any pan drag.
    pub fn apply_pan_press(&mut self, x: f32, y: f32) -> bool {
        self.cancel_native_touch_gestures();
        if self.over_topmost_panel(x, y, self.last_viewport_w, self.last_viewport_h) {
            return true;
        }
        self.drag = Some(DragState {
            last_x: x,
            last_y: y,
        });
        true
    }

    pub fn apply_release(&mut self) -> bool {
        if let Some(consumed) = self.release_agent_settings_touch_gesture() {
            let pressed_released = self.release_pressed_feedback();
            return consumed || pressed_released;
        }
        if let Some(consumed) = self.release_touch_panel_gesture() {
            let pressed_released = self.release_pressed_feedback();
            return consumed || pressed_released;
        }
        let pressed_released = self.release_pressed_feedback();
        // The slides tab needs a viewport to re-derive its row rects; the
        // cached one is what every other viewport-less path here uses, and
        // a row drag left open would strand the deck mid-reorder.
        if self.slides_panel_release(self.last_viewport_w, self.last_viewport_h) {
            return true;
        }
        // Pen owns the release while authoring (TS onMouseUp).
        if self.apply_pen_release() {
            return true;
        }
        if self.panel_resize.take().is_some() {
            return true;
        }
        if self.variables_resize.take().is_some() {
            return true;
        }
        if self.chat_resize.take().is_some() {
            return true;
        }
        if self.rotate_drag.take().is_some() {
            return true;
        }
        if self.handle_drag.take().is_some() {
            return true;
        }
        if self.create_drag.take().is_some() {
            self.editor_state.tool = op_editor_core::Tool::Select;
            self.mark_dirty();
            return true;
        }
        if self.finish_image_crop_drag() {
            return true;
        }
        if let Some(drag) = self.node_drag.take() {
            // Drag ended — drop the transient smart-guide lines, then
            // run the drop policy (auto-layout reorder / reparent).
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
            self.mark_dirty();
            return true;
        }
        if self.code_selection_drag.take().is_some() {
            return true;
        }
        if self.image_input_selection_drag.take().is_some() {
            return true;
        }
        if self.chat_input_selection_drag.take().is_some() {
            return true;
        }
        if self.chat_text_selection_drag.take().is_some() {
            return true;
        }
        if self.text_edit_selection_drag.take().is_some() {
            return true;
        }
        if self.marquee_drag.take().is_some() {
            // No viewport: drop without committing.
            return true;
        }
        if self.layer_drag.take().is_some() {
            // No viewport: drop the candidate.
            return true;
        }
        // Commit path / arc history when the drag actually moved.
        if let Some(drag) = self.path_anchor_drag.take() {
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
            }
            return true;
        }
        if let Some(drag) = self.arc_handle_drag.take() {
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
            }
            return true;
        }
        // Chat drag without viewport — drop it (best effort).
        if self.chat_drag.take().is_some() {
            return true;
        }
        if self.design_md_drag.take().is_some() {
            return true;
        }
        if self.component_browser_drag.take().is_some() {
            return true;
        }
        if self.icon_picker_drag.take().is_some() {
            return true;
        }
        if self.image_adjustment_drag.take().is_some() {
            return true;
        }
        if self.effect_radius_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging || pressed_released
    }
}
