//! Inline rename + inline text-edit session state machines —
//! ported from shell-core's `document/page_mutators.rs`
//! (`start_rename_*` / `rename_append` / `rename_backspace` /
//! `rename_commit` / `rename_cancel`) and the text-edit session
//! (`start_text_edit` / `text_edit_append` / `text_edit_backspace`
//! / `text_edit_commit`).
//!
//! ## Rename
//!
//! An inline rename collects keystrokes into
//! `ui.layer_rename.draft`. `rename_commit` writes the draft into
//! the node's `name` (or the page's `name`) and pushes a single
//! history entry — but only when the name actually changed, so a
//! no-op commit doesn't pollute the undo stack.
//!
//! ## Text edit
//!
//! An inline text-edit session mutates a `Text` node's content in
//! place. To keep a typing burst as one undoable step, history opens
//! lazily: the first keystroke (or any keystroke after a > 500 ms
//! pause) snapshots the pre-edit state and pushes it. The snapshot +
//! the last-keystroke timestamp live on `ui.pending_text_edit_history`
//! / `ui.text_edit_last_ms`.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::ui_draft::{LayerContextTarget, LayerRenameState};
use crate::walkers::find_node_mut;
use jian_ops_schema::node::{PenNode, TextContent};

/// Coalesce window for text-edit history bursts (ms). A pause longer
/// than this opens a fresh undo entry.
const TEXT_COALESCE_MS: u64 = 500;

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

    // --- Inline text edit -------------------------------------------

    /// Enter inline text-edit mode on `id`. `false` when the node
    /// isn't a `Text` node or doesn't exist. History opens lazily on
    /// the first keystroke.
    pub fn start_text_edit(&mut self, id: NodeId) -> bool {
        let Some(node) = crate::walkers::find_node(self.active_children(), &id) else {
            return false;
        };
        if !matches!(node, PenNode::Text(_)) {
            return false;
        }
        self.ui.text_editing = Some(id);
        self.ui.text_edit_select_all = false;
        self.ui.text_edit_last_ms = 0;
        self.ui.pending_text_edit_history = None;
        true
    }

    /// Append `text` to the actively-edited Text node's content.
    /// `now_ms` lets a typing burst coalesce into one undo step: a
    /// pause > 500 ms opens a fresh history entry. `false` when no
    /// text-edit session is active or the node has vanished.
    pub fn text_edit_append(&mut self, text: &str, now_ms: u64) -> bool {
        let Some(id) = self.ui.text_editing.clone() else {
            return false;
        };
        self.maybe_open_text_edit_history(now_ms);
        let replace_all = std::mem::take(&mut self.ui.text_edit_select_all);
        let Some(node) = find_node_mut(self.active_children_mut(), &id) else {
            return false;
        };
        let Some(content) = text_content_mut(node) else {
            return false;
        };
        if replace_all {
            content.clear();
        }
        content.push_str(text);
        self.ui.text_edit_last_ms = now_ms;
        true
    }

    /// Pop the last char from the actively-edited Text node. `false`
    /// when no session is active, the node has vanished, or the
    /// content was already empty.
    pub fn text_edit_backspace(&mut self, now_ms: u64) -> bool {
        let Some(id) = self.ui.text_editing.clone() else {
            return false;
        };
        self.maybe_open_text_edit_history(now_ms);
        let replace_all = std::mem::take(&mut self.ui.text_edit_select_all);
        let Some(node) = find_node_mut(self.active_children_mut(), &id) else {
            return false;
        };
        let Some(content) = text_content_mut(node) else {
            return false;
        };
        if replace_all {
            content.clear();
            self.ui.text_edit_last_ms = now_ms;
            return true;
        }
        if content.pop().is_some() {
            self.ui.text_edit_last_ms = now_ms;
            true
        } else {
            false
        }
    }

    /// Exit text-edit mode. History snapshots were pushed lazily by
    /// `text_edit_append` / `text_edit_backspace`, so commit only
    /// clears the session state. `true` when a session was active.
    pub fn text_edit_commit(&mut self) -> bool {
        self.ui.pending_text_edit_history = None;
        self.ui.text_edit_last_ms = 0;
        self.ui.text_edit_select_all = false;
        self.ui.text_editing.take().is_some()
    }

    /// Open a fresh history burst when the first keystroke since the
    /// session started, or > 500 ms since the last one. Snapshots the
    /// pre-mutation state and pushes it onto the undo stack.
    fn maybe_open_text_edit_history(&mut self, now_ms: u64) {
        let last = self.ui.text_edit_last_ms;
        let elapsed = now_ms.saturating_sub(last);
        if last == 0 || elapsed > TEXT_COALESCE_MS {
            let snap = self.snapshot_for_history();
            self.ui.pending_text_edit_history = None;
            self.history_push_past(snap);
        }
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

/// Mutable handle to a `Text` node's plain content string. Returns
/// `None` for a non-Text node or a `Styled` content variant (the
/// inline editor only handles plain text).
fn text_content_mut(node: &mut PenNode) -> Option<&mut String> {
    match node {
        PenNode::Text(t) => match &mut t.content {
            TextContent::Plain(s) => Some(s),
            TextContent::Styled(_) => None,
        },
        _ => None,
    }
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

    // --- Text edit --------------------------------------------------

    #[test]
    fn text_edit_appends_and_backspaces() {
        let mut s = doc();
        assert!(s.start_text_edit(NodeId::new("n2")));
        assert!(s.text_edit_append("!", 100));
        let node = crate::walkers::find_node(s.active_children(), &NodeId::new("n2")).unwrap();
        match node {
            PenNode::Text(t) => assert_eq!(t.content, TextContent::Plain("Hi!".to_string())),
            _ => panic!("not text"),
        }
        assert!(s.text_edit_backspace(150));
        let node = crate::walkers::find_node(s.active_children(), &NodeId::new("n2")).unwrap();
        match node {
            PenNode::Text(t) => assert_eq!(t.content, TextContent::Plain("Hi".to_string())),
            _ => panic!("not text"),
        }
        assert!(s.text_edit_commit());
    }

    #[test]
    fn text_edit_rejects_non_text_node() {
        let mut s = doc();
        assert!(!s.start_text_edit(NodeId::new("n1")));
        assert!(!s.text_edit_append("x", 0));
    }

    #[test]
    fn text_edit_history_coalesces_within_window() {
        let mut s = doc();
        assert!(s.start_text_edit(NodeId::new("n2")));
        // First keystroke opens one history entry.
        s.text_edit_append("a", 100);
        assert_eq!(s.history.past.len(), 1);
        // Within 500 ms — no new entry.
        s.text_edit_append("b", 300);
        assert_eq!(s.history.past.len(), 1);
        // After a > 500 ms pause — a fresh entry.
        s.text_edit_append("c", 1000);
        assert_eq!(s.history.past.len(), 2);
    }

    #[test]
    fn text_edit_undo_restores_pre_session_text() {
        let mut s = doc();
        assert!(s.start_text_edit(NodeId::new("n2")));
        s.text_edit_append("!!!", 100);
        assert!(s.text_edit_commit());
        assert!(s.undo());
        let node = crate::walkers::find_node(s.active_children(), &NodeId::new("n2")).unwrap();
        match node {
            PenNode::Text(t) => assert_eq!(t.content, TextContent::Plain("Hi".to_string())),
            _ => panic!("not text"),
        }
    }
}
