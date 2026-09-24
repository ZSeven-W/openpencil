//! 改这一页: scoping a follow-up turn to ONE board.
//!
//! The phone reader's 改这一页 stages a [`PageEditTarget`] on the
//! workspace; the launcher that consumes the next send turns it into two
//! guarantees:
//!
//! 1. **The instruction names its scope.** [`page_edit_prompt`] wraps the
//!    user's words in a scope header naming the board (page number, name,
//!    node id), so the design agent edits that board in place instead of
//!    drawing a new screen. The board is also the canvas selection, which
//!    is how the desktop's direct-modify route picks its target frames.
//! 2. **Nothing outside the scope survives.** A model can ignore a prompt,
//!    so the launcher snapshots the page's top-level boards before every
//!    mutating tool call and [`restore_other_boards`] puts every OTHER
//!    board back afterwards — edits inside the target stay, a stray write
//!    to page 2 or a brand-new top-level board is reverted. "修改目标页后，
//!    其他页保持" is enforced, not requested.
//!
//! [`PageEditTarget`]: crate::PageEditTarget

use crate::{EditorState, PageEditTarget, PenNodeExt};
use jian_ops_schema::node::PenNode;

/// The scoped user message for a page edit, or `None` when the bound
/// board no longer exists on the active page (the caller then sends the
/// instruction unscoped rather than naming a board that is gone).
pub fn page_edit_prompt(
    state: &EditorState,
    target: &PageEditTarget,
    instruction: &str,
) -> Option<String> {
    let board = state
        .active_children()
        .iter()
        .find(|node| node.id_str() == target.board_id)?;
    let name = board.base().name.as_deref().unwrap_or("");
    Some(format!(
        "PAGE EDIT SCOPE — this turn changes ONE existing board only: page {page}, \
         board \"{name}\" (node id `{id}`).\n\
         - Edit nodes inside that board in place; keep its size and position.\n\
         - Do NOT create new top-level boards, and do NOT change, move or delete any \
         other board.\n\
         - Changes outside this board are reverted automatically.\n\n\
         INSTRUCTION:\n{instruction}",
        page = target.page_number(),
        id = target.board_id,
    ))
}

/// Put every top-level node of the active page except `target_id` back
/// to its `before` version: the target keeps whatever the turn did to it,
/// nodes the turn added at the top level are dropped, and nodes it
/// deleted come back in their original order. Returns whether anything
/// had to be reverted (the caller reports that to the model).
pub fn restore_other_boards(state: &mut EditorState, before: &[PenNode], target_id: &str) -> bool {
    let current = state.active_children();
    let target_now = current
        .iter()
        .find(|node| node.id_str() == target_id)
        .cloned();
    let restored: Vec<PenNode> = before
        .iter()
        .filter_map(|node| {
            if node.id_str() == target_id {
                // A deleted target stays deleted: the scope allows it.
                target_now.clone()
            } else {
                Some(node.clone())
            }
        })
        .collect();
    if restored.as_slice() == current {
        return false;
    }
    *state.active_children_mut() = restored;
    // Raw `active_children_mut()` bypasses the command path; bump the
    // revision so caches and save-dirty tracking see the revert.
    state.mark_document_changed();
    true
}

#[cfg(test)]
#[path = "workspace_page_edit_tests.rs"]
mod tests;
