//! Turn lifecycle + input editing on [`ChatState`]: model selection,
//! send / stop / new-chat, transcript selection and the text-input
//! draft helpers.

use super::*;

impl ChatState {
    /// The currently selected model, or `None` when the catalog is
    /// empty.
    pub fn selected_model_entry(&self) -> Option<&ModelEntry> {
        self.available_models.get(self.selected_model)
    }

    /// Recompute [`available_models`] = [`discovered_models`] filtered
    /// to the providers the user has connected (`connected` is indexed
    /// by [`AgentProvider::ALL`]). The previously-selected model is
    /// preserved by identity when it survives the filter, otherwise
    /// `selected_model` falls back to `0`.
    ///
    /// Called by the host after model discovery completes and after
    /// every connect / disconnect toggle, so the picker only ever
    /// lists models the user can actually reach.
    ///
    /// [`available_models`]: ChatState::available_models
    /// [`discovered_models`]: ChatState::discovered_models
    pub fn rebuild_available_models(&mut self, connected: &[bool; 7]) {
        let prev = self.available_models.get(self.selected_model).cloned();
        self.available_models = self
            .discovered_models
            .iter()
            .filter(|m| {
                AgentProvider::ALL
                    .iter()
                    .position(|p| *p == m.provider)
                    .is_some_and(|i| connected[i])
            })
            .cloned()
            .collect();
        self.selected_model = prev
            .and_then(|p| {
                self.available_models.iter().position(|m| {
                    m.provider == p.provider
                        && m.value == p.value
                        && m.builtin_provider_id == p.builtin_provider_id
                })
            })
            .unwrap_or(0);
    }

    /// Append the focused input as a new user message + a stub
    /// assistant echo, then clear the buffer. Offline fallback used by
    /// hosts with no real `ChatProvider` wired.
    pub fn send(&mut self) {
        let trimmed = self.input.text().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let echo = format!("(stub) Got it — \"{}\"", trimmed);
        self.auto_title_from_prompt(&trimmed);
        self.messages.push(ChatMessage::user(trimmed));
        self.messages.push(ChatMessage::assistant(echo));
        self.input.set_text("");
    }

    /// Real-send entry point. Pushes the user message + an empty
    /// streaming assistant message, clears the input, and raises
    /// `pending_send` so the desktop event loop launches a real
    /// provider turn. Returns true when a send was queued — a turn
    /// may be queued with text, with staged attachments, or both
    /// (TS parity: an attachment-only message is sendable).
    pub fn begin_send(&mut self) -> bool {
        let trimmed = self.input.text().trim().to_string();
        if trimmed.is_empty() && self.pending_attachments.is_empty() {
            return false;
        }
        // Built-in (API-key) models must not wear a CLI's name — a
        // DeepSeek turn labelled "Codex CLI" reads as the wrong engine
        // (measured, user report 2026-07-12). Design runs later restamp
        // this with the run's agent persona (canvas cursor parity).
        let agent_name = self.selected_model_entry().map(|entry| {
            entry
                .builtin_provider_display_name
                .clone()
                .unwrap_or_else(|| entry.provider.name().to_string())
        });
        self.auto_title_from_prompt(&trimmed);
        self.expand();
        // A turn still in flight is interrupted by this new send — its
        // assistant bubble will never reach `Done`. Clear every
        // `streaming` flag so a stale bubble doesn't animate forever;
        // only the bubble pushed below should stream.
        for msg in &mut self.messages {
            msg.streaming = false;
        }
        // Copy the staged *image* attachments into the user message so
        // the transcript keeps showing them after the input strip is
        // cleared. Each gets a fresh decode-cache id. Non-image
        // attachments are dropped here (the backend can't draw them);
        // the host still drains `pending_attachments` for the request.
        let mut user_msg = ChatMessage::user(trimmed.clone());
        for att in &self.pending_attachments {
            if att.is_image() {
                let id = alloc_image_id();
                user_msg.images.push(ChatImage {
                    id,
                    name: att.name.clone(),
                    media_type: att.media_type.clone(),
                    data: att.data.clone(),
                });
            }
        }
        self.messages.push(user_msg);
        // Empty streaming assistant bubble — provider deltas append here.
        let mut assistant_msg = ChatMessage::assistant_streaming();
        assistant_msg.agent_name = agent_name;
        self.messages.push(assistant_msg);
        self.input.set_text("");
        // Jump to the bottom so the new turn's reply is visible as it
        // streams, even if the user had scrolled up in the prior turn.
        self.transcript_pinned = true;
        self.transcript_scroll.offset = 0.0;
        self.pending_send = Some(trimmed);
        true
    }

    pub fn has_streaming_turn(&self) -> bool {
        self.pending_send.is_some() || self.messages.iter().any(|msg| msg.streaming)
    }

    /// Whether the panel is showing its compact input bar. Reads the
    /// legacy [`collapsed`] flag too so pre-split state resolves to the
    /// bar instead of a panel form that no longer exists.
    ///
    /// [`collapsed`]: ChatState::collapsed
    pub fn is_minimized(&self) -> bool {
        self.minimized || self.collapsed
    }

    /// Drop the panel to its compact input bar. Entering a document
    /// (app launch, open / recent / drop, template, web load) starts
    /// here; the legacy `collapsed` flag is cleared so the panel has one
    /// unambiguous non-expanded form.
    pub fn minimize(&mut self) {
        self.minimized = true;
        self.collapsed = false;
    }

    /// Raise the panel back to its normal form.
    pub fn expand(&mut self) {
        self.minimized = false;
        self.collapsed = false;
    }

    /// Flip between the compact bar and the normal panel. A streaming
    /// turn always expands — hiding a reply as it arrives reads as the
    /// turn having been lost.
    pub fn toggle_minimized(&mut self) {
        if self.has_streaming_turn() || self.is_minimized() {
            self.expand();
        } else {
            self.minimize();
        }
    }

    /// Stop the currently streaming turn while keeping the visible
    /// transcript. Returns true when either a queued send or streaming
    /// bubble was actually cancelled.
    pub fn stop_streaming(&mut self) -> bool {
        let had_pending = self.pending_send.take().is_some();
        let mut had_streaming = false;
        for msg in &mut self.messages {
            if msg.streaming {
                had_streaming = true;
                msg.streaming = false;
            }
        }
        if had_pending || had_streaming {
            self.pending_stop_chat = true;
            true
        } else {
            false
        }
    }

    /// Start a fresh chat transcript and ask the host to abort any
    /// in-flight worker tied to the previous conversation.
    pub fn new_chat(&mut self) {
        self.messages.clear();
        self.title = DEFAULT_CHAT_TITLE.to_string();
        self.input.set_text("");
        self.pending_send = None;
        self.pending_stop_chat = false;
        self.pending_copy_text = None;
        self.transcript_selection = None;
        self.transcript_pinned = true;
        self.transcript_scroll.offset = 0.0;
        self.pending_attachments.clear();
        self.pending_attachment_pick = false;
        self.pending_new_chat = true;
    }

    pub fn queue_copy_text(&mut self, text: impl Into<String>) {
        self.pending_copy_text = Some(text.into());
    }

    fn auto_title_from_prompt(&mut self, prompt: &str) {
        if self.title.trim().is_empty() || self.title == DEFAULT_CHAT_TITLE {
            if let Some(title) = suggest_chat_title(prompt) {
                self.title = title;
            }
        }
    }

    pub fn selected_transcript_text(&self) -> Option<&str> {
        let selection = self.transcript_selection?;
        if selection.is_collapsed() {
            return None;
        }
        let text = &self.messages.get(selection.message_index)?.content;
        if text.is_empty() {
            return None;
        }
        let (start, end) = selection.ordered();
        let start = prev_char_boundary(text, start.min(text.len()));
        let end = prev_char_boundary(text, end.min(text.len()));
        (start < end).then_some(&text[start..end])
    }

    pub fn set_input_text(&mut self, text: impl Into<String>) {
        self.input.set_text(text);
    }

    pub fn focus_input_at_end(&mut self, now_ms: u64) {
        self.focused = true;
        self.input.set_caret(self.input.text().len(), now_ms);
    }

    pub fn blur_input(&mut self, now_ms: u64) {
        self.focused = false;
        // A marked candidate belongs to the focused platform session. Once
        // focus leaves, discard only that transient preedit; the durable
        // prompt remains untouched and the next focus starts cleanly.
        self.input.clear_composition();
        self.input.set_caret(self.input.caret(), now_ms);
    }

    pub fn set_input_caret(&mut self, offset: usize, now_ms: u64) {
        self.input
            .set_caret(offset.min(self.input.text().len()), now_ms);
    }

    pub fn input_caret(&self) -> usize {
        self.input.caret()
    }

    pub fn input_selection(&self) -> Selection {
        self.input.selection()
    }

    pub fn select_all_input(&mut self, now_ms: u64) {
        self.input.select_all();
        self.input.touch(now_ms);
    }

    pub fn drag_input_selection(&mut self, anchor: usize, focus: usize, now_ms: u64) -> bool {
        let before = self.input.selection();
        self.input.set_caret(anchor, now_ms);
        self.input.drag_to(focus, now_ms);
        before != self.input.selection()
    }

    pub fn selected_input_range(&self) -> Option<(usize, usize)> {
        let text = self.input.text();
        if text.is_empty() {
            return None;
        }
        let (start, end) = self.input.highlight_range()?;
        let start = prev_char_boundary(text, start.min(text.len()));
        let end = prev_char_boundary(text, end.min(text.len()));
        (start < end).then_some((start, end))
    }

    pub fn selected_input_text(&self) -> Option<&str> {
        let (start, end) = self.selected_input_range()?;
        Some(&self.input.text()[start..end])
    }

    pub fn insert_input_text(&mut self, text: &str, now_ms: u64) -> bool {
        if text.is_empty() {
            return false;
        }
        self.input.insert_str(text, now_ms);
        true
    }

    pub fn delete_input_selection(&mut self, now_ms: u64) -> bool {
        let Some((start, end)) = self.selected_input_range() else {
            return false;
        };
        debug_assert!(start < end);
        self.input.insert_str("", now_ms);
        true
    }

    pub fn backspace_input(&mut self, now_ms: u64) -> bool {
        let before = (self.input.text().to_owned(), self.input.selection());
        self.input.backspace(now_ms);
        before != (self.input.text().to_owned(), self.input.selection())
    }

    /// Whether the input viewport should ignore [`ChatState::input_scroll`]
    /// and reveal the caret line instead. True until a wheel writes an
    /// offset, and true again the moment the caret leaves where that wheel
    /// left it.
    ///
    /// [`ChatState::input_scroll`]: crate::chat::ChatState::input_scroll
    pub fn input_scroll_follows_caret(&self) -> bool {
        self.input_scroll_caret != self.input.caret()
    }

    /// Record a wheel-driven input scroll offset, pinning it to the caret
    /// position it was taken at. Returns whether anything moved.
    pub fn set_input_scroll(&mut self, offset: f32) -> bool {
        let caret = self.input.caret();
        let changed = self.input_scroll != offset || self.input_scroll_caret != caret;
        self.input_scroll = offset;
        self.input_scroll_caret = caret;
        changed
    }
}
