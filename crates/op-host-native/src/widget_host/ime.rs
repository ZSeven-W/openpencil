//! Native IME composition routing (`Ime::Preedit` / `Ime::Commit`).
//!
//! TS parity target: Electron DOM inputs render preedit inline and
//! the OS anchors the candidate window at the caret. The Rust shell:
//!
//! - `apply_ime_preedit` stores the in-flight composition on
//!   `editor_ui.ime_preedit` (painted by the preedit overlay) — only
//!   while a text input owns the keyboard, mirroring the
//!   `apply_text` router's focus chain.
//! - `apply_ime_commit` clears the preedit and lands the committed
//!   string through `apply_text` char-by-char, so every focus branch
//!   + per-field filter (numeric / hex drafts) applies unchanged.
//! - `ime_anchor_rect` resolves the focused input's screen rect for
//!   `set_ime_cursor_area` + the overlay anchor. v1 coverage: the
//!   chat input (precise) — other inputs fall back to the overlay's
//!   bottom-center bubble; the candidate window then keeps the OS
//!   default position. Extending per-focus anchors is mechanical
//!   (each input's rect walker already exists for paint/hit-test).

use op_editor_core::ime_state::ImePreedit;
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
            || ui.icon_picker_open
            || ui.component_browser_open
    }

    /// `Ime::Preedit` — store / update / clear the composition.
    /// Empty `text` is winit's cancel signal. Returns true when the
    /// state changed (caller schedules a redraw).
    pub fn apply_ime_preedit(&mut self, text: &str, cursor: Option<(usize, usize)>) -> bool {
        if text.is_empty() || !self.text_input_focus_active() {
            let had = self.editor_state.editor_ui.ime_preedit.take().is_some();
            if had {
                self.mark_dirty();
            }
            return had;
        }
        let next = ImePreedit {
            text: text.to_string(),
            cursor,
        };
        if self.editor_state.editor_ui.ime_preedit.as_ref() == Some(&next) {
            return false;
        }
        self.editor_state.editor_ui.ime_preedit = Some(next);
        self.mark_dirty();
        true
    }

    /// `Ime::Commit` — clear the preedit and land the candidate
    /// string into whichever input owns the keyboard.
    pub fn apply_ime_commit(&mut self, text: &str) -> bool {
        if self.editor_state.editor_ui.ime_preedit.take().is_some() {
            self.mark_dirty();
        }
        let mut consumed = false;
        for ch in text.chars() {
            if !ch.is_control() && self.apply_text(ch) {
                consumed = true;
            }
        }
        consumed
    }

    /// Focused-input rect for candidate-window anchoring + the
    /// overlay bubble. v1: chat input only; `None` = fallback.
    pub fn ime_anchor_rect(&mut self, viewport_w: f32, viewport_h: f32) -> Option<Rect> {
        if self.editor_state.chat.focused {
            let chat_rect = self.ai_chat_rect(viewport_w, viewport_h)?;
            let chat = op_editor_ui::widgets::AIChatPlaceholder::from_editor_at(
                &self.editor_state,
                self.now_ms,
            );
            return Some(chat.input_rect(chat_rect));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::WidgetHostNative;

    fn host() -> WidgetHostNative {
        WidgetHostNative::new()
    }

    #[test]
    fn preedit_requires_a_text_input_focus() {
        let mut h = host();
        assert!(!h.apply_ime_preedit("你好", None), "no focus → no preedit");
        assert!(h.editor_state().editor_ui.ime_preedit.is_none());
    }

    #[test]
    fn preedit_stores_and_commit_clears_and_lands_in_chat() {
        let mut h = host();
        h.editor_state_mut().chat.focused = true;
        assert!(h.apply_ime_preedit("nih", Some((0, 3))));
        assert_eq!(
            h.editor_state()
                .editor_ui
                .ime_preedit
                .as_ref()
                .unwrap()
                .text,
            "nih"
        );
        // Same preedit again is a no-op (no redraw churn).
        assert!(!h.apply_ime_preedit("nih", Some((0, 3))));
        assert!(h.apply_ime_commit("你好"));
        assert!(h.editor_state().editor_ui.ime_preedit.is_none());
        assert!(h.editor_state().chat.input.contains("你好"));
    }

    #[test]
    fn empty_preedit_is_the_cancel_signal() {
        let mut h = host();
        h.editor_state_mut().chat.focused = true;
        assert!(h.apply_ime_preedit("ni", None));
        assert!(h.apply_ime_preedit("", None), "clear reports a change");
        assert!(h.editor_state().editor_ui.ime_preedit.is_none());
        assert!(!h.apply_ime_preedit("", None), "already clear → no-op");
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
}
