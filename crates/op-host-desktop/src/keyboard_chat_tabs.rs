//! MT.3 chat-tab run binding — launch / drain wrappers that keep
//! `chat_running_tab` in sync with the in-flight `current_chat` /
//! `current_design` sessions. Pure code motion out of
//! `keyboard_input.rs` to keep it under the 800-line cap.

use crate::{chat_session, design_session, DesktopApp};

impl DesktopApp {
    /// Launch a pending chat send and BIND the run to the tab it started on.
    ///
    /// `launch_if_pending` consumes `pending_send` and parks a `ChatSession` /
    /// `DesignSession` (or, on an honest-error path, neither). When a session
    /// actually starts, record the active tab as `chat_running_tab` so the
    /// pumps target it even after the user switches tabs. When nothing started
    /// (error bubble only), leave the binding untouched.
    pub(crate) fn launch_chat_if_pending(&mut self) -> bool {
        let launched = chat_session::launch_if_pending(
            &mut self.host,
            &mut self.current_chat,
            &mut self.current_design,
        );
        if launched && (self.current_chat.is_some() || self.current_design.is_some()) {
            self.chat_running_tab = Some(self.host.editor_state().chat.active_index());
        }
        launched
    }

    /// Drain a manual subtask-retry click and BIND the run to the tab it
    /// started on — same wrapper shape as [`launch_chat_if_pending`], for the
    /// failed-subtask remediation feature's manual layer. The retry always
    /// targets whichever tab is ACTIVE right now (the click can only happen
    /// on the tab being viewed), so it binds the SAME way a fresh chat send
    /// does.
    pub(crate) fn launch_subtask_retry_if_pending(&mut self) -> bool {
        let launched = design_session::launch_subtask_retry_if_pending(
            &mut self.host,
            &mut self.current_design,
        );
        if launched && self.current_design.is_some() {
            self.chat_running_tab = Some(self.host.editor_state().chat.active_index());
        }
        launched
    }

    /// Drain a New Chat request (the widget handler already opened the fresh
    /// tab). Aborts any in-flight worker and clears the now-stale tab binding.
    pub(crate) fn drain_new_chat(&mut self) -> bool {
        let running_tab = self.chat_running_tab;
        let drained = chat_session::drain_new_chat_request(
            &mut self.host,
            &mut self.current_chat,
            &mut self.current_design,
        );
        if drained {
            crate::sub_agent_session::abort_all(&mut self.sub_agents, &mut self.active_sub_agent);
            if let Some(chat) =
                running_tab.and_then(|idx| self.host.editor_state_mut().chat.tab_mut(idx))
            {
                chat.agents_running = (0, 0);
                chat.pending_send = None;
                chat.pending_stop_chat = false;
                for message in &mut chat.messages {
                    message.streaming = false;
                }
            }
            self.chat_running_tab = None;
        }
        drained
    }

    /// Drain a Stop request — aborts the in-flight worker and clears the tab
    /// binding so a later pump can't target a finished run.
    pub(crate) fn drain_stop_chat(&mut self) -> bool {
        let running_tab = self.chat_running_tab;
        let drained = chat_session::drain_stop_request(
            &mut self.host,
            &mut self.current_chat,
            &mut self.current_design,
            running_tab,
        );
        if drained {
            crate::sub_agent_session::abort_all(&mut self.sub_agents, &mut self.active_sub_agent);
            self.host.editor_state_mut().chat.agents_running = (0, 0);
            self.chat_running_tab = None;
        }
        drained
    }

    /// Close chat tab `idx` (MT.3 `AIChatHit::CloseTab`). When the closed tab
    /// is the one a run is bound to, abort the run FIRST (drop both sessions +
    /// clear the binding) so the pump never targets a removed / shifted tab.
    /// Otherwise the binding is shifted to follow the surviving tab.
    pub(crate) fn close_chat_tab(&mut self, idx: usize) {
        if idx >= self.host.editor_state().chat.tab_count() {
            return; // out of range — mirror ChatSessions::close_tab no-op
        }
        if self.chat_running_tab == Some(idx) {
            // The run's tab is going away — abort it before the index shifts.
            // Drop the chat / design sessions and any sub-agent loops bound to
            // this tab (ending their canvas-indicator epoch so no badge glow
            // gets stuck). The top-level design indicator self-heals next frame
            // (its teardown is gated on `current_chat.is_none()`).
            //
            // Finalize-lifecycle invariant (0718-1-k3-1 postmortem): closing
            // the tab a design loop is bound to must not discard an
            // unfinalized run — see
            // `chat_session::finalize_design_session_if_needed`'s doc comment.
            crate::chat_session::finalize_design_session_if_needed(
                &mut self.host,
                &self.current_chat,
                "teardown-backstop",
            );
            self.current_chat = None;
            self.current_design = None;
            crate::sub_agent_session::abort_all(&mut self.sub_agents, &mut self.active_sub_agent);
            self.chat_running_tab = None;
        } else if let Some(running) = self.chat_running_tab {
            self.chat_running_tab = op_editor_core::adjust_running_tab_after_close(running, idx);
        }
        self.host.editor_state_mut().chat.close_tab(idx);
        self.host.editor_state_mut().rebuild_chat_models();
        // Session set mutated (possibly same-index replacement): rotate the
        // transcript-cache owner so a pre-repaint cursor hint can't pair the
        // closed session's cached geometry with the survivor's messages.
        self.host.force_rotate_chat_owner();
        self.host.mark_editor_state_dirty();
    }

    /// Drain a pending close-tab request raised by the chat tab-row close-×
    /// (MT.3 `AIChatHit::CloseTab` → `editor_ui.pending_close_chat_tab`).
    /// Routes through [`Self::close_chat_tab`] so a run bound to the closed
    /// tab is aborted before the index shifts.
    pub(crate) fn drain_close_chat_tab(&mut self) -> bool {
        let Some(idx) = self
            .host
            .editor_state_mut()
            .editor_ui
            .pending_close_chat_tab
            .take()
        else {
            return false;
        };
        self.close_chat_tab(idx);
        true
    }

    /// Open a fresh chat tab (⌘T / Ctrl+T). Preserves all existing tabs and
    /// does NOT abort an in-flight run — the run keeps streaming into its own
    /// tab while the user composes in the new one.
    pub(crate) fn new_chat_tab(&mut self) {
        self.host.editor_state_mut().chat.new_tab();
        self.host.editor_state_mut().rebuild_chat_models();
        // New active session: rotate the transcript-cache owner so the
        // event-time cursor hint reads `None` until this tab's first paint.
        self.host.force_rotate_chat_owner();
        self.host.mark_editor_state_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_active_tab_reconciles_the_survivors_model_rows() {
        let mut app = DesktopApp::new(None);
        let state = app.host.editor_state_mut();
        state.editor_ui.agent_settings.builtin_agents.clear();
        state.rebuild_chat_models();
        let id = state.editor_ui.agent_settings.add_builtin_agent_config(
            "Provider",
            "sk-new",
            "current-model",
            op_editor_core::BuiltinAgentKind::OpenAiCompat,
            "https://example.test/v1",
        );
        state.chat.available_models = vec![op_editor_core::ModelEntry::builtin(
            op_editor_core::AgentProvider::CodexCli,
            id.clone(),
            format!("builtin:{id}:old-private-model"),
            "Old private model",
        )];
        state.chat.new_tab();
        assert_eq!(state.chat.active_index(), 1);

        app.close_chat_tab(1);

        let state = app.host.editor_state();
        assert_eq!(state.chat.active_index(), 0);
        assert!(state
            .chat
            .available_models
            .iter()
            .any(|entry| entry.builtin_model_id() == Some("current-model")));
        assert!(!state
            .chat
            .available_models
            .iter()
            .any(|entry| entry.builtin_model_id() == Some("old-private-model")));
    }
}
