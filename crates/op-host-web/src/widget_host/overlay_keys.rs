//! Keyboard routing for the floating overlay panels' search boxes
//! (icon picker / component browser). Mirrors the corresponding
//! branches of the native host's `widget_host/keyboard.rs`; split
//! into a sibling module so `keyboard.rs` stays under the repo's
//! 800-line cap.
//!
//! Each helper returns `None` when its panel is closed (the caller
//! falls through to the next keyboard layer) and `Some(changed)`
//! when the panel owns the keystroke.

use super::WidgetHost;

impl WidgetHost {
    /// Icon-picker search box typing — owns every printable char
    /// while the panel is open.
    pub(in crate::widget_host) fn icon_picker_text(&mut self, c: char) -> Option<bool> {
        if !self.editor_state.editor_ui.icon_picker.open || c.is_control() {
            return None;
        }
        if self.editor_state.editor_ui.icon_picker_select_all {
            self.editor_state.editor_ui.icon_picker_search.clear();
            self.editor_state.editor_ui.icon_picker_select_all = false;
            self.editor_state.editor_ui.icon_picker.hover = None;
            self.editor_state.editor_ui.icon_picker.pressed = None;
        }
        self.editor_state.editor_ui.icon_picker_search.push(c);
        self.editor_state.editor_ui.icon_picker.hover = None;
        self.editor_state.editor_ui.icon_picker.pressed = None;
        self.mark_dirty();
        Some(true)
    }

    /// Icon-picker search box backspace — swallows the key while the
    /// panel is open even when the draft is already empty.
    pub(in crate::widget_host) fn icon_picker_backspace(&mut self) -> Option<bool> {
        if !self.editor_state.editor_ui.icon_picker.open {
            return None;
        }
        if self.editor_state.editor_ui.icon_picker_select_all {
            self.editor_state.editor_ui.icon_picker_search.clear();
            self.editor_state.editor_ui.icon_picker_select_all = false;
            self.editor_state.editor_ui.icon_picker.hover = None;
            self.editor_state.editor_ui.icon_picker.pressed = None;
            self.mark_dirty();
            return Some(true);
        }
        if self
            .editor_state
            .editor_ui
            .icon_picker_search
            .pop()
            .is_some()
        {
            self.editor_state.editor_ui.icon_picker.hover = None;
            self.editor_state.editor_ui.icon_picker.pressed = None;
            self.mark_dirty();
            return Some(true);
        }
        Some(false)
    }

    /// Component-browser search box typing — same contract as the
    /// icon picker's.
    pub(in crate::widget_host) fn component_browser_text(&mut self, c: char) -> Option<bool> {
        if !self.editor_state.editor_ui.component_browser_open || c.is_control() {
            return None;
        }
        if self.editor_state.editor_ui.component_browser_select_all {
            self.editor_state.editor_ui.component_browser_search.clear();
            self.editor_state.editor_ui.component_browser_select_all = false;
        }
        self.editor_state.editor_ui.component_browser_search.push(c);
        self.mark_dirty();
        Some(true)
    }

    /// Component-browser search box backspace.
    pub(in crate::widget_host) fn component_browser_backspace(&mut self) -> Option<bool> {
        if !self.editor_state.editor_ui.component_browser_open {
            return None;
        }
        if self.editor_state.editor_ui.component_browser_select_all {
            self.editor_state.editor_ui.component_browser_search.clear();
            self.editor_state.editor_ui.component_browser_select_all = false;
            self.mark_dirty();
            return Some(true);
        }
        if self
            .editor_state
            .editor_ui
            .component_browser_search
            .pop()
            .is_some()
        {
            self.mark_dirty();
            return Some(true);
        }
        Some(false)
    }
}
