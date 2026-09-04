//! Keyboard editing for the mobile Save / Save As file-name dialog.
//!
//! Same shape as [`crate::scene_template_keyboard`]: hosts route platform
//! key events here so selection / UTF-8 caret semantics stay shared. The
//! dialog is modal — while it is open it owns every keystroke, including
//! ones it rejects, so a stray `r` cannot reach the rectangle tool while
//! the user is naming a file.

use crate::EditorState;

/// Characters that cannot appear in a file name on any of the mobile
/// targets (POSIX separators plus the NTFS-reserved set, which also keeps
/// names portable when users export/share the sandbox). The dialog rejects
/// them at the keystroke so the host's sanitizer never has to rewrite a
/// name behind the user's back.
pub fn is_forbidden_file_name_char(c: char) -> bool {
    c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
}

/// Insert one printable character into the file-name field.
///
/// `None` means the dialog is closed. `Some(changed)` means the dialog owns
/// the key, with `changed` indicating whether a repaint is needed.
pub fn text(state: &mut EditorState, c: char, now_ms: u64) -> Option<bool> {
    let dialog = &mut state.editor_ui.save_name_dialog;
    if !dialog.open {
        return None;
    }
    if is_forbidden_file_name_char(c) {
        return Some(false);
    }
    let mut encoded = [0_u8; 4];
    dialog.input.insert_str(c.encode_utf8(&mut encoded), now_ms);
    Some(true)
}

/// Insert clipboard text into the file-name field, dropping characters a
/// file name cannot carry.
pub fn paste(state: &mut EditorState, text: &str, now_ms: u64) -> Option<bool> {
    let dialog = &mut state.editor_ui.save_name_dialog;
    if !dialog.open {
        return None;
    }
    let cleaned: String = text
        .chars()
        .filter(|c| !is_forbidden_file_name_char(*c))
        .collect();
    if cleaned.is_empty() {
        return Some(false);
    }
    dialog.input.insert_str(&cleaned, now_ms);
    Some(true)
}

/// Delete the previous character in the file-name field.
pub fn backspace(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    edit_if_open(state, |input| input.backspace(now_ms))
}

/// Delete the next character in the file-name field.
pub fn delete_forward(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    edit_if_open(state, |input| input.delete_forward(now_ms))
}

/// Move the caret left or right.
pub fn move_caret(state: &mut EditorState, forward: bool, extend: bool, now_ms: u64) -> bool {
    let dialog = &mut state.editor_ui.save_name_dialog;
    if !dialog.open {
        return false;
    }
    if forward {
        dialog.input.move_right(extend, now_ms);
    } else {
        dialog.input.move_left(extend, now_ms);
    }
    true
}

/// Select all text in the file-name field.
pub fn select_all(state: &mut EditorState, now_ms: u64) -> bool {
    let dialog = &mut state.editor_ui.save_name_dialog;
    if !dialog.open {
        return false;
    }
    dialog.input.select_all();
    dialog.input.touch(now_ms);
    true
}

fn edit_if_open(
    state: &mut EditorState,
    edit: impl FnOnce(&mut jian_core::text_input::TextInputState),
) -> Option<bool> {
    let dialog = &mut state.editor_ui.save_name_dialog;
    if !dialog.open {
        return None;
    }
    let before = dialog.input.text().to_owned();
    let selection_before = dialog.input.highlight_range();
    edit(&mut dialog.input);
    Some(dialog.input.text() != before || dialog.input.highlight_range() != selection_before)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_state(seed: &str) -> EditorState {
        let mut state = EditorState::starter();
        state
            .editor_ui
            .save_name_dialog
            .open_with(seed, false, 1_000);
        state
    }

    #[test]
    fn closed_dialog_owns_no_keys() {
        let mut state = EditorState::starter();
        assert_eq!(text(&mut state, 'a', 0), None);
        assert_eq!(backspace(&mut state, 0), None);
        assert!(!move_caret(&mut state, true, false, 0));
        assert!(!select_all(&mut state, 0));
    }

    #[test]
    fn typing_replaces_the_selected_seed_and_separators_are_rejected() {
        let mut state = open_state("untitled");
        assert_eq!(text(&mut state, 'm', 1_001), Some(true));
        assert_eq!(state.editor_ui.save_name_dialog.input.text(), "m");
        assert_eq!(text(&mut state, '/', 1_002), Some(false));
        assert_eq!(text(&mut state, ':', 1_003), Some(false));
        assert_eq!(state.editor_ui.save_name_dialog.input.text(), "m");
        assert_eq!(text(&mut state, '设', 1_004), Some(true));
        assert_eq!(state.editor_ui.save_name_dialog.input.text(), "m设");
    }

    #[test]
    fn paste_strips_forbidden_characters() {
        let mut state = open_state("");
        assert_eq!(paste(&mut state, "a/b\\c:d.op", 1_001), Some(true));
        assert_eq!(state.editor_ui.save_name_dialog.input.text(), "abcd.op");
        assert_eq!(paste(&mut state, "///", 1_002), Some(false));
    }

    #[test]
    fn confirm_requires_a_non_blank_name() {
        let mut state = open_state("   ");
        assert!(!state.editor_ui.save_name_dialog.request_confirm());
        assert_eq!(paste(&mut state, "poster", 1_001), Some(true));
        assert!(state.editor_ui.save_name_dialog.request_confirm());
        assert_eq!(
            state.editor_ui.save_name_dialog.take_confirmed_name(),
            Some("poster".to_string())
        );
        // Drained exactly once.
        assert_eq!(state.editor_ui.save_name_dialog.take_confirmed_name(), None);
    }
}
