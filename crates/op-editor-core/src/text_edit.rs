//! Inline canvas text-edit session — the TS editor's styled
//! `<textarea>` overlay (`apps/web/src/canvas/skia/skia-canvas.tsx`)
//! re-implemented as an explicit editing model, since the Rust shell
//! has no DOM to host a native text field.
//!
//! ## Session
//!
//! `start_text_edit` enters the session on a `Text` node; the caret
//! is a BYTE offset into the node's plain content
//! (`ui.text_edit_caret`), with an optional selection anchor
//! (`ui.text_edit_selection_anchor`; selection = `anchor..caret` in
//! either order). Cmd/Ctrl+A maps to anchor 0 + caret at the end
//! (`text_edit_select_all_now`) while keeping the legacy
//! `text_edit_select_all` flag for the whole-wash paint.
//!
//! ## Keys (TS textarea parity)
//!
//! Enter INSERTS a newline (`text_edit_insert("\n", …)`); only
//! Escape and an outside click commit. Arrows move the caret;
//! Shift+arrows extend the selection. Up/Down move by VISUAL line:
//! the host passes the painted wrap's byte ranges and the caret
//! keeps its CHARACTER column — an approximation of "same x" that
//! ignores proportional glyph widths (see
//! [`EditorState::text_edit_caret_vertical`]).
//!
//! ## History
//!
//! To keep a typing burst as one undoable step, history opens
//! lazily: the first mutation (or any mutation after a > 500 ms
//! pause) snapshots the pre-edit state and pushes it. The
//! last-keystroke timestamp lives on `ui.text_edit_last_ms`.

use crate::node_id::NodeId;
use crate::state::EditorState;
use jian_ops_schema::node::{PenNode, TextContent};

/// Coalesce window for text-edit history bursts (ms). A pause longer
/// than this opens a fresh undo entry.
const TEXT_COALESCE_MS: u64 = 500;

impl EditorState {
    /// Enter inline text-edit mode on `id`. `false` when the node
    /// isn't a `Text` node or doesn't exist. The caret seeds at the
    /// end of the content; history opens lazily on the first edit.
    pub fn start_text_edit(&mut self, id: NodeId) -> bool {
        let Some(node) = crate::walkers::find_node(self.active_children(), &id) else {
            return false;
        };
        if !matches!(node, PenNode::Text(_)) {
            return false;
        }
        let len = text_content(node).map(str::len).unwrap_or(0);
        self.ui.text_editing = Some(id);
        self.ui.text_edit_select_all = false;
        self.ui.text_edit_caret = Some(len);
        self.ui.text_edit_selection_anchor = None;
        self.ui.text_edit_last_ms = 0;
        self.ui.pending_text_edit_history = None;
        true
    }

    /// The edited node's plain content, or `None` when no session is
    /// active / the node vanished / the content is `Styled`.
    pub fn text_edit_content(&self) -> Option<&str> {
        let id = self.ui.text_editing.as_ref()?;
        let node = crate::walkers::find_node(self.active_children(), id)?;
        text_content(node)
    }

    /// Insert `text` at the caret, replacing the active selection (or
    /// the whole content after Cmd/Ctrl+A). `now_ms` lets a typing
    /// burst coalesce into one undo step. `false` when no session is
    /// active or the node has vanished.
    pub fn text_edit_insert(&mut self, text: &str, now_ms: u64) -> bool {
        let Some(id) = self.ui.text_editing.clone() else {
            return false;
        };
        let Some(current) = self.text_edit_content().map(str::to_string) else {
            return false;
        };
        let len = current.len();
        self.maybe_open_text_edit_history(now_ms);
        // Snap every edge to a char boundary — a stale caret left by
        // an external content change must never split a codepoint.
        let (start, end) = match self.text_edit_effective_selection(len) {
            Some((s, e)) => (
                prev_boundary_at_or_before(&current, s),
                prev_boundary_at_or_before(&current, e),
            ),
            None => {
                let caret = prev_boundary_at_or_before(&current, self.text_edit_clamped_caret(len));
                (caret, caret)
            }
        };
        let Some(content) = self.text_edit_content_mut(&id) else {
            return false;
        };
        content.replace_range(start..end, text);
        self.ui.text_edit_caret = Some(start + text.len());
        self.ui.text_edit_selection_anchor = None;
        self.ui.text_edit_select_all = false;
        self.ui.text_edit_last_ms = now_ms;
        true
    }

    /// Delete the selection, or the character BEFORE the caret.
    /// `false` when no session is active, the node has vanished, or
    /// the caret is already at the start with no selection.
    pub fn text_edit_backspace(&mut self, now_ms: u64) -> bool {
        self.text_edit_delete(now_ms, false)
    }

    /// Delete the selection, or the character AFTER the caret (the
    /// Delete key's forward deletion). `false` when nothing deletes.
    pub fn text_edit_delete_forward(&mut self, now_ms: u64) -> bool {
        self.text_edit_delete(now_ms, true)
    }

    fn text_edit_delete(&mut self, now_ms: u64, forward: bool) -> bool {
        let Some(id) = self.ui.text_editing.clone() else {
            return false;
        };
        let Some(current) = self.text_edit_content().map(str::to_string) else {
            return false;
        };
        let len = current.len();
        let range = match self.text_edit_effective_selection(len) {
            Some((s, e)) => Some((
                prev_boundary_at_or_before(&current, s),
                prev_boundary_at_or_before(&current, e),
            )),
            None => {
                let caret = prev_boundary_at_or_before(&current, self.text_edit_clamped_caret(len));
                if forward {
                    (caret < len).then(|| (caret, next_boundary(&current, caret)))
                } else {
                    (caret > 0).then(|| (prev_boundary(&current, caret), caret))
                }
            }
        };
        let Some((start, end)) = range else {
            // Reset stale selection state even when nothing deletes.
            self.ui.text_edit_select_all = false;
            return false;
        };
        self.maybe_open_text_edit_history(now_ms);
        let Some(content) = self.text_edit_content_mut(&id) else {
            return false;
        };
        content.replace_range(start..end, "");
        self.ui.text_edit_caret = Some(start);
        self.ui.text_edit_selection_anchor = None;
        self.ui.text_edit_select_all = false;
        self.ui.text_edit_last_ms = now_ms;
        true
    }

    /// Exit text-edit mode. History snapshots were pushed lazily by
    /// the editing ops, so commit only clears the session state.
    /// `true` when a session was active.
    pub fn text_edit_commit(&mut self) -> bool {
        self.ui.pending_text_edit_history = None;
        self.ui.text_edit_last_ms = 0;
        self.ui.text_edit_select_all = false;
        self.ui.text_edit_caret = None;
        self.ui.text_edit_selection_anchor = None;
        self.ui.text_editing.take().is_some()
    }

    /// Place the caret at byte `offset` (clamped to a char boundary).
    /// `extend == true` keeps / establishes the selection anchor
    /// (Shift+click); otherwise the selection collapses.
    pub fn text_edit_set_caret(&mut self, offset: usize, extend: bool) -> bool {
        let Some(content) = self.text_edit_content() else {
            return false;
        };
        let offset = prev_boundary_at_or_before(content, offset.min(content.len()));
        let old_caret = self.text_edit_clamped_caret(content.len());
        if extend {
            let anchor = self.ui.text_edit_selection_anchor.unwrap_or(old_caret);
            self.ui.text_edit_selection_anchor = (anchor != offset).then_some(anchor);
        } else {
            self.ui.text_edit_selection_anchor = None;
        }
        self.ui.text_edit_caret = Some(offset);
        self.ui.text_edit_select_all = false;
        true
    }

    /// Drag-select: anchor stays at the press offset, caret follows
    /// the cursor. Both clamp to char boundaries.
    pub fn text_edit_select_range(&mut self, anchor: usize, focus: usize) -> bool {
        let Some(content) = self.text_edit_content() else {
            return false;
        };
        let len = content.len();
        let anchor = prev_boundary_at_or_before(content, anchor.min(len));
        let focus = prev_boundary_at_or_before(content, focus.min(len));
        self.ui.text_edit_selection_anchor = (anchor != focus).then_some(anchor);
        self.ui.text_edit_caret = Some(focus);
        self.ui.text_edit_select_all = false;
        true
    }

    /// Select the whole content: anchor 0, caret at the end. Keeps
    /// the legacy `text_edit_select_all` flag set for the whole-wash
    /// paint path.
    pub fn text_edit_select_all_now(&mut self) -> bool {
        let Some(content) = self.text_edit_content() else {
            return false;
        };
        let len = content.len();
        self.ui.text_edit_select_all = true;
        self.ui.text_edit_selection_anchor = (len > 0).then_some(0);
        self.ui.text_edit_caret = Some(len);
        true
    }

    /// Left / Right arrow. With an active selection and no Shift the
    /// caret collapses to the selection edge (textarea behaviour);
    /// otherwise it moves one char, clamped at the content bounds.
    /// `extend` (Shift) keeps / establishes the anchor.
    pub fn text_edit_caret_horizontal(&mut self, forward: bool, extend: bool) -> bool {
        let Some(content) = self.text_edit_content().map(str::to_string) else {
            return false;
        };
        let content = content.as_str();
        let len = content.len();
        let caret = self.text_edit_clamped_caret(len);
        if !extend {
            if let Some((start, end)) = self.text_edit_effective_selection(len) {
                self.ui.text_edit_caret = Some(if forward { end } else { start });
                self.ui.text_edit_selection_anchor = None;
                self.ui.text_edit_select_all = false;
                return true;
            }
        }
        let next = if forward {
            next_boundary(content, caret)
        } else {
            prev_boundary(content, caret)
        };
        self.text_edit_move_caret_to(next, caret, extend);
        true
    }

    /// Up / Down arrow — move by VISUAL line through the painted
    /// wrap. `line_ranges` are the byte `(start, end)` ranges of each
    /// painted line within the content, in paint order (the host
    /// derives them from the same wrap the canvas painter uses).
    ///
    /// APPROXIMATION: the caret keeps its CHARACTER column (chars
    /// from line start), clamped to the target line's char count —
    /// not its pixel x. With proportional fonts or mixed CJK/Latin
    /// lines the landing column can differ from a pixel-exact mapping
    /// by a few glyphs; in exchange the op stays measurer-free and
    /// deterministic. Past the first/last line the caret clamps to
    /// the content start/end (textarea behaviour).
    pub fn text_edit_caret_vertical(
        &mut self,
        down: bool,
        extend: bool,
        line_ranges: &[(usize, usize)],
    ) -> bool {
        let Some(content) = self.text_edit_content() else {
            return false;
        };
        let content = content.to_string();
        let len = content.len();
        let mut caret = self.text_edit_clamped_caret(len);
        let old_caret = caret;
        if !extend {
            // Collapse an active selection to the moving edge first.
            if let Some((start, end)) = self.text_edit_effective_selection(len) {
                caret = if down { end } else { start };
            }
        }
        if line_ranges.is_empty() {
            let next = if down { len } else { 0 };
            self.text_edit_move_caret_to(next, old_caret, extend);
            return true;
        }
        let line_idx = line_index_for_offset(line_ranges, caret);
        let (line_start, _) = line_ranges[line_idx];
        let column = content[line_start.min(len)..caret.min(len)].chars().count();
        let next = if down {
            match line_ranges.get(line_idx + 1) {
                Some(range) => offset_at_column(&content, *range, column),
                None => len,
            }
        } else if line_idx == 0 {
            0
        } else {
            offset_at_column(&content, line_ranges[line_idx - 1], column)
        };
        self.text_edit_move_caret_to(next, old_caret, extend);
        true
    }

    /// Cmd+Left / Cmd+Right — jump to the start / end of the caret's
    /// visual line (same `line_ranges` contract as
    /// [`Self::text_edit_caret_vertical`]).
    pub fn text_edit_line_edge(
        &mut self,
        forward: bool,
        extend: bool,
        line_ranges: &[(usize, usize)],
    ) -> bool {
        let Some(content) = self.text_edit_content() else {
            return false;
        };
        let len = content.len();
        let caret = self.text_edit_clamped_caret(len);
        let next = if line_ranges.is_empty() {
            if forward {
                len
            } else {
                0
            }
        } else {
            let (start, end) = line_ranges[line_index_for_offset(line_ranges, caret)];
            if forward {
                end.min(len)
            } else {
                start.min(len)
            }
        };
        self.text_edit_move_caret_to(next, caret, extend);
        true
    }

    /// Shared caret-move bookkeeping: `extend` keeps / establishes
    /// the anchor (dropping it when collapsed), a plain move clears
    /// it. The select-all flag drops on any caret motion.
    fn text_edit_move_caret_to(&mut self, next: usize, old_caret: usize, extend: bool) {
        if extend {
            let anchor = self.ui.text_edit_selection_anchor.unwrap_or(old_caret);
            self.ui.text_edit_selection_anchor = (anchor != next).then_some(anchor);
        } else {
            self.ui.text_edit_selection_anchor = None;
        }
        self.ui.text_edit_caret = Some(next);
        self.ui.text_edit_select_all = false;
    }

    /// Active selection as an ordered byte range. The select-all flag
    /// counts as `0..len`; a collapsed anchor==caret pair is `None`.
    pub fn text_edit_effective_selection(&self, len: usize) -> Option<(usize, usize)> {
        if self.ui.text_edit_select_all {
            return (len > 0).then_some((0, len));
        }
        let anchor = self.ui.text_edit_selection_anchor?.min(len);
        let caret = self.text_edit_clamped_caret(len);
        (anchor != caret).then(|| (anchor.min(caret), anchor.max(caret)))
    }

    /// The caret byte offset clamped to `len` — `None` (legacy
    /// append-at-end sessions) falls back to the end.
    fn text_edit_clamped_caret(&self, len: usize) -> usize {
        self.ui.text_edit_caret.unwrap_or(len).min(len)
    }

    fn text_edit_content_mut(&mut self, id: &NodeId) -> Option<&mut String> {
        let node = crate::walkers::find_node_mut(self.active_children_mut(), id)?;
        text_content_node_mut(node)
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

/// Read-only handle to a `Text` node's plain content. `None` for a
/// non-Text node or a `Styled` content variant (the inline editor
/// only handles plain text).
fn text_content(node: &PenNode) -> Option<&str> {
    match node {
        PenNode::Text(t) => match &t.content {
            TextContent::Plain(s) => Some(s),
            TextContent::Styled(_) => None,
        },
        _ => None,
    }
}

/// Mutable counterpart of [`text_content`].
fn text_content_node_mut(node: &mut PenNode) -> Option<&mut String> {
    match node {
        PenNode::Text(t) => match &mut t.content {
            TextContent::Plain(s) => Some(s),
            TextContent::Styled(_) => None,
        },
        _ => None,
    }
}

/// Largest char boundary `<= pos` (clamped to `s.len()`).
fn prev_boundary_at_or_before(s: &str, pos: usize) -> usize {
    let mut p = pos.min(s.len());
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

/// Char boundary one character BEFORE `pos` (0 at the start).
fn prev_boundary(s: &str, pos: usize) -> usize {
    let pos = prev_boundary_at_or_before(s, pos);
    s[..pos].char_indices().last().map(|(i, _)| i).unwrap_or(0)
}

/// Char boundary one character AFTER `pos` (`s.len()` at the end).
fn next_boundary(s: &str, pos: usize) -> usize {
    let pos = prev_boundary_at_or_before(s, pos);
    if pos >= s.len() {
        return s.len();
    }
    pos + s[pos..].chars().next().map(char::len_utf8).unwrap_or(0)
}

/// Index of the visual line containing byte `offset`. An offset in
/// the gap between two lines (a wrap-dropped space or a `\n`) counts
/// as the EARLIER line's end; offsets past the last line clamp to it.
fn line_index_for_offset(line_ranges: &[(usize, usize)], offset: usize) -> usize {
    for (i, (start, end)) in line_ranges.iter().enumerate() {
        if offset < *start {
            // In the separator gap before this line — belongs to the
            // previous line's end.
            return i.saturating_sub(1);
        }
        if offset <= *end {
            return i;
        }
    }
    line_ranges.len().saturating_sub(1)
}

/// Byte offset of the `column`-th character within `range` of
/// `content`, clamped to the range end.
fn offset_at_column(content: &str, range: (usize, usize), column: usize) -> usize {
    let (start, end) = (range.0.min(content.len()), range.1.min(content.len()));
    let line = &content[start..end];
    for (taken, (i, _)) in line.char_indices().enumerate() {
        if taken == column {
            return start + i;
        }
    }
    end
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

    fn content(s: &EditorState) -> String {
        let node =
            crate::walkers::find_node(s.active_children(), &NodeId::new("n2")).expect("text node");
        match node {
            PenNode::Text(t) => match &t.content {
                TextContent::Plain(p) => p.clone(),
                TextContent::Styled(_) => panic!("styled"),
            },
            _ => panic!("not text"),
        }
    }

    fn edit(initial: &str) -> EditorState {
        let mut s = state_with(vec![text("n2", "Label", 0.0, 50.0, 100.0, 20.0, initial)]);
        assert!(s.start_text_edit(NodeId::new("n2")));
        s
    }

    // --- Insert / delete ---------------------------------------------

    #[test]
    fn insert_lands_at_end_after_start() {
        let mut s = doc();
        assert!(s.start_text_edit(NodeId::new("n2")));
        assert_eq!(s.ui.text_edit_caret, Some(2)); // after "Hi"
        assert!(s.text_edit_insert("!", 100));
        assert_eq!(content(&s), "Hi!");
        assert_eq!(s.ui.text_edit_caret, Some(3));
        assert!(s.text_edit_backspace(150));
        assert_eq!(content(&s), "Hi");
        assert!(s.text_edit_commit());
        assert_eq!(s.ui.text_edit_caret, None);
    }

    #[test]
    fn insert_at_mid_caret() {
        let mut s = edit("Hi");
        assert!(s.text_edit_caret_horizontal(false, false)); // caret 1
        assert!(s.text_edit_insert("X", 100));
        assert_eq!(content(&s), "HXi");
        assert_eq!(s.ui.text_edit_caret, Some(2));
    }

    #[test]
    fn backspace_at_mid_caret_removes_prev_char() {
        let mut s = edit("abc");
        assert!(s.text_edit_set_caret(2, false));
        assert!(s.text_edit_backspace(100));
        assert_eq!(content(&s), "ac");
        assert_eq!(s.ui.text_edit_caret, Some(1));
        // At the start — nothing to remove.
        assert!(s.text_edit_set_caret(0, false));
        assert!(!s.text_edit_backspace(150));
        assert_eq!(content(&s), "ac");
    }

    #[test]
    fn delete_forward_removes_next_char() {
        let mut s = edit("abc");
        assert!(s.text_edit_set_caret(1, false));
        assert!(s.text_edit_delete_forward(100));
        assert_eq!(content(&s), "ac");
        assert_eq!(s.ui.text_edit_caret, Some(1));
        assert!(s.text_edit_set_caret(2, false));
        assert!(!s.text_edit_delete_forward(150), "at end — no-op");
    }

    #[test]
    fn insert_replaces_active_selection() {
        let mut s = edit("hello world");
        assert!(s.text_edit_select_range(6, 11)); // "world"
        assert!(s.text_edit_insert("there", 100));
        assert_eq!(content(&s), "hello there");
        assert_eq!(s.ui.text_edit_caret, Some(11));
        assert_eq!(s.ui.text_edit_selection_anchor, None);
    }

    #[test]
    fn backspace_deletes_selection_as_one_unit() {
        let mut s = edit("hello world");
        assert!(s.text_edit_select_range(11, 5)); // reversed order
        assert!(s.text_edit_backspace(100));
        assert_eq!(content(&s), "hello");
        assert_eq!(s.ui.text_edit_caret, Some(5));
    }

    #[test]
    fn newline_inserts_instead_of_committing() {
        let mut s = edit("ab");
        assert!(s.text_edit_set_caret(1, false));
        assert!(s.text_edit_insert("\n", 100));
        assert_eq!(content(&s), "a\nb");
        assert!(s.ui.text_editing.is_some(), "session stays open");
        assert_eq!(s.ui.text_edit_caret, Some(2));
    }

    #[test]
    fn select_all_then_insert_replaces_everything() {
        let mut s = edit("old text");
        assert!(s.text_edit_select_all_now());
        assert!(s.ui.text_edit_select_all);
        assert_eq!(s.ui.text_edit_selection_anchor, Some(0));
        assert!(s.text_edit_insert("n", 100));
        assert_eq!(content(&s), "n");
        assert!(!s.ui.text_edit_select_all);
    }

    #[test]
    fn select_all_then_backspace_clears() {
        let mut s = edit("old text");
        assert!(s.text_edit_select_all_now());
        assert!(s.text_edit_backspace(100));
        assert_eq!(content(&s), "");
    }

    #[test]
    fn cjk_caret_moves_on_char_boundaries() {
        let mut s = edit("首页a");
        assert!(s.text_edit_caret_horizontal(false, false)); // before 'a' (byte 6)
        assert_eq!(s.ui.text_edit_caret, Some(6));
        assert!(s.text_edit_caret_horizontal(false, false)); // byte 3
        assert_eq!(s.ui.text_edit_caret, Some(3));
        assert!(s.text_edit_insert("X", 100));
        assert_eq!(content(&s), "首X页a");
    }

    // --- Horizontal caret --------------------------------------------

    #[test]
    fn horizontal_arrows_clamp_at_bounds() {
        let mut s = edit("ab");
        for _ in 0..5 {
            assert!(s.text_edit_caret_horizontal(false, false));
        }
        assert_eq!(s.ui.text_edit_caret, Some(0));
        for _ in 0..5 {
            assert!(s.text_edit_caret_horizontal(true, false));
        }
        assert_eq!(s.ui.text_edit_caret, Some(2));
    }

    #[test]
    fn plain_arrow_collapses_selection_to_edge() {
        let mut s = edit("hello");
        assert!(s.text_edit_select_range(1, 4));
        assert!(s.text_edit_caret_horizontal(false, false));
        assert_eq!(s.ui.text_edit_caret, Some(1), "left collapses to start");
        assert_eq!(s.ui.text_edit_selection_anchor, None);
        assert!(s.text_edit_select_range(1, 4));
        assert!(s.text_edit_caret_horizontal(true, false));
        assert_eq!(s.ui.text_edit_caret, Some(4), "right collapses to end");
    }

    #[test]
    fn shift_arrows_extend_then_collapse_selection() {
        let mut s = edit("abcd");
        assert!(s.text_edit_set_caret(1, false));
        assert!(s.text_edit_caret_horizontal(true, true));
        assert!(s.text_edit_caret_horizontal(true, true));
        assert_eq!(s.ui.text_edit_selection_anchor, Some(1));
        assert_eq!(s.ui.text_edit_caret, Some(3));
        assert_eq!(s.text_edit_effective_selection(4), Some((1, 3)));
        // Shrinking back to the anchor drops the selection.
        assert!(s.text_edit_caret_horizontal(false, true));
        assert!(s.text_edit_caret_horizontal(false, true));
        assert_eq!(s.ui.text_edit_selection_anchor, None);
        assert_eq!(s.text_edit_effective_selection(4), None);
    }

    // --- Vertical caret (visual lines) ---------------------------------

    /// "hello\nworld wide\nhi" painted as three lines. The byte
    /// ranges skip the `\n` separators, mirroring the wrap mapping
    /// the host derives from the painted lines.
    const THREE_LINES: &[(usize, usize)] = &[(0, 5), (6, 16), (17, 19)];

    #[test]
    fn vertical_down_keeps_char_column() {
        let mut s = edit("hello\nworld wide\nhi");
        assert!(s.text_edit_set_caret(3, false)); // line 0, col 3
        assert!(s.text_edit_caret_vertical(true, false, THREE_LINES));
        assert_eq!(s.ui.text_edit_caret, Some(9), "line 1 col 3");
        assert!(s.text_edit_caret_vertical(true, false, THREE_LINES));
        // Line 2 has 2 chars — clamps to its end.
        assert_eq!(s.ui.text_edit_caret, Some(19));
    }

    #[test]
    fn vertical_up_from_first_line_goes_to_start() {
        let mut s = edit("hello\nworld");
        let ranges = &[(0, 5), (6, 11)];
        assert!(s.text_edit_set_caret(3, false));
        assert!(s.text_edit_caret_vertical(false, false, ranges));
        assert_eq!(s.ui.text_edit_caret, Some(0));
    }

    #[test]
    fn vertical_down_past_last_line_goes_to_end() {
        let mut s = edit("hello\nworld");
        let ranges = &[(0, 5), (6, 11)];
        assert!(s.text_edit_set_caret(8, false)); // line 1
        assert!(s.text_edit_caret_vertical(true, false, ranges));
        assert_eq!(s.ui.text_edit_caret, Some(11));
    }

    #[test]
    fn vertical_with_shift_extends_selection() {
        let mut s = edit("hello\nworld wide\nhi");
        assert!(s.text_edit_set_caret(2, false));
        assert!(s.text_edit_caret_vertical(true, true, THREE_LINES));
        assert_eq!(s.ui.text_edit_selection_anchor, Some(2));
        assert_eq!(s.ui.text_edit_caret, Some(8));
        assert_eq!(s.text_edit_effective_selection(19), Some((2, 8)));
    }

    #[test]
    fn vertical_caret_in_separator_gap_maps_to_earlier_line() {
        let mut s = edit("hello\nworld");
        let ranges = &[(0, 5), (6, 11)];
        // Caret at byte 5 == end of line 0 (the `\n` gap edge).
        assert!(s.text_edit_set_caret(5, false));
        assert!(s.text_edit_caret_vertical(true, false, ranges));
        assert_eq!(s.ui.text_edit_caret, Some(11), "line 1 col 5 == its end");
    }

    #[test]
    fn line_edge_jumps_to_line_start_and_end() {
        let mut s = edit("hello\nworld wide");
        let ranges = &[(0, 5), (6, 16)];
        assert!(s.text_edit_set_caret(9, false)); // line 1, col 3
        assert!(s.text_edit_line_edge(false, false, ranges));
        assert_eq!(s.ui.text_edit_caret, Some(6));
        assert!(s.text_edit_line_edge(true, false, ranges));
        assert_eq!(s.ui.text_edit_caret, Some(16));
    }

    // --- Session / history ---------------------------------------------

    #[test]
    fn text_edit_rejects_non_text_node() {
        let mut s = doc();
        assert!(!s.start_text_edit(NodeId::new("n1")));
        assert!(!s.text_edit_insert("x", 0));
    }

    #[test]
    fn text_edit_history_coalesces_within_window() {
        let mut s = doc();
        assert!(s.start_text_edit(NodeId::new("n2")));
        // First keystroke opens one history entry.
        s.text_edit_insert("a", 100);
        assert_eq!(s.history.past.len(), 1);
        // Within 500 ms — no new entry.
        s.text_edit_insert("b", 300);
        assert_eq!(s.history.past.len(), 1);
        // After a > 500 ms pause — a fresh entry.
        s.text_edit_insert("c", 1000);
        assert_eq!(s.history.past.len(), 2);
    }

    #[test]
    fn text_edit_undo_restores_pre_session_text() {
        let mut s = doc();
        assert!(s.start_text_edit(NodeId::new("n2")));
        s.text_edit_insert("!!!", 100);
        assert!(s.text_edit_commit());
        assert!(s.undo());
        assert_eq!(content(&s), "Hi");
    }

    #[test]
    fn caret_moves_push_no_history() {
        let mut s = edit("hello");
        assert!(s.text_edit_caret_horizontal(false, false));
        assert!(s.text_edit_set_caret(2, false));
        assert!(s.text_edit_caret_vertical(true, false, &[(0, 5)]));
        assert_eq!(s.history.past.len(), 0);
    }

    #[test]
    fn click_set_caret_with_shift_extends() {
        let mut s = edit("hello");
        assert!(s.text_edit_set_caret(1, false));
        assert!(s.text_edit_set_caret(4, true));
        assert_eq!(s.text_edit_effective_selection(5), Some((1, 4)));
    }
}
