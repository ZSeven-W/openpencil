//! IME / modifier / key-release handlers for `DesktopApp`'s
//! `window_event` dispatcher. Carved out of the `app_handler.rs` spine
//! to keep it under the 800-line cap; pure code motion.
//!
//! The key-*press* path itself lives in `crate::keyboard_input`
//! (`handle_key_pressed`); this module only holds the thin winit-event
//! adapters the dispatcher calls.

use crate::DesktopApp;
use winit::keyboard::Key;

impl DesktopApp {
    pub(super) fn on_ime_preedit(&mut self, text: &str, cursor: Option<(usize, usize)>) {
        let changed = self.host.apply_ime_preedit(text, cursor);
        self.sync_native_ime();
        if changed {
            self.request_redraw(true);
        }
    }

    pub(super) fn on_ime_commit(&mut self, text: &str) {
        let transaction = self.collab_runtime.begin_local_edit(&mut self.host);
        let changed = self.host.apply_ime_commit(text);
        if transaction {
            self.collab_runtime.finish_local_edit(&mut self.host);
        }
        self.sync_native_ime();
        if changed {
            self.request_redraw(true);
        }
    }

    pub(super) fn on_ime_disabled(&mut self) {
        // Focus left the IME-capable surface mid-composition —
        // drop the preedit so the bubble doesn't linger.
        if self.host.apply_ime_preedit("", None) {
            self.request_redraw(true);
        }
    }

    pub(super) fn on_modifiers_changed(&mut self, mods: winit::event::Modifiers) {
        let state = mods.state();
        self.zoom_modifier = state.super_key() || state.control_key();
        self.shift_modifier = state.shift_key();
        self.alt_modifier = state.alt_key();
        // Forward shift state into the host so the next
        // mouse press can branch on shift+click for
        // multi-select.
        self.host.set_modifier_shift(self.shift_modifier);
        self.host.set_modifier_alt(self.alt_modifier);
    }

    pub(super) fn on_space_released(&mut self) {
        // End transient space-pan (the press side lives in
        // `keyboard_input.rs`).
        self.host.set_space_pan(false);
    }

    pub(super) fn on_key_pressed(&mut self, logical_key: &Key, text: Option<&str>) {
        let collaboration_history_chord = self.zoom_modifier
            && matches!(
                logical_key,
                Key::Character(character)
                    if matches!(character.to_lowercase().as_str(), "z" | "y")
            );
        let transaction =
            !collaboration_history_chord && self.collab_runtime.begin_local_edit(&mut self.host);
        self.handle_key_pressed(logical_key, text);
        if transaction {
            self.collab_runtime.finish_local_edit(&mut self.host);
        }
        self.sync_native_ime();
    }
}
