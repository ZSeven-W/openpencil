//! Inline rename session state machine — ported from shell-core's
//! `document/page_mutators.rs` (`start_rename_*` / `rename_append` /
//! `rename_backspace` / `rename_commit` / `rename_cancel`). The
//! sibling inline canvas text-edit session lives in `text_edit.rs`.
//!
//! An inline rename collects keystrokes into
//! `ui.layer_rename.draft`. `rename_commit` writes the draft into
//! the node's `name` (or the page's `name`) and pushes a single
//! history entry — but only when the name actually changed, so a
//! no-op commit doesn't pollute the undo stack.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::ui_draft::{LayerContextTarget, LayerRenameState};
use crate::walkers::find_node_mut;

impl EditorState {
    // --- Inline rename ----------------------------------------------

    /// Start an inline rename on a layer row. Seeds the draft with the
    /// node's current name. `false` when the node doesn't exist.
    pub fn start_rename_layer(&mut self, id: NodeId) -> bool {
        let Some(node) = crate::walkers::find_node(self.active_children(), &id) else {
            return false;
        };
        let draft = node.base().name.clone().unwrap_or_default();
        let caret = draft.chars().count();
        self.ui.layer_rename = Some(LayerRenameState {
            target: LayerContextTarget::Layer(id),
            draft,
            caret,
            select_all: false,
        });
        true
    }

    /// Start an inline rename on a page row. Seeds the draft with the
    /// page's current name. `false` for an out-of-range index or a
    /// single-page document (the implicit page has no name field).
    pub fn start_rename_page(&mut self, idx: usize) -> bool {
        let Some(pages) = self.doc.pages.as_ref() else {
            return false;
        };
        let Some(page) = pages.get(idx) else {
            return false;
        };
        let draft = page.name.clone();
        let caret = draft.chars().count();
        self.ui.layer_rename = Some(LayerRenameState {
            target: LayerContextTarget::Page(idx),
            draft,
            caret,
            select_all: false,
        });
        true
    }

    /// Insert `text` at the caret of the in-flight rename draft and
    /// advance the caret past it. `false` when no rename is active.
    pub fn rename_append(&mut self, text: &str) -> bool {
        match self.ui.layer_rename.as_mut() {
            Some(state) => {
                if state.select_all {
                    state.draft.clear();
                    state.caret = 0;
                    state.select_all = false;
                }
                let byte = char_to_byte(&state.draft, state.caret);
                state.draft.insert_str(byte, text);
                state.caret += text.chars().count();
                true
            }
            None => false,
        }
    }

    /// Delete the character immediately before the caret. A no-op at
    /// the start of the draft. `false` when no rename is active.
    pub fn rename_backspace(&mut self) -> bool {
        match self.ui.layer_rename.as_mut() {
            Some(state) => {
                if state.select_all {
                    state.draft.clear();
                    state.caret = 0;
                    state.select_all = false;
                    return true;
                }
                if state.caret > 0 {
                    let start = char_to_byte(&state.draft, state.caret - 1);
                    let end = char_to_byte(&state.draft, state.caret);
                    state.draft.replace_range(start..end, "");
                    state.caret -= 1;
                }
                true
            }
            None => false,
        }
    }

    /// Move the rename caret one character left. Returns whether a
    /// rename is active (so the host consumes the arrow key instead
    /// of nudging the selection).
    pub fn rename_caret_left(&mut self) -> bool {
        match self.ui.layer_rename.as_mut() {
            Some(state) => {
                state.select_all = false;
                state.caret = state.caret.saturating_sub(1);
                true
            }
            None => false,
        }
    }

    /// Move the rename caret one character right (clamped to the draft
    /// length). Returns whether a rename is active.
    pub fn rename_caret_right(&mut self) -> bool {
        match self.ui.layer_rename.as_mut() {
            Some(state) => {
                state.select_all = false;
                let len = state.draft.chars().count();
                state.caret = (state.caret + 1).min(len);
                true
            }
            None => false,
        }
    }

    /// Commit the active rename. Returns true when a rename was in
    /// flight (UI changed → repaint). Pushes a history snapshot only
    /// when the name actually changed — an empty draft or a no-op
    /// commit leaves the undo stack untouched.
    pub fn rename_commit(&mut self) -> bool {
        let Some(state) = self.ui.layer_rename.take() else {
            return false;
        };
        if state.draft.trim().is_empty() {
            return true;
        }
        let snap = self.snapshot_for_history();
        let mut changed = false;
        match state.target {
            LayerContextTarget::Page(idx) => {
                if let Some(pages) = self.doc.pages.as_mut() {
                    if let Some(page) = pages.get_mut(idx) {
                        if page.name != state.draft {
                            page.name = state.draft;
                            changed = true;
                        }
                    }
                }
            }
            LayerContextTarget::Layer(id) => {
                if let Some(node) = find_node_mut(self.active_children_mut(), &id) {
                    let base = node.base_mut();
                    if base.name.as_deref() != Some(state.draft.as_str()) {
                        base.name = Some(state.draft);
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.history_push_past(snap);
        }
        true
    }

    /// Cancel an in-flight rename. `true` when one was active.
    pub fn rename_cancel(&mut self) -> bool {
        self.ui.layer_rename.take().is_some()
    }
}

// --- Free helpers ----------------------------------------------------

/// Byte offset of the `char_idx`-th character in `s`, or `s.len()`
/// when `char_idx` is at/after the end. Used to translate the
/// char-based rename caret into a byte index for `insert_str` /
/// `replace_range`.
fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{rect, state_with, text};

    fn doc() -> EditorState {
        state_with(vec![
            rect("n1", "Rectangle", 0.0, 0.0, 40.0, 30.0),
            text("n2", "Label", 0.0, 50.0, 100.0, 20.0, "Hi"),
        ])
    }

    // --- Rename -----------------------------------------------------

    #[test]
    fn rename_layer_round_trip() {
        let mut s = doc();
        assert!(s.start_rename_layer(NodeId::new("n1")));
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().draft, "Rectangle");
        assert!(s.rename_backspace());
        assert!(s.rename_append("X"));
        assert!(s.rename_commit());
        let node = crate::walkers::find_node(s.active_children(), &NodeId::new("n1")).unwrap();
        assert_eq!(node.base().name.as_deref(), Some("RectanglX"));
        // The name changed → exactly one history entry.
        assert_eq!(s.history.past.len(), 1);
    }

    #[test]
    fn rename_caret_moves_and_edits_mid_string() {
        let mut s = doc();
        assert!(s.start_rename_layer(NodeId::new("n1"))); // "Rectangle", caret 9
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().caret, 9);
        // Three lefts → caret 6 ("Rectan|gle").
        assert!(s.rename_caret_left());
        assert!(s.rename_caret_left());
        assert!(s.rename_caret_left());
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().caret, 6);
        // Insert mid-string.
        assert!(s.rename_append("X"));
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().draft, "RectanXgle");
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().caret, 7);
        // Backspace removes the just-inserted 'X' (before caret).
        assert!(s.rename_backspace());
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().draft, "Rectangle");
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().caret, 6);
        // Right past the end clamps to the char count.
        for _ in 0..20 {
            s.rename_caret_right();
        }
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().caret, 9);
        // Left past the start clamps to 0.
        for _ in 0..20 {
            s.rename_caret_left();
        }
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().caret, 0);
    }

    #[test]
    fn rename_caret_is_char_based_for_cjk() {
        let mut s = doc();
        s.ui.layer_rename = Some(crate::ui_draft::LayerRenameState {
            target: crate::ui_draft::LayerContextTarget::Layer(NodeId::new("n1")),
            draft: "首页".to_string(),
            caret: 2,
            select_all: false,
        });
        // One left → between the two CJK chars (char index 1, byte 3).
        assert!(s.rename_caret_left());
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().caret, 1);
        // Insert lands on the char boundary, not mid-codepoint.
        assert!(s.rename_append("X"));
        assert_eq!(s.ui.layer_rename.as_ref().unwrap().draft, "首X页");
    }

    #[test]
    fn rename_caret_methods_false_when_inactive() {
        let mut s = doc();
        assert!(!s.rename_caret_left());
        assert!(!s.rename_caret_right());
    }

    #[test]
    fn rename_unknown_layer_is_no_op() {
        let mut s = doc();
        assert!(!s.start_rename_layer(NodeId::new("missing")));
        assert!(!s.rename_append("x"));
        assert!(!s.rename_backspace());
    }

    #[test]
    fn rename_empty_draft_pushes_no_history() {
        let mut s = doc();
        assert!(s.start_rename_layer(NodeId::new("n1")));
        // Clear the whole draft.
        for _ in 0..20 {
            s.rename_backspace();
        }
        assert!(s.rename_commit());
        assert_eq!(s.history.past.len(), 0);
        // Name unchanged.
        let node = crate::walkers::find_node(s.active_children(), &NodeId::new("n1")).unwrap();
        assert_eq!(node.base().name.as_deref(), Some("Rectangle"));
    }

    #[test]
    fn rename_unchanged_name_pushes_no_history() {
        let mut s = doc();
        assert!(s.start_rename_layer(NodeId::new("n1")));
        // Commit with the seeded draft untouched.
        assert!(s.rename_commit());
        assert_eq!(s.history.past.len(), 0);
    }

    #[test]
    fn rename_cancel_discards_draft() {
        let mut s = doc();
        assert!(s.start_rename_layer(NodeId::new("n1")));
        s.rename_append("zzz");
        assert!(s.rename_cancel());
        let node = crate::walkers::find_node(s.active_children(), &NodeId::new("n1")).unwrap();
        assert_eq!(node.base().name.as_deref(), Some("Rectangle"));
        assert!(!s.rename_cancel());
    }

    #[test]
    fn rename_page_round_trip() {
        let mut s = doc();
        s.add_page();
        assert!(s.start_rename_page(0));
        assert!(s.rename_append(" Renamed"));
        assert!(s.rename_commit());
        assert_eq!(s.doc.pages.as_ref().unwrap()[0].name, "Page 1 Renamed");
    }

    #[test]
    fn rename_page_single_page_doc_rejected() {
        let mut s = doc();
        // Single-page document has no `pages` list.
        assert!(!s.start_rename_page(0));
    }
}
