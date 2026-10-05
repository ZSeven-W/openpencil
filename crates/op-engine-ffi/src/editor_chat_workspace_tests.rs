//! Engine-thread tests for the workspace half of the mobile chat pump:
//! the phone reader's phase settles from real turns, Stop settles
//! Stopped, and 改这一页 both names its board in the prompt and fences
//! every write to it.

use super::*;
use crate::editor_chat::MobileChatHost;
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{BuiltinAgentKind, HomeFamily, PenNodeExt};
use std::io::Write;
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
        "GLM",
        "sk-mobile-chat",
        "glm-5.3-flash",
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
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("fixture deadline");
            let raw =
                crate::test_http::read_request(&mut stream).expect("complete fixture request");
            requests.lock().expect("log").push(raw);
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

/// A structured response tries to edit the scoped board and an unrelated one.
fn out_of_scope_call() -> String {
    let script = concat!(
        r#"I(null,{"type":"frame","id":"b1","name":"暖色封面","x":2000,"y":0,"width":1920,"height":1080,"children":[]});"#,
        r#"I(null,{"type":"frame","id":"b0","name":"被误改","x":0,"y":0,"width":1920,"height":1080,"children":[]});"#,
    );
    sse(&[
        serde_json::json!({"choices":[{"delta":{"content":script}}]}).to_string(),
        r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#.to_string(),
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
    let (base_url, requests) = spawn_server(vec![out_of_scope_call()]);
    let mut host = phone_deck(&base_url);
    host.editor_state_mut()
        .editor_ui
        .workspace
        .stage_page_edit("b1", 1);
    let mut expected = host.editor_state().active_children().to_vec();
    expected[1].base_mut().name = Some("暖色封面".into());
    send(&mut host, "换成暖色");

    let mut chat = MobileChatHost::default();
    pump_to_completion(&mut chat, &mut host);

    let requests = requests.lock().expect("log");
    assert_eq!(
        requests.len(),
        1,
        "a local edit makes one structured request"
    );
    let body: serde_json::Value =
        serde_json::from_str(requests[0].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert!(body.get("tools").is_none(), "no whole-design tool catalog");
    assert_eq!(body["model"], "glm-5.3-flash");
    assert_eq!(body["thinking"]["type"], "disabled");
    let prompt = body["messages"].as_array().unwrap().last().unwrap()["content"]
        .as_str()
        .unwrap();
    assert!(prompt.contains("CONTEXT NODES") && prompt.contains("b1"));
    assert!(!prompt.contains("b0") && !prompt.contains("b2"));
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
    assert_eq!(
        state.active_children(),
        expected.as_slice(),
        "no loop-finalize repair may change authored geometry"
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
fn a_mobile_title_edit_changes_exactly_one_field_without_finalize() {
    let title = serde_json::json!({
        "type":"text", "id":"home-title", "name":"Hero title", "content":"原标题", "x":24, "y":80,
        "width":340, "fontSize":28, "fontWeight":"700"
    });
    let mut edited = title.clone();
    edited["content"] = serde_json::json!("how to create a new page");
    let response = sse(&[
        serde_json::json!({"choices":[{"delta":{"content":format!("I(null,{});", edited)}}]})
            .to_string(),
        r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#.to_string(),
        "[DONE]".to_string(),
    ]);
    let (base_url, requests) = spawn_server(vec![response]);
    let mut host = phone_deck(&base_url);
    let state = host.editor_state_mut();
    state.active_children_mut()[0]
        .children_mut()
        .unwrap()
        .push(serde_json::from_value(title).unwrap());
    let mut expected = state.doc.clone();
    expected.children[0].children_mut().unwrap()[0] = serde_json::from_value(edited).unwrap();
    state.editor_ui.workspace.stage_page_edit("b0", 0);
    send(
        &mut host,
        "change this page title to how to create a new page keep everything else unchanged",
    );
    pump_to_completion(&mut MobileChatHost::default(), &mut host);
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value =
        serde_json::from_str(requests[0].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert!(
        body.get("tools").is_none(),
        "literal title does not start the whole-design loop"
    );
    assert_eq!(body["model"], "glm-5.3-flash");
    assert!(
        body["messages"].as_array().unwrap().last().unwrap()["content"]
            .as_str()
            .unwrap()
            .ends_with(
                "change this page title to how to create a new page keep everything else unchanged"
            )
    );
    assert_eq!(
        host.editor_state().doc,
        expected,
        "only the requested title content changes"
    );
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
    let scope = scope_launch(&mut host, "改标题", op_editor_core::LaunchRoute::Auto);
    assert_eq!(scope.fence.as_deref().unwrap(), ["b2"]);
    assert!(scope.prompt.contains("page 3"));
    let again = scope_launch(&mut host, "再改一次", op_editor_core::LaunchRoute::Auto);
    assert!(again.fence.is_none(), "the binding covered one send only");
    assert_eq!(again.prompt, "再改一次");
}

#[test]
fn a_vanished_board_cannot_run_an_unscoped_edit() {
    let mut host = phone_deck("http://127.0.0.1:9");
    host.editor_state_mut()
        .editor_ui
        .workspace
        .stage_page_edit("gone", 0);
    let scope = scope_launch(&mut host, "改一下", op_editor_core::LaunchRoute::Auto);
    assert!(scope.fence.is_none());
    assert_eq!(scope.prompt, "改一下");
    assert!(scope.error.is_some());
    assert!(host
        .editor_state()
        .editor_ui
        .workspace
        .page_edit_running
        .is_none());
}

#[test]
fn a_named_page_followup_uses_the_same_fence_without_a_reader_button() {
    let mut host = phone_deck("http://127.0.0.1:9");
    host.editor_state_mut().active_children_mut()[0]
        .base_mut()
        .name = Some("Coffee Home".into());
    let scope = scope_launch(
        &mut host,
        "把首页标题改成“周末，来杯好咖啡”，保留其他页面和布局。",
        op_editor_core::LaunchRoute::Auto,
    );
    assert_eq!(scope.fence.as_deref().unwrap(), ["b0"]);
    assert!(scope.error.is_none());
    assert!(scope.prompt.contains("node id `b0`"));
}

#[test]
fn pinned_ppt_refine_fences_every_selected_draft_board_in_normal_mode() {
    let mut host = phone_deck("http://127.0.0.1:9");
    let state = host.editor_state_mut();
    state.selection.set = vec![
        op_editor_core::NodeId::new("b0"),
        op_editor_core::NodeId::new("b1"),
    ];
    assert!(state.editor_ui.workspace.visible);
    let prompt = op_editor_core::refine_prompt(
        HomeFamily::Presentation,
        "为 OpenPencil 做一份 5 页产品介绍 PPT，包含封面和结束页。",
    );
    let scope = scope_launch(&mut host, &prompt, op_editor_core::LaunchRoute::Refine);
    assert!(scope.error.is_none());
    assert_eq!(scope.fence.as_deref().unwrap(), ["b0", "b1"]);
    let before = host.editor_state().active_children().to_vec();
    let (_, reverted) = fenced(host.editor_state_mut(), scope.fence.as_deref(), |state| {
        for board in state.active_children_mut() {
            board.base_mut().name = Some("Edited".into());
        }
    });
    assert!(reverted);
    assert_ne!(host.editor_state().active_children()[0], before[0]);
    assert_ne!(host.editor_state().active_children()[1], before[1]);
    assert_eq!(host.editor_state().active_children()[2], before[2]);
}

#[test]
fn a_tablet_run_through_the_chat_pump_settles_in_the_tablet_reader() {
    const TABLET: (f32, f32) = (1366.0, 1024.0);
    let (base_url, _requests) = spawn_server(std::iter::repeat_with(stop).take(8).collect());
    let mut host = phone_deck(&base_url);
    host.editor_state_mut().editor_ui.size_class = EditorSizeClass::Expanded;
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Generating;
    send(&mut host, "做一份 3 页 PPT");
    let mut chat = MobileChatHost::default();
    let started = Instant::now();
    let mut now_ms = 10;
    while chat.pump(&mut host, now_ms, TABLET).is_some() {
        assert!(started.elapsed() < Duration::from_secs(60), "turn hung");
        std::thread::sleep(Duration::from_millis(5));
        now_ms += 33;
    }
    assert_eq!(
        host.editor_state().editor_ui.workspace.phase,
        WorkspacePhase::Done
    );
    assert!(host.works_reader_visible(), "the tablet reads the result");
    let reader = op_editor_ui::widgets::WorksReader::for_editor(host.editor_state()).unwrap();
    let layout = reader.layout(TABLET.0, TABLET.1);
    assert_eq!(
        layout.form,
        op_editor_ui::widgets::works_reader::ReaderForm::TabletLandscape
    );
    let (x, y, w, h) = op_editor_ui::widgets::host_canvas_geometry::canvas_region(
        host.editor_state(),
        TABLET.0,
        TABLET.1,
    );
    assert_eq!(
        (x, y, w, h),
        (
            layout.stage.origin.x,
            layout.stage.origin.y,
            layout.stage.size.x,
            layout.stage.size.y
        )
    );
}

#[test]
fn a_rate_limited_edit_retry_keeps_its_original_page_and_changes_only_the_requested_copy() {
    let title = serde_json::json!({"type":"text","id":"retry-title","name":"Action title","content":"原标题","x":24,"y":80,"width":340,"fontSize":28,"fontWeight":"700"});
    let mut edited = title.clone();
    edited["content"] = serde_json::json!("checked tutorial");
    let response = sse(&[
        serde_json::json!({"choices":[{"delta":{"content":format!("I(null,{});", edited)}}]})
            .to_string(),
        r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#.into(),
        "[DONE]".into(),
    ]);
    let (base_url, requests) = spawn_server(vec![
        "HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        response,
    ]);
    let mut host = phone_deck(&base_url);
    let state = host.editor_state_mut();
    state.active_children_mut()[2]
        .children_mut()
        .unwrap()
        .push(serde_json::from_value(title).unwrap());
    let before = state.doc.clone();
    let mut expected = before.clone();
    expected.children[2].children_mut().unwrap()[0] = serde_json::from_value(edited).unwrap();
    state.editor_ui.workspace.selected = 2;
    state.editor_ui.workspace.stage_page_edit("b2", 2);
    let instruction = "change only the action title on this page to checked tutorial keep everything else unchanged";
    send(&mut host, instruction);
    let mut chat = MobileChatHost::default();
    pump_to_completion(&mut chat, &mut host);
    assert_eq!(host.editor_state().doc, before);
    assert_eq!(
        host.editor_state().editor_ui.workspace.phase,
        WorkspacePhase::Failed
    );
    let reader = op_editor_ui::widgets::WorksReader::for_editor(host.editor_state()).unwrap();
    assert_eq!(
        reader.status_action(),
        Some(op_editor_core::ReaderHit::Retry)
    );
    host.editor_state_mut().editor_ui.workspace.selected = 0;
    assert!(op_editor_core::workspace_page_edit::retry_page_edit(
        host.editor_state_mut()
    ));
    pump_to_completion(&mut chat, &mut host);
    assert_eq!(host.editor_state().doc, expected);
    assert_eq!(
        host.editor_state().editor_ui.workspace.phase,
        WorkspacePhase::Done
    );
    assert!(host
        .editor_state()
        .editor_ui
        .workspace
        .page_edit_retry
        .is_none());
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests.iter() {
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["model"], "glm-5.3-flash");
        let prompt = body["messages"].as_array().unwrap().last().unwrap()["content"]
            .as_str()
            .unwrap();
        assert!(prompt.ends_with(instruction));
    }
}
