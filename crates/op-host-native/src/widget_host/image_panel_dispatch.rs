//! Image-node section dispatch — Search / Generate popover toggles,
//! search submit / result select, generate lifecycle, and the
//! local-asset Relink intent (TS `image-section.tsx` +
//! `image-search-popover.tsx` + `image-generate-popover.tsx`).

use super::WidgetHostNative;
use jian_ops_schema::node::PenNode;
use op_editor_core::image_panel_state::ImageGeneratePhase;
use op_editor_ui::widgets::PropertyPanel;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// Seed for the search query / generate prompt: the node's
    /// authored `imageSearchQuery` / `imagePrompt`, else its name
    /// (TS `node.imageSearchQuery ?? node.name ?? ''`).
    fn selected_image_seed(&self, prompt: bool) -> String {
        match self.editor_state.selected_node() {
            Some(PenNode::Image(image)) => {
                let authored = if prompt {
                    image.image_prompt.as_deref()
                } else {
                    image.image_search_query.as_deref()
                };
                authored
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .or_else(|| image.base.name.clone())
                    .unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    pub(in crate::widget_host) fn toggle_image_search_popover(&mut self) {
        let opening = !self.editor_state.editor_ui.image_panel.search_open;
        let seed = self.selected_image_seed(false);
        let panel = &mut self.editor_state.editor_ui.image_panel;
        // Opening either popover closes the other (TS popovers are
        // mutually exclusive portals).
        panel.close_popovers();
        if opening {
            panel.search_open = true;
            panel.search_query = seed;
        }
        self.close_other_property_popovers_for_image();
    }

    pub(in crate::widget_host) fn toggle_image_generate_popover(&mut self) {
        let opening = !self.editor_state.editor_ui.image_panel.generate_open;
        let seed = self.selected_image_seed(true);
        let panel = &mut self.editor_state.editor_ui.image_panel;
        panel.close_popovers();
        if opening {
            // TS handleOpenChange: reset prompt + state on open.
            panel.generate_open = true;
            panel.generate_prompt = seed;
            panel.generate_phase = ImageGeneratePhase::Idle;
            panel.generate_preview = None;
            panel.generate_error.clear();
        }
        self.close_other_property_popovers_for_image();
    }

    /// Close the property pickers that would overlap the popovers.
    fn close_other_property_popovers_for_image(&mut self) {
        let ui = &mut self.editor_state.editor_ui;
        ui.close_fill_type_picker();
        ui.image_fill_popover_open = false;
        ui.font_weight_picker_open = false;
        ui.export_scale_picker_open = false;
        ui.export_format_picker_open = false;
        ui.property_color_variable_picker_open = None;
        self.close_font_picker();
    }

    /// Submit the search box (Enter / the icon button). No-op while a
    /// search is in flight or the query is blank (TS disables the
    /// button on both).
    pub(in crate::widget_host) fn run_image_search(&mut self) {
        let panel = &mut self.editor_state.editor_ui.image_panel;
        if !panel.search_open || panel.search_loading || panel.search_query.trim().is_empty() {
            return;
        }
        panel.search_loading = true;
        panel.search_has_searched = true;
        panel.search_epoch = panel.search_epoch.wrapping_add(1);
    }

    /// Write the clicked result's thumbnail into the node's `src`
    /// (TS `onSelect(result.thumbUrl)`) and close the popover.
    pub(in crate::widget_host) fn select_image_search_result(&mut self, index: usize) {
        let Some(url) = self
            .editor_state
            .editor_ui
            .image_panel
            .search_results
            .get(index)
            .map(|hit| hit.thumb_data_url.as_ref().clone())
        else {
            return;
        };
        self.write_selected_image_src(&url);
        self.editor_state.editor_ui.image_panel.close_popovers();
    }

    /// Kick off generation (drained by the desktop pump). The
    /// not-configured gate lives in the popover view; this also
    /// guards so a stale press can't start a job with no profile.
    pub(in crate::widget_host) fn run_image_generate(&mut self) {
        let configured = {
            let settings = &self.editor_state.editor_ui.agent_settings;
            settings
                .image_gen_profiles
                .iter()
                .find(|p| Some(&p.id) == settings.active_image_gen_profile_id.as_ref())
                .or_else(|| settings.image_gen_profiles.first())
                .is_some_and(|p| !p.api_key.trim().is_empty())
        };
        let panel = &mut self.editor_state.editor_ui.image_panel;
        if !panel.generate_open
            || !configured
            || panel.generate_prompt.trim().is_empty()
            || panel.generate_phase == ImageGeneratePhase::Loading
        {
            return;
        }
        panel.generate_phase = ImageGeneratePhase::Loading;
        panel.generate_error.clear();
        panel.generate_preview = None;
        panel.generate_epoch = panel.generate_epoch.wrapping_add(1);
    }

    pub(in crate::widget_host) fn apply_generated_image(&mut self) {
        let Some(url) = self
            .editor_state
            .editor_ui
            .image_panel
            .generate_preview
            .as_ref()
            .map(|u| u.as_ref().clone())
        else {
            return;
        };
        self.write_selected_image_src(&url);
        self.editor_state.editor_ui.image_panel.close_popovers();
    }

    pub(in crate::widget_host) fn retry_image_generate(&mut self) {
        let panel = &mut self.editor_state.editor_ui.image_panel;
        panel.generate_phase = ImageGeneratePhase::Idle;
        panel.generate_preview = None;
        panel.generate_error.clear();
    }

    /// Open the settings modal on the Images tab (TS
    /// `setDialogOpen(true)` from the not-configured view).
    pub(in crate::widget_host) fn open_image_gen_settings(&mut self) {
        self.editor_state.editor_ui.image_panel.close_popovers();
        self.editor_state.editor_ui.agent_settings_open = true;
        self.editor_state.editor_ui.agent_settings.tab =
            op_editor_core::agent_settings::AgentSettingsTab::Images;
    }

    /// Commit `src` onto the selected image node (with history).
    pub(in crate::widget_host) fn write_selected_image_src(&mut self, src: &str) {
        let id = self.editor_state.selection.anchor.clone();
        if !id.is_real() || src.is_empty() {
            return;
        }
        self.editor_state.commit_history();
        if let Some(PenNode::Image(image)) =
            op_editor_core::walkers::find_node_mut(self.editor_state.active_children_mut(), &id)
        {
            image.src = src.into();
        }
        self.mark_editor_state_dirty();
    }

    /// Route a printable char into whichever popover input is open.
    pub(in crate::widget_host) fn apply_image_panel_text(&mut self, c: char) -> bool {
        if c.is_control() {
            return false;
        }
        let panel = &mut self.editor_state.editor_ui.image_panel;
        if panel.search_open {
            panel.search_query.push(c);
            self.mark_dirty();
            return true;
        }
        if panel.generate_open {
            if panel.generate_phase == ImageGeneratePhase::Loading {
                // Swallow typing during generation (no input painted).
                return true;
            }
            panel.generate_prompt.push(c);
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Backspace in the open popover's input. Swallows the key even
    /// on an empty draft so it can't fall through to node deletion.
    pub(in crate::widget_host) fn apply_image_panel_backspace(&mut self) -> bool {
        let panel = &mut self.editor_state.editor_ui.image_panel;
        if panel.search_open {
            if panel.search_query.pop().is_some() {
                self.mark_dirty();
            }
            return true;
        }
        if panel.generate_open {
            if panel.generate_phase != ImageGeneratePhase::Loading
                && panel.generate_prompt.pop().is_some()
            {
                self.mark_dirty();
            }
            return true;
        }
        false
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
        if self.editor_state.editor_ui.image_panel.generate_open {
            return true;
        }
        false
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
            if panel.image_popovers_contain(rect, point) {
                // Inside the popup body (input / textarea / empty
                // state) — swallow, keep open.
                return true;
            }
        }
        self.editor_state.editor_ui.image_panel.close_popovers();
        self.mark_dirty();
        true
    }
}

#[cfg(test)]
mod tests {
    use op_editor_core::image_panel_state::{ImageGeneratePhase, ImageSearchHit};
    use op_editor_core::EditorState;
    use op_editor_ui::widgets::PropertyPanelAction as A;
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
        assert_eq!(panel.search_query, "Hero photo");
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
            .clear();
        assert!(host.apply_image_panel_text('c'));
        assert!(host.apply_image_panel_text('a'));
        assert!(host.apply_image_panel_backspace());
        assert_eq!(host.editor_state().editor_ui.image_panel.search_query, "c");
        // Enter submits the search.
        assert!(host.apply_image_panel_send());
        assert!(host.editor_state().editor_ui.image_panel.search_loading);
    }
}
