//! `DesktopApp::handle_key_pressed` — the editor keyboard-shortcut
//! dispatch table. Split out of `app_handler.rs` to keep that file
//! under the repo's 800-line-per-file cap.

use crate::{html_import_session, persistence, DesktopApp};
use base64::Engine as _;
use winit::keyboard::{Key, NamedKey};

pub(crate) use crate::keyboard_clipboard_payload::ClipboardPayload;

impl DesktopApp {
    /// Dispatch a pressed key (`logical_key` + its `text`) — the
    /// editor's keyboard-shortcut table. Called from the winit
    /// `KeyboardInput` event in `app_handler.rs`.
    pub(crate) fn handle_key_pressed(&mut self, logical_key: &Key, text: Option<&str>) {
        use op_editor_core::ReorderDirection;
        // A presentation claims its keys before the editor table below —
        // several of them (Backspace, Enter, Home, End, Space) have an
        // earlier arm there that would otherwise win. See
        // `keyboard_presenting`.
        if self.handle_presenting_key(logical_key) {
            self.request_redraw(true);
            return;
        }
        let mut consumed = false;
        let nudge = if self.shift_modifier { 10.0 } else { 1.0 };
        // Settings and Git inputs retain text, navigation, and clipboard
        // chords while blocking document shortcuts.
        let settings_focused = self.host.settings_focus_active()
            || self.host.git_commit_focus_active()
            || self.host.git_remote_focus_active()
            || self.host.git_https_focus_active()
            || self.host.git_author_focus_active()
            || self.host.git_branch_create_focus_active()
            || self.host.git_clone_input_active();
        let image_popover_open = {
            let panel = &self.host.editor_state().editor_ui.image_panel;
            panel.search_open || panel.generate_open
        };
        let prompt_center_open = self.host.editor_state().editor_ui.prompt_center.open;
        match logical_key {
            // Named-key shortcuts fire only when no Cmd/Ctrl is held.
            Key::Named(NamedKey::Backspace) if !self.zoom_modifier => {
                consumed = self.host.apply_backspace();
            }
            // The settings-modal input takes forward deletion; the host
            // arm consumes the key while any settings field is focused,
            // so a Delete can never reach the canvas selection behind
            // the modal. The Git inputs below keep swallowing Delete via
            // the `!settings_focused` gate on the generic arm.
            Key::Named(NamedKey::Delete)
                if !self.zoom_modifier && self.host.settings_focus_active() =>
            {
                consumed = self.host.apply_delete();
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
            Key::Named(NamedKey::ArrowLeft) if prompt_center_open && !self.zoom_modifier => {
                consumed = self
                    .host
                    .apply_prompt_center_caret(false, self.shift_modifier);
            }
            Key::Named(NamedKey::ArrowRight) if prompt_center_open && !self.zoom_modifier => {
                consumed = self
                    .host
                    .apply_prompt_center_caret(true, self.shift_modifier);
            }
            Key::Named(
                NamedKey::ArrowLeft
                | NamedKey::ArrowRight
                | NamedKey::ArrowUp
                | NamedKey::ArrowDown,
            ) if prompt_center_open => consumed = true,
            Key::Named(ref arrow) if self.scene_template_owns_arrow(arrow) => {
                consumed = self.apply_scene_template_arrow(arrow);
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
            Key::Named(NamedKey::ArrowLeft) if !self.zoom_modifier && image_popover_open => {
                consumed = self
                    .host
                    .apply_image_panel_caret(false, self.shift_modifier);
            }
            Key::Named(NamedKey::ArrowRight) if !self.zoom_modifier && image_popover_open => {
                consumed = self.host.apply_image_panel_caret(true, self.shift_modifier);
            }
            Key::Named(NamedKey::ArrowLeft) if self.zoom_modifier && image_popover_open => {
                consumed = self.host.apply_image_panel_edge(false, self.shift_modifier);
            }
            Key::Named(NamedKey::ArrowRight) if self.zoom_modifier && image_popover_open => {
                consumed = self.host.apply_image_panel_edge(true, self.shift_modifier);
            }
            Key::Named(NamedKey::Home) if image_popover_open => {
                consumed = self.host.apply_image_panel_edge(false, self.shift_modifier);
            }
            Key::Named(NamedKey::End) if image_popover_open => {
                consumed = self.host.apply_image_panel_edge(true, self.shift_modifier);
            }
            // A single-line image-popover input has no vertical caret motion,
            // but still owns these keys. Never let them nudge the selected
            // image behind the popup.
            Key::Named(NamedKey::ArrowUp | NamedKey::ArrowDown)
                if !self.zoom_modifier && image_popover_open =>
            {
                consumed = true;
            }
            // The bare-arrow priority chains live in `keyboard_input_arrows`.
            Key::Named(NamedKey::ArrowUp) if !self.zoom_modifier && !settings_focused => {
                consumed = self.arrow_vertical(false, nudge);
            }
            Key::Named(NamedKey::ArrowDown) if !self.zoom_modifier && !settings_focused => {
                consumed = self.arrow_vertical(true, nudge);
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
                consumed = self.arrow_horizontal(false, nudge);
            }
            Key::Named(NamedKey::ArrowRight) if !self.zoom_modifier && !settings_focused => {
                consumed = self.arrow_horizontal(true, nudge);
            }
            // Cmd/Ctrl+Alt+U/S/I/X — path boolean ops (Paper.js
            // parity). Gated on `!settings_focused` so they
            // never mutate the document while a settings /
            // Git-commit input owns the keyboard.
            Key::Character(ref ch)
                if self.zoom_modifier
                    && self.alt_modifier
                    && !self.shift_modifier
                    && !settings_focused
                    && !prompt_center_open
                    && !image_popover_open =>
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
                if prompt_center_open && !matches!(lower.as_str(), "a" | "c" | "v" | "x") {
                    consumed = true;
                } else {
                    match lower.as_str() {
                        // Cmd+, toggles the settings modal.
                        "," => consumed = self.host.apply_toggle_agent_settings(),
                        "s" => {
                            // Commit a pending variable edit before saving.
                            self.host.commit_variable_row_focus_if_any_pub();
                            consumed = self.request_background_save();
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
                        // Clipboard chords target the focused input before canvas.
                        "c" => consumed = self.handle_cmd_copy(),
                        "x" => consumed = self.handle_cmd_cut(),
                        "v" => consumed = self.handle_cmd_paste(),
                        "a" => consumed = self.host.apply_select_all(),
                        _ if image_popover_open => consumed = true,
                        _ if settings_focused => {}
                        // Cmd+P mirrors the TopBar Preview button.
                        "p" => {
                            consumed = self.host.toggle_preview_with_cached_viewport();
                        }
                        "d" => consumed = self.host.apply_duplicate(),
                        "z" => {
                            consumed = self.collab_runtime.request_undo(&mut self.host)
                                || self.host.apply_undo()
                        }
                        "y" => {
                            consumed = self.collab_runtime.reject_redo(&mut self.host)
                                || self.host.apply_redo()
                        }
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
            }
            // `!alt_modifier` for the same reason as the
            // Cmd-only arm — Alt+Cmd is the boolean arm's alone.
            Key::Character(ref ch)
                if self.zoom_modifier && self.shift_modifier && !self.alt_modifier =>
            {
                if prompt_center_open {
                    consumed = true;
                } else {
                    match ch.to_lowercase().as_str() {
                        // Cmd+Shift+S = Save As; always allowed.
                        "s" => {
                            self.host.commit_variable_row_focus_if_any_pub();
                            consumed = self.request_background_save_as();
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
                        _ if image_popover_open => consumed = true,
                        _ if settings_focused => {}
                        "z" => {
                            consumed = self.collab_runtime.reject_redo(&mut self.host)
                                || self.host.apply_redo()
                        }
                        "g" => consumed = self.host.apply_ungroup(),
                        "c" => consumed = self.host.apply_toggle_code_panel(),
                        "v" => consumed = self.host.apply_toggle_variables_panel(),
                        "d" => consumed = self.host.apply_toggle_design_md_panel(),
                        "k" => consumed = self.host.apply_toggle_component_browser(),
                        "f" => consumed = self.host.apply_open_figma_import(),
                        "h" => consumed = self.host.apply_open_html_import(),
                        _ => {}
                    }
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
                "[" if !settings_focused && !self.host.input_active_pub() => {
                    consumed = self.host.apply_reorder(ReorderDirection::Down)
                }
                "]" if !settings_focused && !self.host.input_active_pub() => {
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
                // Unbound Cmd/Ctrl chords must not type into an input.
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
        if self.host.chat_input_owns_keyboard_pub() {
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
        if self.host.chat_input_owns_keyboard_pub() {
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
        let payload = if self.host.non_chat_input_owns_keyboard_pub() {
            ClipboardPayload::read_text_system()
        } else if self.host.chat_input_owns_keyboard_pub() {
            ClipboardPayload::read_chat_system()
        } else {
            ClipboardPayload::read_canvas_system()
        };
        self.handle_paste_payload(payload)
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
            if let Some(result) = self.try_figma_clipboard_paste(html.clone()) {
                return result;
            }
            if let Some(result) = self.try_html_clipboard_paste(html) {
                return result;
            }
        }
        if let Some(image) = payload.image {
            return self.paste_image_to_canvas(image);
        }
        self.host.apply_paste()
    }

    /// Stage a clipboard image as a chat attachment. Image data wins over
    /// text, and `add_attachment` enforces the shared count/size limits.
    fn paste_image_into_chat(&mut self, image: crate::clipboard::ClipboardImage) {
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
        if !self.host.gate_collaboration_action(
            op_editor_core::CollabGateAction::Document(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::ClipboardPaste,
                ),
            ),
            op_editor_core::CollabEditSource::User,
        ) {
            return true;
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(&image.png);
        let src = format!("data:image/png;base64,{encoded}");
        let centre = op_editor_ui::widgets::host_canvas_geometry::canvas_centre_doc_point(
            self.host.editor_state(),
            self.viewport_width,
            self.viewport_height,
        );
        let inserted = self
            .host
            .editor_state_mut()
            .insert_image_node_at_doc_point_sized(
                "pasted-image.png",
                &src,
                image.width,
                image.height,
                (centre.x as f64, centre.y as f64),
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
        // Detached one-shot: a pure CPU decode over an owned `String` with no
        // IO and no loop, so it always returns. The epoch guard below (not a
        // thread signal) is what discards a stale result; a dropped `rx` just
        // makes the send a no-op.
        std::thread::spawn(move || {
            let nodes = op_figma::extract_figma_clipboard_data(&html)
                .map(|data| op_figma::figma_clipboard_to_nodes(&data.buffer, Some(&html)).nodes)
                .unwrap_or_default();
            let _ = tx.send(nodes);
        });
        // Capture the document epoch so a decode that finishes after
        // the user opens / creates / imports another document is
        // dropped instead of landing in the wrong document.
        self.pending_figma_paste = Some((self.host.document_epoch(), rx));
        // Consumed — the paste lands asynchronously (or decodes to
        // nothing and is silently dropped, never raw-HTML-pasted).
        Some(true)
    }

    /// Drain a finished clipboard decode — inserts the nodes centred
    /// on the viewport. Called once per frame by the redraw path.
    pub(crate) fn pump_figma_clipboard_paste(&mut self) -> bool {
        let Some((epoch, rx)) = self.pending_figma_paste.as_ref() else {
            return false;
        };
        let epoch = *epoch;
        match rx.try_recv() {
            Ok(nodes) => {
                self.pending_figma_paste = None;
                // Drop the result if the document was replaced while
                // the worker was decoding.
                epoch == self.host.document_epoch()
                    && !nodes.is_empty()
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

    /// Probe pasted HTML (any non-Figma `text/html` payload) and
    /// convert it to editable nodes on a worker thread — the CSS
    /// cascade can be heavy and must not stall the keyboard handler.
    /// `pump_html_clipboard_paste` applies the parsed nodes on a
    /// later frame. `None` when the payload is blank (caller falls
    /// back to the image / internal clipboard branches).
    fn try_html_clipboard_paste(&mut self, html: String) -> Option<bool> {
        if !html_paste_should_consume(&html) {
            return None;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        // Detached one-shot; same contract as `try_figma_clipboard_paste` — a
        // self-terminating CPU decode, staleness handled by the epoch guard.
        std::thread::spawn(move || {
            let result = op_html::import_html(&html, &op_html::HtmlImportOptions::default());
            // TYPED diagnostics travel: the overlay localizes each row.
            let _ = tx.send((result.nodes, result.diagnostics));
        });
        // Epoch-guard the decode (see `try_figma_clipboard_paste`).
        self.pending_html_paste = Some((self.host.document_epoch(), rx));
        // Consumed — the paste lands asynchronously (or decodes to
        // nothing and is silently dropped, never raw-HTML-pasted).
        Some(true)
    }

    /// Drain a finished HTML decode — inserts the nodes centred on
    /// the viewport. Called once per frame by the redraw path.
    pub(crate) fn pump_html_clipboard_paste(&mut self) -> bool {
        let Some((epoch, rx)) = self.pending_html_paste.as_ref() else {
            return false;
        };
        let epoch = *epoch;
        match rx.try_recv() {
            Ok((nodes, diagnostics)) => {
                self.pending_html_paste = None;
                for warning in op_html::render_warnings(&diagnostics) {
                    eprintln!("[import-html] warning: {warning}");
                }
                // Drop the result if the document was replaced while
                // the worker was decoding.
                if epoch != self.host.document_epoch() {
                    return false;
                }
                // A pasted page degrades exactly like an imported file, so it
                // gets the same GUI report instead of stderr-only warnings the
                // user never sees; an empty list clears a stale card.
                let rows = html_import_session::diagnostic_rows(&diagnostics);
                let reported = !rows.is_empty();
                self.host.show_html_import_diagnostics(rows);
                let (w, h) = (self.viewport_width, self.viewport_height);
                (!nodes.is_empty() && self.host.paste_figma_nodes(nodes, w, h)) || reported
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.pending_html_paste = None;
                false
            }
        }
    }
}

/// Whether a pasted `text/html` payload should be consumed by the
/// HTML importer (any non-blank markup qualifies; Figma payloads are
/// intercepted by `try_figma_clipboard_paste` first).
pub(crate) fn html_paste_should_consume(html: &str) -> bool {
    !html.trim().is_empty()
}

#[cfg(test)]
#[path = "keyboard_input_tests.rs"]
mod tests;
