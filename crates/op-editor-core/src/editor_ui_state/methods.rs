//! Inherent methods on [`EditorUiState`] — recent-file bookkeeping,
//! Preview (Play) mode transitions, the picker open / close / toggle
//! helpers, agent-settings draft readers, and `clear_document_derived`.
//!
//! Split out of the `editor_ui_state` spine (800-line file ceiling).

use super::{EditorUiState, FontPickerPurpose, MissingFontSurface, RecentFile, RECENT_FILE_CAP};

impl EditorUiState {
    /// A fresh UI state — sidebar open, dark theme, no menus open.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear_button_press_target(&mut self) {
        self.pressed_button = None;
    }

    pub fn touch_recent_file(&mut self, path: String, modified_at: u64) {
        self.recent_files.retain(|recent| recent.path != path);
        self.recent_files
            .insert(0, RecentFile { path, modified_at });
        self.recent_files.truncate(RECENT_FILE_CAP);
    }

    pub fn remove_recent_file(&mut self, path: &str) -> bool {
        let before = self.recent_files.len();
        self.recent_files.retain(|recent| recent.path != path);
        self.recent_files.len() != before
    }

    /// Enter canvas Preview (Play) mode. The host follows up by
    /// building the jian runtime from the (unchanged) document; this
    /// only flips the flag + clears any stale warnings so the host
    /// knows to build a fresh runtime. Idempotent.
    pub fn enter_preview(&mut self) {
        self.preview.mode = true;
        self.preview.warnings.clear();
    }

    /// Exit Preview mode. The host follows up by dropping the runtime.
    /// Clears the warning list. Idempotent.
    pub fn exit_preview(&mut self) {
        self.preview.mode = false;
        self.preview.warnings.clear();
        self.preview.device = None;
        self.preview.switcher_hover = None;
        self.preview.switcher_pressed = None;
        self.preview.screen_switcher_hover = None;
        self.preview.screen_switcher_pressed = None;
    }

    /// Flip Preview mode on/off. Returns the new state (`true` =
    /// now in Preview).
    pub fn toggle_preview(&mut self) -> bool {
        if self.preview.mode {
            self.exit_preview();
        } else {
            self.enter_preview();
        }
        self.preview.mode
    }

    pub fn button_pressed(&self, target: crate::button_press_state::ButtonPressTarget) -> bool {
        self.pressed_button == Some(target)
    }

    /// Open the Prompt Center with its search field focused.
    pub fn open_prompt_center(&mut self, now_ms: u64) {
        self.close_scene_template_center();
        self.close_icon_picker();
        self.close_chat_model_picker();
        self.close_parallel_agents_picker();
        self.prompt_center.open(now_ms);
    }

    /// Close the Prompt Center and clear its transient interaction state.
    pub fn close_prompt_center(&mut self) -> bool {
        if !self.prompt_center.open {
            return false;
        }
        self.prompt_center.close();
        if matches!(
            self.pressed_button,
            Some(crate::button_press_state::ButtonPressTarget::PromptCenter(
                _
            ))
        ) {
            self.pressed_button = None;
        }
        true
    }

    /// Open the Scene Template Center with its search field focused.
    ///
    /// Closes the Prompt Center: both are full-size centred panels, so
    /// leaving one open behind the other would stack two card grids the user
    /// cannot see past.
    pub fn open_scene_template_center(&mut self, now_ms: u64) {
        self.close_prompt_center();
        self.close_icon_picker();
        self.close_chat_model_picker();
        self.close_parallel_agents_picker();
        self.scene_template_center.open(now_ms);
    }

    /// Close the Scene Template Center and clear its transient state.
    pub fn close_scene_template_center(&mut self) -> bool {
        if !self.scene_template_center.open {
            return false;
        }
        self.scene_template_center.close();
        if matches!(
            self.pressed_button,
            Some(crate::button_press_state::ButtonPressTarget::SceneTemplate(
                _
            ))
        ) {
            self.pressed_button = None;
        }
        true
    }

    /// Toggle the Effects "+" add-menu. The corner-radius editor and
    /// effect menu are mutually exclusive inspector overlays.
    pub fn toggle_effect_add_picker(&mut self) {
        self.effect_add_picker_open = !self.effect_add_picker_open;
        if self.effect_add_picker_open {
            self.corner_expand_open = false;
        }
        self.effect_add_menu_hover = None;
    }

    pub fn toggle_corner_expand(&mut self) {
        self.corner_expand_open = !self.corner_expand_open;
        if self.corner_expand_open {
            self.close_effect_add_picker();
        }
    }

    pub fn close_corner_expand(&mut self) -> bool {
        let was = self.corner_expand_open;
        self.corner_expand_open = false;
        was
    }

    /// Close the Effects add-menu. Returns true when it was open (so
    /// the host knows a repaint / press-swallow is needed).
    pub fn close_effect_add_picker(&mut self) -> bool {
        let was = self.effect_add_picker_open;
        self.effect_add_picker_open = false;
        self.effect_add_menu_hover = None;
        was
    }

    /// Toggle the Interactions section's Navigate/Back/Remove popover.
    pub fn toggle_interaction_menu(&mut self) {
        self.interaction_menu_open = !self.interaction_menu_open;
        self.interaction_menu_hover = None;
    }

    /// Close the Interactions popover. Returns true when it was open.
    pub fn close_interaction_menu(&mut self) -> bool {
        let was = self.interaction_menu_open;
        self.interaction_menu_open = false;
        self.interaction_menu_hover = None;
        was
    }

    pub fn toggle_instance_component_picker(&mut self, anchor: &str) {
        let opening =
            !self.instance_component_picker_open || self.instance_component_picker_anchor != anchor;
        self.instance_component_picker_open = opening;
        if opening {
            self.instance_component_picker_anchor.clear();
            self.instance_component_picker_anchor.push_str(anchor);
        } else {
            self.instance_component_picker_anchor.clear();
        }
    }

    pub fn close_instance_component_picker(&mut self) -> bool {
        let was_open = self.instance_component_picker_open;
        self.instance_component_picker_open = false;
        self.instance_component_picker_anchor.clear();
        was_open
    }

    pub fn toggle_fill_type_picker(&mut self) {
        self.toggle_fill_type_picker_for(0);
    }

    /// Toggle the fill-type dropdown for fill `index`. Opening on a
    /// different row than the one currently open re-targets the picker
    /// (stays open, new index); toggling the same row closes it.
    pub fn toggle_fill_type_picker_for(&mut self, index: usize) {
        let opening = !(self.fill_type_picker.open && self.fill_type_picker_index == index);
        self.fill_type_picker.open = opening;
        self.fill_type_picker_index = index;
        self.fill_type_picker.hover = None;
        self.fill_type_picker.pressed = None;
        if opening {
            self.fill_type_picker.scroll.offset = 0.0;
        }
    }

    pub fn close_fill_type_picker(&mut self) -> bool {
        let changed = self.fill_type_picker.open
            || self.fill_type_picker.hover.is_some()
            || self.fill_type_picker.pressed.is_some()
            || self.fill_type_picker.scroll.offset != 0.0;
        self.fill_type_picker.open = false;
        self.fill_type_picker.hover = None;
        self.fill_type_picker.pressed = None;
        self.fill_type_picker.scroll.offset = 0.0;
        changed
    }

    /// Close the fill / stroke colour-variable popup, dropping the row
    /// hover and list scroll with it so the next open starts clean.
    /// Returns whether anything changed.
    pub fn close_color_variable_picker(&mut self) -> bool {
        let changed = self.property_color_variable_picker_open.is_some()
            || self.property_color_variable_picker_hover.is_some()
            || self.property_color_variable_picker_scroll.offset != 0.0;
        self.property_color_variable_picker_open = None;
        self.property_color_variable_picker_hover = None;
        self.property_color_variable_picker_scroll.offset = 0.0;
        changed
    }

    pub fn open_icon_picker(&mut self, replace_selection: bool) {
        self.close_prompt_center();
        self.close_icon_picker();
        self.icon_picker.open = true;
        self.icon_picker_replace_selection = replace_selection;
    }

    pub fn close_icon_picker(&mut self) -> bool {
        let changed = self.icon_picker.open
            || self.icon_picker.hover.is_some()
            || self.icon_picker.pressed.is_some()
            || self.icon_picker.scroll.offset != 0.0
            || self.icon_picker_replace_selection
            || !self.icon_picker_search.is_empty()
            || self.icon_picker_select_all;
        self.icon_picker.open = false;
        self.icon_picker.hover = None;
        self.icon_picker.pressed = None;
        self.icon_picker.scroll.offset = 0.0;
        self.icon_picker_replace_selection = false;
        self.icon_picker_search.clear();
        self.icon_picker_select_all = false;
        changed
    }

    pub fn toggle_font_picker(&mut self) {
        let opening = !self.font_picker.open
            || self.font_picker_purpose != Some(FontPickerPurpose::PropertyText);
        self.close_font_picker();
        if opening {
            self.font_picker.open = true;
            self.font_picker_purpose = Some(FontPickerPurpose::PropertyText);
        }
    }

    pub fn open_missing_font_picker(&mut self, row: usize, surface: MissingFontSurface) {
        self.close_font_picker();
        self.font_picker.open = true;
        self.font_picker_purpose = Some(FontPickerPurpose::MissingFont { row, surface });
    }

    pub fn close_font_picker(&mut self) -> bool {
        let changed = self.font_picker.open
            || self.font_picker_purpose.is_some()
            || self.font_picker.hover.is_some()
            || self.font_picker.pressed.is_some()
            || self.font_picker.scroll.offset != 0.0
            || !self.font_picker_search.is_empty();
        self.font_picker.open = false;
        self.font_picker_purpose = None;
        self.font_picker.hover = None;
        self.font_picker.pressed = None;
        self.font_picker.scroll.offset = 0.0;
        self.font_picker_import_hover = false;
        self.font_picker_search.clear();
        changed
    }

    pub fn toggle_chat_model_picker(&mut self) -> bool {
        let opening = !self.chat_model_picker.open;
        self.close_chat_model_picker();
        if opening {
            self.chat_model_picker.open = true;
            // Raised here rather than in the click flow so every path
            // that opens the picker asks for a fresh catalog — the
            // request is only a request, and the host that drains it
            // decides (via TTL) whether a probe actually runs.
            self.pending_model_catalog_refresh = true;
        }
        opening
    }

    /// Consume the model-catalog refresh request raised by the last
    /// picker open. Returns true exactly once per open.
    pub fn take_pending_model_catalog_refresh(&mut self) -> bool {
        std::mem::take(&mut self.pending_model_catalog_refresh)
    }

    pub fn close_chat_model_picker(&mut self) -> bool {
        let changed = self.chat_model_picker.open
            || self.chat_model_picker.hover.is_some()
            || self.chat_model_picker.pressed.is_some()
            || self.chat_model_picker.scroll.offset != 0.0
            || !self.chat_model_picker_input.text().is_empty();
        self.chat_model_picker.open = false;
        self.chat_model_picker.hover = None;
        self.chat_model_picker.pressed = None;
        self.chat_model_picker.scroll.offset = 0.0;
        self.chat_model_picker_input.set_text("");
        changed
    }

    /// Toggle the Parallel Agents picker open/closed. Returns `true` if it is
    /// now open (i.e. was previously closed and just opened).
    pub fn toggle_parallel_agents_picker(&mut self) -> bool {
        let opening = !self.parallel_agents_picker_open;
        self.parallel_agents_picker_open = opening;
        if !opening {
            self.parallel_agents_picker_hover = None;
        }
        opening
    }

    /// Close the Parallel Agents picker and clear all interaction state.
    /// Returns `true` when something changed (so the host can skip a repaint
    /// when the picker was already closed).
    pub fn close_parallel_agents_picker(&mut self) -> bool {
        let changed =
            self.parallel_agents_picker_open || self.parallel_agents_picker_hover.is_some();
        self.parallel_agents_picker_open = false;
        self.parallel_agents_picker_hover = None;
        changed
    }

    /// Whether the preset dropdown's save-as-name input owns the
    /// keyboard. Gated on the menu being open so a stale focus flag
    /// (e.g. the menu closed by a panel-side `close_variable_menus`)
    /// can never eat keystrokes.
    pub fn preset_name_input_active(&self) -> bool {
        self.variables_preset_menu_open && self.variables_preset_name_focus
    }

    pub fn builtin_agent_draft_ready(&self) -> bool {
        use crate::agent_settings::BuiltinAgentField;

        let Some(name) = self.builtin_agent_draft_field_text(BuiltinAgentField::DisplayName) else {
            return false;
        };
        let Some(api_key) = self.builtin_agent_draft_field_text(BuiltinAgentField::ApiKey) else {
            return false;
        };
        let Some(model) = self.builtin_agent_draft_field_text(BuiltinAgentField::Model) else {
            return false;
        };
        !name.trim().is_empty() && !api_key.trim().is_empty() && !model.trim().is_empty()
    }

    pub fn acp_agent_draft_ready(&self) -> bool {
        use crate::agent_settings::{AcpAgentField, AcpConnectionType};

        let Some(draft) = self.agent_settings.acp_agent_draft.as_ref() else {
            return false;
        };
        let Some(name) = self.acp_agent_draft_field_text(AcpAgentField::DisplayName) else {
            return false;
        };
        let endpoint_field = match draft.connection_type {
            AcpConnectionType::Local => AcpAgentField::Command,
            AcpConnectionType::Remote => AcpAgentField::Url,
        };
        let Some(endpoint) = self.acp_agent_draft_field_text(endpoint_field) else {
            return false;
        };
        !name.trim().is_empty() && !endpoint.trim().is_empty()
    }

    pub fn builtin_agent_draft_field_text(
        &self,
        field: crate::agent_settings::BuiltinAgentField,
    ) -> Option<&str> {
        use crate::agent_settings::{BuiltinAgentField, SettingsFocus};

        let draft = self.agent_settings.builtin_agent_draft.as_ref()?;
        if self.agent_settings.focus == Some(SettingsFocus::BuiltinAgentDraft(field)) {
            return Some(self.settings_input.text());
        }
        Some(match field {
            BuiltinAgentField::DisplayName => draft.display_name.as_str(),
            BuiltinAgentField::ApiKey => draft.api_key.as_str(),
            BuiltinAgentField::Model => draft.model.as_str(),
            BuiltinAgentField::BaseUrl => draft.base_url.as_str(),
        })
    }

    pub fn acp_agent_draft_field_text(
        &self,
        field: crate::agent_settings::AcpAgentField,
    ) -> Option<std::borrow::Cow<'_, str>> {
        use crate::agent_settings::{AcpAgentField, SettingsFocus};

        let draft = self.agent_settings.acp_agent_draft.as_ref()?;
        if self.agent_settings.focus == Some(SettingsFocus::AcpAgentDraft(field)) {
            return Some(std::borrow::Cow::Borrowed(self.settings_input.text()));
        }
        Some(match field {
            AcpAgentField::DisplayName => std::borrow::Cow::Borrowed(draft.display_name.as_str()),
            AcpAgentField::Command => std::borrow::Cow::Borrowed(draft.command.as_str()),
            AcpAgentField::Args => std::borrow::Cow::Owned(draft.args_text()),
            AcpAgentField::Env => std::borrow::Cow::Owned(draft.env_text()),
            AcpAgentField::Url => std::borrow::Cow::Borrowed(draft.url.as_deref().unwrap_or("")),
        })
    }

    /// Clear transient UI state that references specific document nodes/pages
    /// or the (now-cleared) selection, so a wholesale document replacement
    /// ([`crate::EditorState::replace_document`]) can't leave hover highlights,
    /// an open layer context menu, collapsed-layer entries, alignment guides,
    /// or in-progress property edits pointing at nodes that no longer exist.
    /// Settings, theme/locale, panel sizes, recent files, the Git panel, and
    /// agent/MCP config are PRESERVED — they are not document-derived.
    pub fn clear_document_derived(&mut self) {
        self.hovered_layer_id = None;
        self.hovered_page_index = None;
        self.layer_context_menu = None;
        self.layer_pages_scroll.offset = 0.0;
        self.layer_layers_scroll.offset = 0.0;
        self.layer_pages_h_scroll.offset = 0.0;
        self.layer_layers_h_scroll.offset = 0.0;
        self.collapsed_layers.clear();
        self.last_layer_click = None;
        self.last_canvas_click = None;
        self.entered_container = None;
        self.active_guides.clear();
        self.padding_edit_mode = None;
        self.padding_edit_mode_anchor = String::new();
        self.padding_mode_popover_open = false;
        self.stroke_edit_mode = None;
        self.stroke_edit_mode_anchor = String::new();
        self.stroke_mode_popover_open = false;
        self.stroke_mode_popover_hover = None;
        self.close_color_variable_picker();
        self.image_crop_editing = None;
        self.axis_dropdown_open = None;
        self.variables_theme_rename_axis = None;
        self.variables_variant_rename_value = None;
        self.variables_header_input.set_text("");
        self.variable_row_focus = None;
        self.variable_row_input.set_text("");
        // Document-derived variables-panel transients: the search
        // filter, scroll offset and open row menu all reference the
        // replaced document's variable list. The panel SIZE is a panel
        // metric and is preserved.
        self.variables_search.clear();
        self.variables_search_focus = false;
        self.variables_scroll.offset = 0.0;
        self.variables_row_menu = None;
        self.design_md_panel.scroll.offset = 0.0;
        self.design_md_panel.generating = false;
        self.effect_param_focus = None;
        // Document-derived: set true only by a Figma import to keep that
        // document's authored absolute geometry. A replacement document must
        // not inherit it (it would force the new tree through the
        // preserve-geometry layout path) — matches file-open, which resets it
        // via a fresh `editor_ui`.
        self.preserve_authored_geometry = false;
    }
}
