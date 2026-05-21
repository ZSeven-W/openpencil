//! `DesktopApp::handle_key_pressed` — the editor keyboard-shortcut
//! dispatch table. Split out of `app_handler.rs` to keep that file
//! under the repo's 800-line-per-file cap.

use crate::{chat_session, persistence, DesktopApp};
use winit::keyboard::{Key, NamedKey};

impl DesktopApp {
    /// Dispatch a pressed key (`logical_key` + its `text`) — the
    /// editor's keyboard-shortcut table. Called from the winit
    /// `KeyboardInput` event in `app_handler.rs`.
    pub(crate) fn handle_key_pressed(&mut self, logical_key: &Key, text: Option<&str>) {
        use op_editor_core::ReorderDirection;
        let mut consumed = false;
        let nudge = if self.shift_modifier { 10.0 } else { 1.0 };
        // While a settings-modal input OR the Git panel's
        // commit-message input owns the keyboard, the ONLY
        // allowed paths are text / backspace / send / escape.
        // Editor shortcuts (Cmd+D, Cmd+G, Cmd+Z, arrow nudges,
        // Delete, [ / ], single-letter tool switches, …) would
        // otherwise silently mutate the document while the
        // user thinks they are typing into the input.
        let settings_focused =
            self.host.settings_focus_active() || self.host.git_commit_focus_active();
        match logical_key {
            // Named-key shortcuts fire only when no Cmd/Ctrl is held.
            Key::Named(NamedKey::Backspace) if !self.zoom_modifier => {
                consumed = self.host.apply_backspace();
            }
            Key::Named(NamedKey::Delete) if !self.zoom_modifier && !settings_focused => {
                consumed = self.host.apply_delete();
            }
            Key::Named(NamedKey::Enter) if !self.zoom_modifier => {
                consumed = self.host.apply_send();
                // apply_send may raise pending_send (chat send).
                if chat_session::launch_if_pending(&mut self.host, &mut self.current_chat) {
                    self.request_redraw(true);
                }
            }
            Key::Named(NamedKey::Escape) if !self.zoom_modifier => {
                consumed = self.host.apply_escape();
            }
            Key::Named(NamedKey::ArrowUp) if !self.zoom_modifier && !settings_focused => {
                consumed = self.host.apply_nudge(0.0, -nudge);
            }
            Key::Named(NamedKey::ArrowDown) if !self.zoom_modifier && !settings_focused => {
                consumed = self.host.apply_nudge(0.0, nudge);
            }
            Key::Named(NamedKey::ArrowLeft) if !self.zoom_modifier && !settings_focused => {
                consumed = self.host.apply_nudge(-nudge, 0.0);
            }
            Key::Named(NamedKey::ArrowRight) if !self.zoom_modifier && !settings_focused => {
                consumed = self.host.apply_nudge(nudge, 0.0);
            }
            // Cmd/Ctrl+Alt+U/S/I/X — path boolean ops (Paper.js
            // parity). Gated on `!settings_focused` so they
            // never mutate the document while a settings /
            // Git-commit input owns the keyboard.
            Key::Character(ref ch)
                if self.zoom_modifier
                    && self.alt_modifier
                    && !self.shift_modifier
                    && !settings_focused =>
            {
                use op_editor_core::BooleanOp;
                match ch.to_lowercase().as_str() {
                    "u" => consumed = self.host.apply_boolean_op(BooleanOp::Union),
                    "s" => consumed = self.host.apply_boolean_op(BooleanOp::Subtract),
                    "i" => consumed = self.host.apply_boolean_op(BooleanOp::Intersect),
                    "x" => consumed = self.host.apply_boolean_op(BooleanOp::Exclude),
                    _ => {}
                }
            }
            // `!alt_modifier`: Alt+Cmd combos belong solely to
            // the boolean-op arm above — without this, a gated
            // `Cmd+Alt+S` would fall through here and Save.
            Key::Character(ref ch)
                if self.zoom_modifier && !self.shift_modifier && !self.alt_modifier =>
            {
                let lower = ch.to_lowercase();
                match lower.as_str() {
                    // Cmd+, always allowed — it toggles the
                    // modal itself; closing while focused
                    // also commits via the close path.
                    "," => consumed = self.host.apply_toggle_agent_settings(),
                    "s" => {
                        // Codex stop-gate: commit any pending
                        // variable-row inline edit before save
                        // so the typed value lands on disk.
                        self.host.commit_variable_row_focus_if_any_pub();
                        consumed = persistence::handle_save(
                            &mut self.host,
                            &mut self.current_path,
                            self.window.as_ref(),
                        );
                        if consumed {
                            self.mark_document_saved();
                        }
                    }
                    "o" => {
                        self.host.commit_variable_row_focus_if_any_pub();
                        consumed = persistence::handle_open(
                            &mut self.host,
                            &mut self.current_path,
                            self.window.as_ref(),
                        );
                        if consumed {
                            self.mark_document_saved();
                        }
                    }
                    _ if settings_focused => {}
                    "d" => consumed = self.host.apply_duplicate(),
                    "a" => consumed = self.host.apply_select_all(),
                    "c" => consumed = self.host.apply_copy(),
                    "x" => consumed = self.host.apply_cut(),
                    "v" => consumed = self.host.apply_paste(),
                    "z" => consumed = self.host.apply_undo(),
                    "y" => consumed = self.host.apply_redo(),
                    "g" => consumed = self.host.apply_group(),
                    "j" => consumed = self.host.apply_toggle_chat(),
                    _ => {}
                }
            }
            // `!alt_modifier` for the same reason as the
            // Cmd-only arm — Alt+Cmd is the boolean arm's alone.
            Key::Character(ref ch)
                if self.zoom_modifier && self.shift_modifier && !self.alt_modifier =>
            {
                match ch.to_lowercase().as_str() {
                    // Cmd+Shift+S = Save As; always allowed.
                    "s" => {
                        self.host.commit_variable_row_focus_if_any_pub();
                        consumed = persistence::handle_save_as(
                            &mut self.host,
                            &mut self.current_path,
                            self.window.as_ref(),
                        );
                        if consumed {
                            self.mark_document_saved();
                        }
                    }
                    "p" => {
                        self.host.commit_variable_row_focus_if_any_pub();
                        persistence::run_action(
                            op_editor_core::editor_ui_state::FileAction::ExportImage,
                            &mut self.host,
                            &mut self.current_path,
                            self.window.as_ref(),
                        );
                        consumed = true;
                    }
                    _ if settings_focused => {}
                    "z" => consumed = self.host.apply_redo(),
                    "g" => consumed = self.host.apply_ungroup(),
                    "c" => consumed = self.host.apply_toggle_code_panel(),
                    _ => {}
                }
            }
            // Single-letter tool switches (no modifier). Only
            // fire when no input is focused so typing in a
            // text node / chat / rename doesn't switch tools.
            Key::Character(ref ch) if !self.zoom_modifier && !self.host.input_active_pub() => {
                let lower = ch.to_lowercase();
                let mut handled = true;
                match lower.as_str() {
                    "v" => self.host.apply_set_tool(op_editor_core::Tool::Select),
                    "r" => self.host.apply_set_tool(op_editor_core::Tool::Rect),
                    "o" => self.host.apply_set_tool(op_editor_core::Tool::Ellipse),
                    "l" => self.host.apply_set_tool(op_editor_core::Tool::Line),
                    "t" => self.host.apply_set_tool(op_editor_core::Tool::Text),
                    "f" => self.host.apply_set_tool(op_editor_core::Tool::Frame),
                    "p" => self.host.apply_set_tool(op_editor_core::Tool::Pen),
                    "h" => self.host.apply_set_tool(op_editor_core::Tool::Hand),
                    "[" => {
                        consumed = self.host.apply_reorder(ReorderDirection::Down);
                        handled = false;
                    }
                    "]" => {
                        consumed = self.host.apply_reorder(ReorderDirection::Up);
                        handled = false;
                    }
                    _ => handled = false,
                }
                if handled {
                    consumed = true;
                }
            }
            // `[` / `]` — z-order reorder when an input is focused (still gated by apply_reorder internally).
            Key::Character(ref ch) if !self.zoom_modifier => match ch.as_str() {
                "[" if !settings_focused => {
                    consumed = self.host.apply_reorder(ReorderDirection::Down)
                }
                "]" if !settings_focused => {
                    consumed = self.host.apply_reorder(ReorderDirection::Up)
                }
                _ => {
                    if let Some(s) = text {
                        for c in s.chars() {
                            if !c.is_control() && self.host.apply_text(c) {
                                consumed = true;
                            }
                        }
                    }
                }
            },
            _ => {
                // Suppress apply_text whenever Cmd / Ctrl
                // is held — Cmd-anything that isn't bound
                // above must NOT type into a focused chat
                // / property input. Otherwise Cmd+Shift+D
                // (and other unbound chords) would inject
                // "D" into the focused input.
                if !self.zoom_modifier {
                    if let Some(s) = text {
                        for c in s.chars() {
                            if !c.is_control() && self.host.apply_text(c) {
                                consumed = true;
                            }
                        }
                    }
                }
            }
        }
        if consumed {
            self.request_redraw(true);
        }
    }
}
