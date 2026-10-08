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
        // Home can replace the transcript and queue its first send in one
        // gesture. Retire the OLD worker before launching that send; otherwise
        // the pointer/redraw tail consumes the reset and aborts the NEW run.
        let retired = self.drain_new_chat();
        let launched = chat_session::launch_if_pending(
            &mut self.host,
            &mut self.current_chat,
            &mut self.current_design,
        );
        if launched && (self.current_chat.is_some() || self.current_design.is_some()) {
            self.chat_running_tab = Some(self.host.editor_state().chat.active_index());
        }
        retired || launched
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
        let fresh_turn_tab = self
            .host
            .editor_state()
            .chat
            .pending_send
            .as_ref()
            .map(|_| self.host.editor_state().chat.active_index());
        let drained = chat_session::drain_new_chat_request(
            &mut self.host,
            &mut self.current_chat,
            &mut self.current_design,
        );
        if drained {
            crate::sub_agent_session::abort_all(&mut self.sub_agents, &mut self.active_sub_agent);
            if let Some(chat) = running_tab
                .filter(|idx| Some(*idx) != fresh_turn_tab)
                .and_then(|idx| self.host.editor_state_mut().chat.tab_mut(idx))
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
    fn a_home_send_retires_the_old_chat_before_launching_the_new_design() {
        let _guard = crate::agent_indicator_test_lock::LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut app = DesktopApp::new(None);
        app.host.editor_state_mut().editor_ui.home.visible = true;
        let doc = jian_ops_schema::load_str(r#"{"version":"1.0.0","children":[{"type":"frame","id":"old","width":375,"height":812,"children":[]}]}"#).unwrap().value;
        app.host
            .install_imported_state(op_editor_core::EditorState::from_document(doc));
        let state = app.host.editor_state_mut();
        state.editor_ui.agent_settings.builtin_agents.clear();
        let id = state.editor_ui.agent_settings.add_builtin_agent_config(
            "QA",
            "sk-test",
            "glm-5.3-flash",
            op_editor_core::BuiltinAgentKind::OpenAiCompat,
            "http://127.0.0.1:9/v1",
        );
        state.rebuild_chat_models();
        state.chat.selected_model = state
            .chat
            .available_models
            .iter()
            .position(|entry| entry.builtin_provider_id.as_deref() == Some(&id))
            .unwrap();
        state.editor_ui.home.visible = true;
        state.editor_ui.home.task = op_editor_core::HomeFamily::KnowledgeCards;
        state.editor_ui.home.variants_on = true;
        state.editor_ui.home.set_draft("做一张咖啡活动卡");
        let home = op_editor_ui::widgets::HomeSurface::for_editor(app.host.editor_state()).unwrap();
        let send = home.layout(1440.0, 900.0).send;
        assert!(app.host.apply_press(
            send.origin.x + send.size.x / 2.0,
            send.origin.y + send.size.y / 2.0,
            1440.0,
            900.0
        ));
        assert!(app.host.editor_state().chat.pending_new_chat);
        assert!(app.launch_chat_if_pending());
        assert!(
            !app.host.editor_state().chat.pending_new_chat,
            "the old-chat cleanup must be consumed BEFORE launch"
        );
        assert!(app.current_design.is_some());
        assert_eq!(app.chat_running_tab, Some(0));
        assert!(
            !app.drain_new_chat(),
            "the pointer/redraw tail must not abort the new run"
        );
        assert!(app.current_design.is_some());
        app.host.editor_state_mut().chat.pending_stop_chat = true;
        app.drain_stop_chat();
    }

    #[test]
    fn clearing_a_replaced_chat_keeps_its_already_queued_fresh_turn() {
        let mut app = DesktopApp::new(None);
        app.chat_running_tab = Some(0);
        let chat = &mut app.host.editor_state_mut().chat;
        chat.new_chat();
        chat.set_input_text("新作品的资料");
        assert!(chat.begin_send());
        let messages = chat.messages.clone();
        assert!(app.drain_new_chat());
        assert_eq!(
            app.host.editor_state().chat.pending_send.as_deref(),
            Some("新作品的资料")
        );
        assert_eq!(app.host.editor_state().chat.messages, messages);
        assert_eq!(app.chat_running_tab, None);
    }

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
