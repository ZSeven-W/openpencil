//! Inherent methods on [`EditorUiState`] — recent-file bookkeeping,
//! Preview (Play) mode transitions, the picker open / close / toggle
//! helpers, agent-settings draft readers, and `clear_document_derived`.
//!
//! Split out of the `editor_ui_state` spine (800-line file ceiling).

use super::{
    EditorUiState, FontPickerPurpose, Locale, MissingFontSurface, RecentFile, SceneTemplateFocus,
    ThemeMode, RECENT_FILE_CAP,
};

impl EditorUiState {
    /// A fresh UI state — sidebar open, dark theme, no menus open.
    pub fn new() -> Self {
        Self::default()
    }

    /// Theme currently painted by the editor. An embedding host may override
    /// the user's stored preference for this page without mutating it.
    pub fn effective_theme_mode(&self) -> ThemeMode {
        self.host_theme_override.unwrap_or(self.theme_mode)
    }

    /// Set or clear a page-lifetime embedding-host theme override.
    pub fn set_host_theme_override(&mut self, theme: Option<ThemeMode>) {
        self.host_theme_override = theme;
    }

    /// Locale currently presented by the editor. An embedding host may
    /// override the user's stored preference for this page without mutating it.
    pub fn effective_locale(&self) -> Locale {
        self.host_locale_override.unwrap_or(self.locale)
    }

    /// Set or clear a page-lifetime embedding-host locale override.
    pub fn set_host_locale_override(&mut self, locale: Option<Locale>) {
        self.host_locale_override = locale;
    }

    pub fn clear_button_press_target(&mut self) {
        self.pressed_button = None;
    }

    /// Record which TopBar button the cursor rests on, keeping the
    /// tooltip dwell clock in step. Returns whether anything changed —
    /// hosts use that as their repaint signal.
    ///
    /// The clock starts only when the cursor ENTERS the button row
    /// (`None` → `Some`). Sliding from one button to the next inside a
    /// single visit keeps the original stamp, so the tooltip follows
    /// the cursor immediately instead of making the user wait again at
    /// every button.
    pub fn set_topbar_button_hover(
        &mut self,
        next: Option<crate::topbar_state::TopBarButton>,
        now_ms: u64,
    ) -> bool {
        if self.topbar_button_hover == next {
            return false;
        }
        if self.topbar_button_hover.is_none() {
            self.topbar_hover_since_ms = Some(now_ms);
        }
        self.topbar_button_hover = next;
        true
    }

    /// Record whether the cursor rests on the pinned-style chip, starting or
    /// stopping its detail card's dwell clock. Returns whether anything
    /// changed — hosts use that as their repaint signal.
    ///
    /// Unlike the top bar's row-wide clock this restarts on every entry,
    /// because there is only one chip to enter: there is no neighbouring
    /// target for an already-earned card to follow the cursor onto.
    pub fn set_chat_style_chip_hover(&mut self, hovering: bool, now_ms: u64) -> bool {
        match (self.chat_style_chip_hover_since_ms, hovering) {
            (None, true) => {
                self.chat_style_chip_hover_since_ms = Some(now_ms);
                true
            }
            (Some(_), false) => {
                self.chat_style_chip_hover_since_ms = None;
                true
            }
            _ => false,
        }
    }

    /// Drop the pinned-style chip's hover, so its card cannot outlive the
    /// panel it hangs over. The clear paths call this when a surface painted
    /// above the chat takes the cursor; they carry no clock, and none is
    /// needed to stop one.
    pub fn clear_chat_style_chip_hover(&mut self) -> bool {
        self.chat_style_chip_hover_since_ms.take().is_some()
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
        self.preview.slideshow = None;
        self.preview.toolbar_hover = None;
        self.preview.toolbar_pressed = None;
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
        self.scene_template_center
            .open(now_ms, !self.touch_chrome());
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

    /// Close the style-import layer and resolve keyboard ownership for the
    /// current input mode. Pointer-driven desktop keeps its search focus;
    /// touch returns to browsing until the user taps a field again.
    pub fn close_scene_template_style_import(&mut self) -> bool {
        let changed = self.scene_template_center.close_style_import();
        if changed && self.touch_chrome() {
            self.scene_template_center.input_focus_active = false;
        }
        changed
    }

    /// Aim the generate row at a template: pin its style guide, narrow the
    /// grid to its scene, and put the caret in the topic field.
    ///
    /// Three writes, one gesture, because the user asked one question — "make
    /// me something like this" — and answering it in three places they then
    /// have to find would be the same work done by hand. The pin is the part
    /// that actually reaches the pipeline (`resolve_pinned_style_guide`
    /// short-circuits the whole style menu to it); the filter and the focus
    /// are so the panel now looks like what it is about to do.
    ///
    /// Returns whether anything moved.
    pub fn use_scene_template_as_generate_basis(
        &mut self,
        template: &crate::scene_template_catalog::SceneTemplateDefinition,
    ) -> bool {
        let Some(guide) = template.generate_style_guide() else {
            return false;
        };
        let mut changed = self.pinned_style_guide.as_deref() != Some(guide);
        self.pinned_style_guide = Some(guide.to_string());

        let center = &mut self.scene_template_center;
        let filter = crate::editor_ui_state::SceneFilter::Scene(template.scene);
        if center.filter != filter {
            center.filter = filter;
            // The grid reorders under the pointer, so a retained scroll or
            // hover index would name a different card than the one just
            // pressed — same reason the filter chips reset both.
            center.scroll.offset = 0.0;
            center.hover = None;
            changed = true;
        }
        changed |= center.focus != SceneTemplateFocus::Generate;
        center.focus = SceneTemplateFocus::Generate;
        changed |= !center.input_focus_active;
        center.input_focus_active = true;
        changed |= center.generate_basis.as_deref() != Some(template.id.as_str());
        center.generate_basis = Some(template.id.clone());
        changed
    }

    /// Unpin the style guide, wherever the user pressed to do it.
    ///
    /// One entry point on purpose. The pin is reachable from two surfaces —
    /// the Asset Center card and the chat panel's receipt row — and a second
    /// implementation is how the two would drift into disagreeing about what
    /// "unpinned" means. Returns whether anything changed.
    pub fn clear_pinned_style_guide(&mut self) -> bool {
        if self.pinned_style_guide.take().is_none() {
            return false;
        }
        // The Asset Center's basis chip is a label for a pin that no longer
        // exists; leaving it would name a style nothing is using.
        self.scene_template_center.generate_basis = None;
        true
    }

    /// Drop the generate row's basis chip and the pin it set.
    ///
    /// Both, because the chip is the only visible trace of the pin while the
    /// Templates tab is showing: clearing the label and leaving the guide
    /// pinned would keep steering every later generation with nothing on
    /// screen saying so.
    pub fn clear_scene_template_generate_basis(&mut self) -> bool {
        let center = &mut self.scene_template_center;
        if center.generate_basis.take().is_none() {
            return false;
        }
        self.pinned_style_guide = None;
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
            // Raised here rather than in the click flow so every path that
            // opens the picker can ask its host to refresh CLI catalogs.
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
        let Some(base_url) = self.builtin_agent_draft_field_text(BuiltinAgentField::BaseUrl) else {
            return false;
        };
        !name.trim().is_empty() && !api_key.trim().is_empty() && !base_url.trim().is_empty()
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

impl EditorUiState {
    /// Phone bottom-sheet layout. Kept as a narrow compatibility helper;
    /// tablet and input-density decisions must use the explicit predicates
    /// below so Medium never silently collapses into Compact again.
    pub fn mobile_layout(&self) -> bool {
        self.touch && self.size_class.is_compact()
    }

    /// Native touch chrome is shared by phone and tablet players. Layout
    /// geometry still branches on the three size classes.
    pub fn touch_chrome(&self) -> bool {
        self.touch
    }

    pub fn compact_layout(&self) -> bool {
        self.touch && self.size_class.is_compact()
    }

    pub fn medium_layout(&self) -> bool {
        self.touch && self.size_class.is_medium()
    }

    pub fn expanded_touch_layout(&self) -> bool {
        self.touch && self.size_class.is_expanded()
    }
}
