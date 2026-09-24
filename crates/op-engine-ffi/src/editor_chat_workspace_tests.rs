//! Engine-thread tests for the workspace half of the mobile chat pump:
//! the phone reader's phase settles from real turns, Stop settles
//! Stopped, and 改这一页 both names its board in the prompt and fences
//! every write to it.

use super::*;
use crate::editor_chat::MobileChatHost;
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{BuiltinAgentKind, HomeFamily, PenNodeExt};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const VIEWPORT: (f32, f32) = (390.0, 844.0);

/// A phone host over three 1920×1080 slides with a finished reading of
/// them open, and a ready built-in provider at `base_url`.
fn phone_deck(base_url: &str) -> WidgetHostNative {
    let children: Vec<String> = (0..3)
        .map(|i| {
            format!(
                r#"{{ "type": "frame", "id": "b{i}", "name": "第 {n} 页", "x": {x}, "y": 0,
                     "width": 1920, "height": 1080, "children": [] }}"#,
                n = i + 1,
                x = i * 2000
            )
        })
        .collect();
    let source = format!(
        r#"{{ "version": "1.0.0", "children": [{}] }}"#,
        children.join(",")
    );
    let document = jian_ops_schema::load_str(&source).expect("fixture").value;
    let mut state = EditorState::from_document(document);
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host.editor_state_mut()
        .editor_ui
        .workspace
        .open_for_reading(HomeFamily::Presentation, 1);
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
    chat.selected_model = chat
        .available_models
        .iter()
        .position(|entry| entry.builtin_provider_id.is_some())
        .expect("builtin model entry");
    host
}

fn send(host: &mut WidgetHostNative, text: &str) {
    let chat = &mut host.editor_state_mut().chat;
    chat.set_input_text(text);
    assert!(chat.begin_send());
}

fn spawn_server(responses: Vec<String>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("address");
    let log = Arc::new(Mutex::new(Vec::new()));
    let requests = Arc::clone(&log);
    std::thread::spawn(move || {
        for response in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut raw = Vec::new();
            let mut chunk = [0_u8; 8192];
            loop {
                let Ok(n) = stream.read(&mut chunk) else {
                    return;
                };
                raw.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&raw);
                let complete = text.find("\r\n\r\n").is_some_and(|end| {
                    let length = text
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())?
                        })
                        .unwrap_or(0);
                    raw.len() >= end + 4 + length
                });
                if n == 0 || complete {
                    break;
                }
            }
            requests
                .lock()
                .expect("log")
                .push(String::from_utf8_lossy(&raw).into_owned());
            let _ = stream.write_all(response.as_bytes());
        }
    });
    (format!("http://{address}"), log)
}

fn sse(events: &[String]) -> String {
    let body: String = events.iter().map(|e| format!("data: {e}\n\n")).collect();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

fn stop() -> String {
    sse(&[
        r#"{"choices":[{"delta":{"content":"第 2 页已改成暖色。"}}]}"#.to_string(),
        r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#.to_string(),
        "[DONE]".to_string(),
    ])
}

/// One batch_design call that edits the bound board AND renames another
/// board — the second write must not survive.
fn out_of_scope_call() -> String {
    let operations = concat!(
        r##"U("b1",{"name":"暖色封面"})"##,
        "\n",
        r##"U("b0",{"name":"被误改"})"##,
    );
    let args = serde_json::json!({ "operations": operations }).to_string();
    let call = serde_json::json!({
        "choices": [{ "delta": { "tool_calls": [{
            "index": 0, "id": "call_page_edit",
            "function": { "name": "batch_design", "arguments": args },
        }]}}],
    });
    sse(&[
        call.to_string(),
        r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#.to_string(),
        "[DONE]".to_string(),
    ])
}

fn pump_to_completion(chat: &mut MobileChatHost, host: &mut WidgetHostNative) {
    let started = Instant::now();
    let mut now_ms = 10;
    while chat.pump(host, now_ms, VIEWPORT).is_some() {
        assert!(started.elapsed() < Duration::from_secs(60), "turn hung");
        std::thread::sleep(Duration::from_millis(5));
        now_ms += 33;
    }
}

#[test]
fn edit_this_page_scopes_the_prompt_and_fences_writes_to_the_board() {
    let mut responses = vec![out_of_scope_call()];
    responses.extend(std::iter::repeat_with(stop).take(7));
    let (base_url, requests) = spawn_server(responses);
    let mut host = phone_deck(&base_url);
    host.editor_state_mut()
        .editor_ui
        .workspace
        .stage_page_edit("b1", 1);
    send(&mut host, "换成暖色");

    let mut chat = MobileChatHost::default();
    pump_to_completion(&mut chat, &mut host);

    let requests = requests.lock().expect("log");
    assert!(
        requests[0].contains("PAGE EDIT SCOPE"),
        "the prompt names its scope"
    );
    assert!(requests[0].contains("`b1`"));
    assert!(
        requests.iter().any(|r| r.contains("were reverted")),
        "the model hears its stray write did not stick"
    );
    let state = host.editor_state();
    let ids: Vec<&str> = state.active_children().iter().map(|n| n.id_str()).collect();
    assert_eq!(ids, ["b0", "b1", "b2"], "no stray top-level board survives");
    let names: Vec<Option<&str>> = state
        .active_children()
        .iter()
        .map(|n| n.base().name.as_deref())
        .collect();
    assert_eq!(
        names,
        [Some("第 1 页"), Some("暖色封面"), Some("第 3 页")],
        "the bound board changed, the other page's write was reverted"
    );
    let workspace = &state.editor_ui.workspace;
    assert_eq!(
        workspace.phase,
        WorkspacePhase::Done,
        "the follow-up settled"
    );
    assert!(workspace.page_edit_running.is_none());
    assert!(workspace.run_epoch != 0, "the launch stamped its epoch");
}

#[test]
fn a_send_without_a_provider_settles_the_run_failed() {
    let mut host = phone_deck("http://127.0.0.1:9");
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .builtin_agents
        .clear();
    host.editor_state_mut().rebuild_chat_models();
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Generating;
    send(&mut host, "做一份 5 页 PPT");
    let mut chat = MobileChatHost::default();
    pump_to_completion(&mut chat, &mut host);
    assert_eq!(
        host.editor_state().editor_ui.workspace.phase,
        WorkspacePhase::Failed,
        "a run that never launched must not spin on 生成中 forever"
    );
}

#[test]
fn stop_settles_the_reader_stopped_and_scope_is_one_shot() {
    let mut host = phone_deck("http://127.0.0.1:9");
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Generating;
    host.editor_state_mut().chat.pending_stop_chat = true;
    let mut chat = MobileChatHost::default();
    chat.pump(&mut host, 10, VIEWPORT);
    assert_eq!(
        host.editor_state().editor_ui.workspace.phase,
        WorkspacePhase::Stopped
    );

    host.editor_state_mut()
        .editor_ui
        .workspace
        .stage_page_edit("b2", 2);
    let scope = scope_launch(&mut host, "改标题");
    assert_eq!(scope.fence.as_deref(), Some("b2"));
    assert!(scope.prompt.contains("page 3"));
    let again = scope_launch(&mut host, "再改一次");
    assert!(again.fence.is_none(), "the binding covered one send only");
    assert_eq!(again.prompt, "再改一次");
}

#[test]
fn a_vanished_board_runs_the_turn_unscoped() {
    let mut host = phone_deck("http://127.0.0.1:9");
    host.editor_state_mut()
        .editor_ui
        .workspace
        .stage_page_edit("gone", 0);
    let scope = scope_launch(&mut host, "改一下");
    assert!(scope.fence.is_none());
    assert_eq!(scope.prompt, "改一下");
    assert!(host
        .editor_state()
        .editor_ui
        .workspace
        .page_edit_running
        .is_none());
}
