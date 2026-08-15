// Browser/daemon boundary: chat streams through /api/ai/standard on the desktop
// daemon; this module keeps that web architecture instead of bypassing it.
//! Web chat session — drives a real AI turn through the desktop daemon's
//! `/api/ai/standard` route instead of the offline echo stub.
//!
//! Mirrors the desktop's `chat_session.rs` host-drain pattern with browser
//! plumbing borrowed from `codegen_web.rs`:
//!
//! * The widget layer raises the flags (`ChatState::begin_send` →
//!   `pending_send`; Stop / New Chat → `pending_stop_chat` /
//!   `pending_new_chat`). [`drain_chat_flags`] consumes them from the DOM
//!   listeners once the press/key borrow is released — the same drain points
//!   as `dom_io::drain_pending_file_action`.
//! * A send POSTs the selected model + user message over
//!   [`crate::web_ai_transport::post_ai_stream`]; streamed events land in a
//!   `VecDeque` queue and a `requestAnimationFrame` pump folds them into the
//!   trailing streaming assistant bubble each frame (the desktop winit loop
//!   pumps `ChatSession::poll` the same way at ~30 fps).
//! * Stop / New Chat / a replacing send abort the in-flight XHR via the
//!   transport's [`AiStreamHandle`]; a generation counter on the active-turn
//!   slot makes the old pump (and any late events) die silently.
//!
//! Wire notes: sends go through `/api/ai/standard` with the selected model,
//! current user message, trimmed prior text history, current-turn attachments,
//! and the current document / selection snapshot.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use base64::Engine as _;
use op_ai::chat_history::{trim_chat_history, DEFAULT_MAX_CHARS, DEFAULT_MAX_MESSAGES};
use op_editor_core::chat::ChatState;
use op_editor_core::EditorState;
use op_editor_host_core::chat::chat_history_from_transcript;

use crate::repaint_ctx::RepaintContext;
use crate::web_ai_transport::{post_ai_stream_to, AiEvent, AiStreamHandle};

#[cfg(test)]
use crate::web_model_catalog::{apply_models, parse_models_json, provider_for_model_id};
pub(crate) use crate::web_model_catalog::{fetch_models, reconcile_models};

type EventQueue = Rc<RefCell<VecDeque<AiEvent>>>;

/// The in-flight chat turn: the XHR abort handle plus its generation. wasm is
/// single-threaded, so a thread_local slot is the natural owner — every drain
/// site and the rAF pump reach the same instance.
struct ActiveTurn {
    handle: AiStreamHandle,
    generation: u64,
}

thread_local! {
    static ACTIVE_TURN: RefCell<Option<ActiveTurn>> = const { RefCell::new(None) };
    static NEXT_GENERATION: Cell<u64> = const { Cell::new(1) };
    /// Chat tab index the in-flight turn is bound to (MT.3 session-per-tab).
    /// The rAF pump writes streamed events into THIS tab even after the user
    /// switches the active tab, so a switch mid-stream never corrupts the
    /// now-active (wrong) tab. `None` when no turn is in flight or the binding
    /// is stale (the bound tab was closed → falls back to the active tab).
    static RUNNING_TAB: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Abort and drop the in-flight turn, if any. The rAF pump notices the slot
/// change (generation mismatch) on its next frame and stops. Also clears the
/// run's tab binding — the aborted turn no longer targets any tab.
fn abort_active_turn() {
    let turn = ACTIVE_TURN.with(|slot| slot.borrow_mut().take());
    RUNNING_TAB.with(|t| t.set(None));
    if let Some(turn) = turn {
        turn.handle.abort();
    }
}

/// The tab index the in-flight turn is currently bound to.
fn running_tab() -> Option<usize> {
    RUNNING_TAB.with(|t| t.get())
}

/// Drain the chat host flags raised by the widget layer. Called from the DOM
/// listeners (mousedown / chat keydown) AFTER their `inner` borrow is
/// released. Order mirrors the desktop event loop: New Chat / Stop first
/// (abort the worker so stale deltas can't repopulate the transcript), then
/// launch a pending send.
pub(crate) fn drain_chat_flags<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let (new_chat, stop) = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        let chat = &mut b.host_mut().editor_state_mut().chat;
        (
            std::mem::take(&mut chat.pending_new_chat),
            std::mem::take(&mut chat.pending_stop_chat),
        )
    };
    if new_chat || stop {
        abort_active_turn();
    }
    // A pending close-tab (MT.3): abort the run if it's bound to the closed
    // tab, shift the binding otherwise, then remove the tab. Done before the
    // launch below so a close + send in one drain pass resolves in order.
    {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        let pending_close = b
            .host_mut()
            .editor_state_mut()
            .editor_ui
            .pending_close_chat_tab
            .take();
        if let Some(idx) = pending_close {
            close_chat_tab(b.host_mut().editor_state_mut(), idx);
            // The close can install a different session at the SAME active index
            // (close active tab 0, or close the sole tab), which the index-only
            // poll would miss — force a transcript-cache owner rotation now so the
            // cursor-shape hint can't pair the removed tab's geometry with the
            // surviving session's messages before the next paint re-stamps.
            b.host_mut().force_rotate_chat_owner();
            b.host_mut().mark_editor_state_dirty();
        }
    }
    let prepared = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        let state = b.host_mut().editor_state_mut();
        // Bind a run to the tab it starts on BEFORE launching (the active tab
        // is the sending tab at this point).
        let active = state.chat.active_index();
        prepare_turn(state).inspect(|_| RUNNING_TAB.with(|t| t.set(Some(active))))
    };
    if let Some(prepared) = prepared {
        launch_turn(inner, prepared);
    }
}

/// Close chat tab `idx` (MT.3 `AIChatHit::CloseTab` web path). Aborts the
/// in-flight turn when it is bound to the closed tab (so the rAF pump can't
/// target a removed / shifted tab), shifts the binding when an earlier tab is
/// removed, then removes the tab.
fn close_chat_tab(state: &mut EditorState, idx: usize) {
    if idx >= state.chat.tab_count() {
        return; // out of range — mirror ChatSessions::close_tab no-op
    }
    match running_tab() {
        Some(running) if running == idx => abort_active_turn(),
        Some(running) => {
            RUNNING_TAB
                .with(|t| t.set(op_editor_core::adjust_running_tab_after_close(running, idx)));
        }
        None => {}
    }
    state.chat.close_tab(idx);
    state.rebuild_chat_models();
}

/// Launch one streaming turn: abort any in-flight one (a send fired mid-turn
/// replaces it — desktop parity), POST the prepared body, and start the rAF
/// pump that folds streamed events into the transcript.
fn launch_turn<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, prepared: PreparedTurn) {
    abort_active_turn();
    let generation = NEXT_GENERATION.with(|g| {
        let v = g.get();
        g.set(v + 1);
        v
    });
    let queue: EventQueue = Rc::new(RefCell::new(VecDeque::new()));
    let queue_cb = queue.clone();
    // The transport callback only queues — the pump applies on the next
    // frame, so no `inner` borrow is ever taken re-entrantly from XHR events.
    let on_event: Rc<dyn Fn(AiEvent)> = Rc::new(move |evt: AiEvent| {
        queue_cb.borrow_mut().push_back(evt);
    });
    let base = crate::daemon_base::daemon_base();
    match post_ai_stream_to(&base, prepared.endpoint, prepared.body_json, on_event) {
        Ok(handle) => {
            ACTIVE_TURN.with(|slot| {
                *slot.borrow_mut() = Some(ActiveTurn { handle, generation });
            });
            start_pump(inner.clone(), queue, generation);
        }
        Err(_e) => {
            // Transport refused to even start (XHR open/send failed) —
            // surface that in the streaming bubble instead of hanging it.
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            let target = b
                .host_mut()
                .editor_state_mut()
                .chat
                .run_tab_mut(running_tab());
            let _ = apply_event_to_chat(
                target,
                &AiEvent::Error("AI stream request failed to start".into()),
            );
            b.host_mut().mark_editor_state_dirty();
            let _ = b.repaint();
        }
    }
}

/// Start the rAF pump for one turn. Each frame drains the queued events into
/// the transcript and repaints; stops when the turn ends (terminal event
/// applied) or when this generation is no longer the active turn (Stop / New
/// Chat / replacing send).
fn start_pump<C: RepaintContext + 'static>(
    inner: Rc<RefCell<C>>,
    queue: EventQueue,
    generation: u64,
) {
    let tick: Rc<dyn Fn() -> bool> = Rc::new(move || {
        let still_active = ACTIVE_TURN.with(|slot| {
            slot.borrow()
                .as_ref()
                .is_some_and(|t| t.generation == generation)
        });
        if !still_active {
            return false; // aborted — drop queued late events silently
        }
        let mut terminal = false;
        let mut changed = false;
        let Ok(mut b) = inner.try_borrow_mut() else {
            return true;
        };
        loop {
            // Pop under a tight borrow so the apply below never holds the
            // queue borrow across editor-state mutation.
            let evt = queue.borrow_mut().pop_front();
            let Some(evt) = evt else { break };
            // Write into the tab this run is bound to (MT.3 session-per-tab),
            // not whichever tab is active now.
            let target = b
                .host_mut()
                .editor_state_mut()
                .chat
                .run_tab_mut(running_tab());
            terminal |= apply_event_to_chat(target, &evt);
            changed = true;
        }
        if changed {
            // Repaint once per frame even when several events were queued.
            b.host_mut().mark_editor_state_dirty();
            let _ = b.repaint();
        }
        drop(b);
        if terminal {
            let was_active = ACTIVE_TURN.with(|slot| {
                let mut s = slot.borrow_mut();
                if s.as_ref().is_some_and(|t| t.generation == generation) {
                    *s = None;
                    true
                } else {
                    false
                }
            });
            // This run finished — clear its tab binding (a fresh turn rebinds
            // at launch). Guarded on it still being THIS generation's turn so
            // a replacing send's binding isn't clobbered.
            if was_active {
                RUNNING_TAB.with(|t| t.set(None));
            }
            return false;
        }
        true
    });
    crate::raf_pump::start(tick);
}

/// A prepared AI SSE request.
pub(crate) struct PreparedTurn {
    pub(crate) endpoint: &'static str,
    pub(crate) body_json: String,
}

/// Take `chat.pending_send` and build the standard-turn request body:
/// selected model wire id (or `"default"` — the daemon then picks its
/// configured provider), the user message, per-turn knobs, plus a fresh
/// document/selection snapshot so daemon-side planning sees the same canvas
/// the browser is showing. Clears staged attachments after copying them into
/// the request; `begin_send` already copied image previews into the user
/// transcript bubble.
pub(crate) fn prepare_turn(state: &mut EditorState) -> Option<PreparedTurn> {
    let selected = state.chat.selected_model_entry().cloned();
    // Host CLI/ACP providers are intentionally unavailable on the browser
    // surface. Reconciliation normally makes this impossible, but reject a
    // transient stale CLI selection before consuming the pending send too.
    if selected
        .as_ref()
        .is_some_and(|entry| entry.builtin_provider_id.is_none())
    {
        return None;
    }
    let user_text = state.chat.pending_send.take()?;
    let (model, credential, builtin_provider_id) =
        crate::web_ai_credentials::selected_target(state);
    let provider = selected.as_ref().and_then(|entry| {
        // Legacy string catalogs use an unqualified model id and must stay on
        // the daemon's ambiguity-safe built-in resolver. Structured built-ins
        // and request-scoped browser credentials can carry exact identity.
        (builtin_provider_id.is_some() || credential.is_some()).then(|| entry.provider.wire_id())
    });
    let thinking = state.chat.thinking_mode.as_str();
    let effort = state.chat.effort_level.as_str();
    let agent_team_size = state.chat.agent_team_size;
    let history = trim_chat_history(
        &chat_history_from_transcript(&state.chat.messages),
        DEFAULT_MAX_MESSAGES,
        DEFAULT_MAX_CHARS,
    );
    let history_json: Vec<serde_json::Value> = history
        .iter()
        .map(|(role, content)| {
            serde_json::json!({
                "role": role.as_str(),
                "content": content,
            })
        })
        .collect();
    let attachments_json: Vec<serde_json::Value> = state
        .chat
        .pending_attachments
        .iter()
        .map(|att| {
            serde_json::json!({
                "name": att.name.as_str(),
                "media_type": att.media_type.as_str(),
                "data_base64": base64::engine::general_purpose::STANDARD.encode(&att.data),
            })
        })
        .collect();
    state.chat.pending_attachments.clear();
    let selected_ids: Vec<&str> = state.selection.set.iter().map(|id| id.as_str()).collect();
    let active_page_id = state
        .doc
        .pages
        .as_ref()
        .and_then(|pages| pages.get(state.ui.active_page_index))
        .map(|page| page.id.as_str());
    let body = serde_json::json!({
        "provider": provider,
        "builtinProviderId": builtin_provider_id,
        "model": model,
        "credential": credential,
        // Standard turns route through the daemon classifier + design
        // orchestrator; skills are resolved daemon-side per route.
        "skills": [],
        "user": user_text,
        "max_output_tokens": 4096u32,
        "thinking": thinking,
        "effort": effort,
        "agent_team_size": agent_team_size,
        "history": history_json,
        "attachments": attachments_json,
        "document": state.doc,
        "editorMeta": op_pen_loader::EditorMeta::from_state(state),
        "selectedIds": selected_ids,
        "activePageId": active_page_id,
    });
    Some(PreparedTurn {
        endpoint: "/api/ai/standard",
        body_json: serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
    })
}

/// Fold one streamed event into the trailing streaming assistant bubble.
/// Mirrors the desktop's `apply_poll_to_message`: an error replaces the
/// visible body, answer/thinking text accumulates, a terminal event clears
/// the `streaming` flag. Returns true when the event was terminal. A missing
/// streaming bubble (e.g. the user stopped the turn a frame earlier) drops
/// the event without touching the transcript.
pub(crate) fn apply_event_to_chat(chat: &mut ChatState, evt: &AiEvent) -> bool {
    let terminal = evt.is_terminal();
    let Some(msg) = chat.messages.iter_mut().rev().find(|m| m.streaming) else {
        return terminal;
    };
    match evt {
        AiEvent::AgentIdentity { name, color } => {
            msg.agent_name = Some(name.clone());
            msg.agent_color = Some(color.clone());
        }
        AiEvent::Delta(text) => msg.content.push_str(text),
        AiEvent::Thinking(text) => msg.thinking.push_str(text),
        AiEvent::Error(e) => {
            msg.content = format!("error: {e}");
            msg.streaming = false;
        }
        AiEvent::Done => msg.streaming = false,
    }
    terminal
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::chat::{AgentProvider, ModelEntry};

    fn state_with_queued_send(text: &str) -> EditorState {
        let mut state = EditorState::new();
        state.chat.set_input_text(text);
        assert!(state.chat.begin_send());
        state
    }

    // Returns the whole `ChatSessions` (the `.chat` field is the multi-tab
    // container since the multi-chat-tabs change); callers drive it through its
    // `DerefMut<Target = ChatState>` so `&mut chat` still coerces to the
    // `&mut ChatState` that `apply_event_to_chat` expects.
    fn chat_with_queued_send(text: &str) -> op_editor_core::ChatSessions {
        state_with_queued_send(text).chat
    }

    #[test]
    fn prepare_turn_carries_model_and_message() {
        let mut state = state_with_queued_send("design a login page");
        state.editor_ui.preserve_authored_geometry = true;
        state.chat.available_models = vec![ModelEntry::builtin_with_display_name(
            AgentProvider::ClaudeCode,
            "daemon-builtin:server-1",
            "Server API Key",
            "builtin:server-1:claude-sonnet-4-5",
            "Claude Sonnet 4.5",
        )];
        state.chat.selected_model = 0;
        state.chat.agent_team_size = 3;
        let prepared = prepare_turn(&mut state).expect("send was pending");
        assert_eq!(prepared.endpoint, "/api/ai/standard");
        let body: serde_json::Value =
            serde_json::from_str(&prepared.body_json).expect("body is JSON");
        assert_eq!(body["provider"], "claude-code");
        assert_eq!(body["model"], "builtin:server-1:claude-sonnet-4-5");
        assert_eq!(body["user"], "design a login page");
        assert_eq!(body["max_output_tokens"], 4096);
        assert_eq!(body["agent_team_size"], 3);
        assert!(body["document"].is_object());
        assert_eq!(body["editorMeta"]["activePageIndex"], 0);
        assert_eq!(body["editorMeta"]["preserveAuthoredGeometry"], true);
        assert!(body["skills"].as_array().is_some_and(Vec::is_empty));
        // The drain consumed the flag — a second drain is idle.
        assert!(prepare_turn(&mut state).is_none());
    }

    #[test]
    fn prepare_turn_carries_prior_history_without_current_turn() {
        let mut state = EditorState::new();
        state
            .chat
            .messages
            .push(op_editor_core::ChatMessage::user("previous request"));
        state
            .chat
            .messages
            .push(op_editor_core::ChatMessage::assistant("previous answer"));
        state.chat.set_input_text("current request");
        assert!(state.chat.begin_send());

        let prepared = prepare_turn(&mut state).expect("send was pending");
        let body: serde_json::Value =
            serde_json::from_str(&prepared.body_json).expect("body is JSON");
        let history = body["history"].as_array().expect("history array");

        assert_eq!(history.len(), 2, "{history:?}");
        assert_eq!(history[0]["role"], "user");
        assert_eq!(history[0]["content"], "previous request");
        assert_eq!(history[1]["role"], "assistant");
        assert_eq!(history[1]["content"], "previous answer");
        assert!(
            !history
                .iter()
                .any(|item| item["content"] == "current request"),
            "current user message rides `user`, not history: {history:?}"
        );
    }

    #[test]
    fn prepare_turn_defaults_model_when_catalog_empty() {
        let mut state = state_with_queued_send("hi");
        let thinking = state.chat.thinking_mode;
        let effort = state.chat.effort_level;
        let prepared = prepare_turn(&mut state).expect("send was pending");
        let body: serde_json::Value =
            serde_json::from_str(&prepared.body_json).expect("body is JSON");
        assert!(body["provider"].is_null());
        assert_eq!(body["model"], "default");
        assert_eq!(body["thinking"], thinking.as_str());
        assert_eq!(body["effort"], effort.as_str());
    }

    #[test]
    fn prepare_turn_clears_staged_attachments() {
        let mut state = EditorState::new();
        state.chat.set_input_text("look at this");
        state
            .chat
            .pending_attachments
            .push(op_editor_core::chat::ChatAttachment {
                name: "a.png".into(),
                media_type: "image/png".into(),
                data: vec![1, 2, 3],
            });
        assert!(state.chat.begin_send());
        let _ = prepare_turn(&mut state).expect("send was pending");
        assert!(state.chat.pending_attachments.is_empty());
    }

    #[test]
    fn prepare_turn_carries_staged_attachments_on_the_wire() {
        let mut state = EditorState::new();
        state.chat.set_input_text("look at this");
        state
            .chat
            .pending_attachments
            .push(op_editor_core::chat::ChatAttachment {
                name: "a.png".into(),
                media_type: "image/png".into(),
                data: vec![1, 2, 3],
            });
        assert!(state.chat.begin_send());

        let prepared = prepare_turn(&mut state).expect("send was pending");
        let body: serde_json::Value =
            serde_json::from_str(&prepared.body_json).expect("body is JSON");
        let attachments = body["attachments"].as_array().expect("attachments array");

        assert_eq!(attachments.len(), 1, "{attachments:?}");
        assert_eq!(attachments[0]["name"], "a.png");
        assert_eq!(attachments[0]["media_type"], "image/png");
        assert_eq!(attachments[0]["data_base64"], "AQID");
        assert!(state.chat.pending_attachments.is_empty());
    }

    #[test]
    fn streamed_events_fold_into_streaming_bubble() {
        let mut chat = chat_with_queued_send("hello");
        assert!(!apply_event_to_chat(
            &mut chat,
            &AiEvent::AgentIdentity {
                name: "Mochi".into(),
                color: "#4ECDC4".into(),
            }
        ));
        assert!(!apply_event_to_chat(
            &mut chat,
            &AiEvent::Delta("Hi ".into())
        ));
        assert!(!apply_event_to_chat(
            &mut chat,
            &AiEvent::Thinking("consider…".into())
        ));
        assert!(!apply_event_to_chat(
            &mut chat,
            &AiEvent::Delta("there".into())
        ));
        assert!(apply_event_to_chat(&mut chat, &AiEvent::Done));
        let msg = chat.messages.last().expect("assistant bubble");
        assert_eq!(msg.content, "Hi there");
        assert_eq!(msg.thinking, "consider…");
        assert_eq!(msg.agent_name.as_deref(), Some("Mochi"));
        assert_eq!(msg.agent_color.as_deref(), Some("#4ECDC4"));
        assert!(!msg.streaming, "Done clears the streaming flag");
    }

    #[test]
    fn error_replaces_bubble_body_and_ends_turn() {
        let mut chat = chat_with_queued_send("hello");
        let _ = apply_event_to_chat(&mut chat, &AiEvent::Delta("partial".into()));
        assert!(apply_event_to_chat(
            &mut chat,
            &AiEvent::Error("no model configured".into())
        ));
        let msg = chat.messages.last().expect("assistant bubble");
        assert_eq!(msg.content, "error: no model configured");
        assert!(!msg.streaming);
    }

    #[test]
    fn late_events_without_streaming_bubble_are_dropped() {
        let mut chat = chat_with_queued_send("hello");
        assert!(chat.stop_streaming());
        let before = chat.messages.clone();
        assert!(!apply_event_to_chat(
            &mut chat,
            &AiEvent::Delta("late".into())
        ));
        assert!(apply_event_to_chat(&mut chat, &AiEvent::Done));
        assert_eq!(chat.messages, before, "stopped transcript is untouched");
    }
    #[test]
    fn parse_models_json_reads_string_arrays_leniently() {
        let models = parse_models_json(r#"["claude-sonnet-4-5","gpt-5.5"]"#);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].value, "claude-sonnet-4-5");
        assert_eq!(models[1].value, "gpt-5.5");
        let models = parse_models_json(r#"["", "  ", 3, "gemini-2.5-pro"]"#);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].value, "gemini-2.5-pro");
        assert!(parse_models_json("{}").is_empty());
        assert!(parse_models_json("not json").is_empty());
    }

    #[test]
    fn structured_catalog_keeps_builtin_identity_and_drops_cli_rows() {
        let models = parse_models_json(
            r#"[
                {"provider":"codex-cli","value":"builtin:server-1:gpt-5.4","displayName":"GPT-5.4","providerDisplayName":"Server OpenAI","builtinProviderId":"server-1"},
                {"provider":"grok-build","value":"default","displayName":"Default"}
            ]"#,
        );
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].provider, AgentProvider::CodexCli);
        assert_eq!(models[0].value, "builtin:server-1:gpt-5.4");
        assert_eq!(
            models[0].builtin_provider_id.as_deref(),
            Some("daemon-builtin:server-1")
        );
    }

    #[test]
    fn structured_catalog_keeps_equal_models_and_colon_provider_ids_distinct() {
        let models = parse_models_json(
            r#"[
                {"provider":"codex-cli","value":"builtin:account:one:shared:model","displayName":"shared:model","providerDisplayName":"First","builtinProviderId":"account:one"},
                {"provider":"codex-cli","value":"builtin:account:two:shared:model","displayName":"shared:model","providerDisplayName":"Second","builtinProviderId":"account:two"}
            ]"#,
        );

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].value, "builtin:account:one:shared:model");
        assert_eq!(models[1].value, "builtin:account:two:shared:model");
        assert_eq!(
            models[0].builtin_provider_id.as_deref(),
            Some("daemon-builtin:account:one")
        );
        assert_eq!(
            models[1].builtin_provider_id.as_deref(),
            Some("daemon-builtin:account:two")
        );
    }

    #[test]
    fn apply_models_populates_picker_and_preserves_selection() {
        let mut state = EditorState::new();
        let ids = vec![
            "claude-sonnet-4-5".to_string(),
            "gemini-2.5-pro".to_string(),
        ];
        apply_models(&mut state, &ids);
        assert_eq!(state.chat.available_models.len(), 2);
        assert_eq!(state.chat.discovered_models.len(), 2);
        assert_eq!(state.chat.selected_model, 0);
        assert!(state
            .chat
            .available_models
            .iter()
            .all(|entry| entry.builtin_provider_id.is_some()));
        assert_eq!(
            state.chat.available_models[1].provider,
            AgentProvider::CodexCli
        );
        // Select the second model, then re-apply a re-ordered catalog — the
        // selection follows the entry by identity.
        state.chat.selected_model = 1;
        let ids2 = vec![
            "gemini-2.5-pro".to_string(),
            "claude-sonnet-4-5".to_string(),
        ];
        apply_models(&mut state, &ids2);
        assert_eq!(state.chat.selected_model, 0);
        assert_eq!(
            state.chat.selected_model_entry().map(|m| m.value.as_str()),
            Some("gemini-2.5-pro")
        );
    }

    #[test]
    fn provider_heuristic_groups_known_id_shapes() {
        assert_eq!(
            provider_for_model_id("claude-sonnet-4-5"),
            AgentProvider::ClaudeCode
        );
        assert_eq!(provider_for_model_id("gpt-5.5"), AgentProvider::CodexCli);
        assert_eq!(
            provider_for_model_id("gemini-2.5-pro"),
            AgentProvider::CodexCli
        );
        assert_eq!(
            provider_for_model_id("copilot-fast"),
            AgentProvider::GithubCopilot
        );
        assert_eq!(
            provider_for_model_id("minimax-m2"),
            AgentProvider::ClaudeCode
        );
    }
}

#[cfg(test)]
#[path = "web_chat_credential_tests.rs"]
mod credential_tests;
