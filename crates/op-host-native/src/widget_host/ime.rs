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
//!   string through each focus branch's text transition. The multiline
//!   provider Model field preserves normalized newlines; numeric / hex /
//!   other single-line drafts keep their existing filters.
//! - `ime_anchor_rect` resolves the focused input's caret rect for
//!   `set_ime_cursor_area`. Chat and image-popover inputs expose precise
//!   carets; the desktop shell supplies a cursor-position fallback for older
//!   focused fields that have not yet published their own geometry.

use op_editor_ui::widgets::PropertyPanel;
use op_editor_ui::Rect;

use super::WidgetHostNative;

impl WidgetHostNative {
    /// True when a text input currently owns the keyboard — the same
    /// conditions `apply_text` routes on (keep in sync with
    /// `keyboard.rs::apply_text`).
    pub fn text_input_focus_active(&self) -> bool {
        if self.editor_state.editor_ui.prompt_center.open {
            return true;
        }
        let panel = &self.editor_state.editor_ui.image_panel;
        if panel.search_open || panel.generate_open {
            let configured = self
                .editor_state
                .editor_ui
                .agent_settings
                .image_generation_configured();
            return panel.active_input(configured).is_some();
        }
        self.input_active()
    }

    /// `Ime::Preedit` — canvas text edit paints inline composition;
    /// other inputs keep the legacy no-floating-overlay behavior.
    pub fn apply_ime_preedit(&mut self, text: &str, cursor: Option<(usize, usize)>) -> bool {
        let had = self.editor_state.editor_ui.ime_preedit.take().is_some();
        if self.editor_state.editor_ui.prompt_center.open {
            if had {
                self.mark_dirty();
            }
            return had;
        }
        if self
            .editor_state
            .editor_ui
            .scene_template_center
            .input_active()
        {
            if had {
                self.mark_dirty();
            }
            return had;
        }
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            if had {
                self.mark_dirty();
            }
            return had;
        }
        if self.editor_state.ui.text_editing.is_some() {
            if !text.is_empty()
                && !self.collab_allows_document_mutation(
                    op_editor_core::CollabDocumentMutation::NodeProperty(
                        op_editor_core::CollabNodeField::Content,
                    ),
                )
            {
                return true;
            }
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
        if self.editor_state.editor_ui.prompt_center.open {
            let mut consumed = false;
            for ch in text.chars() {
                if !ch.is_control() && self.apply_text(ch) {
                    consumed = true;
                }
            }
            return consumed;
        }
        // Above the canvas-text branch on purpose: the gallery covers the
        // canvas, so a text node left mid-edit underneath it must not take
        // the candidate the user composed into the panel. Same stale-focus
        // rule the Prompt Center branch above encodes.
        if self
            .editor_state
            .editor_ui
            .scene_template_center
            .input_active()
        {
            let mut consumed = false;
            for ch in text.chars() {
                if !ch.is_control() && self.apply_text(ch) {
                    consumed = true;
                }
            }
            return consumed;
        }
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            let mut consumed = false;
            for ch in text.chars() {
                if !ch.is_control() && self.apply_image_panel_text(ch) {
                    consumed = true;
                }
            }
            return consumed;
        }
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_text_payload(text);
        }
        if self.editor_state.ui.text_editing.is_some() {
            if !text.is_empty()
                && !self.collab_allows_document_mutation(
                    op_editor_core::CollabDocumentMutation::NodeProperty(
                        op_editor_core::CollabNodeField::Content,
                    ),
                )
            {
                return true;
            }
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

    /// Focused-input caret rect for candidate-window anchoring. Persistent
    /// image-popover inputs take priority over a stale chat-focus bit.
    pub fn ime_anchor_rect(&mut self, viewport_w: f32, viewport_h: f32) -> Option<Rect> {
        let generate_configured = self
            .editor_state
            .editor_ui
            .agent_settings
            .image_generation_configured();
        let image_popover_open = self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open;
        if let (Some(panel), Some(rect)) = (
            op_editor_ui::widgets::PromptCenterPanel::for_editor(&self.editor_state),
            self.prompt_center_panel_rect(viewport_w, viewport_h),
        ) {
            return Some(panel.focused_input_caret_rect(rect));
        }
        if let (Some(panel), Some(rect)) = (
            op_editor_ui::widgets::SceneTemplatePanel::for_editor(&self.editor_state),
            self.scene_template_panel_rect(viewport_w, viewport_h),
        ) {
            return Some(panel.focused_input_caret_rect(rect));
        }
        if image_popover_open {
            self.editor_state
                .editor_ui
                .image_panel
                .active_input(generate_configured)?;
            if let Some(rect) = self.cached_image_input_caret_rect() {
                return Some(rect);
            }
            let panel = PropertyPanel::for_selection(&self.editor_state)?;
            let property_rect = self.property_rect(viewport_w, viewport_h);
            return panel.image_popover_input_caret_rect(property_rect);
        }
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
    use op_editor_core::{NodeId, PromptCenterFocus};
    use op_editor_ui::widgets::PropertyPanelAction as A;

    const TEXT_DOC: &str = r#"{"version":"1.0.0","children":[
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
    fn image_search_ime_commit_beats_stale_canvas_text_edit() {
        let mut h = host();
        start_canvas_text_edit(&mut h);
        h.editor_state_mut().editor_ui.image_panel.search_open = true;
        h.editor_state_mut()
            .editor_ui
            .image_panel
            .search_query
            .set_text("");

        assert!(h.apply_ime_commit("你好"));
        assert_eq!(
            h.editor_state().editor_ui.image_panel.search_query.text(),
            "你好"
        );
        assert_eq!(h.editor_state().text_edit_content(), Some("hello"));
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

    #[test]
    fn image_search_ime_anchor_tracks_persistent_caret_and_beats_stale_chat() {
        let mut h = host();
        let mut state = op_editor_core::EditorState::sample();
        let _ = state.insert_image_node_at_viewport("Hero photo", "https://x/y.png");
        state.chat.focused = true;
        *h.editor_state_mut() = state;
        h.apply_property_action(A::ToggleImageSearchPopover);
        h.last_viewport_w = 1200.0;
        h.last_viewport_h = 800.0;

        let input = &mut h.editor_state_mut().editor_ui.image_panel.search_query;
        input.set_text("abcd");
        input.set_caret(0, 0);
        let start = h
            .ime_anchor_rect(1200.0, 800.0)
            .expect("image search should yield IME anchor");

        h.editor_state_mut()
            .editor_ui
            .image_panel
            .search_query
            .set_caret(3, 0);
        // Simulate an independently stale bit: image input must still win.
        h.editor_state_mut().chat.focused = true;
        let after_three = h
            .ime_anchor_rect(1200.0, 800.0)
            .expect("image search should keep IME anchor");

        assert!(after_three.origin.x > start.origin.x + 5.0);
        assert_eq!(after_three.origin.y, start.origin.y);
    }

    #[test]
    fn prompt_ime_commit_beats_stale_canvas_edit_and_chat_focus() {
        let mut h = host();
        start_canvas_text_edit(&mut h);
        h.editor_state_mut().chat.focused = true;
        h.editor_state_mut().editor_ui.open_prompt_center(1);

        assert!(h.text_input_focus_active());
        assert!(
            !h.apply_ime_preedit("ni", Some((0, 2))),
            "non-canvas prompt preedit does not paint a floating overlay"
        );
        assert!(
            h.editor_state().ui.text_edit_input.composition().is_none(),
            "prompt preedit must not leak into the covered canvas editor"
        );

        assert!(h.apply_ime_commit("你好"));
        assert_eq!(
            h.editor_state().editor_ui.prompt_center.search.text(),
            "你好"
        );
        assert_eq!(h.editor_state().text_edit_content(), Some("hello"));
        assert!(
            h.editor_state().chat.input.text().is_empty(),
            "stale chat focus must not take the prompt commit"
        );
    }

    #[test]
    fn prompt_ime_anchor_tracks_search_and_save_title_carets() {
        let mut h = host();
        h.editor_state_mut().chat.focused = true;
        h.editor_state_mut().editor_ui.open_prompt_center(1);
        h.editor_state_mut()
            .editor_ui
            .prompt_center
            .search
            .set_text("abcd");
        h.editor_state_mut()
            .editor_ui
            .prompt_center
            .search
            .set_caret(0, 0);
        let search_start = h
            .ime_anchor_rect(1200.0, 800.0)
            .expect("prompt search should yield an IME anchor");

        h.editor_state_mut()
            .editor_ui
            .prompt_center
            .search
            .set_caret(3, 0);
        let search_after_three = h
            .ime_anchor_rect(1200.0, 800.0)
            .expect("prompt search should keep its IME anchor");
        assert!(search_after_three.origin.x > search_start.origin.x + 12.0);
        assert_eq!(search_after_three.origin.y, search_start.origin.y);

        {
            let prompt = &mut h.editor_state_mut().editor_ui.prompt_center;
            prompt.save_open = true;
            prompt.focus = PromptCenterFocus::SaveTitle;
            prompt.save_title.set_text("title");
            prompt.save_title.set_caret(4, 0);
        }
        let save_title = h
            .ime_anchor_rect(1200.0, 800.0)
            .expect("prompt save title should yield an IME anchor");
        assert_ne!(
            save_title.origin.y, search_after_three.origin.y,
            "the candidate window must move to the active prompt field"
        );
    }
}
