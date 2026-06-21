//! Image-node property dispatch for the web host.
//!
//! Mirrors the native host's state-machine pieces: popover toggles,
//! search / generate epoch intents, result application, and the browser
//! Relink file-picker intent. Actual network / file IO is drained by
//! the web shell outside this dispatch layer.

use super::WidgetHost;
use jian_ops_schema::node::PenNode;
use op_editor_core::image_panel_state::ImageGeneratePhase;
use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
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
            panel.generate_open = true;
            panel.generate_prompt = seed;
            panel.generate_phase = ImageGeneratePhase::Idle;
            panel.generate_preview = None;
            panel.generate_error.clear();
        }
        self.close_other_property_popovers_for_image();
    }

    fn close_other_property_popovers_for_image(&mut self) {
        let ui = &mut self.editor_state.editor_ui;
        ui.close_fill_type_picker();
        ui.image_fill_popover_open = false;
        ui.font_weight_picker_open = false;
        ui.export_scale_picker_open = false;
        ui.export_format_picker_open = false;
        ui.property_color_variable_picker_open = None;
        ui.close_font_picker();
    }

    pub(in crate::widget_host) fn run_image_search(&mut self) {
        let panel = &mut self.editor_state.editor_ui.image_panel;
        if !panel.search_open || panel.search_loading || panel.search_query.trim().is_empty() {
            return;
        }
        panel.search_loading = true;
        panel.search_has_searched = true;
        panel.search_epoch = panel.search_epoch.wrapping_add(1);
    }

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

    pub(in crate::widget_host) fn open_image_gen_settings(&mut self) {
        self.editor_state.editor_ui.image_panel.close_popovers();
        self.editor_state.editor_ui.agent_settings_open = true;
        self.editor_state.editor_ui.agent_settings.tab =
            op_editor_core::agent_settings::AgentSettingsTab::Images;
    }

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
        self.mark_dirty();
    }

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
                return true;
            }
            panel.generate_prompt.push(c);
            self.mark_dirty();
            return true;
        }
        false
    }

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

    pub(in crate::widget_host) fn apply_image_panel_send(&mut self) -> bool {
        if self.editor_state.editor_ui.image_panel.search_open {
            self.run_image_search();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.image_panel.generate_open {
            return true;
        }
        false
    }

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
            let rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
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
                return true;
            }
        }
        self.editor_state.editor_ui.image_panel.close_popovers();
        self.mark_dirty();
        true
    }
}
