//! Engine-thread contracts for the native mobile Code panel runtime.

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use jian_ops_schema::PenDocument;
use op_editor_core::{codegen::Framework, BuiltinAgentKind, EditOrigin, NodeId};
use op_editor_host_core::codegen_session::{CodegenDelta, CodegenResult};

fn session(
    framework: Framework,
    identity: CodegenDocumentIdentity,
) -> (CodegenSession, Sender<CodegenDelta>) {
    let (tx, rx) = std::sync::mpsc::channel();
    (
        CodegenSession {
            rx,
            finished: false,
            framework,
            document_identity: identity,
            selection_snapshot: vec!["selected-node".into()],
            model: None,
            cancel: Arc::new(AtomicBool::new(false)),
            run_epoch: 1,
        },
        tx,
    )
}

fn artifact(name: &str) -> CodegenArtifact {
    CodegenArtifact {
        file_name: name.into(),
        mime_type: "application/octet-stream",
        bytes: vec![1, 2, 3],
    }
}

fn document_with_rect(id: &str) -> PenDocument {
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "rectangle",
            "id": id,
            "name": "Remote",
            "x": 0,
            "y": 0,
            "width": 10,
            "height": 10
        }]
    }))
    .expect("valid collaboration document")
}

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

fn sse_text_response(text: &str) -> String {
    let event = serde_json::json!({
        "choices": [{ "delta": { "content": text } }]
    });
    let body = format!("data: {event}\n\ndata: [DONE]\n\n");
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn spawn_codegen_server(
    responses: Vec<String>,
) -> (String, Arc<Mutex<Vec<String>>>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind codegen provider");
    let address = listener.local_addr().expect("codegen provider address");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&requests);
    let handle = std::thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().expect("accept codegen request");
            let mut raw = Vec::new();
            let mut chunk = [0_u8; 8192];
            loop {
                let length = stream.read(&mut chunk).expect("read codegen request");
                raw.extend_from_slice(&chunk[..length]);
                if length == 0 || request_complete(&raw) {
                    break;
                }
            }
            log.lock()
                .expect("request log")
                .push(String::from_utf8_lossy(&raw).into_owned());
            stream
                .write_all(response.as_bytes())
                .expect("write codegen response");
        }
    });
    (format!("http://{address}/v1"), requests, handle)
}

fn host_with_codegen_input(base_url: &str) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let node: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
        "type": "rectangle",
        "id": "n1",
        "name": "Root",
        "x": 0,
        "y": 0,
        "width": 240,
        "height": 120
    }))
    .expect("rectangle fixture");
    let state = host.editor_state_mut();
    state.doc.children = vec![node];
    state.set_single_selection(NodeId::new("n1"));
    state.editor_ui.agent_settings.builtin_agents.clear();
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Mobile E2E",
        "sk-codegen-e2e",
        "deepseek-codegen-test",
        BuiltinAgentKind::OpenAiCompat,
        base_url,
    );
    state.rebuild_chat_models();
    state.chat.selected_model = state
        .chat
        .available_models
        .iter()
        .position(|entry| entry.builtin_provider_id.is_some())
        .expect("built-in model row");
    state.codegen.pending_generate = true;
    state.codegen.phase = CodegenPhase::Generating;
    host
}

#[test]
fn artifact_queue_is_bounded_and_keeps_the_latest_two_actions() {
    let mut runtime = MobileCodegenHost::default();
    runtime.queue_artifact(artifact("one"));
    runtime.queue_artifact(artifact("two"));
    runtime.queue_artifact(artifact("three"));

    assert_eq!(runtime.artifacts.len(), MAX_STAGED_ARTIFACTS);
    assert_eq!(runtime.artifacts[0].file_name, "two");
    assert_eq!(runtime.artifacts[1].file_name, "three");
}

#[test]
fn pending_generate_runs_plan_chunk_and_assembly_over_real_mobile_sse() {
    let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"root","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
    let chunk = "export default function Root(){}\n---CONTRACT---\n{\"componentName\":\"Root\"}";
    let assembly = "export default function App(){ return <main>mobile-codegen-marker</main> }";
    let (base_url, requests, server) = spawn_codegen_server(vec![
        sse_text_response(plan),
        sse_text_response(chunk),
        sse_text_response(assembly),
    ]);
    let mut host = host_with_codegen_input(&base_url);
    let mut runtime = MobileCodegenHost::default();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut now_ms = 10;
    while runtime.pump(&mut host, now_ms).is_some() {
        assert!(Instant::now() < deadline, "mobile codegen did not settle");
        std::thread::sleep(Duration::from_millis(5));
        now_ms += CODEGEN_POLL_INTERVAL_MS;
    }

    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Complete);
    assert!(host
        .editor_state()
        .codegen
        .code
        .contains("mobile-codegen-marker"));
    assert_eq!(host.editor_state().codegen.selection_snapshot, ["n1"]);
    server.join().expect("provider served every phase");
    let requests = requests.lock().expect("request log");
    assert_eq!(requests.len(), 3, "plan, chunk, assembly each use HTTP");
    for request in requests.iter() {
        let lower = request.to_ascii_lowercase();
        assert!(lower.starts_with("post /v1/chat/completions http/1.1"));
        assert!(lower.contains("authorization: bearer sk-codegen-e2e"));
        assert!(request.contains("\"model\":\"deepseek-codegen-test\""));
        assert!(!request.contains("\"thinking\":{"));
        assert!(!request.contains("\"reasoning_effort\":"));
    }
}

#[test]
fn whole_document_replacement_clears_results_and_frozen_artifacts() {
    let mut host = WidgetHostNative::new();
    let old_identity = document_identity(&host);
    let mut runtime = MobileCodegenHost::default();
    runtime.document_identity = Some(old_identity);
    runtime.results.insert(
        old_identity,
        Framework::React,
        CodegenResult {
            code: "old".into(),
            framework_ext: "tsx".into(),
            assets: Vec::new(),
        },
    );
    runtime.queue_artifact(artifact("old.zip"));

    assert!(host.replace_editor_state(op_editor_core::EditorState::starter()));
    runtime.rotate_document(document_identity(&host));

    assert!(runtime.results.is_empty());
    assert!(runtime.artifacts.is_empty());
}

#[test]
fn remote_commit_reset_epoch_rejects_queued_late_done_and_clears_runtime_state() {
    let mut host = WidgetHostNative::new();
    let old_host_epoch = host.document_epoch();
    let old_generation = host.editor_state().document_generation();
    let old_identity = document_identity(&host);
    let (active, tx) = session(Framework::React, old_identity);
    let mut runtime = MobileCodegenHost::default();
    runtime.current = Some(active);
    runtime.document_identity = Some(old_identity);
    runtime.results.insert(
        old_identity,
        Framework::React,
        CodegenResult {
            code: "old cached output".into(),
            framework_ext: "tsx".into(),
            assets: Vec::new(),
        },
    );
    runtime.queue_artifact(artifact("old.zip"));

    host.install_collaboration_document(document_with_rect("peer-1:1"), EditOrigin::RemoteCommit)
        .expect("remote commit installs");
    assert_eq!(host.document_epoch(), old_host_epoch);
    assert_eq!(host.editor_state().document_generation(), old_generation);
    assert_ne!(document_identity(&host), old_identity);
    tx.send(CodegenDelta::Done {
        code: "late remote output".into(),
        degraded: false,
        assets: Vec::new(),
    })
    .expect("late terminal delta queued before runtime observes reset");

    assert!(runtime.pump(&mut host, 10).is_none());
    assert!(host.editor_state().codegen.code.is_empty());
    assert!(runtime.results.is_empty());
    assert!(runtime.artifacts.is_empty());
}

#[test]
fn ui_cancel_drops_a_queued_late_done_without_resurrecting_code() {
    let mut host = WidgetHostNative::new();
    let identity = document_identity(&host);
    let (active, tx) = session(Framework::React, identity);
    let mut runtime = MobileCodegenHost::default();
    runtime.current = Some(active);
    runtime.document_identity = Some(identity);
    host.editor_state_mut().codegen.pending_cancel = true;
    host.editor_state_mut().codegen.phase = CodegenPhase::Idle;
    tx.send(CodegenDelta::Done {
        code: "late output".into(),
        degraded: false,
        assets: Vec::new(),
    })
    .expect("queued late terminal delta");
    drop(tx);

    assert!(runtime.pump(&mut host, 10).is_none());
    assert!(host.editor_state().codegen.code.is_empty());
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Idle);
    assert!(runtime.results.is_empty());
}

#[test]
fn os_cancel_retires_work_immediately_and_rejects_late_output() {
    let mut host = WidgetHostNative::new();
    let identity = document_identity(&host);
    let (active, tx) = session(Framework::React, identity);
    let mut runtime = MobileCodegenHost::default();
    runtime.current = Some(active);
    runtime.document_identity = Some(identity);
    host.editor_state_mut().codegen.phase = CodegenPhase::Generating;

    assert!(runtime.cancel_background_work(&mut host));
    assert!(!runtime.has_background_work(&host));
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Idle);
    assert!(tx
        .send(CodegenDelta::Done {
            code: "too late".into(),
            degraded: false,
            assets: Vec::new(),
        })
        .is_err());
}

#[test]
fn framework_results_stay_separate_and_switch_back_restores_react() {
    let mut host = WidgetHostNative::new();
    let identity = document_identity(&host);
    let mut runtime = MobileCodegenHost::default();
    runtime.document_identity = Some(identity);

    let (react, react_tx) = session(Framework::React, identity);
    runtime.current = Some(react);
    react_tx
        .send(CodegenDelta::Done {
            code: "react output".into(),
            degraded: false,
            assets: Vec::new(),
        })
        .unwrap();
    drop(react_tx);
    runtime.pump(&mut host, 10);

    assert!(host
        .editor_state_mut()
        .codegen
        .select_framework(Framework::Vue));
    let (vue, vue_tx) = session(Framework::Vue, identity);
    runtime.current = Some(vue);
    vue_tx
        .send(CodegenDelta::Done {
            code: "vue output".into(),
            degraded: false,
            assets: Vec::new(),
        })
        .unwrap();
    drop(vue_tx);
    runtime.pump(&mut host, 20);

    assert_eq!(
        runtime
            .results
            .get(identity, Framework::React)
            .expect("react result")
            .code,
        "react output"
    );
    assert_eq!(
        runtime
            .results
            .get(identity, Framework::Vue)
            .expect("vue result")
            .code,
        "vue output"
    );
    assert!(host
        .editor_state_mut()
        .codegen
        .select_framework(Framework::React));
    assert_eq!(host.editor_state().codegen.code, "react output");
}

#[test]
fn synthetic_framework_change_cancels_before_late_done_can_be_folded() {
    let mut host = WidgetHostNative::new();
    let identity = document_identity(&host);
    let (active, tx) = session(Framework::React, identity);
    let mut runtime = MobileCodegenHost::default();
    runtime.current = Some(active);
    runtime.document_identity = Some(identity);
    host.editor_state_mut().codegen.framework = Framework::Vue;
    tx.send(CodegenDelta::Done {
        code: "mislabelled".into(),
        degraded: false,
        assets: Vec::new(),
    })
    .unwrap();

    runtime.pump(&mut host, 10);
    assert!(host.editor_state().codegen.code.is_empty());
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Error);
    assert!(runtime.results.is_empty());
}
