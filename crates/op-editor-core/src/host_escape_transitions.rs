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

    /// Close the Prompt Center before Escape reaches chat focus or selection.
    /// Escape closes the Scene Template Center. It has one flat layer — no
    /// inline form like the Prompt Center's — so a single press dismisses it.
    pub fn escape_scene_template_center(&mut self) -> bool {
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
