//! Narrowed `EditorState` clones for off-thread request snapshots.
//!
//! Two hot paths hand a whole-editor snapshot to a worker thread:
//!
//!   - `op-host-services::mcp_live`'s `UiRequest::Snapshot` — the live
//!     MCP HTTP server asks the UI thread for a fresh state for *every*
//!     tool call, and an MCP agent issues many per turn.
//!   - `op-host-desktop::chat_session_launch` — the design turn's
//!     `RemoteDocSink` mirror (`op-editor-host-core::design`).
//!
//! Both need an **owned, mutable** `EditorState`: the MCP connection
//! thread re-applies the ack'd command to its local copy so a tool's
//! post-apply read is consistent, and the design worker mutates its
//! mirror. So neither can share an `Arc` with the live UI state, which
//! keeps mutating underneath them. What they *can* skip is the part of
//! the state their consumers never look at.
//!
//! ### Field audit (workspace grep over the consumer crates)
//!
//! | Field            | Read downstream? | In the snapshot |
//! | ---------------- | ---------------- | --------------- |
//! | `doc`            | yes (~63 sites)  | cloned          |
//! | `ui`             | yes (~13)        | cloned          |
//! | `selection`      | yes (~9)         | cloned          |
//! | `history`        | yes — `get_history_depth` reads `past/future.len()` | cloned |
//! | `editor_ui`      | yes — `document_save` reads `preserve_authored_geometry` | cloned, except collaboration session/profile state |
//! | `viewport`       | yes — `get_viewport` | cloned      |
//! | `ui_kits`        | yes — `batch_program` kit lookup | cloned |
//! | `components`     | yes — `tools.rs` resolved_root | cloned |
//! | `clipboard`      | yes — the `paste_clipboard` tool applies `EditorCommand::PasteClipboard` against it | cloned |
//! | `chat`           | **no**           | detached        |
//! | `codegen`        | **no**           | detached        |
//! | `theme_presets`  | **no**           | detached        |
//!
//! `chat` carries every tab's full transcript (design turns park large
//! JSON blocks in assistant messages), `codegen` carries generated
//! source text, and `theme_presets` carries a themes + variables table
//! per saved preset — none of which any snapshot consumer reads, and
//! all of which grow with session length rather than with the document.
//! Collaboration state is also detached: off-thread document consumers
//! neither need nor may retain authenticated participant profiles.
//!
//! No `EditorCommand` reads `chat` / `codegen` / `theme_presets`
//! either, so re-applying a command against the narrowed copy behaves
//! exactly as it does against a full clone.

use crate::state::EditorState;

/// Clone `state` for an off-thread consumer, leaving out the
/// session-scoped sub-states audited above.
///
/// Takes `&mut` because the cheap way to skip a field is to move it
/// out, clone, and move it straight back. The take/clone/restore
/// sequence is straight-line on the calling (UI) thread — no early
/// return, no reentrancy — so the live state is whole again before any
/// other code can observe it.
pub fn narrowed_snapshot(state: &mut EditorState) -> EditorState {
    let chat = std::mem::take(&mut state.chat);
    let codegen = std::mem::take(&mut state.codegen);
    let theme_presets = std::mem::take(&mut state.theme_presets);
    let collab = std::mem::take(&mut state.editor_ui.collab);

    let snapshot = state.clone();

    state.editor_ui.collab = collab;
    state.chat = chat;
    state.codegen = codegen;
    state.theme_presets = theme_presets;
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrowed_snapshot_restores_the_live_state() {
        let mut state = EditorState::new();
        state.chat.title = "long running turn".to_string();
        state.theme_presets.push(crate::theme_presets::ThemePreset {
            id: "p1".into(),
            name: "Preset".into(),
            themes: Default::default(),
            variables: Default::default(),
            created_at: 0,
        });
        state.editor_ui.collab.set_authenticated_session(
            crate::CollabConnectionPhase::Active,
            crate::AuthenticatedCollabSession {
                session_name: "private session".into(),
                role: crate::CollabUiRole::Editor,
                share_endpoint: None,
            },
            vec![crate::CollabParticipantUi::new(
                "private-participant",
                "Private Name",
                0x112233ff,
                crate::CollabUiRole::Editor,
                false,
            )],
        );

        let snapshot = narrowed_snapshot(&mut state);

        // The live state is untouched by the detour.
        assert_eq!(state.chat.title, "long running turn");
        assert_eq!(state.theme_presets.len(), 1);
        assert_eq!(state.editor_ui.collab.participants().len(), 1);
        // The snapshot keeps everything a consumer reads...
        assert_eq!(snapshot.doc.children.len(), state.doc.children.len());
        assert_eq!(snapshot.revision, state.revision);
        // ...and drops the session-scoped sub-states it does not.
        assert_ne!(snapshot.chat.title, "long running turn");
        assert!(snapshot.theme_presets.is_empty());
        assert!(snapshot.editor_ui.collab.participants().is_empty());
        assert!(snapshot.editor_ui.collab.authenticated_session().is_none());
    }
}
