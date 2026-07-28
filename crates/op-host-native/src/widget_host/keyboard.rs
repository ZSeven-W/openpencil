//! Keyboard input handlers on `WidgetHostNative` — text input,
//! delete / duplicate / nudge, send, escape. Click routing +
//! marquee / layer-drag commit live in the sibling `click.rs`.
//!
//! `EditorState` is the host's source of truth: every focus / draft
//! / chat field is read + written on `editor_state`; mutations flag
//! the paint snapshot dirty.

use super::WidgetHostNative;
use op_editor_core::host_keyboard_transitions as shared;
use op_editor_core::host_preset_name_draft as preset_name;

impl WidgetHostNative {
    /// Typed-char router: settings → rename → text-edit → variable
    /// row → property → chat.
    pub fn apply_text(&mut self, c: char) -> bool {
        // Preview (Play) mode owns the keyboard: printable chars go to
        // the live runtime's focused widget, never editor editing.
        if self.preview.is_some() {
            if c.is_control() {
                return false;
            }
            let mut s = [0u8; 4];
            return self.preview_dispatch_text(c.encode_utf8(&mut s));
        }
        if let Some(changed) =
            op_editor_ui::widgets::collab_ui::join_address_text(&mut self.editor_state.editor_ui, c)
        {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        // This popover is painted above every other editor input, so it wins
        // even if a lower surface retained stale focus.
        if self.apply_image_panel_text(c) {
            return true;
        }
        // Color-picker hex field owns the keyboard while focused.
        if self.editor_state.color_picker_hex_focused() {
            if !self.collab_allows_color_picker_mutation() {
                return true;
            }
            if c.is_control() {
                return false;
            }
            self.editor_state.color_picker_hex_char(c, self.now_ms);
            self.mark_dirty();
            return true;
        }
        // Color-picker R/G/B numeric field owns the keyboard while focused.
        if self.editor_state.color_picker_rgb_focused() {
            if !self.collab_allows_color_picker_mutation() {
                return true;
            }
            if c.is_control() {
                return false;
            }
            self.editor_state.color_picker_rgb_char(c, self.now_ms);
            self.mark_dirty();
            return true;
        }
        // Settings input owns the keyboard while focused.
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_text(c);
        }
        // The inline clone wizard owns the keyboard while it is open: a
        // focused URL / destination field takes the character (unless a
        // clone is already running), and every other key is swallowed so
        // nothing reaches the canvas.
        if self.git_clone_input_active() {
            if c.is_control() {
                return false;
            }
            let now = self.now_ms;
            if let Some(form) = self.editor_state.editor_ui.git_panel.clone_form.as_mut() {
                if !form.cloning {
                    let mut s = [0u8; 4];
                    match form.focus {
                        Some(op_editor_core::CloneField::Url) => {
                            form.url_input.insert_str(c.encode_utf8(&mut s), now)
                        }
                        Some(op_editor_core::CloneField::Dest) => {
                            form.dest_input.insert_str(c.encode_utf8(&mut s), now)
                        }
                        None => {}
                    }
                    form.error = None;
                }
            }
            self.mark_dirty();
            return true;
        }
        // Git panel's commit-message input owns the keyboard next.
        if self.git_commit_focus_active() {
            if !c.is_control() {
                let now = self.now_ms;
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel.commit_input.insert_str(c.encode_utf8(&mut s), now);
                panel.commit_no_changes = false;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the Git panel's remote-URL input.
        if self.git_remote_focus_active() {
            if !c.is_control() {
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel
                    .remote_input
                    .insert_str(c.encode_utf8(&mut s), self.now_ms);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the Git panel's HTTPS-credential input.
        if self.git_https_focus_active() {
            if !c.is_control() {
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel
                    .https_input
                    .insert_str(c.encode_utf8(&mut s), self.now_ms);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the commit-signature form's name / email inputs.
        if self.git_author_focus_active() {
            if !c.is_control() {
                let now = self.now_ms;
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                if panel.author_email_focused {
                    panel
                        .author_email_input
                        .insert_str(c.encode_utf8(&mut s), now);
                } else {
                    panel
                        .author_name_input
                        .insert_str(c.encode_utf8(&mut s), now);
                }
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.git_branch_create_focus_active() {
            if !c.is_control() {
                let now = self.now_ms;
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel
                    .branch_create_input
                    .insert_str(c.encode_utf8(&mut s), now);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if let Some(changed) = shared::rename_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.ui.text_editing.is_some()
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::NodeProperty(
                    op_editor_core::CollabNodeField::Content,
                ),
            )
        {
            return true;
        }
        if let Some(changed) = shared::text_edit_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Variables-panel search filter — live append, no draft /
        // commit machinery (TS controlled `<input>`; same append/pop
        // discipline as the font-picker search).
        if shared::variables_search_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        if shared::variables_header_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        if preset_name::preset_name_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        if let Some(changed) = shared::variable_row_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Effect-param value box + property-panel inputs share
        // `ui.property_input`; the gate is per-focus (numeric / hex /
        // free text) and lives in the shared router.
        if let Some(changed) = shared::property_input_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Font-family picker search box (font_picker_dispatch.rs).
        if self.apply_font_picker_text(c) {
            return true;
        }
        if self.editor_state.editor_ui.icon_picker.open && !c.is_control() {
            if self.editor_state.editor_ui.icon_picker_select_all {
                self.editor_state.editor_ui.icon_picker_search.clear();
                self.editor_state.editor_ui.icon_picker_select_all = false;
                self.editor_state.editor_ui.icon_picker.hover = None;
                self.editor_state.editor_ui.icon_picker.pressed = None;
            }
            self.editor_state.editor_ui.icon_picker_search.push(c);
            self.editor_state.editor_ui.icon_picker.hover = None;
            self.editor_state.editor_ui.icon_picker.pressed = None;
            // New filter → scroll the list back to the top.
            self.editor_state.editor_ui.icon_picker.scroll.offset = 0.0;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            return self.apply_chat_model_picker_text(c);
        }
        if self.editor_state.editor_ui.component_browser_open && !c.is_control() {
            if self.editor_state.editor_ui.component_browser_select_all {
                self.editor_state.editor_ui.component_browser_search.clear();
                self.editor_state.editor_ui.component_browser_select_all = false;
            }
            self.editor_state.editor_ui.component_browser_search.push(c);
            self.mark_dirty();
            return true;
        }
        if shared::chat_input_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }
}
