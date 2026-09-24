//! The mobile chat pump's half of the generation workspace.
//!
//! The phone reader shows the same `WorkspaceState` the desktop runner
//! drives, so the engine's pump has to feed it the same edges: stamp the
//! run epoch when a design turn launches, settle Done / Failed when the
//! turn finishes (or never launches), mark Stopped when the user stops,
//! and keep the reader's camera on the boards as they land. The verdicts
//! are the shared `op_editor_core::workspace_run` ones, so a phone and a
//! desktop agree on what 已完成 and 失败 mean.
//!
//! It also owns 改这一页 on this host: the staged `PageEditTarget` scopes
//! the next turn's prompt (`workspace_page_edit::page_edit_prompt`) and
//! every mutating tool call of that turn runs inside a fence that puts
//! every OTHER top-level board back (`restore_other_boards`).

use op_ai::chat_provider::ChatToolResult;
use op_editor_core::{workspace_page_edit, workspace_run, ChatRole, EditorState, WorkspacePhase};
use op_host_native::WidgetHostNative;

/// What the launcher should send for `user_text`: the text itself, or —
/// when 改这一页 staged a board and that board still exists — the scoped
/// instruction plus the board id every tool call of the turn is fenced to.
pub(crate) struct LaunchScope {
    pub(crate) prompt: String,
    pub(crate) fence: Option<String>,
}

/// Consume the staged page edit (one send, one binding) into the turn's
/// prompt and fence.
pub(crate) fn scope_launch(host: &mut WidgetHostNative, user_text: &str) -> LaunchScope {
    let state = host.editor_state_mut();
    let Some(target) = state.editor_ui.workspace.begin_page_edit_turn() else {
        return LaunchScope {
            prompt: user_text.to_string(),
            fence: None,
        };
    };
    match workspace_page_edit::page_edit_prompt(state, &target, user_text) {
        Some(prompt) => LaunchScope {
            prompt,
            fence: Some(target.board_id),
        },
        None => {
            // The bound board is gone: the turn runs unscoped, and the
            // reader must not claim it is editing a page that no longer
            // exists.
            state.editor_ui.workspace.page_edit_running = None;
            LaunchScope {
                prompt: user_text.to_string(),
                fence: None,
            }
        }
    }
}

/// A design turn launched under `epoch`: the workspace this document
/// carries adopts it (a follow-up or retry puts it back into 生成中).
pub(crate) fn stamp_run_epoch(host: &mut WidgetHostNative, epoch: Option<u64>) {
    let Some(epoch) = epoch else {
        return;
    };
    let workspace = &mut host.editor_state_mut().editor_ui.workspace;
    if !workspace.active || workspace.run_epoch == epoch {
        return;
    }
    workspace.resume_generating(epoch);
    host.mark_editor_state_dirty();
}

/// The user stopped the run.
pub(crate) fn mark_run_stopped(host: &mut WidgetHostNative) -> bool {
    let workspace = &mut host.editor_state_mut().editor_ui.workspace;
    if !workspace.active {
        return false;
    }
    let epoch = workspace.run_epoch;
    workspace.mark_stopped(epoch)
}

/// Mobile errors land as `error: …` transcript text rather than failed
/// subtask rows, so they count as a failed run here too.
fn last_assistant_errored(state: &EditorState) -> bool {
    state
        .chat
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ChatRole::Assistant)
        .is_some_and(|message| message.content.trim_start().starts_with("error:"))
}

/// The run ended (finished, or never launched): settle the workspace
/// through the shared verdicts and the epoch fence.
pub(crate) fn settle_finished_run(host: &mut WidgetHostNative, viewport: (f32, f32)) -> bool {
    let state = host.editor_state();
    let workspace = &state.editor_ui.workspace;
    if !workspace.active || workspace.phase != WorkspacePhase::Generating {
        return false;
    }
    let epoch = workspace.run_epoch;
    let boards = workspace_run::produced_board_count(state);
    let failed = workspace_run::last_assistant_failed(state) || last_assistant_errored(state);
    host.settle_workspace_idle_edge(epoch, boards, failed, viewport.0, viewport.1)
}

/// Per-frame reader camera while a workspace is up.
pub(crate) fn pump_generation(
    host: &mut WidgetHostNative,
    generating: bool,
    viewport: (f32, f32),
) -> bool {
    host.pump_workspace_generation(viewport.0, viewport.1, generating)
}

/// Run one document-mutating step of a page-edit turn inside the fence:
/// every top-level board except `fence` is restored afterwards. Returns
/// the step's value and whether anything had to be reverted.
pub(crate) fn fenced<R>(
    state: &mut EditorState,
    fence: Option<&str>,
    step: impl FnOnce(&mut EditorState) -> R,
) -> (R, bool) {
    let Some(board) = fence else {
        return (step(state), false);
    };
    let before = state.active_children().to_vec();
    let value = step(state);
    let reverted = workspace_page_edit::restore_other_boards(state, &before, board);
    (value, reverted)
}

/// Tell the model its out-of-scope writes did not stick, keeping the
/// tool's own result for everything it did inside the board.
pub(crate) fn note_reverted(result: ChatToolResult, board: &str) -> ChatToolResult {
    ChatToolResult {
        content: serde_json::json!({
            "success": !result.is_error,
            "scope": format!(
                "This turn may only change board `{board}`. Changes to other top-level \
                 nodes were reverted; keep every edit inside that board."
            ),
            "result": result.content,
        })
        .to_string(),
        is_error: result.is_error,
    }
}

#[cfg(test)]
#[path = "editor_chat_workspace_tests.rs"]
mod tests;
