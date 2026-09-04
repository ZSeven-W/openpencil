//! Engine-thread tests for the mobile chat pump: a built-in provider send
//! must stream into the transcript, and every failure path must land an
//! error in the assistant bubble instead of leaving it on "Thinking…".

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use op_editor_core::BuiltinAgentKind;

const TEST_VIEWPORT: (f32, f32) = (402.0, 782.0);

fn host_with_builtin_provider(base_url: &str) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
    settings.builtin_agents.clear();
    settings.add_builtin_agent_config(
        "DeepSeek",
        "sk-mobile-chat",
        "deepseek-chat",
        BuiltinAgentKind::OpenAiCompat,
        base_url,
    );
    host.editor_state_mut().rebuild_chat_models();
    let chat = &mut host.editor_state_mut().chat;
    let builtin_index = chat
        .available_models
        .iter()
        .position(|entry| entry.builtin_provider_id.is_some())
        .expect("a ready builtin agent must surface a chat model entry");
    chat.selected_model = builtin_index;
    host
}

fn send_user_message(host: &mut WidgetHostNative, text: &str) {
    let chat = &mut host.editor_state_mut().chat;
    chat.set_input_text(text);
    assert!(chat.begin_send(), "begin_send must queue the turn");
    assert!(chat.pending_send.is_some());
}

/// Serve one canned HTTP response for one connection, returning the raw
/// request bytes for assertions.
fn spawn_chat_server(response: String) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local chat server");
    let address = listener.local_addr().expect("local chat address");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept chat request");
        let mut request = [0_u8; 8192];
        let length = stream.read(&mut request).expect("read chat request");
        let request = String::from_utf8_lossy(&request[..length]).into_owned();
        stream
            .write_all(response.as_bytes())
            .expect("write chat response");
        request
    });
    (format!("http://{address}"), handle)
}

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

/// Pump until the in-flight turn retires (pump returns no wake deadline).
fn pump_to_completion(chat_host: &mut MobileChatHost, host: &mut WidgetHostNative) {
    let started = Instant::now();
    let mut now_ms = 10;
    loop {
        let wake = chat_host.pump(host, now_ms, TEST_VIEWPORT);
        if wake.is_none() {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "chat turn did not complete within the test deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
        now_ms += STREAM_POLL_INTERVAL_MS;
    }
}

fn last_assistant(host: &WidgetHostNative) -> op_editor_core::ChatMessage {
    host.editor_state()
        .chat
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ChatRole::Assistant)
        .expect("transcript holds an assistant bubble")
        .clone()
}

#[test]
fn builtin_send_streams_reply_into_transcript() {
    let (base_url, server) = spawn_chat_server(sse_ok_response(&[
        r#"{"choices":[{"delta":{"content":"Hello"}}]}"#,
        r#"{"choices":[{"delta":{"content":", world"}}]}"#,
        "[DONE]",
    ]));
    let mut host = host_with_builtin_provider(&base_url);
    send_user_message(&mut host, "hi there");

    let mut chat_host = MobileChatHost::default();
    pump_to_completion(&mut chat_host, &mut host);

    let reply = last_assistant(&host);
    assert_eq!(reply.content, "Hello, world");
    assert!(!reply.streaming, "a finished turn must stop streaming");
    assert!(host.editor_state().chat.pending_send.is_none());

    let request = server.join().expect("chat server exits");
    assert!(request.starts_with("POST /chat/completions HTTP/1.1"));
    assert!(request.contains("authorization: Bearer sk-mobile-chat"));
    assert!(request.contains("\"model\":\"deepseek-chat\""));
}

#[test]
fn provider_http_error_lands_in_bubble_instead_of_eternal_thinking() {
    let (base_url, server) = spawn_chat_server(
        "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_string(),
    );
    let mut host = host_with_builtin_provider(&base_url);
    send_user_message(&mut host, "hi there");

    let mut chat_host = MobileChatHost::default();
    pump_to_completion(&mut chat_host, &mut host);

    let reply = last_assistant(&host);
    assert!(
        reply.content.starts_with("error: "),
        "provider failure must surface in the transcript, got {:?}",
        reply.content
    );
    assert!(reply.content.contains("http 500"));
    assert!(!reply.streaming, "an aborted turn must stop streaming");
    let _ = server.join();
}

#[test]
fn non_builtin_selection_reports_honest_error() {
    let mut host = WidgetHostNative::new();
    // No builtin agents configured — the selection resolves to a CLI
    // provider, which cannot exist on a phone.
    send_user_message(&mut host, "hi there");

    let mut chat_host = MobileChatHost::default();
    assert!(
        chat_host.pump(&mut host, 10, TEST_VIEWPORT).is_none(),
        "no turn launches"
    );

    let reply = last_assistant(&host);
    assert!(
        reply.content.starts_with("error: "),
        "unroutable selection must land an error, got {:?}",
        reply.content
    );
    assert!(reply.content.contains("not available on this device"));
    assert!(!reply.streaming);
}

#[test]
fn stop_request_drops_the_inflight_turn() {
    // A server that never responds — the turn would stream forever.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stalled server");
    let base_url = format!("http://{}", listener.local_addr().expect("local address"));
    let mut host = host_with_builtin_provider(&base_url);
    send_user_message(&mut host, "hi there");

    let mut chat_host = MobileChatHost::default();
    assert!(
        chat_host.pump(&mut host, 10, TEST_VIEWPORT).is_some(),
        "a launched turn schedules the streaming repoll"
    );
    host.editor_state_mut().chat.pending_stop_chat = true;
    assert!(
        chat_host.pump(&mut host, 43, TEST_VIEWPORT).is_none(),
        "stop must retire the in-flight turn"
    );
    drop(listener);
}
