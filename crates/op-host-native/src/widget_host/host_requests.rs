//! Host-driven request drains on `WidgetHostNative` — component-browser
//! inserts, Figma paste, settings-input flush — plus the interaction /
//! animation-deadline accessors the runner polls each frame.
//!
//! Split out of the `widget_host.rs` spine to keep it under the repo's
//! 800-line cap.

use super::*;

/// Grace period after pan/zoom before full-quality painting resumes.
pub(in crate::widget_host) const INTERACTION_HOT_MS: u64 = 150;

impl WidgetHostNative {
    /// Drain a queued Component-Browser insert: place the chosen
    /// UIKit component at the viewport's centre (top-left = centre −
    /// half the component's size) and call
    /// [`EditorState::instantiate_kit_component`]. Returns `true`
    /// when an instantiate landed (the desktop runner schedules a
    /// repaint on `true`).
    pub fn drain_component_browser_insert(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        let Some((kit_id, comp_id)) = self
            .editor_state
            .editor_ui
            .component_browser_pending_insert
            .take()
        else {
            return false;
        };
        if !self.collab_allows_document_mutation(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::UIKit,
            ),
        ) {
            return true;
        }
        let dims = self
            .editor_state
            .ui_kits
            .iter()
            .find(|k| k.id == kit_id)
            .and_then(|k| k.components.iter().find(|c| c.id == comp_id))
            .map(|c| (c.width as f64, c.height as f64));
        let Some((cw_comp, ch_comp)) = dims else {
            return false;
        };
        let doc =
            canvas_geometry::canvas_centre_doc_point(&self.editor_state, viewport_w, viewport_h);
        let dx = doc.x as f64 - cw_comp / 2.0;
        let dy = doc.y as f64 - ch_comp / 2.0;
        let result = if let Some(allocator) = self.collab_id_allocator.as_mut() {
            self.editor_state
                .instantiate_kit_component_under_parent_with_allocator(
                    &kit_id,
                    &comp_id,
                    &op_editor_core::NodeId::NONE,
                    dx,
                    dy,
                    None,
                    allocator,
                )
        } else {
            Ok(self
                .editor_state
                .instantiate_kit_component(&kit_id, &comp_id, dx, dy))
        };
        let inserted = match result {
            Ok(id) => id.is_some(),
            Err(error) => {
                self.show_collab_id_error(error);
                return true;
            }
        };
        if inserted {
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Insert nodes parsed from the Figma clipboard, centred on the
    /// viewport, with fresh ids, batched undo, and the pasted roots
    /// selected — mirrors TS `use-figma-paste.ts:67-100`.
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
        if !self.collab_allows_document_mutation(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ExternalAssets,
            ),
        ) {
            return true;
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
        let centre =
            canvas_geometry::canvas_centre_doc_point(&self.editor_state, viewport_w, viewport_h);
        let dx = centre.x as f64 - (min_x + max_x) / 2.0;
        let dy = centre.y as f64 - (min_y + max_y) / 2.0;

        let snap = self.editor_state.snapshot_for_history();
        let mut taken = self.editor_state.collect_node_ids();
        let mut new_ids = Vec::with_capacity(nodes.len());
        let mut clones = Vec::with_capacity(nodes.len());
        for node in &nodes {
            let result = if let Some(allocator) = self.collab_id_allocator.as_mut() {
                op_editor_core::walkers::deep_clone_with_allocator(node, allocator, &mut taken)
            } else {
                Ok(op_editor_core::walkers::deep_clone_with_new_ids(
                    node,
                    &mut self.next_node_id,
                    &mut taken,
                ))
            };
            let mut clone = match result {
                Ok(clone) => clone,
                Err(error) => {
                    self.show_collab_id_error(error);
                    return true;
                }
            };
            op_editor_core::walkers::translate_subtree(&mut clone, dx, dy);
            new_ids.push(op_editor_core::NodeId::new(clone.base().id.clone()));
            clones.push(clone);
        }
        self.editor_state.active_children_mut().extend(clones);
        if let Some(anchor) = new_ids.first().cloned() {
            self.editor_state.set_single_selection(anchor);
            for id in new_ids.into_iter().skip(1) {
                self.editor_state.toggle_selection(id);
            }
        }
        self.editor_state.history_push_past(snap);
        self.mark_dirty();
        self.refresh_missing_fonts_after_document_change();
        true
    }

    /// Commit any in-progress settings-modal input draft (currently
    /// the MCP port). Used by the desktop runner before persisting
    /// settings on quick-quit so a focused-but-uncommitted port edit
    /// isn't silently dropped.
    pub fn flush_settings_input(&mut self) {
        self.commit_settings_focus_if_any();
    }

    /// Whether the chat input is focused — runner uses this to
    /// decide whether to schedule a periodic wake-up for caret
    /// blink.
    pub fn chat_focused(&self) -> bool {
        self.editor_state.chat.focused
    }

    /// Record a live canvas pan gesture tick: the canvas paints
    /// interactive-degraded until `INTERACTION_HOT_MS` after the
    /// last tick, then the scheduler-driven repaint restores quality.
    pub(in crate::widget_host) fn note_viewport_gesture(&mut self) {
        self.interaction_hot_until_ms = self.now_ms.saturating_add(INTERACTION_HOT_MS);
        self.last_gesture_was_zoom = false;
    }

    /// Record a live canvas ZOOM gesture tick. Same degrade window as
    /// a pan, but the pan bitmap cache must NOT rebuild per tick — the
    /// zoom invalidates it every frame, so building (2× a plain frame)
    /// would be pure loss; zoom frames paint direct in degrade mode.
    pub(in crate::widget_host) fn note_viewport_zoom_gesture(&mut self) {
        self.interaction_hot_until_ms = self.now_ms.saturating_add(INTERACTION_HOT_MS);
        self.last_gesture_was_zoom = true;
    }

    /// Whether the current frame should paint in interactive-degrade
    /// mode (a pan/zoom gesture ticked within the hot window).
    pub(in crate::widget_host) fn fast_interaction_active(&self) -> bool {
        self.now_ms < self.interaction_hot_until_ms
    }

    /// Low-cost canvas paint mode for direct manipulation. Unlike
    /// [`Self::fast_interaction_active`], this does not make the pan bitmap
    /// cache eligible because edited geometry changes on every frame.
    pub(in crate::widget_host) fn canvas_fast_interaction_active(&self) -> bool {
        self.fast_interaction_active()
            || self.node_drag.as_ref().is_some_and(|drag| drag.moved)
            || self.handle_drag.is_some()
            || self.rotate_drag.is_some()
            || self.create_drag.is_some()
    }

    /// Whether a top-bar hover tooltip is currently due to be on screen.
    ///
    /// The runner needs this SEPARATELY from
    /// [`Self::next_animation_deadline_ms`]. That deadline goes to
    /// `None` the instant the tooltip becomes due — which is precisely
    /// the wake that has to repaint. A runner gating its redraw on the
    /// deadline alone would wake up, find nothing pending, and go back
    /// to sleep without ever drawing the tooltip.
    pub fn top_bar_tooltip_showing(&self) -> bool {
        op_editor_ui::widgets::top_bar_tooltip::visible_button(
            &self.editor_state.editor_ui,
            self.now_ms,
        )
        .is_some()
    }

    /// Next millisecond at which the host should wake to repaint
    /// the caret blink phase. `None` = no animation pending.
    pub fn next_animation_deadline_ms(&self) -> Option<u64> {
        // Indicators + layout transition + caret blink are shared with the
        // web host; only the clauses below are native-platform concerns.
        let mut next = bookkeeping::base_animation_deadline_ms(
            &self.editor_state,
            self.layout_transition.as_ref(),
            self.now_ms,
        );
        // Gesture-end full-quality repaint: wake once the
        // interactive-degrade window closes. Quantized UP to a 50 ms
        // grid so consecutive gesture ticks report the SAME deadline —
        // the desktop runner's waker dedups identical instants, and a
        // per-tick sliding deadline re-armed the OS timer every frame.
        if self.fast_interaction_active() {
            next = bookkeeping::earliest(next, self.interaction_hot_until_ms.div_ceil(50) * 50);
        }
        // Progressive quality restore: one tile per frame until the
        // visible region is sharp again.
        if self.pan_cache_restore.is_some() {
            next = bookkeeping::earliest(next, self.now_ms.saturating_add(16));
        }
        // Slides-tab thumbnails: the rail renders a bounded batch per
        // frame and waits out an edit before re-rendering, so it asks
        // for the next wake here rather than dirtying the whole host.
        if let Some(at) = self.slide_thumbs.wake_deadline_ms() {
            next = bookkeeping::earliest(next, at);
        }
        // While previewing, keep the loop ticking (~30 fps) so the live
        // runtime's caret blink + any time-driven widget state animates.
        if self.preview.is_some() {
            next = bookkeeping::earliest(next, self.now_ms.saturating_add(33));
        }
        // While a `git clone` runs, keep the loop ticking so
        // `poll_git_clone_job` drains the worker's result later.
        if let Some(form) = &self.editor_state.editor_ui.git_panel.clone_form {
            if form.cloning {
                next = bookkeeping::earliest(next, self.now_ms.saturating_add(100));
            }
        }
        next
    }
}
