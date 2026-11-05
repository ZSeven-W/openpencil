//! GUI state and transitions for canvas comments.
//!
//! Both hosts drive the same flow: the toolbar toggles comment mode, a
//! canvas press in comment mode opens a draft pinned to the node (or
//! point) under the cursor, typing fills the popover input, Enter posts,
//! and the popover's buttons change status or delete the thread. Every
//! document change goes through [`EditorCommand::Comment`], gated by the
//! collaboration policy and recorded in the undo history like any other
//! edit.
//!
//! Wasm32-clean: geometry is plain `[x, y, w, h]` arrays so the widget
//! layer can cache its hit regions here without a widget dependency.

use crate::comments::{
    CommentAnchor, CommentAuthor, CommentAuthorKind, CommentCommand, CommentLookup, CommentStatus,
};
use crate::{CollabEditSource, CollabGateReason, EditorCommand, EditorState};
use jian_core::text_input::TextInputState;

/// What the comment popover is showing.
#[derive(Debug, Clone, PartialEq)]
pub enum CommentPopover {
    /// A new thread being composed at `anchor`; nothing is in the
    /// document until it is posted.
    Draft { anchor: CommentAnchor },
    /// An existing thread.
    Thread { id: String },
}

/// A clickable region of the popover.
#[derive(Debug, Clone, PartialEq)]
pub enum CommentPopoverHit {
    Close,
    Delete,
    Input,
    Submit,
    SetStatus(CommentStatus),
    /// Inside the card but on no control — swallowed.
    Body,
}

/// Comment UI state on `EditorUiState`. Transient: never serialized.
#[derive(Debug, Clone, Default)]
pub struct CommentUiState {
    /// Comment mode is armed (toolbar toggle): canvas presses place pins.
    pub mode: bool,
    /// The open popover, if any.
    pub open: Option<CommentPopover>,
    /// The popover's text input (new comment or reply).
    pub input: TextInputState,
    /// Popover hover target — drives the button wash.
    pub hover: Option<CommentPopoverHit>,
    /// Card rect from the last paint (`[x, y, w, h]`). Popover height
    /// depends on wrapped message text, which only paint can measure, so
    /// the press path reads the regions paint cached here.
    pub popover_rect: Option<[f32; 4]>,
    /// Control regions from the last paint, top-most last.
    pub popover_hits: Vec<(CommentPopoverHit, [f32; 4])>,
}

impl CommentUiState {
    /// Whether the popover input owns the keyboard.
    pub fn owns_keyboard(&self) -> bool {
        self.open.is_some()
    }

    /// Resolve a press against the regions cached by the last paint.
    pub fn popover_hit(&self, x: f32, y: f32) -> Option<CommentPopoverHit> {
        let inside = |r: &[f32; 4]| x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3];
        let card = self.popover_rect.filter(|_| self.open.is_some())?;
        if !inside(&card) {
            return None;
        }
        Some(
            self.popover_hits
                .iter()
                .rev()
                .find(|(_, r)| inside(r))
                .map(|(hit, _)| hit.clone())
                .unwrap_or(CommentPopoverHit::Body),
        )
    }

    fn close(&mut self) {
        self.open = None;
        self.input = TextInputState::default();
        self.hover = None;
        self.popover_rect = None;
        self.popover_hits.clear();
    }
}

/// Toggle comment mode. Leaving it drops an unposted draft.
pub fn toggle_comment_mode(state: &mut EditorState) {
    let ui = &mut state.editor_ui.comments;
    ui.mode = !ui.mode;
    if !ui.mode && matches!(ui.open, Some(CommentPopover::Draft { .. })) {
        ui.close();
    }
}

/// Open a draft at `anchor`, replacing whatever the popover showed.
pub fn begin_comment_draft(state: &mut EditorState, anchor: CommentAnchor) {
    let ui = &mut state.editor_ui.comments;
    ui.close();
    ui.open = Some(CommentPopover::Draft { anchor });
}

/// Open an existing thread.
pub fn open_comment_thread(state: &mut EditorState, id: &str) {
    let ui = &mut state.editor_ui.comments;
    if matches!(&ui.open, Some(CommentPopover::Thread { id: open }) if open == id) {
        return;
    }
    ui.close();
    ui.open = Some(CommentPopover::Thread { id: id.to_owned() });
}

/// Close the popover. `true` when something was open.
pub fn close_comment_popover(state: &mut EditorState) -> bool {
    let was_open = state.editor_ui.comments.open.is_some();
    state.editor_ui.comments.close();
    was_open
}

/// Escape: close the popover first, then leave comment mode.
pub fn comment_escape(state: &mut EditorState) -> bool {
    if close_comment_popover(state) {
        return true;
    }
    if state.editor_ui.comments.mode {
        state.editor_ui.comments.mode = false;
        return true;
    }
    false
}

/// Typed character into the popover input. `None` when it is closed.
pub fn comment_text(state: &mut EditorState, c: char, now_ms: u64) -> Option<bool> {
    let ui = &mut state.editor_ui.comments;
    if !ui.owns_keyboard() {
        return None;
    }
    if c.is_control() {
        return Some(false);
    }
    let mut buf = [0u8; 4];
    ui.input.insert_str(c.encode_utf8(&mut buf), now_ms);
    Some(true)
}

/// Pasted / IME-committed text into the popover input.
pub fn comment_insert_str(state: &mut EditorState, text: &str, now_ms: u64) -> Option<bool> {
    let ui = &mut state.editor_ui.comments;
    if !ui.owns_keyboard() {
        return None;
    }
    let clean: String = text.chars().filter(|c| !c.is_control()).collect();
    if clean.is_empty() {
        return Some(false);
    }
    ui.input.insert_str(&clean, now_ms);
    Some(true)
}

/// Backspace in the popover input. `None` when it is closed.
pub fn comment_backspace(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    let ui = &mut state.editor_ui.comments;
    if !ui.owns_keyboard() {
        return None;
    }
    let before = ui.input.text().len();
    ui.input.backspace(now_ms);
    Some(ui.input.text().len() != before)
}

/// Forward-Delete in the popover input. `None` when it is closed.
pub fn comment_delete_forward(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    let ui = &mut state.editor_ui.comments;
    if !ui.owns_keyboard() {
        return None;
    }
    let before = ui.input.text().len();
    ui.input.delete_forward(now_ms);
    Some(ui.input.text().len() != before)
}

/// Enter / the Post button: post the draft or the reply.
///
/// `None` when the popover is closed; `Some(Ok(changed))` otherwise, or
/// the collaboration policy's refusal.
pub fn comment_submit(
    state: &mut EditorState,
    now_ms: u64,
) -> Option<Result<bool, CollabGateReason>> {
    let open = state.editor_ui.comments.open.clone()?;
    let text = state.editor_ui.comments.input.text().trim().to_owned();
    if text.is_empty() {
        return Some(Ok(false));
    }
    let author = user_author(state);
    let (cmd, opened) = match open {
        CommentPopover::Draft { anchor } => {
            let id = state.next_comment_id(now_ms);
            (
                CommentCommand::Add {
                    id: id.clone(),
                    anchor,
                    author,
                    text,
                    at_ms: now_ms,
                },
                Some(id),
            )
        }
        CommentPopover::Thread { id } => (
            CommentCommand::Reply {
                comment_id: id,
                author,
                text,
                at_ms: now_ms,
            },
            None,
        ),
    };
    let result = apply_comment_ui_command(state, cmd);
    if matches!(result, Ok(true)) {
        state.editor_ui.comments.input = TextInputState::default();
        if let Some(id) = opened {
            state.editor_ui.comments.open = Some(CommentPopover::Thread { id });
        }
    }
    Some(result)
}

/// Enter from a host keyboard path: post, surfacing a collaboration
/// refusal as the session notice. `None` when the popover is closed.
pub fn comment_enter(state: &mut EditorState, now_ms: u64) -> Option<bool> {
    let result = comment_submit(state, now_ms)?;
    Some(settle(state, result, now_ms))
}

/// Resolve a GUI comment edit: `Ok(changed)` passes through; a refusal
/// becomes the collaboration notice (and a repaint).
pub fn settle(
    state: &mut EditorState,
    result: Result<bool, CollabGateReason>,
    now_ms: u64,
) -> bool {
    match result {
        Ok(changed) => changed,
        Err(reason) => {
            state
                .editor_ui
                .collab
                .set_notice(reason.notice_kind(), now_ms);
            true
        }
    }
}

/// Change the open thread's status from a popover button.
pub fn set_open_comment_status(
    state: &mut EditorState,
    status: CommentStatus,
    now_ms: u64,
) -> Result<bool, CollabGateReason> {
    let Some(CommentPopover::Thread { id }) = state.editor_ui.comments.open.clone() else {
        return Ok(false);
    };
    let result = apply_comment_ui_command(
        state,
        CommentCommand::SetStatus {
            comment_id: id,
            status,
            at_ms: now_ms,
        },
    );
    // An archived thread's pin disappears, so its popover goes too.
    if matches!(result, Ok(true)) && status == CommentStatus::Archived {
        state.editor_ui.comments.close();
    }
    result
}

/// Delete the open thread (or discard the draft).
pub fn delete_open_comment(state: &mut EditorState) -> Result<bool, CollabGateReason> {
    match state.editor_ui.comments.open.clone() {
        Some(CommentPopover::Thread { id }) => {
            let result = apply_comment_ui_command(state, CommentCommand::Delete { comment_id: id });
            if matches!(result, Ok(true)) {
                state.editor_ui.comments.close();
            }
            result
        }
        Some(CommentPopover::Draft { .. }) => Ok(close_comment_popover(state)),
        None => Ok(false),
    }
}

/// Apply one comment command from the GUI: policy check, validation,
/// one undo step, then apply. `Ok(false)` when validation refuses it.
pub fn apply_comment_ui_command(
    state: &mut EditorState,
    cmd: CommentCommand,
) -> Result<bool, CollabGateReason> {
    let command = EditorCommand::Comment(cmd);
    state
        .editor_ui
        .collab
        .gate(command.collab_gate_action(), CollabEditSource::User)?;
    let EditorCommand::Comment(cmd) = &command else {
        return Ok(false);
    };
    if state.check_comment_command(cmd).is_err() {
        return Ok(false);
    }
    state.commit_history();
    Ok(state.apply(command))
}

/// The signed-in display name, or an empty name the UI shows as "You".
fn user_author(state: &EditorState) -> CommentAuthor {
    let name = match &state.editor_ui.account {
        crate::account_state::AccountState::SignedIn { display_name, .. } => display_name.clone(),
        crate::account_state::AccountState::Anonymous => String::new(),
    };
    CommentAuthor {
        kind: CommentAuthorKind::User,
        name,
    }
}

/// Anchor for a press at `doc_point` on the active page: pinned to
/// `node` (with the offset from its top-left) when one is under the
/// cursor, else to the point itself.
pub fn comment_anchor_at(
    state: &EditorState,
    node: Option<(&str, [f32; 2])>,
    doc_point: [f32; 2],
) -> CommentAnchor {
    let page_id = state
        .doc
        .pages
        .as_ref()
        .and_then(|pages| pages.get(state.ui.active_page_index))
        .map(|page| page.id.clone());
    match node {
        Some((id, origin)) => CommentAnchor {
            page_id,
            node_id: Some(id.to_owned()),
            x: Some(f64::from(doc_point[0] - origin[0])),
            y: Some(f64::from(doc_point[1] - origin[1])),
        },
        None => CommentAnchor {
            page_id,
            node_id: None,
            x: Some(f64::from(doc_point[0])),
            y: Some(f64::from(doc_point[1])),
        },
    }
}

#[cfg(test)]
#[path = "comment_ui_tests.rs"]
mod tests;
