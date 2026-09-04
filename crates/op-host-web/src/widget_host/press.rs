//! Web press handlers split from `widget_host.rs`; mirrors the
//! native press/click split and keeps `EditorState` as source of truth.
//! `apply_click` lives in `click.rs`, the StatusBar / overlay rect
//! helpers in `overlay_rects.rs`, and the per-overlay press
//! dispatchers in their own sibling modules (mirroring the native
//! host's layout) so this file stays under the 800-line cap.
//!
//! `apply_press` itself is now just the tier spine: the tier bodies live
//! in the `press_*_tiers.rs` siblings and `press_ctx.rs` carries the
//! per-event state they share.
use op_editor_ui::widgets::press_flow::{
    self, LayerContextMenuPress, LayerContextStep, PropertyOverlayPress,
};
use op_editor_ui::Point2D;

/// Pointer id the mouse-driven preview wrappers dispatch under (R4 keeps
/// real ids available for pointer-event-capable callers through `_id`
/// variants added alongside these seams).
pub(in crate::widget_host) const LEGACY_WEB_PREVIEW_POINTER_ID: u32 = 1;

use super::press_ctx::PressCtx;
use super::WidgetHost;

impl WidgetHost {
    /// Right-click handler — opens the LayerPanel context menu on
    /// a layer or page row.
    pub fn apply_right_press(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        self.commit_variable_row_focus_if_any();
        self.close_image_popovers_for_higher_overlay();
        if self.over_topmost_panel(x, y, viewport_w, viewport_h) {
            return true;
        }
        // The model-picker card may hang over the LayerPanel. Keep a
        // secondary press on that visible overlay from opening the covered
        // layer/page context menu underneath it.
        if self
            .chat_model_picker_rect(viewport_w, viewport_h)
            .is_some_and(|rect| rect.contains(Point2D::new(x, y)))
        {
            return true;
        }
        if self.try_open_path_anchor_menu(x, y, viewport_w, viewport_h) {
            return true;
        }
        if !self.editor_state.editor_ui.sidebar_open {
            return self.blur_text_inputs_on_blank_press();
        }
        self.refresh_layout_scene();
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = self.layer_panel();
        let hit = panel.hit_test(layer_rect, Point2D::new(x, y));
        match press_flow::open_layer_context_menu(&mut self.editor_state, hit, x, y) {
            LayerContextMenuPress::Opened | LayerContextMenuPress::Dismissed => {
                self.mark_dirty();
                true
            }
            LayerContextMenuPress::Missed => self.blur_text_inputs_on_blank_press(),
        }
    }

    pub(in crate::widget_host) fn dispatch_layer_context_action(
        &mut self,
        action: op_editor_ui::widgets::layer_context_menu::LayerContextAction,
        target: op_editor_core::ui_draft::LayerContextTarget,
    ) {
        let step = if let Some(allocator) = self.collab_id_allocator.as_mut() {
            match press_flow::apply_layer_context_action_with_allocator(
                &mut self.editor_state,
                allocator,
                action,
                target,
                self.now_ms,
            ) {
                Ok(step) => step,
                Err(error) => {
                    self.show_collab_id_error(error);
                    return;
                }
            }
        } else {
            press_flow::apply_layer_context_action(
                &mut self.editor_state,
                &mut self.next_node_id,
                action,
                target,
                self.now_ms,
            )
        };
        match step {
            LayerContextStep::Done => {}
            LayerContextStep::Group => {
                let _ = self.apply_group();
            }
            LayerContextStep::Boolean(op) => {
                let _ = self.apply_boolean_op(op);
            }
            LayerContextStep::Refit => {
                self.fit_active_page_after_switch(self.last_viewport_w, self.last_viewport_h);
            }
        }
        self.mark_dirty();
    }

    /// Platform tail for a press routed to an open property-panel
    /// popover (`press_flow::press_*`). Every outcome consumes the
    /// press.
    pub(in crate::widget_host) fn finish_property_overlay_press(
        &mut self,
        press: PropertyOverlayPress,
    ) -> bool {
        match press {
            PropertyOverlayPress::Action(action) => self.apply_property_action(action),
            PropertyOverlayPress::Swallow => {}
            PropertyOverlayPress::Dismissed => self.mark_dirty(),
        }
        true
    }

    /// Mouse-press handler. Returns whether anything visible changed.
    ///
    /// A strictly ordered hit-test ladder: overlays before panels before
    /// canvas. Each tier helper returns `Option<bool>` — `None` means
    /// "declined, fall through to the next tier", `Some(dirty)` means
    /// "claimed the press, and this is the repaint signal".
    ///
    /// THE CALL ORDER BELOW *IS* THE BEHAVIOUR. The tier bodies live in
    /// the `press_*_tiers.rs` siblings only to respect the per-file line
    /// cap. This ladder is deliberately NOT shared with the native host:
    /// the two differ in tier order and gating in several places.
    pub fn apply_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // Cache the viewport dims so `apply_cursor_move(x, y)` (no
        // viewport params in signature) can rebuild the canvas region
        // for the floating align toolbar's hover sync. Mirrors the
        // native host's `last_viewport_w` / `_h` cache.
        self.last_viewport_w = viewport_width;
        self.last_viewport_h = viewport_height;
        self.last_cursor_x = x;
        self.last_cursor_y = y;
        // Refresh the derived paint doc once up front — every hit-test
        // below reads `&self.layout_scene`, so it must be current.
        self.refresh_layout_scene();
        // 0-pre. Commit any in-flight rename + canvas text-edit on
        // first press anywhere. Tracked so the final return reports
        // the visible change.
        let rename_committed =
            self.editor_state.ui.layer_rename.is_some() && self.editor_state.rename_commit();
        let text_edit_was_active = self.editor_state.ui.text_editing.is_some();
        let text_edit_committed = self.editor_state.text_edit_commit();
        if rename_committed || text_edit_committed {
            self.mark_dirty();
        }
        let mut ctx = PressCtx {
            x,
            y,
            viewport_width,
            viewport_height,
            rename_committed,
            text_edit_was_active,
            text_edit_committed,
            // Both resolved below, at the exact points the flat ladder did.
            over_chat_model_picker: false,
            property_focus_committed: false,
        };
        // Tier 1 — top-most overlays / floating panels / context menus.
        if let Some(consumed) = self.press_topmost_overlay_tiers(&ctx) {
            return consumed;
        }
        // Tier 2 — import dropdown + locale picker.
        if let Some(consumed) = self.press_import_locale_tiers(&ctx) {
            return consumed;
        }
        // Tier 3 — shape picker, file / export / figma / login / account.
        if let Some(consumed) = self.press_menu_modal_tiers(&ctx) {
            return consumed;
        }
        // Tier 4 — image-fill popover, StatusBar, and the model-picker
        // slice that lifts above the TopBar.
        if let Some(consumed) = self.press_rail_overlay_tiers(&ctx) {
            return consumed;
        }
        ctx.over_chat_model_picker = self
            .chat_model_picker_rect(viewport_width, viewport_height)
            .is_some_and(|rect| rect.contains(Point2D::new(x, y)));
        // Tier 5 — theme-preset dropdown + floating VariablesPanel.
        if let Some(consumed) = self.press_variables_tiers(&ctx) {
            return consumed;
        }
        // Tier 6 — TopBar chrome (and its blank-press gaps).
        if let Some(consumed) = self.press_top_bar_tier(&ctx) {
            return consumed;
        }
        // Tier 7 — property-panel popovers, then the fonts + model-picker
        // overlay band.
        if let Some(consumed) = self.press_property_overlay_tiers(&ctx) {
            return consumed;
        }
        if let Some(consumed) = self.press_font_and_picker_tiers(&ctx) {
            return consumed;
        }
        // Tier 8 — PropertyPanel input row.
        if let Some(consumed) = self.press_property_panel_tier(&ctx) {
            return consumed;
        }
        ctx.property_focus_committed = self.commit_property_family_focus_if_any();
        let property_focus_committed = ctx.property_focus_committed;
        // Tier 9 — AI chat panel.
        if let Some(consumed) = self.press_chat_tier(&ctx) {
            return consumed;
        }
        // Tier 10 — toolbar.
        if let Some(consumed) = self.press_toolbar_tier(&ctx) {
            return consumed;
        }
        // Tier 11 — LayerPanel drag peek, align toolbar, `apply_click`.
        if let Some(consumed) = self.press_layer_align_click_tiers(&ctx) {
            return consumed;
        }
        // Tier 12 — the canvas, branching on the active tool.
        if let Some(consumed) = self.press_canvas_tier(&ctx) {
            return consumed;
        }
        // Final fall-through — the press hit no interactive chrome
        // (panel-rail gaps, property-panel padding, …): blank press.
        let blurred = self.blur_text_inputs_on_blank_press();
        blurred || rename_committed || text_edit_committed || property_focus_committed
    }

    /// Route a screen-space press into the live preview runtime as a
    /// pointer Down. Returns `true` (consumed) only when preview is active
    /// and the point is inside the canvas. No-op (false) when not in preview.
    #[cfg(feature = "canvaskit")]
    pub(in crate::widget_host) fn preview_dispatch_press(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        use jian_core::gesture::pointer::PointerPhase;

        // Slideshow presentation: check toolbar and board presses first,
        // before converting to scene space (slideshow operates in screen space).
        if self.preview_slideshow_active() {
            // Toolbar has priority — it must stay clickable above the board.
            if self.slideshow_toolbar_press(screen_x, screen_y, viewport_w, viewport_h) {
                return true;
            }
            // Board press records the position for swipe detection on release.
            self.slideshow_board_press(screen_x, screen_y);
            return true;
        }

        let doc_point =
            match self.preview_screen_to_scene_point(screen_x, screen_y, viewport_w, viewport_h) {
                Some(p) => p,
                None => return false,
            };

        let Some(session) = self.preview.as_mut() else {
            return false;
        };
        let handled = session.dispatch_pointer_for_id_at(
            LEGACY_WEB_PREVIEW_POINTER_ID,
            jian_core::gesture::pointer::PointerKind::Mouse,
            doc_point.x,
            doc_point.y,
            PointerPhase::Down,
            self.now_ms,
        );
        let first_finger = self.preview_pressed_pids.is_empty();
        if !self
            .preview_pressed_pids
            .contains(&LEGACY_WEB_PREVIEW_POINTER_ID)
        {
            self.preview_pressed_pids
                .push(LEGACY_WEB_PREVIEW_POINTER_ID);
        }
        self.preview_last_doc_by_pid
            .insert(LEGACY_WEB_PREVIEW_POINTER_ID, (doc_point.x, doc_point.y));
        // Arm edge-swipe candidate after the runtime has the pointer Down,
        // and only for the FIRST finger — a second finger never arms.
        if first_finger {
            self.arm_edge_swipe_candidate(screen_x);
            self.preview_edge_swipe_pid = Some(LEGACY_WEB_PREVIEW_POINTER_ID);
        }
        if handled {
            self.mark_dirty();
        }
        handled
    }

    /// Route a cursor move into the live preview runtime.
    /// Returns `true` (consumed) when the point is on-canvas and a gesture
    /// is active or the preview handled the move event.
    #[cfg(feature = "canvaskit")]
    pub(in crate::widget_host) fn preview_dispatch_move(
        &mut self,
        screen_x: f32,
        screen_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        use jian_core::gesture::pointer::PointerPhase;

        // Slideshow presentation: update toolbar hover and cursor tracking
        // before the runtime sees the move.
        if self.preview_slideshow_active() {
            self.slideshow_toolbar_hover(screen_x, screen_y, viewport_w, viewport_h);
            return true;
        }

        let Some(_session) = self.preview.as_mut() else {
            return false;
        };
        let Some(doc_point) =
            self.preview_screen_to_scene_point(screen_x, screen_y, viewport_w, viewport_h)
        else {
            return false;
        };
        // Check edge-swipe BEFORE updating preview_last_doc, so cancel
        // dispatches at the gesture's last real position
        if self
            .preview_pressed_pids
            .contains(&LEGACY_WEB_PREVIEW_POINTER_ID)
            && self.preview_edge_swipe_pid == Some(LEGACY_WEB_PREVIEW_POINTER_ID)
            && self.maybe_fire_edge_swipe(screen_x)
        {
            self.cancel_preview_gesture_for_edge_swipe();
            self.mark_dirty();
            return true;
        }
        // Move ONLY while the press is held; a bare cursor pass is a
        // Hover. Feeding the runtime a stream of `Move` with no button
        // down leaves its gesture machine believing a drag is in flight,
        // and every later Down/Up stops resolving as a tap — which reads
        // as "the nav works once, then goes dead".
        let phase = if self
            .preview_pressed_pids
            .contains(&LEGACY_WEB_PREVIEW_POINTER_ID)
        {
            PointerPhase::Move
        } else {
            PointerPhase::Hover
        };
        self.preview_last_doc_by_pid
            .insert(LEGACY_WEB_PREVIEW_POINTER_ID, (doc_point.x, doc_point.y));
        if let Some(session) = self.preview.as_mut() {
            let emitted = session.dispatch_pointer_for_id_at(
                LEGACY_WEB_PREVIEW_POINTER_ID,
                jian_core::gesture::pointer::PointerKind::Mouse,
                doc_point.x,
                doc_point.y,
                phase,
                self.now_ms,
            );
            if emitted
                || self
                    .preview_pressed_pids
                    .contains(&LEGACY_WEB_PREVIEW_POINTER_ID)
            {
                self.mark_dirty();
            }
            true
        } else {
            false
        }
    }

    /// Complete a preview gesture: pointer Up.
    /// Returns `true` (consumed) when a preview press was in flight.
    #[cfg(feature = "canvaskit")]
    pub(in crate::widget_host) fn preview_dispatch_release(&mut self) -> bool {
        use jian_core::gesture::pointer::PointerPhase;

        // Slideshow presentation: resolve the release through slideshow handlers
        // (tap/swipe detection, toolbar activation).
        if self.preview_slideshow_active() {
            // Update the cursor to the final position before release analysis.
            if let Some((x, y)) = self
                .preview_last_doc_by_pid
                .get(&LEGACY_WEB_PREVIEW_POINTER_ID)
                .copied()
            {
                let _ = self.preview_slideshow_release_point(
                    x,
                    y,
                    self.last_viewport_w,
                    self.last_viewport_h,
                );
            }
            // Toolbar release handles button activation; board release handles
            // taps and swipes. Both consume the gesture if they were armed.
            let handled = self.slideshow_toolbar_release() || self.slideshow_board_release();
            self.preview_last_doc_by_pid
                .remove(&LEGACY_WEB_PREVIEW_POINTER_ID);
            return handled;
        }

        // Mirrors the native host's `preview_dispatch_release`.
        self.preview_surface_capture = None;
        self.disarm_edge_swipe();
        if !self
            .preview_pressed_pids
            .contains(&LEGACY_WEB_PREVIEW_POINTER_ID)
        {
            // No Down in flight. Sending an unpaired Up wedges the
            // runtime's gesture state and it swallows the next Down —
            // which reads as "the nav only works once".
            return false;
        }
        self.preview_pressed_pids
            .retain(|p| *p != LEGACY_WEB_PREVIEW_POINTER_ID);
        let Some((x, y)) = self
            .preview_last_doc_by_pid
            .remove(&LEGACY_WEB_PREVIEW_POINTER_ID)
        else {
            return false;
        };
        // Release AT the gesture's last scene point: the runtime resolves
        // a tap by where the pointer came up, so an Up at the origin never
        // completes the tap on the widget the Down landed on.
        if let Some(session) = self.preview.as_mut() {
            session.dispatch_pointer_for_id_at(
                LEGACY_WEB_PREVIEW_POINTER_ID,
                jian_core::gesture::pointer::PointerKind::Mouse,
                x,
                y,
                PointerPhase::Up,
                self.now_ms,
            );
        }
        self.mark_dirty();
        true
    }

    /// Convert screen-space point to preview scene-space.
    /// Returns `None` if the point is outside the canvas region.
    #[cfg(feature = "canvaskit")]
    fn preview_screen_to_scene_point(
        &self,
        screen_x: f32,
        screen_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Point2D> {
        // The hit test must invert whatever transform preview PAINTED
        // through, and the two segments paint through different ones.
        // Getting this wrong errors nowhere — taps just land in empty
        // space — so the branch mirrors the paint branch exactly.
        if self.device_mode_active() {
            // Phone / Desktop: through the device frame, honouring the
            // pinned strips and the captured gesture surface.
            return self.device_preview_doc_point(screen_x, screen_y);
        }
        // Canvas: the editor's own pan/zoom, via the same shared helper
        // the native host's `preview_doc_point` falls back to.
        op_editor_ui::widgets::host_canvas_geometry::canvas_doc_point(
            &self.editor_state,
            screen_x,
            screen_y,
            viewport_w,
            viewport_h,
        )
    }
}
