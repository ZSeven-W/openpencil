//! Keyboard-edit state machines for the colour picker's hex bar and R/G/B
//! numeric boxes — extracted from `color_picker.rs` to keep it under the
//! 800-line cap. Both fields render through jian `TextInputView`; the host
//! routes keystrokes here (`color_picker_hex_*` / `color_picker_rgb_*`) and
//! each live-applies through the shared `color_picker_set_hsv` commit so
//! fill / stroke / variable targets all update.

use crate::color_convert::{hsv_to_rgb, parse_hex_rgb, rgb_to_hex, rgb_to_hsv};
use crate::state::EditorState;

impl EditorState {
    /// Whether the picker's hex field currently has keyboard focus.
    pub fn color_picker_hex_focused(&self) -> bool {
        self.ui.color_picker.as_ref().is_some_and(|s| s.hex_focused)
    }

    /// Focus the hex field, seeding the unified `hex_input` from the current
    /// colour (caret at end) so the user edits the live `#RRGGBB`.
    pub fn color_picker_focus_hex(&mut self) {
        if let Some(state) = self.ui.color_picker.as_mut() {
            let (r, g, b) = hsv_to_rgb(state.hue, state.sat, state.val);
            state.hex_input = jian_core::text_input::TextInputState::with_text(rgb_to_hex(r, g, b));
            state.hex_focused = true;
        }
    }

    /// Insert one hex character (or a leading `#`) at the caret, capped at
    /// `#RRGGBB`, then live-apply when the draft is a complete colour. Mirrors
    /// the property-panel hex input's gating (`keyboard.rs` `is_hex_focus`).
    pub fn color_picker_hex_char(&mut self, ch: char, now_ms: u64) {
        let Some(state) = self.ui.color_picker.as_mut() else {
            return;
        };
        if !state.hex_focused {
            return;
        }
        let input = &state.hex_input;
        let replacing_all = input.is_select_all();
        let draft = input.text();
        let pos = if replacing_all {
            0
        } else {
            input.caret().min(draft.len())
        };
        let allowed = (replacing_all || draft.len() < 7)
            && (ch.is_ascii_hexdigit() || (ch == '#' && pos == 0 && !draft.starts_with('#')));
        if !allowed {
            return;
        }
        let mut buf = [0u8; 4];
        let lower = ch.to_ascii_lowercase();
        state
            .hex_input
            .insert_str(lower.encode_utf8(&mut buf), now_ms);
        self.color_picker_apply_hex_draft();
    }

    /// Delete the character before the caret.
    pub fn color_picker_hex_backspace(&mut self, now_ms: u64) {
        let Some(state) = self.ui.color_picker.as_mut() else {
            return;
        };
        if !state.hex_focused {
            return;
        }
        state.hex_input.backspace(now_ms);
        self.color_picker_apply_hex_draft();
    }

    /// Commit + blur the hex field. No-op when the hex field isn't focused (so
    /// callers can blur unconditionally without re-applying a stale draft).
    pub fn color_picker_blur_hex(&mut self) {
        if !self.color_picker_hex_focused() {
            return;
        }
        self.color_picker_apply_hex_draft();
        if let Some(state) = self.ui.color_picker.as_mut() {
            state.hex_focused = false;
        }
    }

    /// Parse the hex draft and, when it is a valid colour, route it through the
    /// normal HSV commit so fill / stroke / variable targets all update.
    fn color_picker_apply_hex_draft(&mut self) {
        let draft = self
            .ui
            .color_picker
            .as_ref()
            .map(|s| s.hex_input.text().to_owned())
            .unwrap_or_default();
        if let Some((r, g, b)) = parse_hex_rgb(&draft) {
            let (h, s, v) = rgb_to_hsv((r, g, b));
            self.color_picker_set_hsv(h, s, v);
        }
    }

    /// Whether one of the R/G/B numeric fields currently has keyboard focus.
    pub fn color_picker_rgb_focused(&self) -> bool {
        self.ui
            .color_picker
            .as_ref()
            .is_some_and(|s| s.rgb_focus.is_some())
    }

    /// Focus one R/G/B channel (0=R, 1=G, 2=B), seeding `rgb_input` with its
    /// current 0..255 value (caret at end).
    pub fn color_picker_focus_rgb(&mut self, channel: u8) {
        let channel = channel.min(2);
        if let Some(state) = self.ui.color_picker.as_mut() {
            let (r, g, b) = hsv_to_rgb(state.hue, state.sat, state.val);
            let chan = match channel {
                0 => r,
                1 => g,
                _ => b,
            };
            let value = (chan * 255.0).round() as u8;
            state.rgb_input = jian_core::text_input::TextInputState::with_text(value.to_string());
            state.rgb_focus = Some(channel);
        }
    }

    /// Insert one digit into the focused R/G/B channel (max 3 chars, clamped to
    /// 0..255 on apply), then live-apply.
    pub fn color_picker_rgb_char(&mut self, ch: char, now_ms: u64) {
        let Some(state) = self.ui.color_picker.as_mut() else {
            return;
        };
        if state.rgb_focus.is_none() {
            return;
        }
        let input = &state.rgb_input;
        let allowed = ch.is_ascii_digit() && (input.is_select_all() || input.text().len() < 3);
        if !allowed {
            return;
        }
        let mut buf = [0u8; 4];
        state.rgb_input.insert_str(ch.encode_utf8(&mut buf), now_ms);
        self.color_picker_apply_rgb_draft();
    }

    /// Delete the character before the caret in the focused R/G/B channel.
    pub fn color_picker_rgb_backspace(&mut self, now_ms: u64) {
        let Some(state) = self.ui.color_picker.as_mut() else {
            return;
        };
        if state.rgb_focus.is_none() {
            return;
        }
        state.rgb_input.backspace(now_ms);
        self.color_picker_apply_rgb_draft();
    }

    /// Commit + blur the focused R/G/B channel. No-op when none is focused.
    pub fn color_picker_blur_rgb(&mut self) {
        if !self.color_picker_rgb_focused() {
            return;
        }
        self.color_picker_apply_rgb_draft();
        if let Some(state) = self.ui.color_picker.as_mut() {
            state.rgb_focus = None;
        }
    }

    /// Parse the focused channel's draft (clamped 0..255), splice it into the
    /// current RGB, and route the result through the normal HSV commit.
    fn color_picker_apply_rgb_draft(&mut self) {
        let Some((Some(channel), draft, hue, sat, val)) = self.ui.color_picker.as_ref().map(|s| {
            (
                s.rgb_focus,
                s.rgb_input.text().to_owned(),
                s.hue,
                s.sat,
                s.val,
            )
        }) else {
            return;
        };
        let Ok(parsed) = draft.parse::<u32>() else {
            return;
        };
        let chan = parsed.min(255) as f32 / 255.0;
        let (mut r, mut g, mut b) = hsv_to_rgb(hue, sat, val);
        match channel {
            0 => r = chan,
            1 => g = chan,
            _ => b = chan,
        }
        let (h, s, v) = rgb_to_hsv((r, g, b));
        self.color_picker_set_hsv(h, s, v);
    }
}
