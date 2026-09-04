//! Escape-ladder steps shared by the native and web widget hosts.
//!
//! Escape dismisses exactly ONE layer per press. The two hosts run
//! *different* ladders (the native shell has surfaces the browser bundle
//! doesn't — preview mode, the pen tool, the Git panel's nested forms),
//! so the ORDER stays host-side in `widget_host/keyboard.rs` /
//! `widget_host/keyboard_escape.rs`. What each step *does* — which
//! flags, hover targets and drafts it clears — is single-sourced here so
//! the two ladders can never drift on the semantics of a shared rung.
//!
//! Every step returns `true` when it acted; the host then `mark_dirty()`s
//! and reports the key consumed.

use crate::editor_ui_state::EditorUiState;
use crate::state::EditorState;

impl EditorUiState {
    /// Close the export dialog.
    pub fn escape_export_dialog(&mut self) -> bool {
        if !self.export_dialog_open {
            return false;
        }
        self.export_dialog_open = false;
        self.export_dialog_hover = None;
        true
    }

    /// Close the TopBar file menu.
    pub fn escape_file_menu(&mut self) -> bool {
        if !self.file_menu_open {
            return false;
        }
        self.file_menu_open = false;
        self.file_menu.hover = None;
        true
    }

    /// Close the TopBar export quick menu.
    pub fn escape_export_quick_menu(&mut self) -> bool {
        if !self.export_quick_menu_open {
            return false;
        }
        self.export_quick_menu_open = false;
        self.export_quick_menu_hover = None;
        true
    }

    /// Close an open layer / page right-click context menu
    /// (layer-context-menu.tsx:101 — keydown Escape → onClose).
    pub fn escape_layer_context_menu(&mut self) -> bool {
        self.layer_context_menu.take().is_some()
    }

    /// Close the agent-settings modal.
    pub fn escape_agent_settings_modal(&mut self) -> bool {
        if !self.agent_settings_open {
            return false;
        }
        self.agent_settings_open = false;
        self.agent_settings_drag = None;
        true
    }

    /// Close the TopBar locale picker.
    pub fn escape_locale_picker(&mut self) -> bool {
        if !self.locale_picker.open {
            return false;
        }
        self.locale_picker.open = false;
        self.locale_picker.hover = None;
        true
    }

    /// Close the toolbar shape picker.
    pub fn escape_shape_picker(&mut self) -> bool {
        if !self.shape_picker.open {
            return false;
        }
        self.shape_picker.open = false;
        self.shape_picker.hover = None;
        self.shape_picker.pressed = None;
        true
    }

    /// Close the icon picker overlay.
    pub fn escape_icon_picker(&mut self) -> bool {
        if !self.icon_picker.open {
            return false;
        }
        self.close_icon_picker();
        true
    }

    /// Escape leaves the Scene Template Center one layer at a time: a focused
    /// generate topic returns focus to the search field first, and only a
    /// second press closes the panel. The alternative — closing outright —
    /// throws away a typed topic on the keystroke people reach for to undo a
    /// mis-click into the field.
    pub fn escape_scene_template_center(&mut self) -> bool {
        if !self.scene_template_center.open {
            return false;
        }
        // The paste box is the topmost layer inside the panel, so it is the
        // first thing Escape takes back.
        if self.close_scene_template_style_import() {
            return true;
        }
        if self.scene_template_center.focus == crate::SceneTemplateFocus::Generate {
            self.scene_template_center.focus = crate::SceneTemplateFocus::Search;
            return true;
        }
        self.close_scene_template_center()
    }

    pub fn escape_prompt_center(&mut self) -> bool {
        if !self.prompt_center.open {
            return false;
        }
        if self.prompt_center.save_open {
            self.prompt_center.save_open = false;
            self.prompt_center.save_title.set_text("");
            self.prompt_center.focus = crate::PromptCenterFocus::Search;
            self.prompt_center.hover = None;
            if matches!(
                self.pressed_button,
                Some(crate::ButtonPressTarget::PromptCenter(_))
            ) {
                self.pressed_button = None;
            }
            return true;
        }
        self.close_prompt_center()
    }

    /// Close the chat model picker dropdown.
    pub fn escape_chat_model_picker(&mut self) -> bool {
        if !self.chat_model_picker.open {
            return false;
        }
        self.close_chat_model_picker();
        true
    }

    /// Close the component (UIKit) browser. One layer per press: an
    /// open kit-filter popover closes before the panel itself does.
    pub fn escape_component_browser(&mut self) -> bool {
        if !self.component_browser_open {
            return false;
        }
        if self.component_browser_kit_picker_open {
            self.component_browser_kit_picker_open = false;
            return true;
        }
        self.component_browser_open = false;
        self.component_browser_select_all = false;
        self.component_browser_hover = None;
        self.component_browser_confirm_delete_kit = None;
        true
    }

    /// Close the instance component picker.
    pub fn escape_instance_component_picker(&mut self) -> bool {
        if !self.instance_component_picker_open {
            return false;
        }
        self.close_instance_component_picker();
        true
    }

    /// Close the property-panel fill-type dropdown.
    pub fn escape_fill_type_picker(&mut self) -> bool {
        if !self.fill_type_picker.open {
            return false;
        }
        self.close_fill_type_picker();
        true
    }

    /// Close the property-panel image-fill popover.
    pub fn escape_image_fill_popover(&mut self) -> bool {
        if !self.image_fill_popover_open {
            return false;
        }
        self.image_fill_popover_open = false;
        true
    }

    /// Close the Effects add-menu.
    pub fn escape_effect_add_picker(&mut self) -> bool {
        if !self.effect_add_picker_open {
            return false;
        }
        self.close_effect_add_picker();
        true
    }

    /// Close the Figma / HTML import modal.
    ///
    /// The native host posts a `FinishFigmaImport(Cancel)` file action
    /// FIRST when its multi-page picker is up — only the desktop shell
    /// runs that picker, so the post-back stays host-side.
    pub fn escape_import_modal(&mut self) -> bool {
        if !self.figma_import_open {
            return false;
        }
        self.figma_import_open = false;
        self.figma_import_hover = None;
        true
    }

    /// Close an open variable-row `⋯` menu.
    pub fn escape_variables_row_menu(&mut self) -> bool {
        self.variables_row_menu.take().is_some()
    }

    /// Close every editor-owned transient surface before Preview builds.
    /// Persistent preferences, host requests and preview state stay intact.
    pub fn close_preview_owned_overlays(&mut self) {
        self.file_menu_open = false;
        self.file_menu = Default::default();
        self.export_quick_menu_open = false;
        self.export_quick_menu_hover = None;
        self.export_dialog_open = false;
        self.export_dialog_hover = None;
        self.export_scale_picker_open = false;
        self.export_format_picker_open = false;
        self.export_picker_hover = None;
        self.import_menu_open = false;
        self.import_menu = Default::default();
        self.figma_import_open = false;
        self.figma_import_hover = None;
        self.figma_import_pages.clear();
        self.figma_import_page_select = Default::default();
        self.figma_import_in_progress = false;
        self.file_drop_active = false;
        self.file_drop_target = None;
        self.save_name_dialog.close();

        self.agent_settings_open = false;
        self.agent_settings_drag = None;
        self.agent_settings.focus = None;
        self.agent_settings.builtin_preset_menu_open = None;
        self.agent_settings.builtin_preset_menu_hover = None;
        self.agent_settings.builtin_model_menu_open = None;
        self.agent_settings.builtin_model_menu_hover = None;
        self.agent_settings.image_gen_provider_menu_open = None;
        self.agent_settings.hover_image_gen_provider_option = None;
        self.agent_settings.hover_agent_settings_close = false;
        self.agent_settings.hover_mcp_server_button = false;
        self.agent_settings.hover_mcp_client_config_copy = false;
        self.agent_settings.hover_image_search_test_button = false;
        self.agent_settings.hover_image_search_register_link = false;
        self.agent_settings.hover_image_gen_add_button = false;
        self.agent_settings.hover_image_gen_profile_header = None;
        self.agent_settings.hover_image_gen_profile_remove = None;
        self.agent_settings.hover_image_gen_profile_provider = None;
        self.agent_settings.hover_image_gen_profile_test = None;
        self.agent_settings.hover_provider = usize::MAX;
        self.agent_settings.hover_builtin_agent = usize::MAX;
        self.agent_settings.hover_acp_agent = usize::MAX;
        self.agent_settings.hover_add_provider = false;
        self.agent_settings.hover_add_acp_agent = false;
        self.agent_settings.hover_acp_preset = None;
        self.agent_settings.hover_nav = None;
        self.settings_input.reset_transient();
        self.account_menu_open = false;
        self.account_menu_hover = None;
        self.login_modal_open = false;
        self.login_modal_hover = None;
        self.login_modal_stub_hint_shown = false;
        self.login_modal_status = None;

        self.locale_picker = Default::default();
        self.shape_picker = Default::default();
        self.close_icon_picker();
        self.close_chat_model_picker();
        self.parallel_agents_picker_open = false;
        self.parallel_agents_picker_hover = None;
        self.fill_type_picker = Default::default();
        self.fill_type_picker_index = 0;
        self.compositing_picker = Default::default();
        self.compositing_picker_target = None;
        self.instance_component_picker_open = false;
        self.instance_component_picker_anchor.clear();
        self.corner_expand_open = false;
        self.effect_add_picker_open = false;
        self.effect_add_menu_hover = None;
        self.interaction_menu_open = false;
        self.interaction_menu_hover = None;
        self.close_color_variable_picker();
        self.image_fill_popover_open = false;
        self.image_crop_editing = None;
        self.image_panel.close_popovers();
        self.close_font_picker();
        self.font_picker_import_hover = false;
        self.font_weight_picker_open = false;
        self.font_weight_picker_hover = None;
        self.padding_edit_mode = None;
        self.padding_edit_mode_anchor.clear();
        self.padding_mode_popover_open = false;
        self.padding_mode_popover_hover = None;
        self.stroke_edit_mode = None;
        self.stroke_edit_mode_anchor.clear();
        self.stroke_mode_popover_open = false;
        self.stroke_mode_popover_hover = None;

        self.prompt_center.close();
        self.prompt_center.save_title.set_text("");
        self.prompt_center.search.reset_transient();
        self.scene_template_center.close();
        self.scene_template_center.search.reset_transient();
        self.scene_template_center.generate.reset_transient();
        self.scene_template_center.import.text.reset_transient();
        self.component_browser_open = false;
        self.component_browser_kit_picker_open = false;
        self.component_browser_hover = None;
        self.component_browser_confirm_delete_kit = None;
        self.design_md_panel.open = false;
        self.design_md_panel.hover = None;
        self.git_panel.open = false;
        self.git_panel.empty_hovered_card = None;
        self.git_panel.branch_button_hovered = false;
        self.git_panel.button_hover = None;
        self.git_panel.overflow_open = false;
        self.git_panel.overflow_menu = Default::default();
        self.git_panel.branch_picker_open = false;
        self.git_panel.branch_picker_menu = Default::default();
        self.git_panel.close_tracked_picker();
        self.git_panel.defocus_text_inputs();

        self.variables_panel_open = false;
        crate::host_variables_transitions::close_variable_menus(self);
        self.variable_row_focus = None;
        self.variables_theme_rename_axis = None;
        self.variables_variant_rename_value = None;
        self.variables_preset_name_focus = false;
        self.variables_preset_menu_hover = None;
        self.variables_search_focus = false;
        self.variables_panel_hover = None;
        self.variable_row_input.reset_transient();
        self.variables_header_input.reset_transient();
        self.effect_param_focus = None;
        self.ime_preedit = None;

        self.layer_context_menu = None;
        self.slides_panel.clear_pointer();
        self.collab.panel.open = false;
        self.collab.panel.join_address_focused = false;
        self.collab.panel.join_input.reset_transient();
        self.collab.panel.hover = None;
        self.missing_fonts_modal_open = false;
        self.missing_fonts_hover = None;
        self.html_import_diagnostics_open = false;
        self.html_import_diagnostics_hover = None;
        self.editor_toast = None;
        self.pressed_button = None;
        self.property_action_hover = None;
        self.property_tab_hover = None;
        self.toolbar_hover = None;
        self.align_toolbar_hover = None;
        self.topbar_traffic_hover = false;
        self.topbar_button_hover = None;
        self.topbar_hover_since_ms = None;
        self.statusbar_hover = None;
        self.chat_design_block_hover = None;
        self.chat_example_hover = None;
        self.chat_header_hover = None;
        self.chat_footer_hover = None;
        self.chat_tab_hover = None;
        self.chat_style_chip_hover_since_ms = None;
        self.hovered_layer_id = None;
        self.hovered_page_index = None;
        self.last_layer_click = None;
        self.last_canvas_click = None;
        self.last_variable_name_click = None;
        self.active_guides.clear();
        self.canvas_drop_indicator = None;
        self.canvas_hover_node = None;
        self.entered_container = None;
    }
}

/// Clear transient overlays stored on both halves of [`EditorState`].
/// Hosts commit document drafts before calling this helper.
pub fn close_preview_owned_overlays(state: &mut EditorState, now_ms: u64) {
    state.color_picker_blur_hex();
    state.color_picker_blur_rgb();
    let _ = state.close_color_picker();
    state.ui.path_anchor_menu = None;
    state.ui.property_focus = None;
    state.ui.property_input = Default::default();
    state.ui.property_input_draft.clear();
    state.ui.property_caret_pos = 0;
    state.ui.property_caret_anchor_ms = 0;
    state.ui.property_draft_select_all = false;
    state.ui.text_edit_input.reset_transient();
    state.chat.blur_input(now_ms);
    state.editor_ui.close_preview_owned_overlays();
}

/// Discard the focused property-panel draft.
pub fn escape_property_focus(state: &mut EditorState) -> bool {
    if state.ui.property_focus.take().is_none() {
        return false;
    }
    state.ui.property_input.set_text("");
    state.ui.property_input_draft.clear();
    state.ui.property_draft_select_all = false;
    true
}

/// Discard the focused effect-parameter draft.
pub fn escape_effect_param_focus(state: &mut EditorState) -> bool {
    if state.editor_ui.effect_param_focus.take().is_none() {
        return false;
    }
    state.ui.property_input.set_text("");
    state.ui.property_input_draft.clear();
    state.ui.property_draft_select_all = false;
    true
}

/// Discard the focused variables row / cell draft (native ladder — the
/// web ladder COMMITS the draft instead, see `keyboard_escape.rs`).
pub fn escape_variable_row_focus(state: &mut EditorState) -> bool {
    if state.editor_ui.variable_row_focus.take().is_none() {
        return false;
    }
    state.editor_ui.variable_row_input.set_text("");
    state.ui.property_input_draft.clear();
    state.ui.property_draft_select_all = false;
    true
}

/// Blur the chat input.
pub fn escape_chat_focus(state: &mut EditorState, now_ms: u64) -> bool {
    if !state.chat.focused {
        return false;
    }
    state.chat.blur_input(now_ms);
    true
}

/// Clear the canvas selection.
pub fn escape_selection(state: &mut EditorState) -> bool {
    if state.selection.is_empty() {
        return false;
    }
    state.deselect_all();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_cleanup_clears_nested_or_orphaned_transients_idempotently() {
        let document = jian_ops_schema::load_str(
            r##"{"version":"1.0.0","children":[{"type":"rectangle","id":"n1","x":0,"y":0,"width":10,"height":10,"fill":[{"type":"solid","color":"#ffffff"}]}]}"##,
        )
        .unwrap()
        .value;
        let mut state = EditorState::from_document(document);
        state.set_single_selection(crate::NodeId::new("n1"));
        assert!(state.open_color_picker(crate::ui_draft::ColorTarget::Fill, 10.0));
        state.ui.path_anchor_menu = Some(crate::ui_draft::PathAnchorMenuState {
            node_id: crate::NodeId::new("n1"),
            anchor_index: 0,
            x: 1.0,
            y: 2.0,
            menu: Default::default(),
        });
        state.ui.property_focus = Some(crate::PropertyFocus::PositionX);
        state.ui.property_input.set_text("42");
        state.ui.property_input.set_composition("si", 2, 1);
        state.ui.property_input_draft = "legacy".into();
        state.chat.input.set_text("keep chat draft");
        state.chat.focus_input_at_end(1);
        state.chat.input.set_composition("wu", 2, 1);

        let ui = &mut state.editor_ui;
        ui.figma_import_open = true;
        ui.figma_import_in_progress = true;
        ui.figma_import_pages.push(crate::FigmaImportPage {
            name: "Page".into(),
            layer_count: 1,
        });
        ui.figma_import_page_select.open = true;
        ui.file_drop_active = true;
        ui.file_drop_target = Some(crate::NodeId::new("n1"));
        ui.save_name_dialog.open_with("draft", false, 1);
        ui.prompt_center.open = false;
        ui.prompt_center.save_open = true;
        ui.scene_template_center.open = false;
        ui.scene_template_center.import.open = true;
        ui.shape_picker.hover = Some(1);
        ui.shape_picker.pressed = Some(1);
        ui.hovered_layer_id = Some(crate::NodeId::new("n1"));
        ui.pressed_button = Some(crate::ButtonPressTarget::PromptCenter(1));
        ui.variables_panel_open = true;
        ui.ime_preedit = Some(crate::ime_state::ImePreedit {
            text: "liu".into(),
            cursor: Some((3, 3)),
        });
        ui.layer_context_menu = Some(crate::LayerContextMenuState {
            target: crate::LayerContextTarget::Page(0),
            anchor_x: 10.0,
            anchor_y: 20.0,
            menu: Default::default(),
        });

        close_preview_owned_overlays(&mut state, 2);
        assert!(state.ui.color_picker.is_none());
        assert!(state.ui.path_anchor_menu.is_none());
        assert!(state.ui.property_focus.is_none());
        assert!(state.ui.property_input.composition().is_none());
        assert!(!state.chat.focused);
        assert!(state.chat.input.composition().is_none());
        assert_eq!(state.chat.input.text(), "keep chat draft");
        let ui = &state.editor_ui;
        assert!(!ui.figma_import_in_progress && ui.figma_import_pages.is_empty());
        assert!(!ui.file_drop_active && ui.file_drop_target.is_none());
        assert!(!ui.save_name_dialog.open);
        assert!(!ui.prompt_center.save_open && !ui.scene_template_center.import.open);
        assert!(ui.shape_picker.hover.is_none() && ui.shape_picker.pressed.is_none());
        assert!(ui.hovered_layer_id.is_none() && ui.pressed_button.is_none());
        assert!(!ui.variables_panel_open && ui.ime_preedit.is_none());

        close_preview_owned_overlays(&mut state, 3);
        assert!(state.ui.color_picker.is_none());
        assert!(!state.editor_ui.prompt_center.save_open);
    }

    #[test]
    fn preview_cleanup_preserves_persistent_state_and_authored_requests() {
        let mut state = EditorState::new();
        let ui = &mut state.editor_ui;
        ui.sidebar_open = false;
        ui.layer_panel_width = 312.0;
        ui.property_panel_width = 364.0;
        ui.variables_panel_size = Some((420.0, 320.0));
        ui.theme_mode = crate::ThemeMode::Light;
        ui.locale = crate::Locale::Ja;
        ui.pinned_style_guide = Some("editorial-dark".to_string());
        ui.account = crate::AccountState::dev_fake_signed_in();
        ui.scenario = Some(crate::scene_template_catalog::TemplateScene::Slides);
        ui.pending_file_action = Some(crate::FileAction::Save);
        ui.preview.mode = true;
        ui.preview.warnings = vec!["keep".into()];
        ui.prompt_center.open = true;
        state
            .ui
            .variables
            .active_theme
            .insert("mode".into(), "dark".into());
        let document = state.doc.clone();
        let account = ui.account.clone();
        let preview = ui.preview.clone();
        let variables = state.ui.variables.clone();
        close_preview_owned_overlays(&mut state, 0);
        let ui = &state.editor_ui;
        assert_eq!(state.doc, document);
        assert!(!ui.sidebar_open);
        assert_eq!(ui.layer_panel_width, 312.0);
        assert_eq!(ui.property_panel_width, 364.0);
        assert_eq!(ui.variables_panel_size, Some((420.0, 320.0)));
        assert_eq!(ui.theme_mode, crate::ThemeMode::Light);
        assert_eq!(ui.locale, crate::Locale::Ja);
        assert_eq!(ui.pinned_style_guide.as_deref(), Some("editorial-dark"));
        assert_eq!(ui.account, account);
        assert_eq!(ui.pending_file_action, Some(crate::FileAction::Save));
        assert_eq!(ui.preview, preview);
        assert_eq!(state.ui.variables, variables);
        assert_eq!(
            ui.scenario,
            Some(crate::scene_template_catalog::TemplateScene::Slides)
        );
    }
}
