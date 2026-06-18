// UNVERIFIED: needs EMSDK wasm32 build + browser; run tools/check-wasm-bundle.sh
//! Web chat session — drives a real AI turn through the desktop daemon's
//! `/api/ai/stream` proxy instead of the offline echo stub.
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
//! Wire notes: the `/api/ai/stream` body carries `model` + a single `user`
//! message (`ai_proxy::parse_ai_stream_body`) — no history and no attachment
//! fields exist on the wire yet, so multi-turn context and staged attachments
//! are NOT forwarded (the daemon proxies single-turn requests; parity item
//! deferred until the wire grows those fields).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use op_editor_core::chat::{AgentProvider, ChatState, ModelEntry};
use op_editor_core::EditorState;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::repaint_ctx::RepaintContext;
use crate::web_ai_transport::{post_ai_stream, AiEvent, AiStreamHandle};

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
}

/// Abort and drop the in-flight turn, if any. The rAF pump notices the slot
/// change (generation mismatch) on its next frame and stops.
fn abort_active_turn() {
    let turn = ACTIVE_TURN.with(|slot| slot.borrow_mut().take());
    if let Some(turn) = turn {
        turn.handle.abort();
    }
}

/// Drain the chat host flags raised by the widget layer. Called from the DOM
/// listeners (mousedown / chat keydown) AFTER their `inner` borrow is
/// released. Order mirrors the desktop event loop: New Chat / Stop first
/// (abort the worker so stale deltas can't repopulate the transcript), then
/// launch a pending send.
pub(crate) fn drain_chat_flags<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let (new_chat, stop) = {
        let mut b = inner.borrow_mut();
        let chat = &mut b.host_mut().editor_state_mut().chat;
        (
            std::mem::take(&mut chat.pending_new_chat),
            std::mem::take(&mut chat.pending_stop_chat),
        )
    };
    if new_chat || stop {
        abort_active_turn();
    }
    let prepared = {
        let mut b = inner.borrow_mut();
        prepare_turn(&mut b.host_mut().editor_state_mut().chat)
    };
    if let Some(prepared) = prepared {
        launch_turn(inner, prepared);
    }
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
    match post_ai_stream(&base, prepared.body_json, on_event) {
        Ok(handle) => {
            ACTIVE_TURN.with(|slot| {
                *slot.borrow_mut() = Some(ActiveTurn { handle, generation });
            });
            start_pump(inner.clone(), queue, generation);
        }
        Err(_e) => {
            // Transport refused to even start (XHR open/send failed) —
            // surface that in the streaming bubble instead of hanging it.
            let mut b = inner.borrow_mut();
            let _ = apply_event_to_chat(
                &mut b.host_mut().editor_state_mut().chat,
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
        loop {
            // Pop under a tight borrow so the apply below never holds the
            // queue borrow across editor-state mutation.
            let evt = queue.borrow_mut().pop_front();
            let Some(evt) = evt else { break };
            let mut b = inner.borrow_mut();
            terminal |= apply_event_to_chat(&mut b.host_mut().editor_state_mut().chat, &evt);
            b.host_mut().mark_editor_state_dirty();
            changed = true;
        }
        if changed {
            // Repaint once per frame even when several events were queued.
            let _ = inner.borrow_mut().repaint();
        }
        if terminal {
            ACTIVE_TURN.with(|slot| {
                let mut s = slot.borrow_mut();
                if s.as_ref().is_some_and(|t| t.generation == generation) {
                    *s = None;
                }
            });
            return false;
        }
        true
    });
    crate::raf_pump::start(tick);
}

/// A prepared `/api/ai/stream` request body.
pub(crate) struct PreparedTurn {
    pub(crate) body_json: String,
}

/// Take `chat.pending_send` and build the request body: selected model wire
/// id (or `"default"` — the daemon then picks its configured provider) plus
/// the user message and the per-turn knobs. Pure, so the drain→body mapping
/// is unit-testable. Clears the staged attachments (begin_send already copied
/// the images into the user message; the wire has no attachment field yet).
pub(crate) fn prepare_turn(chat: &mut ChatState) -> Option<PreparedTurn> {
    let user_text = chat.pending_send.take()?;
    let model = chat
        .selected_model_entry()
        .map(|e| e.value.clone())
        .unwrap_or_else(|| "default".to_string());
    chat.pending_attachments.clear();
    let body = serde_json::json!({
        "model": model,
        // Chat turns carry no skill corpus (skills are the codegen
        // pipeline's per-phase prompts).
        "skills": [],
        "user": user_text,
        "max_output_tokens": 4096u32,
        "thinking": chat.thinking_mode.as_str(),
        "effort": chat.effort_level.as_str(),
    });
    Some(PreparedTurn {
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

// ---------------------------------------------------------------------
// Model discovery (`GET /api/ai/models`)
// ---------------------------------------------------------------------

/// Fetch the daemon's model catalog once (called from `mount()`) and populate
/// the chat model picker. Best-effort: a missing daemon / bad payload leaves
/// the catalog empty and sends fall back to the `"default"` model.
pub(crate) fn fetch_models<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let base = crate::daemon_base::daemon_base();
    let url = format!("{base}/api/ai/models");
    let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
        return;
    };
    if xhr.open_with_async("GET", &url, true).is_err() {
        return;
    }
    let xhr_cb = xhr.clone();
    let inner_cb = inner.clone();
    // `once_into_js` self-cleans after the single firing (live_sync idiom).
    let onloadend = Closure::<dyn FnMut()>::once_into_js(move || {
        let Ok(Some(text)) = xhr_cb.response_text() else {
            return;
        };
        let ids = parse_models_json(&text);
        if ids.is_empty() {
            return;
        }
        let mut b = inner_cb.borrow_mut();
        apply_models(b.host_mut().editor_state_mut(), &ids);
        b.host_mut().mark_editor_state_dirty();
        let _ = b.repaint();
    });
    xhr.set_onloadend(Some(onloadend.unchecked_ref()));
    let _ = xhr.send();
}

/// Parse the `GET /api/ai/models` body — a JSON array of model-id strings
/// (`ai_proxy::models_json`). Lenient: non-string entries and blanks are
/// skipped; a non-array payload yields the empty catalog.
pub(crate) fn parse_models_json(body: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Heuristic provider tag for a daemon model id — drives only the picker's
/// branding/grouping on web (the daemon resolves the real provider by id, so
/// a mismatch here cannot misroute a request).
pub(crate) fn provider_for_model_id(id: &str) -> AgentProvider {
    let lower = id.to_ascii_lowercase();
    if lower.contains("gemini") {
        AgentProvider::GeminiCli
    } else if lower.contains("gpt") || lower.contains("codex") {
        AgentProvider::CodexCli
    } else if lower.contains("copilot") {
        AgentProvider::GithubCopilot
    } else {
        // claude-* and anything unrecognized group under Claude.
        AgentProvider::ClaudeCode
    }
}

/// Install the daemon catalog into the chat model picker. Writes both
/// `discovered_models` and `available_models` directly — the web shell has no
/// connected-CLI mask (the daemon owns provider configuration), so the
/// desktop's connected-providers filter does not apply. The previously
/// selected model is preserved by identity when it survives, else selection
/// falls back to the first entry.
pub(crate) fn apply_models(state: &mut EditorState, ids: &[String]) {
    let entries: Vec<ModelEntry> = ids
        .iter()
        .map(|id| ModelEntry::new(provider_for_model_id(id), id.clone(), id.clone()))
        .collect();
    let prev = state
        .chat
        .available_models
        .get(state.chat.selected_model)
        .cloned();
    state.chat.discovered_models = entries.clone();
    state.chat.available_models = entries;
    state.chat.selected_model = prev
        .and_then(|p| {
            state
                .chat
                .available_models
                .iter()
                .position(|m| m.provider == p.provider && m.value == p.value)
        })
        .unwrap_or(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chat_with_queued_send(text: &str) -> ChatState {
        let mut chat = ChatState::default();
        chat.set_input_text(text);
        assert!(chat.begin_send());
        chat
    }

    #[test]
    fn prepare_turn_carries_model_and_message() {
        let mut chat = chat_with_queued_send("design a login page");
        chat.available_models = vec![ModelEntry::new(
            AgentProvider::ClaudeCode,
            "claude-sonnet-4-5",
            "Claude Sonnet 4.5",
        )];
        chat.selected_model = 0;
        let prepared = prepare_turn(&mut chat).expect("send was pending");
        let body: serde_json::Value =
            serde_json::from_str(&prepared.body_json).expect("body is JSON");
        assert_eq!(body["model"], "claude-sonnet-4-5");
        assert_eq!(body["user"], "design a login page");
        assert_eq!(body["max_output_tokens"], 4096);
        assert!(body["skills"].as_array().is_some_and(Vec::is_empty));
        // The drain consumed the flag — a second drain is idle.
        assert!(prepare_turn(&mut chat).is_none());
    }

    #[test]
    fn prepare_turn_defaults_model_when_catalog_empty() {
        let mut chat = chat_with_queued_send("hi");
        let prepared = prepare_turn(&mut chat).expect("send was pending");
        let body: serde_json::Value =
            serde_json::from_str(&prepared.body_json).expect("body is JSON");
        assert_eq!(body["model"], "default");
        assert_eq!(body["thinking"], chat.thinking_mode.as_str());
        assert_eq!(body["effort"], chat.effort_level.as_str());
    }

    #[test]
    fn prepare_turn_clears_staged_attachments() {
        let mut chat = ChatState::default();
        chat.set_input_text("look at this");
        chat.pending_attachments
            .push(op_editor_core::chat::ChatAttachment {
                name: "a.png".into(),
                media_type: "image/png".into(),
                data: vec![1, 2, 3],
            });
        assert!(chat.begin_send());
        let _ = prepare_turn(&mut chat).expect("send was pending");
        assert!(chat.pending_attachments.is_empty());
    }

    #[test]
    fn streamed_events_fold_into_streaming_bubble() {
        let mut chat = chat_with_queued_send("hello");
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
        assert_eq!(
            parse_models_json(r#"["claude-sonnet-4-5","gpt-5.5"]"#),
            vec!["claude-sonnet-4-5".to_string(), "gpt-5.5".to_string()]
        );
        assert_eq!(
            parse_models_json(r#"["", "  ", 3, "gemini-2.5-pro"]"#),
            vec!["gemini-2.5-pro".to_string()]
        );
        assert!(parse_models_json("{}").is_empty());
        assert!(parse_models_json("not json").is_empty());
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
        assert_eq!(
            state.chat.available_models[1].provider,
            AgentProvider::GeminiCli
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
            AgentProvider::GeminiCli
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
