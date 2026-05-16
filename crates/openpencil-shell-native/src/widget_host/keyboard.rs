//! Keyboard input handlers on `WidgetHostNative` — text input,
//! delete / duplicate / nudge, send, escape. Click routing +
//! marquee / layer-drag commit live in the sibling `click.rs`.
//!
//! `EditorState` is the host's source of truth: every focus / draft
//! / chat field is read + written on `editor_state`; mutations flag
//! the paint snapshot dirty.

use super::WidgetHostNative;
use op_editor_core::ui_draft::PropertyFocus;
use op_editor_core::editor_ui_state::VariableRowFocus;

impl WidgetHostNative {
    /// Typed-char router: settings → rename → text-edit → variable
    /// row → property → chat.
    pub fn apply_text(&mut self, c: char) -> bool {
        // Settings input owns the keyboard while focused.
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            if c.is_ascii_digit()
                && self.editor_state.editor_ui.settings_input_draft.len() < 5
            {
                self.editor_state.editor_ui.settings_input_draft.push(c);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.ui.layer_rename.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            let _ = self.editor_state.rename_append(c.encode_utf8(&mut s));
            self.editor_state.editor_ui.rename_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.text_editing.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            if self
                .editor_state
                .text_edit_append(c.encode_utf8(&mut s), self.now_ms)
            {
                self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if let Some(focus) = self.editor_state.editor_ui.variable_row_focus {
            self.editor_state.ui.property_draft_select_all = false;
            let allowed = match focus {
                VariableRowFocus::Number(_) => {
                    c.is_ascii_digit()
                        || (c == '-'
                            && self.editor_state.ui.property_input_draft.is_empty())
                        || (c == '.'
                            && !self
                                .editor_state
                                .ui
                                .property_input_draft
                                .contains('.'))
                }
                VariableRowFocus::String(_) => !c.is_control(),
            };
            if !allowed {
                return false;
            }
            self.editor_state.ui.property_input_draft.push(c);
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if let Some(focus) = self.editor_state.ui.property_focus {
            self.editor_state.ui.property_draft_select_all = false;
            let is_hex_focus =
                matches!(focus, PropertyFocus::FillHex | PropertyFocus::StrokeHex);
            let allowed = if is_hex_focus {
                self.editor_state.ui.property_input_draft.len() < 7
                    && (c.is_ascii_hexdigit()
                        || (c == '#'
                            && self
                                .editor_state
                                .ui
                                .property_input_draft
                                .is_empty()))
            } else {
                c.is_ascii_digit()
                    || (c == '-' && self.editor_state.ui.property_input_draft.is_empty())
                    || (c == '.'
                        && matches!(
                            focus,
                            PropertyFocus::Opacity
                                | PropertyFocus::Rotation
                                | PropertyFocus::PositionR
                                | PropertyFocus::StrokeWidth
                        )
                        && !self
                            .editor_state
                            .ui
                            .property_input_draft
                            .contains('.'))
            };
            if !allowed {
                return false;
            }
            self.editor_state.ui.property_input_draft.push(c);
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if !self.editor_state.chat.focused {
            return false;
        }
        self.editor_state.chat.input.push(c);
        self.editor_state.chat.caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    pub fn apply_backspace(&mut self) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            self.editor_state.editor_ui.settings_input_draft.pop();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.layer_rename.is_some() {
            let ok = self.editor_state.rename_backspace();
            if ok {
                self.editor_state.editor_ui.rename_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.text_editing.is_some() {
            let ok = self.editor_state.text_edit_backspace(self.now_ms);
            if ok {
                self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            self.editor_state.ui.property_draft_select_all = false;
            if self.editor_state.ui.property_input_draft.pop().is_some() {
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.ui.property_focus.is_some() {
            self.editor_state.ui.property_draft_select_all = false;
            if self.editor_state.ui.property_input_draft.pop().is_some() {
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.chat.focused {
            if self.editor_state.chat.input.pop().is_some() {
                self.editor_state.chat.caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        let snap = self.editor_state.snapshot_for_history();
        if self.editor_state.delete_selected() {
            self.editor_state.history_push_past(snap);
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Delete — pops a char from rename / text-edit when active;
    /// otherwise deletes the selected node.
    pub fn apply_delete(&mut self) -> bool {
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            self.editor_state.ui.property_draft_select_all = false;
            if self.editor_state.ui.property_input_draft.pop().is_some() {
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.ui.layer_rename.is_some() {
            let ok = self.editor_state.rename_backspace();
            if ok {
                self.editor_state.editor_ui.rename_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.text_editing.is_some() {
            let ok = self.editor_state.text_edit_backspace(self.now_ms);
            if ok {
                self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.property_focus.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        let snap = self.editor_state.snapshot_for_history();
        if self.editor_state.delete_selected() {
            self.editor_state.history_push_past(snap);
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd-D — duplicate selection as a sibling at +10 doc px.
    pub fn apply_duplicate(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        self.editor_state.commit_history();
        let dup = self
            .editor_state
            .duplicate_selected(&mut self.next_node_id, 10.0)
            .is_some();
        if dup {
            self.mark_dirty();
        }
        dup
    }

    /// Arrow-key nudge — translate selection by (dx, dy) doc px.
    pub fn apply_nudge(&mut self, dx: f32, dy: f32) -> bool {
        if self.input_active() {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        self.editor_state.commit_history();
        self.editor_state.translate_selected(dx as f64, dy as f64);
        self.mark_dirty();
        true
    }

    pub fn apply_send(&mut self) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            self.commit_settings_focus_if_any();
            return true;
        }
        if self.editor_state.ui.layer_rename.is_some() {
            let ok = self.editor_state.rename_commit();
            if ok {
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.text_editing.is_some() {
            let ok = self.editor_state.text_edit_commit();
            if ok {
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.pen_in_progress.is_some() {
            let ok = self.editor_state.finish_pen_path();
            if ok {
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            self.commit_variable_row_focus_if_any();
            return true;
        }
        if self.editor_state.ui.property_focus.is_some() {
            self.commit_property_focus_if_any();
            return true;
        }
        if self.editor_state.chat.input.trim().is_empty() {
            return false;
        }
        // Real provider turn — raises `chat.pending_send`.
        let sent = self.editor_state.chat.begin_send();
        if sent {
            self.mark_dirty();
        }
        sent
    }

    /// Escape — priority cascade: rename → property → pickers →
    /// chat → selection. One layer per press.
    pub fn apply_escape(&mut self) -> bool {
        if self
            .editor_state
            .editor_ui
            .agent_settings
            .focus
            .take()
            .is_some()
        {
            self.editor_state.editor_ui.settings_input_draft.clear();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.agent_settings_open {
            self.editor_state.editor_ui.agent_settings_open = false;
            self.editor_state.editor_ui.agent_settings_drag = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.export_dialog_open {
            self.editor_state.editor_ui.export_dialog_open = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.figma_import_open {
            self.editor_state.editor_ui.figma_import_open = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.file_menu_open {
            self.editor_state.editor_ui.file_menu_open = false;
            self.editor_state.editor_ui.file_menu_hover = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.rename_cancel() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.text_edit_commit() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.finish_pen_path() {
            self.mark_dirty();
            return true;
        }
        if self
            .editor_state
            .editor_ui
            .variable_row_focus
            .take()
            .is_some()
        {
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_draft_select_all = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.property_focus.take().is_some() {
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_draft_select_all = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.locale_picker_open {
            self.editor_state.editor_ui.locale_picker_open = false;
            self.editor_state.editor_ui.locale_picker_hover = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.shape_picker_open {
            self.editor_state.editor_ui.shape_picker_open = false;
            self.editor_state.editor_ui.shape_picker_hover = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.fill_type_picker_open {
            self.editor_state.editor_ui.fill_type_picker_open = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.chat.focused {
            self.editor_state.chat.focused = false;
            self.mark_dirty();
            return true;
        }
        if !self.editor_state.selection.is_empty() {
            self.editor_state.deselect_all();
            self.mark_dirty();
            return true;
        }
        false
    }
}
