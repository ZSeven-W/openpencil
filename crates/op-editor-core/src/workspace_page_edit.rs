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

/// Queue the original in-place edit, restoring its captured board even after
/// paging elsewhere. A removed board or document swap never becomes generation.
pub fn retry_page_edit(state: &mut EditorState) -> bool {
    let workspace = &state.editor_ui.workspace;
    if !workspace.active
        || !matches!(
            workspace.phase,
            crate::WorkspacePhase::Failed | crate::WorkspacePhase::Stopped
        )
    {
        return false;
    }
    let Some(retry) = workspace.page_edit_retry.clone() else {
        return false;
    };
    let boards = crate::preview_slideshow::active_page_boards(state);
    let Some(index) = boards.iter().position(|id| id == &retry.target.board_id) else {
        return false;
    };
    if retry.instruction.trim().is_empty() {
        return false;
    }
    state.editor_ui.workspace.selected = index;
    state
        .editor_ui
        .workspace
        .stage_page_edit(&retry.target.board_id, index);
    state.selection.set.clear();
    state
        .selection
        .set
        .push(crate::NodeId::new(&retry.target.board_id));
    state.chat.launch_route = crate::LaunchRoute::Auto;
    state.chat.set_input_text(retry.instruction);
    if !state.chat.begin_send() {
        return false;
    }
    state.editor_ui.workspace.resume_generating(0);
    true
}

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
    restore_boards_outside_scope(state, before, &[target_id.to_string()])
}

/// The same write fence for an explicitly selected multi-board refinement.
pub fn restore_boards_outside_scope(
    state: &mut EditorState,
    before: &[PenNode],
    target_ids: &[String],
) -> bool {
    let current = state.active_children();
    let restored: Vec<PenNode> = before
        .iter()
        .filter_map(|node| {
            if target_ids.iter().any(|id| id == node.id_str()) {
                // A deleted target stays deleted: the scope allows it.
                current
                    .iter()
                    .find(|now| now.id_str() == node.id_str())
                    .cloned()
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
