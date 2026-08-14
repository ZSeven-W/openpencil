//! Construction + per-frame bookkeeping on `WidgetHostNative`: the
//! constructor, transcript/layer-panel cache owner rotation, modifier
//! and clock setters.
//!
//! Split out of the `widget_host.rs` spine to keep it under the repo's
//! 800-line cap.

use super::*;

impl WidgetHostNative {
    pub fn new() -> Self {
        // A fresh launch opens with a single empty starter Frame —
        // see `EditorState::starter`.
        let editor_state = op_editor_core::EditorState::starter();
        // Read the font generation BEFORE building the initial scene. An
        // import landing during construction then leaves this stale-low, so
        // the next `refresh_layout_scene` rebuilds — reading it AFTER the
        // build would record the new generation against the pre-import scene
        // and skip the rebuild until an unrelated dirty event (same race we
        // fixed in `FontResolver` / `SkiaMeasure`).
        let layout_scene_font_generation = jian_skia::font_generation();
        // Seed the render scene once up front; subsequent frames
        // re-derive only when `editor_state_dirty` is set.
        let layout_scene = op_pen_loader::editor_state_to_active_page_layout_scene(&editor_state);
        let last_chat_session_index = editor_state.chat.active_index();
        Self {
            editor_state,
            layout_scene,
            layout_transition: None,
            scene_cache: op_pen_loader::SceneBuildCache::new(),
            editor_state_dirty: false,
            document_epoch: 0,
            layout_scene_font_generation,
            theme: Theme::dark(),
            drag: None,
            agent_settings_touch_gesture: None,
            touch_panel_gesture: None,
            space_pan: false,
            last_hover_probe: None,
            chat_drag: None,
            chat_resize: None,
            design_md_drag: None,
            component_browser_drag: None,
            icon_picker_drag: None,
            image_adjustment_drag: None,
            image_crop_drag: None,
            effect_radius_drag: None,
            code_selection_drag: None,
            chat_input_selection_drag: None,
            image_input_selection_drag: None,
            image_input_geometry: None,
            chat_text_selection_drag: None,
            text_edit_selection_drag: None,
            text_measure: std::cell::RefCell::new(None),
            panel_resize: None,
            variables_resize: None,
            node_drag: None,
            canvas_drop_index: None,
            option_drag_source_ids: Vec::new(),
            path_anchor_drag: None,
            arc_handle_drag: None,
            handle_drag: None,
            rotate_drag: None,
            create_drag: None,
            marquee_drag: None,
            layer_drag: None,
            next_node_id: 100,
            collab_id_allocator: None,
            now_ms: 0,
            interaction_hot_until_ms: 0,
            pan_cache: None,
            pan_cache_restore: None,
            pan_cache_blits: 0,
            pan_cache_scrolls: 0,
            pan_cache_builds: 0,
            last_gesture_was_zoom: false,
            shift_held: false,
            alt_held: false,
            toast_rect: None,
            last_viewport_w: 0.0,
            last_viewport_h: 0.0,
            keyboard_occlusion: 0.0,
            preview: None,
            preview_device_frame: None,
            preview_scroll_y: 0.0,
            preview_manual_pick: None,
            preview_surface_capture: None,
            preview_mode_transition: None,
            preview_press_active: false,
            preview_last_doc: None,
            slideshow_press_screen: None,
            slideshow_cursor: None,
            preview_edge_swipe_start_x: None,
            chat_panel_owner: op_editor_ui::widgets::AIChatPlaceholder::next_owner(),
            layer_panel_owner: op_editor_ui::widgets::LayerPanel::next_layer_panel_owner(),
            slide_thumbs: Default::default(),
            last_chat_session_index,
            auth_login_handle: None,
            auth_pending_browser_url: None,
            auth_browser_opened: false,
            auth_session_refresh_deadline: None,
            auth_account_avatar_revision: None,
        }
    }

    /// Rotate the chat-panel transcript-cache owner when the active chat session
    /// (tab) changed since the last call. A fresh owner means the new tab's
    /// display-frame cursor hint reads `None` (the slot still belongs to the old
    /// owner) until this tab's next paint re-resolves and re-stamps it — the
    /// documented one-frame isolation. Called at the top of the paint / probe
    /// entry points so the very next resolve stores under the rotated owner.
    pub(in crate::widget_host) fn rotate_chat_owner_if_session_changed(&mut self) {
        bookkeeping::rotate_chat_owner_if_session_changed(
            &self.editor_state,
            &mut self.chat_panel_owner,
            &mut self.last_chat_session_index,
        );
    }

    /// Force a chat-panel transcript-cache owner rotation NOW, unconditionally —
    /// even when `chat.active_index()` is unchanged. Called synchronously at each
    /// host session-mutation site (tab switch / new tab). A tab switch changes the
    /// active session but a `CursorMoved` can arrive before the next paint and run
    /// the event-time cursor-shape hint (`geometry::cursor_hint` →
    /// `hit_test_current_build`), which would otherwise pair the previous session's
    /// cached geometry with the new session's live messages. Rotating here means
    /// that hint reads `None` (default arrow) until the new session's first paint
    /// re-stamps the slot. Rotating unconditionally also covers same-index session
    /// replacement — closing active tab 0 installs the next session at index 0,
    /// closing the sole tab replaces it in place — which the index-only poll in
    /// [`Self::rotate_chat_owner_if_session_changed`] misses.
    ///
    /// Public because some session mutations run outside this crate: the desktop
    /// runner's ⌘T `new_chat_tab` and tab-close `close_chat_tab` mutate
    /// `chat` directly and must rotate synchronously for the same reason.
    pub fn force_rotate_chat_owner(&mut self) {
        bookkeeping::force_rotate_chat_owner(
            &self.editor_state,
            &mut self.chat_panel_owner,
            &mut self.last_chat_session_index,
        );
    }

    /// Force a LayerPanel row-model-cache owner rotation NOW. The cache key is
    /// `(document_revision, active_page_index, collapsed_fp, rename_fp)`, and a
    /// freshly loaded / imported / MCP-replaced document restarts its revision
    /// at 0 and its active page index at 0 — so a WHOLE-DOCUMENT replacement
    /// leaves the key byte-identical to the previous document's, while the
    /// owner never rotates on its own. The paint path would then serve the
    /// PREVIOUS document's cached rows indefinitely. Rotating the owner at every
    /// replacement seam makes the next owned paint resolve miss the stale slot
    /// and rebuild against the new document. (Page/tab switches WITHIN a live
    /// document need no rotation — they change `active_page_index`, which is in
    /// the key.)
    ///
    /// Public because some replacement seams run outside this crate: the desktop
    /// runner replaces `editor_state` on Open / New (`persistence.rs`) and MCP
    /// `ReplaceDocument` (`mcp_runtime.rs`) and must rotate synchronously.
    pub fn force_rotate_layer_panel_owner(&mut self) {
        self.layer_panel_owner = op_editor_ui::widgets::LayerPanel::next_layer_panel_owner();
    }

    /// Build the Layer panel through this host's owner-scoped row cache.
    ///
    /// Paint and event-time hit tests must share this path: page/layer presses,
    /// hover, scrolling, and accessibility otherwise rebuild and reallocate the
    /// whole active-page row model independently on large documents.
    pub(in crate::widget_host) fn layer_panel(&self) -> op_editor_ui::widgets::LayerPanel {
        op_editor_ui::widgets::LayerPanel::from_editor_owned(
            &self.editor_state,
            self.layer_panel_owner,
        )
    }

    /// Push the host's current shift-key state. Runners call this
    /// on every modifier-change event so `apply_press` can branch
    /// on shift+click semantics.
    pub fn set_modifier_shift(&mut self, held: bool) {
        self.shift_held = held;
    }

    pub fn set_modifier_alt(&mut self, held: bool) {
        self.alt_held = held;
    }

    /// Publish the geometry used by this frame before any focused surface
    /// resolves its keyboard-safe scroll position.
    pub(in crate::widget_host) fn publish_viewport_geometry(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let changed = self.last_viewport_w != viewport_w || self.last_viewport_h != viewport_h;
        self.last_viewport_w = viewport_w;
        self.last_viewport_h = viewport_h;
        if changed {
            self.reveal_property_keyboard_owner();
        }
        self.ensure_focused_agent_settings_visible(viewport_w, viewport_h);
    }

    /// Set the bottom keyboard occlusion inside the safe-area-local editor
    /// viewport. The shell owns safe-area de-duplication before calling this.
    pub fn set_keyboard_occlusion(&mut self, height: f32) -> bool {
        let next = if height.is_finite() && height > 0.0 {
            height
        } else {
            0.0
        };
        if (self.keyboard_occlusion - next).abs() <= f32::EPSILON {
            return self
                .ensure_focused_agent_settings_visible(self.last_viewport_w, self.last_viewport_h);
        }
        self.keyboard_occlusion = next;
        self.reveal_property_keyboard_owner();
        self.ensure_focused_agent_settings_visible(self.last_viewport_w, self.last_viewport_h);
        true
    }

    /// Return the local y-coordinate immediately above the keyboard.
    pub fn keyboard_visible_bottom(&self, viewport_height: f32) -> f32 {
        let height = if viewport_height.is_finite() {
            viewport_height.max(0.0)
        } else {
            0.0
        };
        (height - self.keyboard_occlusion).max(0.0)
    }

    /// Push the host's monotonic millisecond timestamp into the
    /// host. Drives caret blink + any future time-based
    /// animations via `jian_core::anim`. Also forwarded to the live
    /// preview runtime (caret blink in Preview mode).
    pub fn set_now_ms(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        if let Some(preview) = self.preview.as_mut() {
            preview.set_now_ms(now_ms);
        }
    }
}

impl Default for WidgetHostNative {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod keyboard_occlusion_tests {
    use super::*;

    #[test]
    fn keyboard_visible_bottom_is_sanitized_and_clamped() {
        let mut host = WidgetHostNative::new();
        assert_eq!(host.keyboard_visible_bottom(600.0), 600.0);

        assert!(host.set_keyboard_occlusion(240.0));
        assert!(!host.set_keyboard_occlusion(240.0));
        assert_eq!(host.keyboard_visible_bottom(600.0), 360.0);
        assert_eq!(host.keyboard_visible_bottom(120.0), 0.0);

        assert!(host.set_keyboard_occlusion(f32::NAN));
        assert_eq!(host.keyboard_visible_bottom(600.0), 600.0);
        assert!(!host.set_keyboard_occlusion(-20.0));
        assert_eq!(host.keyboard_visible_bottom(f32::NAN), 0.0);
    }
}
