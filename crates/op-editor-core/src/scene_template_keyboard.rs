//! Keyboard editing for the Scene Template Center's focused text field.
//!
//! Same shape as [`crate::prompt_center_keyboard`]: hosts route platform key
//! events here and all selection / UTF-8 caret semantics stay shared.
//!
//! The panel owns the keyboard whenever it is open, including keystrokes it
//! rejects. A floating panel that let unhandled keys fall through would hand
//! `r` to the rectangle tool and Backspace to the selection while the user is
//! typing a search query or a deck topic into it.

use crate::editor_ui_state::SceneTemplateFocus;
use crate::EditorState;

/// Insert one printable character into the focused field.
///
/// `None` means the panel is closed. `Some(changed)` means the panel owns the
/// key, with `changed` indicating whether a repaint is needed.
pub fn text(state: &mut EditorState, c: char, now_ms: u64) -> Option<bool> {
    if !state.editor_ui.scene_template_center.input_active() {
        return None;
    }
    if c.is_control() {
        return Some(false);
    }
    let mut encoded = [0_u8; 4];
    let center = &mut state.editor_ui.scene_template_center;
    center
        .focused_input_mut()
        .insert_str(c.encode_utf8(&mut encoded), now_ms);
    reset_filter_feedback(center);
    Some(true)
}

/// Insert clipboard text into the focused field.
///
/// The two single-line fields drop newlines the way the rest of the chrome's
/// inputs do, but the import box must not: a `DESIGN.md` pasted into it is a
/// markdown document, and flattening it to one line destroys the headings and
/// list structure the parser and the model both read. That is the whole reason
/// this exists rather than the host's char-by-char `apply_input_paste`.
pub fn paste(state: &mut EditorState, text: &str, now_ms: u64) -> Option<bool> {
    if !state.editor_ui.scene_template_center.input_active() {
        return None;
    }
    let center = &mut state.editor_ui.scene_template_center;
    let keep_newlines = center.focus == SceneTemplateFocus::Import;
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_control() || (keep_newlines && matches!(c, '\n' | '\t')))
        .collect();
    if cleaned.is_empty() {
        return Some(false);
    }
    center.focused_input_mut().insert_str(&cleaned, now_ms);
    if keep_newlines {
        center.import.error_key = None;
    }
    reset_filter_feedback(center);
    Some(true)
}

/// Delete the previous character in the focused field.
pub fn backspace(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    edit_if_open(state, |input| input.backspace(now_ms))
}

/// Delete the next character in the focused field.
pub fn delete_forward(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    edit_if_open(state, |input| input.delete_forward(now_ms))
}

/// Move the focused caret left or right.
pub fn move_caret(state: &mut EditorState, forward: bool, extend: bool, now_ms: u64) -> bool {
    if !state.editor_ui.scene_template_center.input_active() {
        return false;
    }
    let input = state.editor_ui.scene_template_center.focused_input_mut();
    if forward {
        input.move_right(extend, now_ms);
    } else {
        input.move_left(extend, now_ms);
    }
    true
}

/// Select all text in the focused field.
pub fn select_all(state: &mut EditorState, now_ms: u64) -> bool {
    if !state.editor_ui.scene_template_center.input_active() {
        return false;
    }
    let input = state.editor_ui.scene_template_center.focused_input_mut();
    input.select_all();
    input.touch(now_ms);
    true
}

/// Return selected text from the focused field.
pub fn selected_text(state: &EditorState) -> Option<&str> {
    if !state.editor_ui.scene_template_center.input_active() {
        return None;
    }
    let center = &state.editor_ui.scene_template_center;
    let input = match center.focus {
        SceneTemplateFocus::Search => &center.search,
        SceneTemplateFocus::Generate => &center.generate,
        SceneTemplateFocus::Import => &center.import.text,
    };
    let (start, end) = input.highlight_range()?;
    input.text().get(start..end)
}

fn edit_if_open(
    state: &mut EditorState,
    edit: impl FnOnce(&mut jian_core::text_input::TextInputState),
) -> Option<bool> {
    if !state.editor_ui.scene_template_center.input_active() {
        return None;
    }
    let center = &mut state.editor_ui.scene_template_center;
    let before = center.focused_input_mut().text().to_owned();
    edit(center.focused_input_mut());
    let changed = center.focused_input_mut().text() != before;
    if changed {
        reset_filter_feedback(center);
    }
    Some(changed)
}

/// A query edit reorders the grid, so a retained scroll offset or hover index
/// would point at a different card than the one under the pointer. Editing
/// the topic changes nothing about the grid, so it leaves both alone.
fn reset_filter_feedback(center: &mut crate::SceneTemplateCenterState) {
    if center.focus != SceneTemplateFocus::Search {
        return;
    }
    center.hover = None;
    center.scroll.offset = 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_search_and_topic_without_crossing_fields() {
        let mut state = EditorState::new();
        assert_eq!(text(&mut state, 'x', 1), None);

        state.editor_ui.open_scene_template_center(2);
        assert_eq!(text(&mut state, '演', 3), Some(true));
        assert_eq!(state.editor_ui.scene_template_center.search.text(), "演");

        state.editor_ui.scene_template_center.focus = SceneTemplateFocus::Generate;
        assert_eq!(text(&mut state, 'Q', 4), Some(true));
        assert_eq!(state.editor_ui.scene_template_center.generate.text(), "Q");
        assert_eq!(state.editor_ui.scene_template_center.search.text(), "演");

        assert_eq!(backspace(&mut state, 5), Some(true));
        assert!(state
            .editor_ui
            .scene_template_center
            .generate
            .text()
            .is_empty());
        assert_eq!(state.editor_ui.scene_template_center.search.text(), "演");
    }

    #[test]
    fn an_open_panel_owns_every_keystroke_even_the_rejected_ones() {
        let mut state = EditorState::new();
        state.editor_ui.open_scene_template_center(0);
        // Backspace on an empty field still reports "mine": falling through
        // would delete the canvas selection under the caret.
        assert_eq!(backspace(&mut state, 1), Some(false));
        assert_eq!(text(&mut state, '\u{1}', 2), Some(false));
    }

    #[test]
    fn typing_a_topic_leaves_the_grid_scroll_alone() {
        let mut state = EditorState::new();
        state.editor_ui.open_scene_template_center(0);
        state.editor_ui.scene_template_center.scroll.offset = 120.0;
        state.editor_ui.scene_template_center.focus = SceneTemplateFocus::Generate;

        assert_eq!(text(&mut state, 'a', 1), Some(true));
        assert_eq!(state.editor_ui.scene_template_center.scroll.offset, 120.0);

        state.editor_ui.scene_template_center.focus = SceneTemplateFocus::Search;
        assert_eq!(text(&mut state, 'b', 2), Some(true));
        assert_eq!(state.editor_ui.scene_template_center.scroll.offset, 0.0);
    }

    #[test]
    fn shift_arrows_extend_utf8_selection_in_the_focused_field() {
        let mut state = EditorState::new();
        state.editor_ui.open_scene_template_center(0);
        state.editor_ui.scene_template_center.focus = SceneTemplateFocus::Generate;
        state
            .editor_ui
            .scene_template_center
            .generate
            .set_text("a演b");
        state
            .editor_ui
            .scene_template_center
            .generate
            .set_caret(1, 1);

        assert!(move_caret(&mut state, true, true, 2));
        assert_eq!(selected_text(&state), Some("演"));
    }
}
