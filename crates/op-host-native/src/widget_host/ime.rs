//! Native IME composition routing (`Ime::Preedit` / `Ime::Commit`).
//!
//! TS parity target: Electron DOM inputs render preedit inline and
//! the OS anchors the candidate window at the caret. The Rust shell:
//!
//! - Canvas text edit stores preedit in `TextInputState::composition`
//!   so the canvas painter can render it inline with an underline.
//! - Other inputs still consume preedit updates without painting a
//!   floating overlay. `Ime::Commit` is where text enters OpenPencil.
//! - `apply_ime_commit` clears the preedit and lands the committed
//!   string through `apply_text` char-by-char, so every focus branch
//!   + per-field filter (numeric / hex drafts) applies unchanged.
//! - `ime_anchor_rect` resolves the focused input's caret rect for
//!   `set_ime_cursor_area`. v1 coverage: the chat input (precise).

use op_editor_ui::Rect;

use super::WidgetHostNative;

impl WidgetHostNative {
    /// True when a text input currently owns the keyboard — the same
    /// conditions `apply_text` routes on (keep in sync with
    /// `keyboard.rs::apply_text`).
    pub fn text_input_focus_active(&self) -> bool {
        let ui = &self.editor_state.editor_ui;
        ui.agent_settings.focus.is_some()
            || self.git_clone_input_active()
            || self.git_commit_focus_active()
            || self.git_remote_focus_active()
            || self.editor_state.ui.layer_rename.is_some()
            || self.editor_state.ui.text_editing.is_some()
            || ui.variable_row_focus.is_some()
            || self.editor_state.ui.property_focus.is_some()
            || self.editor_state.chat.focused
            || ui.icon_picker.open
            || ui.component_browser_open
    }

    /// `Ime::Preedit` — canvas text edit paints inline composition;
    /// other inputs keep the legacy no-floating-overlay behavior.
    pub fn apply_ime_preedit(&mut self, text: &str, cursor: Option<(usize, usize)>) -> bool {
        let had = self.editor_state.editor_ui.ime_preedit.take().is_some();
        if self.editor_state.ui.text_editing.is_some() {
            let changed = if text.is_empty() {
                self.editor_state.text_edit_clear_composition()
            } else {
                let cursor = cursor.map(|(_, end)| end).unwrap_or(text.len());
                self.editor_state
                    .text_edit_set_composition(text, cursor, self.now_ms)
            };
            if changed || had {
                self.mark_dirty();
            }
            return changed || had;
        }
        if had {
            self.mark_dirty();
        }
        had
    }

    /// `Ime::Commit` — clear the preedit and land the candidate
    /// string into whichever input owns the keyboard.
    pub fn apply_ime_commit(&mut self, text: &str) -> bool {
        if self.editor_state.editor_ui.ime_preedit.take().is_some() {
            self.mark_dirty();
        }
        if self.editor_state.ui.text_editing.is_some() {
            let consumed = if text.is_empty() {
                self.editor_state.text_edit_clear_composition()
            } else {
                self.editor_state
                    .text_edit_set_composition(text, text.len(), self.now_ms)
                    && self.editor_state.text_edit_commit_composition(self.now_ms)
            };
            if consumed {
                self.mark_dirty();
            }
            return consumed;
        }
        let mut consumed = false;
        for ch in text.chars() {
            if !ch.is_control() && self.apply_text(ch) {
                consumed = true;
            }
        }
        consumed
    }

    /// Focused-input caret rect for candidate-window anchoring. v1:
    /// chat input only; `None` = fallback.
    pub fn ime_anchor_rect(&mut self, viewport_w: f32, viewport_h: f32) -> Option<Rect> {
        if self.editor_state.chat.focused {
            let chat_rect = self.ai_chat_rect(viewport_w, viewport_h)?;
            let chat = op_editor_ui::widgets::AIChatPlaceholder::from_editor_at(
                &self.editor_state,
                self.now_ms,
            );
            return Some(chat.input_caret_rect(chat_rect));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::WidgetHostNative;
    use op_editor_core::NodeId;

    const TEXT_DOC: &str = r#"{"version":"0.8.0","children":[
      {"type":"text","id":"t1","name":"Label","x":0,"y":0,"width":100,"height":40,
       "content":"hello","fontSize":20}
    ]}"#;

    fn host() -> WidgetHostNative {
        WidgetHostNative::new()
    }

    fn start_canvas_text_edit(h: &mut WidgetHostNative) {
        let doc = jian_ops_schema::load_str(TEXT_DOC)
            .expect("fixture JSON parses")
            .value;
        *h.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
        assert!(h.editor_state_mut().start_text_edit(NodeId::new("t1")));
    }

    #[test]
    fn preedit_requires_a_text_input_focus() {
        let mut h = host();
        assert!(!h.apply_ime_preedit("你好", None), "no focus → no preedit");
        assert!(h.editor_state().editor_ui.ime_preedit.is_none());
    }

    #[test]
    fn preedit_does_not_create_floating_overlay_and_commit_lands_in_chat() {
        let mut h = host();
        h.editor_state_mut().chat.focused = true;
        assert!(
            !h.apply_ime_preedit("nih", Some((0, 3))),
            "preedit should not request redraw for a floating overlay"
        );
        assert!(h.editor_state().editor_ui.ime_preedit.is_none());

        assert!(!h.apply_ime_preedit("nih", Some((0, 3))));
        assert!(h.apply_ime_commit("你好"));
        assert!(h.editor_state().editor_ui.ime_preedit.is_none());
        assert!(h.editor_state().chat.input.text().contains("你好"));
    }

    #[test]
    fn empty_preedit_is_the_cancel_signal() {
        let mut h = host();
        h.editor_state_mut().chat.focused = true;
        assert!(!h.apply_ime_preedit("ni", None));
        assert!(
            !h.apply_ime_preedit("", None),
            "clear is a no-op when preedit overlay state is not stored"
        );
        assert!(h.editor_state().editor_ui.ime_preedit.is_none());
        assert!(!h.apply_ime_preedit("", None), "already clear → no-op");
    }

    #[test]
    fn canvas_text_edit_preedit_stays_in_composition_until_commit() {
        let mut h = host();
        start_canvas_text_edit(&mut h);

        assert!(h.apply_ime_preedit("ni", Some((0, 2))));
        assert_eq!(h.editor_state().text_edit_content(), Some("hello"));
        assert_eq!(
            h.editor_state()
                .ui
                .text_edit_input
                .composition()
                .map(|c| c.text.as_str()),
            Some("ni")
        );

        assert!(h.apply_ime_commit("你"));
        assert_eq!(h.editor_state().text_edit_content(), Some("hello你"));
        assert!(h.editor_state().ui.text_edit_input.composition().is_none());
    }

    #[test]
    fn chat_focus_yields_an_anchor_rect() {
        let mut h = host();
        h.editor_state_mut().chat.focused = true;
        h.last_viewport_w = 1200.0;
        h.last_viewport_h = 800.0;
        assert!(h.ime_anchor_rect(1200.0, 800.0).is_some());
        h.editor_state_mut().chat.focused = false;
        assert!(h.ime_anchor_rect(1200.0, 800.0).is_none());
    }

    #[test]
    fn chat_ime_anchor_tracks_input_caret() {
        let mut h = host();
        h.editor_state_mut().chat.focused = true;
        h.editor_state_mut().chat.set_input_text("abcd");
        h.editor_state_mut().chat.set_input_caret(0, 0);
        let start = h
            .ime_anchor_rect(1200.0, 800.0)
            .expect("chat focus should yield ime anchor");

        h.editor_state_mut().chat.set_input_caret(3, 0);
        let after_three = h
            .ime_anchor_rect(1200.0, 800.0)
            .expect("chat focus should yield ime anchor");

        assert!(
            after_three.origin.x > start.origin.x + 12.0,
            "expected IME anchor to move with caret: start={start:?}, after={after_three:?}"
        );
        assert!(
            after_three.size.x <= 4.0,
            "IME anchor should describe the caret, not the whole input: {after_three:?}"
        );
    }
}
