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
        // While a settings-modal input, the Git commit-message input, OR
        // the inline clone wizard owns the keyboard, the ONLY allowed
        // paths are text / backspace / send / escape. Editor shortcuts
        // (Cmd+D, Cmd+G, Cmd+Z, arrow nudges, Delete, [ / ], single-letter
        // tool switches, …) would otherwise silently mutate the document
        // while the user thinks they are typing into the input.
        let settings_focused = self.host.settings_focus_active()
            || self.host.git_commit_focus_active()
            || self.host.git_clone_input_active();
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
                if chat_session::launch_if_pending(
                    &mut self.host,
                    &mut self.current_chat,
                    &mut self.current_design,
                ) {
                    self.request_redraw(true);
                }
            }
            Key::Named(NamedKey::Space) if !self.zoom_modifier && !self.host.input_active_pub() => {
                // Transient space-pan (TS parity) — released in the
                // app handler's KeyboardInput Released arm.
                self.host.set_space_pan(true);
                consumed = true;
            }
            Key::Named(NamedKey::Escape) if !self.zoom_modifier => {
                consumed = self.host.apply_escape();
            }
            Key::Named(NamedKey::ArrowUp) if !self.zoom_modifier && !settings_focused => {
                // The inline canvas text editor moves its caret by
                // visual line first; then a focused numeric property
                // input steps its value; otherwise the arrow nudges
                // the selection.
                consumed = self.host.apply_text_edit_vertical(false)
                    || self.host.apply_property_step(nudge)
                    || self.host.apply_nudge(0.0, -nudge);
            }
            Key::Named(NamedKey::ArrowDown) if !self.zoom_modifier && !settings_focused => {
                consumed = self.host.apply_text_edit_vertical(true)
                    || self.host.apply_property_step(-nudge)
                    || self.host.apply_nudge(0.0, nudge);
            }
            Key::Named(NamedKey::ArrowLeft)
                if !self.zoom_modifier && self.host.settings_focus_active() =>
            {
                consumed = self.host.apply_settings_caret(false);
            }
            Key::Named(NamedKey::ArrowRight)
                if !self.zoom_modifier && self.host.settings_focus_active() =>
            {
                consumed = self.host.apply_settings_caret(true);
            }
            // Cmd/Ctrl+Left / Right — line start / end while the
            // inline canvas text editor is active (textarea Home/End
            // parity). Unbound otherwise, so a `false` just drops.
            Key::Named(NamedKey::ArrowLeft) if self.zoom_modifier && !settings_focused => {
                consumed = self.host.apply_text_edit_line_edge(false);
            }
            Key::Named(NamedKey::ArrowRight) if self.zoom_modifier && !settings_focused => {
                consumed = self.host.apply_text_edit_line_edge(true);
            }
            Key::Named(NamedKey::ArrowLeft) if !self.zoom_modifier && !settings_focused => {
                // An active inline rename moves its caret first, then
                // the canvas text editor, then a focused property
                // input; otherwise the arrow nudges the selection.
                consumed = self.host.apply_chat_model_picker_caret(false)
                    || self.host.apply_rename_caret(false)
                    || self.host.apply_text_edit_caret(false)
                    || self.host.apply_property_caret(false)
                    || self.host.apply_nudge(-nudge, 0.0);
            }
            Key::Named(NamedKey::ArrowRight) if !self.zoom_modifier && !settings_focused => {
                consumed = self.host.apply_chat_model_picker_caret(true)
                    || self.host.apply_rename_caret(true)
                    || self.host.apply_text_edit_caret(true)
                    || self.host.apply_property_caret(true)
                    || self.host.apply_nudge(nudge, 0.0);
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
                    "k" => consumed = self.host.apply_create_component(),
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
                    // Cmd+V pastes OS clipboard text into a focused text
                    // input (clone-wizard URL / destination, commit
                    // message, settings field) — placed before the
                    // `settings_focused` swallow below, which would
                    // otherwise eat the paste and route nothing.
                    "v" if settings_focused => {
                        if let Some(text) = crate::clipboard::get_text() {
                            self.host.apply_input_paste(&text);
                        }
                        consumed = true;
                    }
                    "a" if settings_focused => consumed = self.host.apply_select_all(),
                    _ if settings_focused => {}
                    "d" => consumed = self.host.apply_duplicate(),
                    "a" => consumed = self.host.apply_select_all(),
                    // Cmd+C / X / V route to the OS *text* clipboard
                    // when the AI chat input owns the keyboard, and
                    // to the document *node* clipboard otherwise.
                    "c" => {
                        consumed = if self.host.editor_state().chat.focused {
                            let text = self.host.editor_state().chat.input.clone();
                            if !text.is_empty() {
                                crate::clipboard::set_text(&text);
                            }
                            true
                        } else {
                            self.host.apply_copy()
                        };
                    }
                    "x" => {
                        consumed = if self.host.editor_state().chat.focused {
                            if let Some(text) = self.host.chat_input_cut() {
                                crate::clipboard::set_text(&text);
                            }
                            true
                        } else {
                            self.host.apply_cut()
                        };
                    }
                    "v" => {
                        consumed = if self.host.editor_state().chat.focused {
                            // TS ai-chat-input.tsx:85-94 — clipboard
                            // image data takes priority over text when
                            // pasting into the chat input (the paste
                            // is consumed either way).
                            if !self.try_paste_image_into_chat() {
                                if let Some(text) = crate::clipboard::get_text() {
                                    self.host.chat_input_paste(&text);
                                }
                            }
                            true
                        } else if let Some(result) = self.try_figma_clipboard_paste() {
                            result
                        } else {
                            self.host.apply_paste()
                        };
                    }
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
                    "v" => consumed = self.host.apply_toggle_variables_panel(),
                    "d" => consumed = self.host.apply_toggle_design_md_panel(),
                    "k" => consumed = self.host.apply_toggle_component_browser(),
                    "f" => consumed = self.host.apply_open_figma_import(),
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
                    "y" => self.host.apply_set_tool(op_editor_core::Tool::Polygon),
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
    /// Stage a clipboard image as a chat attachment (Cmd+V while the
    /// chat input is focused). Mirrors the TS chat input's paste
    /// handler (`ai-chat-input.tsx:85-94`): image data wins over text
    /// and the paste is consumed even when nothing stages — TS
    /// filters oversized files after `preventDefault()`, and
    /// `add_attachment` enforces the same 4 × 5 MB caps here.
    /// Returns false when the clipboard holds no image (caller falls
    /// through to the text paste).
    fn try_paste_image_into_chat(&mut self) -> bool {
        let Some(png) = crate::clipboard::get_image() else {
            return false;
        };
        // TS names pasted clipboard images "pasted-image.png".
        self.host
            .editor_state_mut()
            .chat
            .add_attachment(op_editor_core::chat::ChatAttachment {
                name: "pasted-image.png".to_string(),
                media_type: "image/png".to_string(),
                data: png,
            });
        true
    }

    /// Probe the system clipboard for Figma HTML (Cmd+C in Figma) and
    /// kick off the decode on a worker thread — the base64 + kiwi
    /// `.fig` parse can be heavy and must not stall the keyboard
    /// handler. The UI thread only reads the clipboard and sniffs the
    /// marker; `pump_figma_clipboard_paste` applies the parsed nodes
    /// on a later frame. `None` when the clipboard holds no Figma
    /// payload (caller falls back to the internal node clipboard).
    fn try_figma_clipboard_paste(&mut self) -> Option<bool> {
        let html = crate::clipboard::get_html()?;
        if !op_figma::is_figma_clipboard_html(&html) {
            return None;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let nodes = op_figma::extract_figma_clipboard_data(&html)
                .map(|data| op_figma::figma_clipboard_to_nodes(&data.buffer, Some(&html)).nodes)
                .unwrap_or_default();
            let _ = tx.send(nodes);
        });
        self.pending_figma_paste = Some(rx);
        // Consumed — the paste lands asynchronously (or decodes to
        // nothing and is silently dropped, never raw-HTML-pasted).
        Some(true)
    }

    /// Drain a finished clipboard decode — inserts the nodes centred
    /// on the viewport. Called once per frame by the redraw path.
    pub(crate) fn pump_figma_clipboard_paste(&mut self) -> bool {
        let Some(rx) = self.pending_figma_paste.as_ref() else {
            return false;
        };
        match rx.try_recv() {
            Ok(nodes) => {
                self.pending_figma_paste = None;
                !nodes.is_empty()
                    && self
                        .host
                        .paste_figma_nodes(nodes, self.viewport_width, self.viewport_height)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.pending_figma_paste = None;
                false
            }
        }
    }
}
