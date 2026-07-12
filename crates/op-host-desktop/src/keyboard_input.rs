//! `DesktopApp::handle_key_pressed` — the editor keyboard-shortcut
//! dispatch table. Split out of `app_handler.rs` to keep that file
//! under the repo's 800-line-per-file cap.

use crate::{chat_session, persistence, DesktopApp};
use base64::Engine as _;
use winit::keyboard::{Key, NamedKey};

/// Snapshot of every system-clipboard flavour relevant to Cmd/Ctrl+V.
/// Tests inject this directly so paste precedence is deterministic and
/// never depends on the developer machine's clipboard contents.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClipboardPayload {
    pub(crate) text: Option<String>,
    pub(crate) html: Option<String>,
    pub(crate) image: Option<crate::clipboard::ClipboardImage>,
}

impl ClipboardPayload {
    fn read_system() -> Self {
        let (text, html, image) = crate::clipboard::read_paste_flavours();
        Self { text, html, image }
    }
}

impl DesktopApp {
    /// Dispatch a pressed key (`logical_key` + its `text`) — the
    /// editor's keyboard-shortcut table. Called from the winit
    /// `KeyboardInput` event in `app_handler.rs`.
    pub(crate) fn handle_key_pressed(&mut self, logical_key: &Key, text: Option<&str>) {
        use op_editor_core::ReorderDirection;
        let mut consumed = false;
        let nudge = if self.shift_modifier { 10.0 } else { 1.0 };
        // While a settings-modal input OR any Git-panel text input (commit
        // message, remote URL, HTTPS credential, author name / email,
        // branch-create, or the inline clone wizard) owns the keyboard, the
        // ONLY allowed paths are text / backspace / send / escape and the
        // C/X/V/A clipboard chords. Editor shortcuts (Cmd+D, Cmd+G, Cmd+Z,
        // Cmd+J, arrow nudges, Delete, [ / ], single-letter tool switches,
        // …) would otherwise silently mutate the document or shift focus
        // while the user thinks they are typing into the input.
        let settings_focused = self.host.settings_focus_active()
            || self.host.git_commit_focus_active()
            || self.host.git_remote_focus_active()
            || self.host.git_https_focus_active()
            || self.host.git_author_focus_active()
            || self.host.git_branch_create_focus_active()
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
                if self.launch_chat_if_pending() {
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
            // Preview (Play) mode owns Tab + arrows: Tab advances the
            // widget focus chain (the runner otherwise drops Tab as a
            // control char, so the user could never focus an input
            // without clicking), and arrows move the caret / selection
            // in the focused field. These must precede the editor arrow
            // arms below so preview wins while active.
            Key::Named(NamedKey::Tab) if self.host.preview_active() => {
                consumed = self.host.preview_focus(self.shift_modifier);
            }
            Key::Named(NamedKey::ArrowLeft) if self.host.preview_active() => {
                consumed = self
                    .host
                    .preview_dispatch_key("ArrowLeft", self.shift_modifier);
            }
            Key::Named(NamedKey::ArrowRight) if self.host.preview_active() => {
                consumed = self
                    .host
                    .preview_dispatch_key("ArrowRight", self.shift_modifier);
            }
            Key::Named(NamedKey::ArrowUp) if self.host.preview_active() => {
                consumed = self
                    .host
                    .preview_dispatch_key("ArrowUp", self.shift_modifier);
            }
            Key::Named(NamedKey::ArrowDown) if self.host.preview_active() => {
                consumed = self
                    .host
                    .preview_dispatch_key("ArrowDown", self.shift_modifier);
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
                    || self.host.apply_chat_input_caret(false)
                    || self.host.apply_rename_caret(false)
                    || self.host.apply_text_edit_caret(false)
                    || self.host.apply_property_caret(false)
                    || self.host.apply_nudge(-nudge, 0.0);
            }
            Key::Named(NamedKey::ArrowRight) if !self.zoom_modifier && !settings_focused => {
                consumed = self.host.apply_chat_model_picker_caret(true)
                    || self.host.apply_chat_input_caret(true)
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
                    // Cmd+C / X / V / A operate on whichever text input
                    // owns the keyboard (chat / settings / git / property
                    // / rename / variables / model-picker / canvas text
                    // editor), falling back to the document node clipboard
                    // / canvas select-all when nothing is focused. Handled
                    // BEFORE the `settings_focused` swallow so they keep
                    // working inside the settings / Git / clone inputs.
                    "c" => consumed = self.handle_cmd_copy(),
                    "x" => consumed = self.handle_cmd_cut(),
                    "v" => consumed = self.handle_cmd_paste(),
                    "a" => consumed = self.host.apply_select_all(),
                    _ if settings_focused => {}
                    "d" => consumed = self.host.apply_duplicate(),
                    "z" => consumed = self.host.apply_undo(),
                    "y" => consumed = self.host.apply_redo(),
                    "g" => consumed = self.host.apply_group(),
                    "j" => consumed = self.host.apply_toggle_chat(),
                    // Cmd/Ctrl+T — open a fresh chat tab (MT.3). Preserves
                    // existing tabs and leaves any in-flight run bound to its
                    // own tab. Gated after the `settings_focused` swallow like
                    // the other chrome chords (Cmd+J), so it doesn't fire while
                    // a settings / Git input owns the keyboard.
                    "t" => {
                        self.new_chat_tab();
                        consumed = true;
                    }
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
    /// Cmd+C dispatch. The chat input copies its whole buffer (its own
    /// selection model); any other focused text field copies its
    /// highlighted slice; with no input focused, the document node
    /// clipboard (or a selected code / transcript slice) is copied via
    /// [`WidgetHostNative::apply_copy`]. The text-input branches always
    /// consume the chord so Cmd+C never falls through to node copy
    /// while a field owns the keyboard.
    ///
    /// `pub(crate)` so the native Edit-menu dispatch
    /// (`menu_action.rs`) routes through the same input-aware path —
    /// on macOS / Windows the menu accelerator owns Cmd+C and the
    /// winit keydown never reaches `handle_key_pressed`.
    pub(crate) fn handle_cmd_copy(&mut self) -> bool {
        if self.host.editor_state().chat.focused {
            let text = self.host.editor_state().chat.input.text();
            if !text.is_empty() {
                crate::clipboard::set_text(text);
            }
            return true;
        }
        if self.host.input_active_pub() {
            if let Some(text) = self.host.input_copy_text() {
                crate::clipboard::set_text(&text);
            }
            return true;
        }
        self.host.apply_copy()
    }

    /// Cmd+X dispatch. Cuts the focused text input's selection to the OS
    /// clipboard (chat input via its own cut path), else cuts the
    /// document node selection. `pub(crate)` for the Edit-menu path.
    pub(crate) fn handle_cmd_cut(&mut self) -> bool {
        if self.host.editor_state().chat.focused {
            if let Some(text) = self.host.chat_input_cut() {
                crate::clipboard::set_text(&text);
            }
            return true;
        }
        if self.host.input_active_pub() {
            if let Some(text) = self.host.input_cut_text() {
                crate::clipboard::set_text(&text);
            }
            return true;
        }
        self.host.apply_cut()
    }

    /// Cmd+V dispatch. Pastes OS clipboard text into whichever text
    /// input owns the keyboard (the chat input also accepts a pasted
    /// image, per `ai-chat-input.tsx:85-94`); with no input focused,
    /// pastes Figma clipboard HTML or the document node clipboard onto
    /// the canvas. `pub(crate)` for the Edit-menu path.
    pub(crate) fn handle_cmd_paste(&mut self) -> bool {
        self.handle_paste_payload(ClipboardPayload::read_system())
    }

    /// Input-aware paste router over an already-read clipboard snapshot.
    /// Keeping OS access outside this seam makes precedence directly
    /// testable and guarantees one clipboard handle per paste gesture.
    pub(crate) fn handle_paste_payload(&mut self, mut payload: ClipboardPayload) -> bool {
        // Non-chat text inputs (settings, git, rename, etc.) are
        // checked first so that opening a modal (e.g. agent settings
        // via Cmd+,) while the chat input is focused routes the paste
        // to the modal's field instead of the chat.
        if self.host.non_chat_input_owns_keyboard_pub() {
            if let Some(text) = payload.text.as_deref() {
                self.host.apply_input_paste(text);
            }
            return true;
        }
        if self.host.chat_input_owns_keyboard_pub() {
            // Clipboard image data wins over text when pasting into the
            // chat input; the paste is consumed either way.
            if let Some(image) = payload.image.take() {
                self.paste_image_into_chat(image);
            } else if let Some(text) = payload.text.as_deref() {
                self.host.chat_input_paste(text);
            }
            return true;
        }
        if let Some(html) = payload.html.take() {
            if let Some(result) = self.try_figma_clipboard_paste(html) {
                return result;
            }
        }
        if let Some(image) = payload.image {
            return self.paste_image_to_canvas(image);
        }
        self.host.apply_paste()
    }

    /// Stage a clipboard image as a chat attachment (Cmd+V while the
    /// chat input is focused). Mirrors the TS chat input's paste
    /// handler (`ai-chat-input.tsx:85-94`): image data wins over text
    /// and the paste is consumed even when nothing stages — TS
    /// filters oversized files after `preventDefault()`, and
    /// `add_attachment` enforces the same 4 × 5 MB caps here.
    fn paste_image_into_chat(&mut self, image: crate::clipboard::ClipboardImage) {
        // TS names pasted clipboard images "pasted-image.png".
        self.host
            .editor_state_mut()
            .chat
            .add_attachment(op_editor_core::chat::ChatAttachment {
                name: "pasted-image.png".to_string(),
                media_type: "image/png".to_string(),
                data: image.png,
            });
    }

    fn paste_image_to_canvas(&mut self, image: crate::clipboard::ClipboardImage) -> bool {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&image.png);
        let src = format!("data:image/png;base64,{encoded}");
        let inserted = self
            .host
            .editor_state_mut()
            .insert_image_node_at_viewport_sized(
                "pasted-image.png",
                &src,
                image.width,
                image.height,
            )
            .is_some();
        if inserted {
            self.host.mark_editor_state_dirty();
        }
        inserted
    }

    /// Probe the system clipboard for Figma HTML (Cmd+C in Figma) and
    /// kick off the decode on a worker thread — the base64 + kiwi
    /// `.fig` parse can be heavy and must not stall the keyboard
    /// handler. The UI thread only reads the clipboard and sniffs the
    /// marker; `pump_figma_clipboard_paste` applies the parsed nodes
    /// on a later frame. `None` when the clipboard holds no Figma
    /// payload (caller falls back to the internal node clipboard).
    fn try_figma_clipboard_paste(&mut self, html: String) -> Option<bool> {
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

    // ----------------------------------------------------------------------
    // MT.3 chat-tab run binding — launch / drain wrappers that keep
    // `chat_running_tab` in sync with the in-flight `current_chat` /
    // `current_design` sessions.
    // ----------------------------------------------------------------------

    /// Launch a pending chat send and BIND the run to the tab it started on.
    ///
    /// `launch_if_pending` consumes `pending_send` and parks a `ChatSession` /
    /// `DesignSession` (or, on an honest-error path, neither). When a session
    /// actually starts, record the active tab as `chat_running_tab` so the
    /// pumps target it even after the user switches tabs. When nothing started
    /// (error bubble only), leave the binding untouched.
    pub(crate) fn launch_chat_if_pending(&mut self) -> bool {
        let launched = chat_session::launch_if_pending(
            &mut self.host,
            &mut self.current_chat,
            &mut self.current_design,
        );
        if launched && (self.current_chat.is_some() || self.current_design.is_some()) {
            self.chat_running_tab = Some(self.host.editor_state().chat.active_index());
        }
        launched
    }

    /// Drain a New Chat request (the widget handler already opened the fresh
    /// tab). Aborts any in-flight worker and clears the now-stale tab binding.
    pub(crate) fn drain_new_chat(&mut self) -> bool {
        let drained = chat_session::drain_new_chat_request(
            &mut self.host,
            &mut self.current_chat,
            &mut self.current_design,
        );
        if drained {
            self.chat_running_tab = None;
        }
        drained
    }

    /// Drain a Stop request — aborts the in-flight worker and clears the tab
    /// binding so a later pump can't target a finished run.
    pub(crate) fn drain_stop_chat(&mut self) -> bool {
        let drained = chat_session::drain_stop_request(
            &mut self.host,
            &mut self.current_chat,
            &mut self.current_design,
        );
        if drained {
            self.chat_running_tab = None;
        }
        drained
    }

    /// Close chat tab `idx` (MT.3 `AIChatHit::CloseTab`). When the closed tab
    /// is the one a run is bound to, abort the run FIRST (drop both sessions +
    /// clear the binding) so the pump never targets a removed / shifted tab.
    /// Otherwise the binding is shifted to follow the surviving tab.
    pub(crate) fn close_chat_tab(&mut self, idx: usize) {
        if idx >= self.host.editor_state().chat.tab_count() {
            return; // out of range — mirror ChatSessions::close_tab no-op
        }
        if self.chat_running_tab == Some(idx) {
            // The run's tab is going away — abort it before the index shifts.
            // Drop the chat / design sessions and any sub-agent loops bound to
            // this tab (ending their canvas-indicator epoch so no badge glow
            // gets stuck). The top-level design indicator self-heals next frame
            // (its teardown is gated on `current_chat.is_none()`).
            self.current_chat = None;
            self.current_design = None;
            crate::sub_agent_session::abort_all(&mut self.sub_agents, &mut self.active_sub_agent);
            self.chat_running_tab = None;
        } else if let Some(running) = self.chat_running_tab {
            self.chat_running_tab = op_editor_core::adjust_running_tab_after_close(running, idx);
        }
        self.host.editor_state_mut().chat.close_tab(idx);
        // Session set mutated (possibly same-index replacement): rotate the
        // transcript-cache owner so a pre-repaint cursor hint can't pair the
        // closed session's cached geometry with the survivor's messages.
        self.host.force_rotate_chat_owner();
        self.host.mark_editor_state_dirty();
    }

    /// Drain a pending close-tab request raised by the chat tab-row close-×
    /// (MT.3 `AIChatHit::CloseTab` → `editor_ui.pending_close_chat_tab`).
    /// Routes through [`Self::close_chat_tab`] so a run bound to the closed
    /// tab is aborted before the index shifts.
    pub(crate) fn drain_close_chat_tab(&mut self) -> bool {
        let Some(idx) = self
            .host
            .editor_state_mut()
            .editor_ui
            .pending_close_chat_tab
            .take()
        else {
            return false;
        };
        self.close_chat_tab(idx);
        true
    }

    /// Open a fresh chat tab (⌘T / Ctrl+T). Preserves all existing tabs and
    /// does NOT abort an in-flight run — the run keeps streaming into its own
    /// tab while the user composes in the new one.
    pub(crate) fn new_chat_tab(&mut self) {
        self.host.editor_state_mut().chat.new_tab();
        // New active session: rotate the transcript-cache owner so the
        // event-time cursor hint reads `None` until this tab's first paint.
        self.host.force_rotate_chat_owner();
        self.host.mark_editor_state_dirty();
    }
}

#[cfg(test)]
#[path = "keyboard_input_tests.rs"]
mod tests;
