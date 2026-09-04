//! Render-free mobile runtime pump used while the platform surface is suspended.
//!
//! A design turn is not just a network stream: tool calls must execute on the
//! engine owner thread before the worker can continue. Mobile display pumps stop
//! when their surface is backgrounded, so platform shells drive this narrow pump
//! from their user-visible background task instead.

use crate::error::{FfiError, FfiResult};
use crate::lifecycle::{call_session, Session};
use crate::{OpEngine, OpStatus};

impl Session {
    /// Whether a user-started generation still needs the owner-thread pump.
    pub(crate) fn has_background_work(&self) -> bool {
        #[cfg(feature = "editor")]
        {
            self.editor.as_ref().is_some_and(|host| {
                self.chat.has_background_work(host)
                    || self.codegen.has_background_work(host)
                    || self.image_search.has_background_work()
            })
        }
        #[cfg(not(feature = "editor"))]
        {
            false
        }
    }

    /// Advance generation without touching a Metal/EGL surface.
    pub(crate) fn pump_background_work(&mut self, now_ms: u64) -> FfiResult<bool> {
        self.advance_global_clock(now_ms);
        #[cfg(feature = "editor")]
        {
            let revision_before = self
                .editor
                .as_ref()
                .map(|host| host.editor_state().document_revision());
            self.pump_editor_chat(now_ms);
            self.pump_editor_codegen(now_ms);
            self.pump_editor_image_search(now_ms);

            let revision_after = self
                .editor
                .as_ref()
                .map(|host| host.editor_state().document_revision());
            // `suspend()` flushed the pre-background state, but design tools can
            // keep mutating afterwards. Refresh any reachable bound file/shadow
            // after each content revision. Never-saved documents keep the
            // existing `flush_on_suspend` policy and are not invented here.
            if self.suspended && revision_before != revision_after {
                crate::editor_document::flush_on_suspend(self);
            }
        }
        Ok(self.has_background_work())
    }

    /// Cancel the user-started generation represented by this mobile session.
    ///
    /// The platform uses this when its user-visible background task is
    /// cancelled or expires. It runs on the same owner thread as the normal
    /// pump so design-loop finalization and indicator teardown remain ordered.
    pub(crate) fn cancel_background_work(&mut self) {
        #[cfg(feature = "editor")]
        {
            let revision_before = self
                .editor
                .as_ref()
                .map(|host| host.editor_state().document_revision());
            if let Some(host) = self.editor.as_mut() {
                if self.chat.has_background_work(host) {
                    let chat = &mut host.editor_state_mut().chat;
                    let _ = chat.stop_streaming();
                    // Also cover the narrow interval after the visible stream
                    // closed but before the host retired its worker.
                    chat.pending_stop_chat = true;
                }
                self.codegen.cancel_background_work(host);
            }
            let _ = self.pump_editor_chat(self.now_ms);
            self.image_search.cancel_background_work();

            let revision_after = self
                .editor
                .as_ref()
                .map(|host| host.editor_state().document_revision());
            if self.suspended && revision_before != revision_after {
                crate::editor_document::flush_on_suspend(self);
            }
        }
    }
}

/// Read whether a user-started generation needs background execution.
///
/// # Safety
///
/// `engine` must be live on its owner thread and `active` must be writable.
#[no_mangle]
pub unsafe extern "C" fn op_has_background_work(
    engine: *mut OpEngine,
    active: *mut bool,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if active.is_null() {
                return Err(FfiError::invalid("background-work output pointer is null"));
            }
            active.write(session.has_background_work());
            Ok(())
        })
    }
}

/// Advance generation while no drawable surface is available and return
/// whether more background work remains.
///
/// # Safety
///
/// `engine` must be live on its owner thread and `active` must be writable.
#[no_mangle]
pub unsafe extern "C" fn op_background_tick(
    engine: *mut OpEngine,
    now_ms: u64,
    active: *mut bool,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if active.is_null() {
                return Err(FfiError::invalid("background-work output pointer is null"));
            }
            active.write(session.pump_background_work(now_ms)?);
            Ok(())
        })
    }
}

/// Cancel the current mobile generation and retire its owner-thread work.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_cancel_background_work(engine: *mut OpEngine) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            session.cancel_background_work();
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desc::{Callbacks, CreateOptions};
    #[cfg(feature = "editor")]
    use op_editor_core::{codegen::CodegenPhase, BuiltinAgentKind, ChatRole, PenNodeExt};
    #[cfg(feature = "editor")]
    use std::io::{Read, Write};
    #[cfg(feature = "editor")]
    use std::net::TcpListener;
    #[cfg(feature = "editor")]
    use std::time::{Duration, Instant};

    const SAMPLE_DOC: &str =
        include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

    fn engine() -> OpEngine {
        OpEngine::new(
            Session::new(CreateOptions {
                document: SAMPLE_DOC.to_owned(),
                width: 800.0,
                height: 600.0,
                dpr: 1.0,
                callbacks: Callbacks::default(),
                asset_base: None,
                #[cfg(feature = "editor")]
                editor_mode: true,
                #[cfg(feature = "editor")]
                documents_root: None,
            })
            .expect("engine session"),
        )
    }

    #[test]
    fn null_outputs_are_rejected() {
        let mut engine = engine();
        let engine = &mut engine as *mut OpEngine;
        assert_eq!(
            unsafe { op_has_background_work(engine, std::ptr::null_mut()) },
            OpStatus::InvalidArg
        );
        assert_eq!(
            unsafe { op_background_tick(engine, 1, std::ptr::null_mut()) },
            OpStatus::InvalidArg
        );
    }

    #[test]
    fn idle_engine_reports_no_background_work() {
        let mut engine = engine();
        let engine = &mut engine as *mut OpEngine;
        let mut active = true;
        assert_eq!(
            unsafe { op_has_background_work(engine, &mut active) },
            OpStatus::Ok
        );
        assert!(!active);
        assert_eq!(
            unsafe { op_background_tick(engine, 1, &mut active) },
            OpStatus::Ok
        );
        assert!(!active);
    }

    #[cfg(feature = "editor")]
    #[test]
    fn a_queued_send_is_background_work_before_its_first_render_frame() {
        let mut engine = engine();
        let session = engine.session_mut_for_test();
        session
            .editor
            .as_mut()
            .expect("editor host")
            .editor_state_mut()
            .chat
            .pending_send = Some("design a settings screen".into());
        assert!(session.has_background_work());
    }

    #[cfg(feature = "editor")]
    #[test]
    fn cancelling_a_queued_send_retires_background_work() {
        let mut engine = engine();
        let session = engine.session_mut_for_test();
        let chat = &mut session
            .editor
            .as_mut()
            .expect("editor host")
            .editor_state_mut()
            .chat;
        chat.set_input_text("design a settings screen");
        assert!(chat.begin_send());
        assert!(session.has_background_work());

        let engine_ptr = &mut engine as *mut OpEngine;
        assert_eq!(
            unsafe { op_cancel_background_work(engine_ptr) },
            OpStatus::Ok
        );
        let session = engine.session_mut_for_test();
        assert!(!session.has_background_work());
        let chat = &session
            .editor
            .as_ref()
            .expect("editor host")
            .editor_state()
            .chat;
        assert!(chat.pending_send.is_none());
        assert!(chat.messages.iter().all(|message| !message.streaming));
    }

    #[cfg(feature = "editor")]
    #[test]
    fn queued_codegen_is_background_work_and_cancel_retires_it() {
        let mut engine = engine();
        let session = engine.session_mut_for_test();
        let codegen = &mut session
            .editor
            .as_mut()
            .expect("editor host")
            .editor_state_mut()
            .codegen;
        codegen.phase = CodegenPhase::Generating;
        codegen.pending_generate = true;
        assert!(session.has_background_work());

        session.cancel_background_work();
        assert!(!session.has_background_work());
        let codegen = &session
            .editor
            .as_ref()
            .expect("editor host")
            .editor_state()
            .codegen;
        assert!(!codegen.pending_generate);
        assert!(!codegen.pending_regenerate);
        assert!(!codegen.pending_cancel);
        assert_eq!(codegen.phase, CodegenPhase::Idle);
    }

    #[cfg(feature = "editor")]
    #[test]
    fn suspended_design_turn_applies_tools_and_finishes_without_a_render_frame() {
        let turn_one = design_tool_turn_events();
        let mut responses = vec![sse_ok_response(
            &turn_one.iter().map(String::as_str).collect::<Vec<_>>(),
        )];
        // The shared loop may spend corrective/finalization rounds depending
        // on the deterministic quality report. Extra canned stops make that
        // choreography hermetic without requiring the server thread to join.
        responses.extend(std::iter::repeat_with(stop_response).take(7));
        let base_url = spawn_sequential_chat_server(responses);

        let mut engine = engine();
        configure_builtin_provider(&mut engine, &base_url);
        queue_design_turn(&mut engine, "Design a compact mobile proof screen");
        let engine_ptr = &mut engine as *mut OpEngine;

        let mut active = false;
        assert_eq!(
            unsafe { op_has_background_work(engine_ptr, &mut active) },
            OpStatus::Ok
        );
        assert!(active, "the queued send must request a background lease");
        assert_eq!(unsafe { crate::op_suspend(engine_ptr) }, OpStatus::Ok);
        assert!(engine.session_mut_for_test().suspended);

        let started = Instant::now();
        let mut now_ms = 1_u64;
        let mut saw_active_tick = false;
        loop {
            assert_eq!(
                unsafe { op_background_tick(engine_ptr, now_ms, &mut active) },
                OpStatus::Ok
            );
            if !active {
                break;
            }
            saw_active_tick = true;
            assert!(
                started.elapsed() < Duration::from_secs(60),
                "background-only design turn did not complete"
            );
            std::thread::sleep(Duration::from_millis(5));
            now_ms = now_ms.saturating_add(33);
        }
        assert!(
            saw_active_tick,
            "the fake provider must exercise an in-flight background turn"
        );

        let session = engine.session_mut_for_test();
        let host = session.editor.as_ref().expect("editor host");
        let proof = host
            .editor_state()
            .active_children()
            .iter()
            .find(|node| node.base().name.as_deref() == Some("BackgroundProof"))
            .expect("background tick must ack batch_design and apply its root");
        assert!(
            proof
                .children()
                .is_some_and(|children| !children.is_empty()),
            "the applied proof screen keeps its generated content"
        );
        let reply = host
            .editor_state()
            .chat
            .messages
            .iter()
            .rev()
            .find(|message| message.role == ChatRole::Assistant)
            .expect("design turn has an assistant transcript bubble");
        assert!(!reply.streaming, "terminal background turn stops streaming");
        let card = reply
            .tool_calls
            .iter()
            .find(|call| call.name == "batch_design")
            .expect("background tool call is folded into the transcript");
        let envelope: serde_json::Value =
            serde_json::from_str(&card.args).expect("tool card envelope");
        assert_eq!(envelope["status"], "done");
        assert_eq!(envelope["result"]["success"], true);
        assert_eq!(host.editor_state().chat.agents_running, (0, 0));

        // Once the terminal tick retires the job, later background ticks stay
        // idle and cannot apply/finalize the same run a second time.
        let terminal_revision = host.editor_state().document_revision();
        for offset in [33_u64, 66] {
            active = true;
            assert_eq!(
                unsafe {
                    op_background_tick(engine_ptr, now_ms.saturating_add(offset), &mut active)
                },
                OpStatus::Ok
            );
            assert!(!active, "a retired turn must remain idle");
            assert_eq!(
                engine
                    .session_mut_for_test()
                    .editor
                    .as_ref()
                    .expect("editor host")
                    .editor_state()
                    .document_revision(),
                terminal_revision,
                "an idle tick must not replay the terminal mutation"
            );
        }
    }

    #[cfg(feature = "editor")]
    fn configure_builtin_provider(engine: &mut OpEngine, base_url: &str) {
        let host = engine
            .session_mut_for_test()
            .editor
            .as_mut()
            .expect("editor host");
        let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
        settings.builtin_agents.clear();
        settings.add_builtin_agent_config(
            "Background Test",
            "sk-background-test",
            "test-model",
            BuiltinAgentKind::OpenAiCompat,
            base_url,
        );
        host.editor_state_mut().rebuild_chat_models();
        let chat = &mut host.editor_state_mut().chat;
        chat.selected_model = chat
            .available_models
            .iter()
            .position(|entry| entry.builtin_provider_id.is_some())
            .expect("ready builtin model entry");
    }

    #[cfg(feature = "editor")]
    fn queue_design_turn(engine: &mut OpEngine, prompt: &str) {
        let chat = &mut engine
            .session_mut_for_test()
            .editor
            .as_mut()
            .expect("editor host")
            .editor_state_mut()
            .chat;
        chat.set_input_text(prompt);
        assert!(chat.begin_send(), "design send must be queued");
    }

    #[cfg(feature = "editor")]
    fn spawn_sequential_chat_server(responses: Vec<String>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake provider");
        let address = listener.local_addr().expect("fake provider address");
        std::thread::spawn(move || {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut request = Vec::new();
                let mut chunk = [0_u8; 8192];
                loop {
                    let Ok(length) = stream.read(&mut chunk) else {
                        return;
                    };
                    request.extend_from_slice(&chunk[..length]);
                    if length == 0 || request_complete(&request) {
                        break;
                    }
                }
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{address}")
    }

    #[cfg(feature = "editor")]
    fn request_complete(raw: &[u8]) -> bool {
        let text = String::from_utf8_lossy(raw);
        let Some(header_end) = text.find("\r\n\r\n") else {
            return false;
        };
        let content_length = text
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        raw.len() >= header_end + 4 + content_length
    }

    #[cfg(feature = "editor")]
    fn sse_ok_response(events: &[&str]) -> String {
        let body: String = events
            .iter()
            .map(|event| format!("data: {event}\n\n"))
            .collect();
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
    }

    #[cfg(feature = "editor")]
    fn stop_response() -> String {
        sse_ok_response(&[
            r#"{"choices":[{"delta":{"content":"Background design complete."}}]}"#,
            r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
            "[DONE]",
        ])
    }

    #[cfg(feature = "editor")]
    fn design_tool_turn_events() -> Vec<String> {
        let operations = concat!(
            r##"root=I(null,{"type":"frame","name":"BackgroundProof","width":390,"height":844,"fill":"#111318","layout":"vertical","padding":24,"gap":16})"##,
            "\n",
            r##"title=I(root,{"type":"text","content":"Background proof","fontSize":28,"fontWeight":"700","textColor":"#FFFFFF"})"##,
            "\n",
            r##"body=I(root,{"type":"text","content":"Generated without a render frame","fontSize":15,"textColor":"#9CA3AF"})"##,
        );
        let args = serde_json::json!({ "operations": operations }).to_string();
        let call = serde_json::json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_background_design_1",
                        "function": { "name": "batch_design", "arguments": args },
                    }],
                },
            }],
        });
        vec![
            r#"{"choices":[{"delta":{"content":"Building in the background."}}]}"#.to_string(),
            call.to_string(),
            r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#.to_string(),
            "[DONE]".to_string(),
        ]
    }
}
