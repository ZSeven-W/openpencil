//! Page mutators — split out of `mutators.rs` to honor the
//! 800-line file cap.

use super::walkers::{deep_clone_with_new_ids, set_name_walk, text_mut_walk};
use super::*;

impl Document {
    /// Switch the active page to `idx`. Returns true on success,
    /// false when `idx` is out of bounds. Mirrors TS
    /// `setActivePage(idx)` — used by the MCP `set_active_page`
    /// write tool so LLM-driven sessions can target different
    /// pages for subsequent insert / batch_design ops.
    pub fn set_active_page(&mut self, idx: usize) -> bool {
        if idx >= self.pages.len() {
            return false;
        }
        if self.active_page_index == idx {
            return true;
        }
        self.active_page_index = idx;
        self.clear_selection();
        true
    }

    /// Append a fresh empty page named `"Page N"` (where N =
    /// `pages.len() + 1` BEFORE the insert) and switch to it. The
    /// new page's id is minted past `max_node_id() + 1`. Returns
    /// the new index, or `None` on id overflow. Mirrors TS
    /// `addPage()` — backs the LayerPanel `+` button.
    pub fn add_page(&mut self) -> Option<usize> {
        let next_id = self.max_node_id().checked_add(1)?;
        let n = self.pages.len() + 1;
        let page = Page::new(format!("n{next_id}"), format!("Page {}", n), Vec::new());
        self.pages.push(page);
        let new_index = self.pages.len() - 1;
        self.active_page_index = new_index;
        self.clear_selection();
        Some(new_index)
    }

    /// Duplicate page at `idx`, inserting clone after. New ids
    /// minted past max_node_id. Switches active page to clone.
    pub fn duplicate_page(&mut self, idx: usize) -> Option<usize> {
        let mut next_id = self.max_node_id().checked_add(1)?;
        let mut taken = self.collect_node_ids();
        let new_page_id = super::walkers::alloc_n_id(&mut next_id, &mut taken)?;
        let source = self.pages.get(idx)?;
        let new_children: Vec<Node> = source
            .children
            .iter()
            .map(|c| deep_clone_with_new_ids(c, &mut next_id, &mut taken))
            .collect();
        let clone = Page::new(new_page_id, format!("{} copy", source.name), new_children);
        let new_index = idx + 1;
        self.pages.insert(new_index, clone);
        self.active_page_index = new_index;
        self.clear_selection();
        Some(new_index)
    }

    /// Swap page at `idx` with previous. No-op at 0 / OOB.
    pub fn move_page_up(&mut self, idx: usize) -> bool {
        if idx == 0 || idx >= self.pages.len() {
            return false;
        }
        self.pages.swap(idx, idx - 1);
        if self.active_page_index == idx {
            self.active_page_index = idx - 1;
        } else if self.active_page_index == idx - 1 {
            self.active_page_index = idx;
        }
        true
    }

    /// Start inline rename on a layer row.
    pub fn start_rename_layer(&mut self, id: NodeId) -> bool {
        let Some(page) = self.active_page() else {
            return false;
        };
        let Some(node) = page.find(&id) else {
            return false;
        };
        let name = node.name.clone();
        self.ui.layer_rename = Some(LayerRenameState {
            target: LayerContextTarget::Layer(id),
            draft: name,
        });
        true
    }

    /// Set a page's name directly (MCP-friendly entry point —
    /// the UI rename flow uses `start_rename_page` +
    /// `rename_append`). Rejects out-of-range indices and empty
    /// / whitespace-only names so list_pages never returns a
    /// page label a user can't recognize.
    pub fn rename_page(&mut self, idx: usize, name: impl Into<String>) -> bool {
        let name: String = name.into();
        if name.trim().is_empty() {
            return false;
        }
        let Some(page) = self.pages.get_mut(idx) else {
            return false;
        };
        page.name = name;
        true
    }

    /// Start inline rename on a page row.
    pub fn start_rename_page(&mut self, idx: usize) -> bool {
        let Some(page) = self.pages.get(idx) else {
            return false;
        };
        let name = page.name.clone();
        self.ui.layer_rename = Some(LayerRenameState {
            target: LayerContextTarget::Page(idx),
            draft: name,
        });
        true
    }

    /// Append text to the in-flight rename draft.
    pub fn rename_append(&mut self, text: &str) -> bool {
        match self.ui.layer_rename.as_mut() {
            Some(state) => {
                state.draft.push_str(text);
                true
            }
            None => false,
        }
    }

    /// Pop the last char of the rename draft.
    pub fn rename_backspace(&mut self) -> bool {
        match self.ui.layer_rename.as_mut() {
            Some(state) => {
                state.draft.pop();
                true
            }
            None => false,
        }
    }

    /// Commit the active rename. Returns true when a rename was in
    /// flight (UI changed → repaint). Pushes a history snapshot
    /// only when the name actually changed, avoiding phantom undo
    /// entries on empty-draft or no-op commits.
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
                if let Some(page) = self.pages.get_mut(idx) {
                    if page.name != state.draft {
                        page.name = state.draft;
                        changed = true;
                    }
                }
            }
            LayerContextTarget::Layer(id) => {
                if let Some(page) = self.pages.get_mut(self.active_page_index) {
                    for child in &mut page.children {
                        if set_name_walk(child, &id, &state.draft) {
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }
        if changed {
            self.history_push_past(snap);
        }
        true
    }

    /// Cancel an in-flight rename.
    pub fn rename_cancel(&mut self) -> bool {
        self.ui.layer_rename.take().is_some()
    }

    /// Enter inline text-edit mode on the given node. Returns false
    /// when the node isn't a `NodeKind::Text` or doesn't exist.
    pub fn start_text_edit(&mut self, id: NodeId) -> bool {
        let Some(page) = self.active_page() else {
            return false;
        };
        let Some(node) = page.find(&id) else {
            return false;
        };
        if !matches!(node.kind, NodeKind::Text) {
            return false;
        }
        self.ui.text_editing = Some(id);
        // History opens lazily on the first mutating keystroke
        // via `maybe_open_text_edit_history` (driven by
        // `text_edit_last_ms`). Entering edit mode without typing
        // leaves the undo stack untouched.
        self.ui.text_edit_last_ms = 0;
        true
    }

    /// Append `text` to the actively-edited Text node's content.
    /// `now_ms` lets us coalesce a typing burst into one undo step:
    /// a pause > 500 ms opens a fresh history entry, otherwise the
    /// new char piggy-backs on the in-flight snapshot.
    pub fn text_edit_append(&mut self, text: &str, now_ms: u64) -> bool {
        let Some(id) = self.ui.text_editing.clone() else {
            return false;
        };
        self.maybe_open_text_edit_history(now_ms);
        let active = self.active_page_index;
        let Some(page) = self.pages.get_mut(active) else {
            return false;
        };
        for child in &mut page.children {
            if let Some(s) = text_mut_walk(child, &id) {
                s.push_str(text);
                self.ui.text_edit_last_ms = now_ms;
                return true;
            }
        }
        false
    }

    /// Pop the last char from the actively-edited Text node.
    pub fn text_edit_backspace(&mut self, now_ms: u64) -> bool {
        let Some(id) = self.ui.text_editing.clone() else {
            return false;
        };
        self.maybe_open_text_edit_history(now_ms);
        let active = self.active_page_index;
        let Some(page) = self.pages.get_mut(active) else {
            return false;
        };
        for child in &mut page.children {
            if let Some(s) = text_mut_walk(child, &id) {
                let ok = s.pop().is_some();
                if ok {
                    self.ui.text_edit_last_ms = now_ms;
                }
                return ok;
            }
        }
        false
    }

    /// Open a new history burst if the user paused longer than
    /// 500 ms (or this is the first keystroke since starting the
    /// session). Snapshots taken with `pre_snap` machinery already;
    /// here we just flush + reset the pending snapshot.
    fn maybe_open_text_edit_history(&mut self, now_ms: u64) {
        const COALESCE_MS: u64 = 500;
        let last = self.ui.text_edit_last_ms;
        let elapsed = now_ms.saturating_sub(last);
        if last == 0 || elapsed > COALESCE_MS {
            // Snapshot current pages BEFORE the imminent mutation.
            let snap = self.snapshot_for_history();
            // Drop any older pending — this burst supersedes it.
            self.ui.pending_text_edit_history = None;
            self.history_push_past(snap);
        }
    }

    /// Exit text-edit mode. Returns true iff a session was active.
    /// Pushes the pending pre-edit snapshot onto the past stack
    /// only when the node's text actually changed during the
    /// session — type-then-backspace-to-original is a no-op for
    /// history, so Cmd-Z always corresponds to a visible change.
    /// Exit text-edit mode. History snapshots were pushed lazily
    /// by `text_edit_append/backspace` during the session, so
    /// commit only needs to clear UI state.
    pub fn text_edit_commit(&mut self) -> bool {
        self.ui.pending_text_edit_history = None;
        self.ui.text_edit_last_ms = 0;
        self.ui.text_editing.take().is_some()
    }

    /// Move page from `from` to `to`. `to` is clamped into
    /// `[0, pages.len())`. No-op when from == to or from is
    /// out of range. Keeps `active_page_index` pointing at the
    /// same logical page (follows the move). Mirrors TS
    /// `reorderPage(from, to)`.
    pub fn reorder_page(&mut self, from: usize, to: usize) -> bool {
        if from >= self.pages.len() {
            return false;
        }
        let to = to.min(self.pages.len().saturating_sub(1));
        if from == to {
            return false;
        }
        let page = self.pages.remove(from);
        self.pages.insert(to, page);
        // Adjust the active index so it tracks the moved page or
        // the shift caused by the move.
        if self.active_page_index == from {
            self.active_page_index = to;
        } else if from < self.active_page_index && to >= self.active_page_index {
            self.active_page_index -= 1;
        } else if from > self.active_page_index && to <= self.active_page_index {
            self.active_page_index += 1;
        }
        true
    }

    /// Swap page at `idx` with next. No-op at end.
    pub fn move_page_down(&mut self, idx: usize) -> bool {
        if idx + 1 >= self.pages.len() {
            return false;
        }
        self.pages.swap(idx, idx + 1);
        if self.active_page_index == idx {
            self.active_page_index = idx + 1;
        } else if self.active_page_index == idx + 1 {
            self.active_page_index = idx;
        }
        true
    }

    /// Remove the page at `idx`. No-op when out-of-range OR when
    /// only one page remains (`validate` rejects empty pages).
    /// Adjusts `active_page_index` and clears the selection.
    pub fn remove_page(&mut self, idx: usize) -> bool {
        if idx >= self.pages.len() || self.pages.len() <= 1 {
            return false;
        }
        self.pages.remove(idx);
        if self.active_page_index >= self.pages.len() {
            self.active_page_index = self.pages.len() - 1;
        } else if idx < self.active_page_index {
            self.active_page_index -= 1;
        }
        self.clear_selection();
        true
    }
}
