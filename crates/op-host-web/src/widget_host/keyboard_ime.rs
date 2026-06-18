//! IME and paste text routing for the web widget host.

use super::WidgetHost;

impl WidgetHost {
    /// IME composition forwarding. Only the final COMMIT lands in the
    /// focused input — preedit text is not painted (matches the native
    /// host, which routes winit `Ime::Commit` through `apply_text`
    /// char-by-char in `app_handler.rs` and ignores `Ime::Preedit`).
    /// Routing therefore covers every `apply_text` focus branch.
    #[allow(dead_code)]
    pub fn apply_ime(&mut self, event: &op_editor_ui::ImeEvent) -> bool {
        if !matches!(event.kind, op_editor_ui::ImeKind::CompositionEnd) {
            return false;
        }
        self.apply_paste_text(&event.text)
    }

    /// Route a multi-character text payload (IME commit, clipboard paste)
    /// into whichever input owns the keyboard, char-by-char through
    /// `apply_text` so every focus branch + per-field filter applies.
    pub fn apply_paste_text(&mut self, text: &str) -> bool {
        let mut consumed = false;
        for c in text.chars() {
            if !c.is_control() && self.apply_text(c) {
                consumed = true;
            }
        }
        consumed
    }
}
