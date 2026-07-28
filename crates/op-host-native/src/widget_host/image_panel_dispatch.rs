//! Image-node section dispatch — Search / Generate popover toggles,
//! search submit / result select, generate lifecycle, and the
//! local-asset Relink intent (TS `image-section.tsx` +
//! `image-search-popover.tsx` + `image-generate-popover.tsx`).
//!
//! The state machine itself is shared with the web host in
//! `op_editor_core::host_image_panel_transitions`; what stays here is
//! the platform glue — popover-input selection drag, chrome-input blur,
//! the property-focus commit, and the layout-scene-backed hit-test.

use super::WidgetHostNative;
use op_editor_core::host_image_panel_transitions as image_ops;
use op_editor_ui::widgets::PropertyPanel;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    pub(in crate::widget_host) fn toggle_image_search_popover(&mut self) {
        let opening = !self.editor_state.editor_ui.image_panel.search_open;
        // Seed from the pre-blur selection.
        let seed = image_ops::selected_image_seed(&self.editor_state, false);
        self.clear_image_input_selection_drag();
        if opening {
            self.blur_text_inputs_on_blank_press();
        }
        image_ops::apply_image_search_toggle(&mut self.editor_state, opening, seed, self.now_ms);
        self.close_other_property_popovers_for_image();
    }

    pub(in crate::widget_host) fn toggle_image_generate_popover(&mut self) {
        let opening = !self.editor_state.editor_ui.image_panel.generate_open;
        let seed = image_ops::selected_image_seed(&self.editor_state, true);
        self.clear_image_input_selection_drag();
        if opening {
            self.blur_text_inputs_on_blank_press();
        }
        image_ops::apply_image_generate_toggle(&mut self.editor_state, opening, seed, self.now_ms);
        self.close_other_property_popovers_for_image();
    }

    /// Close the property pickers that would overlap the popovers.
    fn close_other_property_popovers_for_image(&mut self) {
        self.commit_image_tile_scale_focus_if_any();
        image_ops::close_other_property_popovers_for_image(&mut self.editor_state.editor_ui);
    }

    pub(in crate::widget_host) fn run_image_search(&mut self) {
        image_ops::run_image_search(&mut self.editor_state);
    }

    /// Write the clicked result's thumbnail into the node's `src`
    /// (TS `onSelect(result.thumbUrl)`) and close the popover.
    pub(in crate::widget_host) fn select_image_search_result(&mut self, index: usize) {
        let Some(url) = image_ops::image_search_result_url(&self.editor_state, index) else {
            return;
        };
        self.write_selected_image_src(&url);
        self.clear_image_input_selection_drag();
        self.editor_state.editor_ui.image_panel.close_popovers();
    }

    pub(in crate::widget_host) fn run_image_generate(&mut self) {
        image_ops::run_image_generate(&mut self.editor_state);
    }

    pub(in crate::widget_host) fn apply_generated_image(&mut self) {
        let Some(url) = image_ops::generated_preview_url(&self.editor_state) else {
            return;
        };
        self.write_selected_image_src(&url);
        self.clear_image_input_selection_drag();
        self.editor_state.editor_ui.image_panel.close_popovers();
    }

    pub(in crate::widget_host) fn retry_image_generate(&mut self) {
        image_ops::retry_image_generate(&mut self.editor_state);
    }

    /// Open the settings modal on the Images tab (TS
    /// `setDialogOpen(true)` from the not-configured view).
    pub(in crate::widget_host) fn open_image_gen_settings(&mut self) {
        self.clear_image_input_selection_drag();
        image_ops::open_image_gen_settings(&mut self.editor_state);
    }

    /// Commit `src` onto the selected image node (with history).
    pub(in crate::widget_host) fn write_selected_image_src(&mut self, src: &str) {
        if !self.collab_allows_document_mutation(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ExternalAssets,
            ),
        ) {
            return;
        }
        if image_ops::write_selected_image_src(&mut self.editor_state, src) {
            self.mark_dirty();
        }
    }

    /// Route a printable char into whichever popover input is open.
    pub(in crate::widget_host) fn apply_image_panel_text(&mut self, c: char) -> bool {
        let effect = image_ops::image_panel_text(&mut self.editor_state, c, self.now_ms);
        self.finish_image_panel_input(effect)
    }

    /// Backspace in the open popover's input. Swallows the key even
    /// on an empty draft so it can't fall through to node deletion.
    pub(in crate::widget_host) fn apply_image_panel_backspace(&mut self) -> bool {
        let effect = image_ops::image_panel_backspace(&mut self.editor_state, self.now_ms);
        self.finish_image_panel_input(effect)
    }

    /// Forward Delete in the visible image-popover input. The popover still
    /// consumes Delete when no glyph changes so the selected image node behind
    /// it can never be removed accidentally.
    pub(in crate::widget_host) fn apply_image_panel_delete(&mut self) -> bool {
        let effect = image_ops::image_panel_delete(&mut self.editor_state, self.now_ms);
        self.finish_image_panel_input(effect)
    }

    /// Move the persistent image-popover caret. Consumes the arrow at text
    /// boundaries so it never falls through to canvas nudge.
    pub fn apply_image_panel_caret(&mut self, forward: bool, extend: bool) -> bool {
        let effect =
            image_ops::image_panel_caret(&mut self.editor_state, forward, extend, self.now_ms);
        self.finish_image_panel_input(effect)
    }

    /// Move the visible image-popover input to its start or end. The key is
    /// still consumed when the generate view has no editable field.
    pub fn apply_image_panel_edge(&mut self, end: bool, extend: bool) -> bool {
        let effect = image_ops::image_panel_edge(&mut self.editor_state, end, extend, self.now_ms);
        self.finish_image_panel_input(effect)
    }

    pub(in crate::widget_host) fn apply_image_panel_select_all(&mut self) -> bool {
        let effect = image_ops::image_panel_select_all(&mut self.editor_state, self.now_ms);
        self.finish_image_panel_input(effect)
    }

    /// Repaint when the shared routing changed a draft, and report
    /// whether the popover consumed the key.
    fn finish_image_panel_input(&mut self, effect: image_ops::ImageInputEffect) -> bool {
        if effect.changed {
            self.mark_dirty();
        }
        effect.consumed
    }

    /// Enter while the search popover is open submits the query (TS
    /// input onKeyDown Enter → handleSearch).
    pub(in crate::widget_host) fn apply_image_panel_send(&mut self) -> bool {
        if self.editor_state.editor_ui.image_panel.search_open {
            self.run_image_search();
            self.mark_dirty();
            return true;
        }
        // Generate popover: Enter is swallowed (TS textarea would
        // insert a newline; the popup submits via the button only).
        self.editor_state.editor_ui.image_panel.generate_open
    }

    /// Close property-owned floating popovers before a higher-z overlay or
    /// selection-changing secondary click takes focus so keyboard / pointer
    /// ownership cannot remain hidden underneath it.
    pub(in crate::widget_host) fn close_image_popovers_for_higher_overlay(&mut self) -> bool {
        self.clear_image_input_selection_drag();
        // Hoisted ahead of the shared close: the tile-scale commit runs
        // through host-owned variable/effect commits, and it touches
        // state disjoint from the popover flags below.
        if self.editor_state.editor_ui.image_fill_popover_open {
            self.commit_image_tile_scale_focus_if_any();
        }
        let changed = image_ops::close_image_popovers_for_higher_overlay(&mut self.editor_state);
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Outside-click dismiss for the Search / Generate popovers.
    /// Returns `true` when the press was consumed.
    pub(in crate::widget_host) fn dismiss_image_popovers_on_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::PropertyPanelAction as A;
        let panel_state = &self.editor_state.editor_ui.image_panel;
        if !panel_state.search_open && !panel_state.generate_open {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let rect = self.property_rect(viewport_width, viewport_height);
            let point = Point2D::new(x, y);
            if let Some(action) = panel.hit_test_action(rect, point) {
                if matches!(
                    action,
                    A::RunImageSearch
                        | A::SelectImageSearchResult(_)
                        | A::RunImageGenerate
                        | A::ApplyGeneratedImage
                        | A::RetryImageGenerate
                        | A::OpenImageGenSettings
                        | A::ToggleImageSearchPopover
                        | A::ToggleImageGeneratePopover
                ) {
                    self.apply_property_action(action);
                    return true;
                }
            }
            if let Some((kind, offset)) = self.image_popover_input_at(&panel, rect, point) {
                self.begin_image_input_selection_drag(kind, offset);
                return true;
            }
            if panel.image_popovers_contain(rect, point) {
                // Inside the popup body (input / textarea / empty
                // state) — swallow, keep open.
                return true;
            }
        }
        self.clear_image_input_selection_drag();
        self.editor_state.editor_ui.image_panel.close_popovers();
        self.mark_dirty();
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::widget_host::CursorHint;
    use op_editor_core::image_panel_state::{ImageGeneratePhase, ImageSearchHit};
    use op_editor_core::EditorState;
    use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A};
    use std::sync::Arc;

    fn host_with_image_selected() -> crate::widget_host::WidgetHostNative {
        let mut host = crate::widget_host::WidgetHostNative::new();
        let mut state = EditorState::sample();
        let _ = state.insert_image_node_at_viewport("Hero photo", "https://x/y.png");
        *host.editor_state_mut() = state;
        host
    }

    #[test]
    fn toggle_search_seeds_query_from_node_name() {
        let mut host = host_with_image_selected();
        host.apply_property_action(A::ToggleImageSearchPopover);
        let panel = &host.editor_state().editor_ui.image_panel;
        assert!(panel.search_open);
        assert_eq!(panel.search_query.text(), "Hero photo");
        assert!(!panel.search_has_searched);
        // Toggle again closes + clears transients.
        host.apply_property_action(A::ToggleImageSearchPopover);
        assert!(!host.editor_state().editor_ui.image_panel.search_open);
    }

    #[test]
    fn run_search_raises_epoch_and_loading() {
        let mut host = host_with_image_selected();
        host.apply_property_action(A::ToggleImageSearchPopover);
        let before = host.editor_state().editor_ui.image_panel.search_epoch;
        host.apply_property_action(A::RunImageSearch);
        let panel = &host.editor_state().editor_ui.image_panel;
        assert!(panel.search_loading);
        assert!(panel.search_has_searched);
        assert_eq!(panel.search_epoch, before + 1);
        // While loading, re-submit is a no-op (TS disables button).
        let epoch = panel.search_epoch;
        host.apply_property_action(A::RunImageSearch);
        assert_eq!(
            host.editor_state().editor_ui.image_panel.search_epoch,
            epoch
        );
    }

    #[test]
    fn selecting_a_result_writes_src_and_closes() {
        let mut host = host_with_image_selected();
        host.apply_property_action(A::ToggleImageSearchPopover);
        host.editor_state_mut()
            .editor_ui
            .image_panel
            .search_results
            .push(ImageSearchHit {
                id: "1".into(),
                thumb_data_url: Arc::new("data:image/png;base64,AA==".into()),
                attribution: String::new(),
            });
        host.apply_property_action(A::SelectImageSearchResult(0));
        let node = host.editor_state().selected_node().expect("image");
        let jian_ops_schema::node::PenNode::Image(image) = node else {
            panic!("image node expected");
        };
        assert_eq!(image.src, "data:image/png;base64,AA==");
        assert!(!host.editor_state().editor_ui.image_panel.search_open);
        // The write is undoable (commit_history ran).
        assert!(host.editor_state_mut().undo());
        let node = host.editor_state().selected_node();
        if let Some(jian_ops_schema::node::PenNode::Image(image)) = node {
            assert_eq!(image.src, "https://x/y.png");
        }
    }

    #[test]
    fn generate_gates_on_configured_profile() {
        let mut host = host_with_image_selected();
        host.apply_property_action(A::ToggleImageGeneratePopover);
        host.apply_property_action(A::RunImageGenerate);
        // No profile → stays idle (UI shows the not-configured view).
        assert_eq!(
            host.editor_state().editor_ui.image_panel.generate_phase,
            ImageGeneratePhase::Idle
        );
        // Configure a profile → generation kicks off.
        let id = host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .add_image_gen_profile();
        if let Some(p) = host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .image_gen_profiles
            .iter_mut()
            .find(|p| p.id == id)
        {
            p.api_key = "sk-test".into();
        }
        host.apply_property_action(A::RunImageGenerate);
        let panel = &host.editor_state().editor_ui.image_panel;
        assert_eq!(panel.generate_phase, ImageGeneratePhase::Loading);
        assert_eq!(panel.generate_epoch, 1);
    }

    #[test]
    fn open_settings_routes_to_images_tab() {
        let mut host = host_with_image_selected();
        host.apply_property_action(A::ToggleImageGeneratePopover);
        host.apply_property_action(A::OpenImageGenSettings);
        assert!(host.editor_state().editor_ui.agent_settings_open);
        assert_eq!(
            host.editor_state().editor_ui.agent_settings.tab,
            op_editor_core::agent_settings::AgentSettingsTab::Images
        );
        assert!(!host.editor_state().editor_ui.image_panel.generate_open);
    }

    #[test]
    fn relink_queues_the_file_action() {
        let mut host = host_with_image_selected();
        host.apply_property_action(A::RelinkImage);
        assert_eq!(
            host.editor_state().editor_ui.pending_file_action,
            Some(op_editor_core::editor_ui_state::FileAction::RelinkImage)
        );
    }

    #[test]
    fn popover_keystrokes_route_into_the_open_input() {
        let mut host = host_with_image_selected();
        assert!(!host.apply_image_panel_text('x'));
        host.apply_property_action(A::ToggleImageSearchPopover);
        host.editor_state_mut()
            .editor_ui
            .image_panel
            .search_query
            .set_text("");
        assert!(host.apply_image_panel_text('c'));
        assert!(host.apply_image_panel_text('a'));
        assert!(host.apply_image_panel_backspace());
        assert_eq!(
            host.editor_state()
                .editor_ui
                .image_panel
                .search_query
                .text(),
            "c"
        );
        // Enter submits the search.
        host.editor_state_mut().ui.text_editing =
            Some(op_editor_core::NodeId::new("independently-stale-text-edit"));
        let epoch = host.editor_state().editor_ui.image_panel.search_epoch;
        assert!(host.apply_send());
        assert!(host.editor_state().editor_ui.image_panel.search_loading);
        assert_eq!(
            host.editor_state().editor_ui.image_panel.search_epoch,
            epoch + 1
        );
    }

    #[test]
    fn popover_input_supports_middle_edit_delete_and_select_all() {
        let mut host = host_with_image_selected();
        host.apply_property_action(A::ToggleImageSearchPopover);
        let input = &mut host.editor_state_mut().editor_ui.image_panel.search_query;
        input.set_text("abcd");
        input.set_caret(2, 0);

        assert!(host.apply_image_panel_text('你'));
        assert_eq!(
            host.editor_state()
                .editor_ui
                .image_panel
                .search_query
                .text(),
            "ab你cd"
        );
        assert!(host.apply_image_panel_backspace());
        assert_eq!(
            host.editor_state()
                .editor_ui
                .image_panel
                .search_query
                .text(),
            "abcd"
        );
        assert!(host.apply_image_panel_delete());
        assert_eq!(
            host.editor_state()
                .editor_ui
                .image_panel
                .search_query
                .text(),
            "abd"
        );

        assert!(host.apply_select_all());
        assert!(host.apply_image_panel_text('x'));
        assert_eq!(
            host.editor_state()
                .editor_ui
                .image_panel
                .search_query
                .text(),
            "x"
        );

        let input = &mut host.editor_state_mut().editor_ui.image_panel.search_query;
        input.set_text("a你b");
        input.set_caret("a你b".len(), 0);
        assert!(host.apply_image_panel_caret(false, true));
        assert!(host.apply_image_panel_caret(false, true));
        assert_eq!(host.input_copy_text().as_deref(), Some("你b"));
        assert_eq!(host.input_cut_text().as_deref(), Some("你b"));
        assert_eq!(
            host.editor_state()
                .editor_ui
                .image_panel
                .search_query
                .text(),
            "a"
        );
    }

    #[test]
    fn open_popover_owns_shortcuts_and_blurs_stale_chat_focus() {
        let mut host = host_with_image_selected();
        host.editor_state_mut().chat.focused = true;
        host.apply_property_action(A::ToggleImageSearchPopover);
        assert!(!host.editor_state().chat.focused);
        assert!(host.input_active_pub());

        let selected = host.editor_state().selection.anchor.clone();
        assert!(host.apply_delete(), "Delete is consumed by the query");
        assert_eq!(host.editor_state().selection.anchor, selected);
        assert!(host.editor_state().selected_node().is_some());

        // Native menu accelerators call these host methods directly instead
        // of passing through DesktopApp's keydown guard. They must still not
        // mutate history while the query owns the keyboard.
        let snapshot = host.editor_state().snapshot_for_history();
        host.editor_state_mut().history_push_past(snapshot);
        let past_len = host.editor_state().history.past.len();
        assert!(!host.apply_undo());
        assert_eq!(host.editor_state().history.past.len(), past_len);
    }

    #[test]
    fn hidden_generate_view_does_not_expose_stale_text_selection() {
        let mut host = host_with_image_selected();
        host.editor_state_mut().chat.focused = true;
        host.editor_state_mut().chat.set_input_text("stale chat");
        host.editor_state_mut().chat.input.select_all();
        host.editor_state_mut().editor_ui.image_panel.generate_open = true;

        assert!(host.input_active_pub());
        assert!(host.input_copy_text().is_none());
        assert_eq!(host.editor_state().chat.input.text(), "stale chat");
    }

    #[test]
    fn image_popover_beats_and_clears_stale_git_focus() {
        let mut host = host_with_image_selected();
        {
            let git = &mut host.editor_state_mut().editor_ui.git_panel;
            git.open = true;
            git.commit_focused = true;
            git.commit_input.set_text("commit message");
        }
        host.apply_property_action(A::ToggleImageSearchPopover);
        assert!(!host.editor_state().editor_ui.git_panel.commit_focused);

        // Even an independently stale bit cannot split text routing from the
        // clipboard/IME resolver: the painted image overlay wins.
        host.editor_state_mut().editor_ui.git_panel.commit_focused = true;
        assert!(host.apply_text('x'));
        assert_eq!(
            host.editor_state()
                .editor_ui
                .image_panel
                .search_query
                .text(),
            "Hero photox"
        );
        assert_eq!(
            host.editor_state().editor_ui.git_panel.commit_input.text(),
            "commit message"
        );

        host.editor_state_mut()
            .editor_ui
            .image_panel
            .search_query
            .set_text("query");
        assert!(host.apply_select_all());
        assert_eq!(
            host.editor_state()
                .editor_ui
                .image_panel
                .search_query
                .highlight_range(),
            Some((0, 5))
        );
        assert!(
            host.editor_state()
                .editor_ui
                .git_panel
                .commit_input
                .highlight_range()
                .is_none(),
            "stale Git focus must not win Cmd+A"
        );
    }

    #[test]
    fn unconfigured_generate_view_swallows_keys_without_editing_hidden_prompt() {
        let mut host = host_with_image_selected();
        host.apply_property_action(A::ToggleImageGeneratePopover);
        let before = host
            .editor_state()
            .editor_ui
            .image_panel
            .generate_prompt
            .text()
            .to_owned();

        assert!(!host.text_input_focus_active());
        assert!(host.apply_text('x'));
        assert!(host.apply_backspace());
        assert!(host.apply_delete());
        assert_eq!(
            host.editor_state()
                .editor_ui
                .image_panel
                .generate_prompt
                .text(),
            before
        );
    }

    #[test]
    fn image_search_input_uses_text_cursor() {
        let mut host = host_with_image_selected();
        host.apply_property_action(A::ToggleImageSearchPopover);
        let viewport_w = 1200.0;
        let viewport_h = 800.0;
        let property_rect = host.property_rect(viewport_w, viewport_h);
        let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
        let caret = panel
            .image_popover_input_caret_rect(property_rect)
            .expect("search caret");
        assert_eq!(
            host.cursor_hint(
                caret.origin.x + 0.5,
                caret.origin.y + caret.size.y / 2.0,
                viewport_w,
                viewport_h,
            ),
            CursorHint::Text
        );
    }
}
