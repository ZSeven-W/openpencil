//! Editor keyboard shortcuts (duplicate / nudge / clipboard / undo /
//! reorder) on the web host — split from `keyboard.rs` to honor the
//! 800-line cap. Mirrors the native `widget_host/keyboard.rs` ops.

use super::WidgetHost;
use op_editor_core::figma_import_state::ImportSource;
use op_editor_core::host_keyboard_transitions as shared;
use op_editor_core::host_preset_name_draft as preset_name;

impl WidgetHost {
    /// Copy `text` for the current surface. The VS Code embed relays the
    /// payload to the extension host (`op-shell/copy` → `vscode.env.
    /// clipboard`) because the nested-iframe permissions chain rejects
    /// `navigator.clipboard` writes there; the direct browser write stays
    /// as a best-effort in every mode.
    pub(in crate::widget_host) fn host_copy_text(&self, text: &str) {
        dispatch_web_copy(
            self.editor_state.editor_ui.embed,
            text,
            crate::web_clipboard::copy_text,
            crate::web_clipboard::post_copy_to_parent,
        );
    }

    /// Single-key tool switch (V / R / O / L / T / F / P / Y / H). Mirrors the
    /// native host's `shortcuts.rs::apply_set_tool`: drops any in-flight pen
    /// path (TS `onToolChange`, `skia-pen-tool.ts:38-50`), clears the canvas
    /// hover outline, and syncs the toolbar shape slot for shape variants.
    pub(crate) fn apply_set_tool(&mut self, tool: op_editor_core::Tool) {
        self.exit_image_crop_edit();
        self.commit_variable_row_focus_if_any();
        // Pen-path discard stays host-side: the native twin routes it
        // through its own `cancel_pen_on_tool_switch` helper.
        if !matches!(tool, op_editor_core::Tool::Pen) {
            let _ = self.editor_state.cancel_pen_path();
        }
        shared::set_active_tool(&mut self.editor_state, tool);
        self.mark_dirty();
    }

    /// Map a bare (no Cmd/Ctrl) letter to a tool switch when no text input owns
    /// the keyboard. Returns true when a tool was set so the keydown router
    /// falls back to `apply_text` for every other letter (and while typing).
    /// Web parity fix: the native host had this single-key router but the web
    /// keydown handler only ever fell through to `apply_text`, so R / T / P /
    /// etc. silently did nothing on the canvas.
    pub(crate) fn apply_tool_shortcut(&mut self, key: &str) -> bool {
        if self.input_active() {
            return false;
        }
        let tool = match key.to_ascii_lowercase().as_str() {
            "v" => op_editor_core::Tool::Select,
            "r" => op_editor_core::Tool::Rect,
            "o" => op_editor_core::Tool::Ellipse,
            "l" => op_editor_core::Tool::Line,
            "t" => op_editor_core::Tool::Text,
            "f" => op_editor_core::Tool::Frame,
            "p" => op_editor_core::Tool::Pen,
            "y" => op_editor_core::Tool::Polygon,
            "h" => op_editor_core::Tool::Hand,
            _ => return false,
        };
        self.apply_set_tool(tool);
        true
    }

    /// Enter in the preset save-as-name input — saves the current theme as a
    /// named preset, clears the draft, and defocuses (the dropdown stays open).
    /// Native parity (`variables_preset_press.rs::commit_variables_preset_name_if_any`).
    /// Blank names keep the input open. Returns whether the input was active.
    pub(in crate::widget_host) fn commit_variables_preset_name_if_any(&mut self) -> bool {
        if !self.editor_state.editor_ui.preset_name_input_active() {
            return false;
        }
        let name = self.editor_state.ui.property_input_draft.clone();
        if name.trim().is_empty() {
            return true;
        }
        let now_ms = self.now_ms;
        let _ = self.editor_state.save_theme_preset(&name, now_ms);
        self.editor_state.editor_ui.variables_preset_name_focus = false;
        self.editor_state.ui.property_input_draft.clear();
        self.editor_state.ui.property_caret_pos = 0;
        self.mark_dirty();
        true
    }

    /// Escape in the preset save-as-name input — closes just the input, leaving
    /// the preset dropdown open. Native parity.
    pub(in crate::widget_host) fn escape_variables_preset_name(&mut self) -> bool {
        if !self.editor_state.editor_ui.preset_name_input_active() {
            return false;
        }
        self.editor_state.editor_ui.variables_preset_name_focus = false;
        self.editor_state.ui.property_input_draft.clear();
        self.editor_state.ui.property_caret_pos = 0;
        self.mark_dirty();
        true
    }

    /// Cmd/Ctrl+T — open a fresh chat tab (MT.3). Preserves all existing tabs
    /// and leaves any in-flight run bound to its own tab (the web run binding
    /// lives in `web_chat`'s RUNNING_TAB, untouched here).
    ///
    /// The VS Code embed has no chat panel to show the new tab in (its chat
    /// is MCP-driven), and this shortcut isn't gated on `ai_chat_rect` like
    /// every mouse-driven chat path — left unguarded it would silently eat
    /// the host IDE's own Cmd/Ctrl+T binding, so no-op instead of consuming.
    pub fn apply_new_chat_tab(&mut self) -> bool {
        if self.editor_state.editor_ui.embed == op_editor_core::EmbedHost::VsCode {
            return false;
        }
        self.editor_state.chat.new_tab();
        // Session set mutated: rotate the transcript-cache owner NOW so a pointer
        // move before the next paint can't cross-pair the previous tab's geometry.
        self.force_rotate_chat_owner();
        self.mark_dirty();
        true
    }

    /// Cmd/Ctrl+D — duplicate the selected node as a sibling
    /// offset by ~10 doc px. Selection follows the clone.
    pub fn apply_duplicate(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        let dup = shared::duplicate_selection(&mut self.editor_state, &mut self.next_node_id);
        if dup {
            self.mark_dirty();
        }
        dup
    }

    /// Left / Right arrow during an inline rename — moves the rename
    /// caret one character. Returns whether a rename is active, so the
    /// caller falls back to node-nudge when it isn't.
    pub fn apply_rename_caret(&mut self, forward: bool) -> bool {
        let moved = shared::rename_caret(&mut self.editor_state, forward, self.now_ms);
        if moved {
            self.mark_dirty();
        }
        moved
    }

    /// Arrow-key nudge — translate the selected node by
    /// `(dx, dy)` document px. Shift-arrow callers pass 10 px;
    /// plain arrows pass 1 px.
    pub fn apply_nudge(&mut self, dx: f32, dy: f32) -> bool {
        if self.input_active() {
            return false;
        }
        if shared::nudge_selection(&mut self.editor_state, dx, dy) {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd/Ctrl+A — replace selection with every top-level node
    /// on the active page (TS `setSelection(topLevelIds, …)`).
    pub fn apply_select_all(&mut self) -> bool {
        // The font picker owns the keyboard while its search field is open.
        // It has no range-selection model, so consume Cmd/Ctrl+A instead of
        // selecting canvas nodes behind the overlay.
        if self.editor_state.editor_ui.font_picker.open {
            return true;
        }
        if self.apply_input_select_all() {
            return true;
        }
        if self.editor_state.select_all_top_level() {
            self.mark_dirty();
            return true;
        }
        false
    }

    fn apply_input_select_all(&mut self) -> bool {
        if self.apply_image_panel_select_all() {
            return true;
        }
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            let ui = &mut self.editor_state.editor_ui;
            ui.settings_input.select_all();
            ui.settings_input.touch(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if let Some(consumed) = self.apply_git_input_select_all() {
            return consumed;
        }
        // Rename → canvas text edit → property / effect-param →
        // variables header / row → icon picker → model picker →
        // component browser → chat input.
        if shared::select_all_focused_input(&mut self.editor_state, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    pub fn apply_property_caret(&mut self, forward: bool) -> bool {
        if shared::property_caret_move(&mut self.editor_state, forward, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        // #20: the preset-name input rides the flat legacy draft, not a
        // `TextInputState`, so it has its own caret module. Consumed
        // even when the caret can't move — an arrow over a focused
        // input must never fall through to nudging the selected node.
        if let Some(moved) =
            preset_name::preset_name_caret_move(&mut self.editor_state, forward, self.now_ms)
        {
            if moved {
                self.mark_dirty();
            }
            return true;
        }
        false
    }

    /// Left / Right arrow on the focused chat input. Consumes the key
    /// even at text boundaries so it never falls through to canvas nudge.
    pub fn apply_chat_input_caret(&mut self, forward: bool) -> bool {
        if shared::chat_input_caret(&mut self.editor_state, forward, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd/Ctrl+C — copy the selection into the clipboard.
    pub fn apply_copy(&mut self) -> bool {
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            if let Some(text) = self.focused_input_selected_text() {
                #[cfg(feature = "canvaskit")]
                self.host_copy_text(&text);
                #[cfg(not(feature = "canvaskit"))]
                let _ = text;
            }
            return true;
        }
        if self.editor_state.chat.focused {
            if let Some(text) = self
                .editor_state
                .chat
                .selected_input_text()
                .map(str::to_string)
            {
                #[cfg(feature = "canvaskit")]
                self.host_copy_text(&text);
                #[cfg(not(feature = "canvaskit"))]
                self.editor_state.chat.queue_copy_text(text);
                return true;
            }
            return false;
        }
        // Any other focused text input owns the keyboard: copy its highlighted
        // slice to the system clipboard and swallow the chord so Ctrl/Cmd+C
        // never falls through to canvas-node copy.
        if self.input_active() {
            if let Some(text) = self.focused_input_selected_text() {
                #[cfg(feature = "canvaskit")]
                self.host_copy_text(&text);
                #[cfg(not(feature = "canvaskit"))]
                let _ = text;
            }
            return true;
        }
        if let Some(text) = self.editor_state.codegen.selected_code_text() {
            #[cfg(feature = "canvaskit")]
            self.host_copy_text(text);
            #[cfg(not(feature = "canvaskit"))]
            let _ = text;
            return true;
        }
        if let Some(text) = self
            .editor_state
            .chat
            .selected_transcript_text()
            .map(str::to_string)
        {
            #[cfg(feature = "canvaskit")]
            self.host_copy_text(&text);
            #[cfg(not(feature = "canvaskit"))]
            self.editor_state.chat.queue_copy_text(text);
            return true;
        }
        if self.editor_state.copy_selected() {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd/Ctrl+X — cut the focused text input's selection to the
    /// system clipboard (mirrors `apply_copy`'s input priority), else
    /// copy then delete the selected canvas nodes.
    pub fn apply_cut(&mut self) -> bool {
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            if let Some(text) = self.focused_input_selected_text() {
                #[cfg(feature = "canvaskit")]
                self.host_copy_text(&text);
                #[cfg(not(feature = "canvaskit"))]
                let _ = &text;
                self.apply_backspace();
            }
            return true;
        }
        // Chat input cut — its own selection model.
        if self.editor_state.chat.focused {
            if let Some(text) = self
                .editor_state
                .chat
                .selected_input_text()
                .map(str::to_string)
            {
                #[cfg(feature = "canvaskit")]
                self.host_copy_text(&text);
                #[cfg(not(feature = "canvaskit"))]
                self.editor_state.chat.queue_copy_text(text);
                self.editor_state.chat.delete_input_selection(self.now_ms);
                self.mark_dirty();
            }
            // Swallow either way so Cmd+X never falls through to node cut
            // while the chat input owns the keyboard.
            return true;
        }
        // Any other focused text input: cut its highlighted slice. With a
        // live selection `apply_backspace` removes the whole range; with
        // none, nothing is cut (the chord never deletes a single char).
        if self.input_active() {
            if let Some(text) = self.focused_input_selected_text() {
                #[cfg(feature = "canvaskit")]
                self.host_copy_text(&text);
                #[cfg(not(feature = "canvaskit"))]
                let _ = &text;
                self.apply_backspace();
            }
            return true;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        self.editor_state.commit_history();
        if self.editor_state.cut_selected() {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd/Ctrl+V — paste the clipboard at the active page,
    /// offset by 10 doc px from the originals. Selection follows
    /// the new clones.
    pub fn apply_paste(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        let pasted = shared::paste_clipboard_at_default_offset(
            &mut self.editor_state,
            &mut self.next_node_id,
        );
        if pasted {
            self.mark_dirty();
        }
        pasted
    }

    pub fn apply_undo(&mut self) -> bool {
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if self.editor_state.undo() {
            self.mark_dirty();
            self.refresh_missing_fonts_after_history_change();
            return true;
        }
        false
    }

    pub fn apply_redo(&mut self) -> bool {
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if self.editor_state.redo() {
            self.mark_dirty();
            self.refresh_missing_fonts_after_history_change();
            return true;
        }
        false
    }

    /// Cmd+Shift+K — toggle the component (UIKit) browser panel.
    /// Mirrors the native host's `apply_toggle_component_browser`
    /// (TS `editor-layout.tsx` Cmd+Shift+K → `toggleBrowser`); the
    /// open-position default is the viewport centre via the
    /// `component_browser_panel_rect` `None`-position fallback.
    pub fn apply_toggle_component_browser(&mut self) -> bool {
        let ui = &mut self.editor_state.editor_ui;
        ui.component_browser_open = !ui.component_browser_open;
        if !ui.component_browser_open {
            ui.component_browser_kit_picker_open = false;
            ui.component_browser_confirm_delete_kit = None;
            ui.component_browser_hover = None;
        }
        self.mark_dirty();
        true
    }

    /// CanvasKit keydown chord routing for editor shortcuts that are not
    /// printable text. `is_mod` is Cmd on macOS or Ctrl elsewhere.
    pub(crate) fn apply_keydown_shortcut(
        &mut self,
        key: &str,
        is_mod: bool,
        shift: bool,
        alt: bool,
    ) -> bool {
        if !is_mod || !shift || alt {
            return false;
        }
        if key.eq_ignore_ascii_case("k") {
            self.apply_toggle_component_browser()
        } else if key.eq_ignore_ascii_case("f") {
            self.apply_open_import(ImportSource::Figma)
        } else if key.eq_ignore_ascii_case("h") {
            self.apply_open_import(ImportSource::Html)
        } else {
            false
        }
    }

    pub(in crate::widget_host) fn apply_open_import(&mut self, source: ImportSource) -> bool {
        // Consume the chord without touching the active import or a visible
        // settings/Git input. An existing higher modal keeps ownership too;
        // otherwise the import modal would open invisibly beneath it and
        // unexpectedly appear only after that modal closes.
        if shared::import_modal_blocked_by_overlay(&self.editor_state.editor_ui)
            || self.git_commit_focus_active()
            || self.git_remote_focus_active()
            || self.git_https_focus_active()
            || self.git_author_focus_active()
            || self.git_branch_create_focus_active()
            || self.git_clone_input_active()
        {
            return true;
        }

        // The import modal covers every editor text surface. Commit canvas /
        // layer editing, then reuse the canonical chrome-input blur path so a
        // hidden property, chat, or model-picker input cannot keep receiving
        // keyboard/IME events behind the scrim.
        shared::commit_editing_for_modal(&mut self.editor_state);
        self.blur_text_inputs_on_blank_press();
        self.close_image_popovers_for_higher_overlay();
        self.close_import_menu();
        shared::open_import_modal(&mut self.editor_state.editor_ui, source);
        self.mark_dirty();
        true
    }

    /// `[` / `]` — bump the selected node down / up by one
    /// position in its parent's children vec (changing paint
    /// order).
    pub fn apply_reorder(&mut self, direction: op_editor_core::ReorderDirection) -> bool {
        if self.input_active() {
            return false;
        }
        if shared::reorder_selection(&mut self.editor_state, direction) {
            self.mark_dirty();
            return true;
        }
        false
    }
}

fn dispatch_web_copy(
    embed: op_editor_core::EmbedHost,
    text: &str,
    direct: impl FnOnce(&str),
    relay: impl FnOnce(&str),
) {
    if embed == op_editor_core::EmbedHost::VsCode {
        relay(text);
    }
    direct(text);
}

#[cfg(test)]
mod copy_tests {
    use super::dispatch_web_copy;
    use std::cell::RefCell;

    #[test]
    fn browser_copy_uses_direct_clipboard_and_vscode_keeps_relay_fallback() {
        let direct = RefCell::new(Vec::new());
        let relay = RefCell::new(Vec::new());
        dispatch_web_copy(
            op_editor_core::EmbedHost::None,
            "192.168.1.8:43120",
            |text| direct.borrow_mut().push(text.to_string()),
            |text| relay.borrow_mut().push(text.to_string()),
        );
        assert_eq!(direct.borrow().as_slice(), ["192.168.1.8:43120"]);
        assert!(relay.borrow().is_empty());

        direct.borrow_mut().clear();
        dispatch_web_copy(
            op_editor_core::EmbedHost::VsCode,
            "192.168.1.8:43120",
            |text| direct.borrow_mut().push(text.to_string()),
            |text| relay.borrow_mut().push(text.to_string()),
        );
        assert_eq!(direct.borrow().as_slice(), ["192.168.1.8:43120"]);
        assert_eq!(relay.borrow().as_slice(), ["192.168.1.8:43120"]);
    }
}
