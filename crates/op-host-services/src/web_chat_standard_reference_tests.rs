use super::*;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason, ThinkingMode,
};
use op_editor_core::EditorState;

use crate::ai_proxy::AiStreamRequest;
use crate::web_canvas_server::{SseHub, WebCanvasState};

struct QueuedProvider {
    responses: std::sync::Mutex<VecDeque<String>>,
    seen: Arc<Mutex<Vec<ChatRequest>>>,
}

impl QueuedProvider {
    fn new(responses: &[&str]) -> Self {
        Self::with_seen(responses).0
    }

    fn with_seen(responses: &[&str]) -> (Self, Arc<Mutex<Vec<ChatRequest>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let provider = Self {
            responses: std::sync::Mutex::new(
                responses
                    .iter()
                    .map(|response| (*response).to_string())
                    .collect(),
            ),
            seen: seen.clone(),
        };
        (provider, seen)
    }
}

impl ChatProvider for QueuedProvider {
    fn provider_label(&self) -> &str {
        "queued-reference-test"
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.seen.lock().expect("seen requests").push(request);
        let response = self
            .responses
            .lock()
            .expect("response queue")
            .pop_front()
            .unwrap_or_default();
        Box::new(
            [
                ChatDelta::TextDelta(response),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]
            .into_iter(),
        )
    }
}

fn new_design_request(user: &str, attachments: Vec<ChatAttachment>) -> WebStandardTurnRequest {
    WebStandardTurnRequest {
        ai: AiStreamRequest {
            provider: None,
            builtin_provider_id: None,
            model: "test-model".into(),
            skills: Vec::new(),
            user: user.into(),
            max_output_tokens: 2048,
            thinking: ThinkingMode::Disabled,
            effort: EffortLevel::Low,
            transient_builtin: None,
        },
        document_json: None,
        editor_meta: None,
        selected_ids: Vec::new(),
        active_page_id: None,
        agent_team_size: None,
        history: Vec::new(),
        attachments,
        transient_builtin: None,
    }
}

#[test]
fn rejected_reference_is_reported_and_new_design_still_runs() {
    let request = new_design_request("参考 http://127.0.0.1:1/ 做首页", Vec::new());
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    let hub = SseHub::default();
    let snapshot = state.lock().expect("state").editor.clone();
    let provider = QueuedProvider::new(&[
        r#"{"rootFrame":{"id":"root","name":"Page","width":375,"height":200,"layout":"vertical"},"subtasks":[{"id":"hero","label":"Hero","elements":"headline","region":{"width":375,"height":200}}]}"#,
        r#"I(null,{"type":"frame","name":"Hero","x":0,"y":0,"width":375,"height":200,"children":[{"type":"text","content":"Home","fontSize":18}]});"#,
    ]);
    let mut out = Vec::new();

    stream_new_design_route(
        &mut out,
        request,
        snapshot,
        Box::new(provider),
        None,
        CanvasWriteTarget {
            state: &state,
            hub: &hub,
            write_barrier: None,
        },
    )
    .expect("new-design route");

    let streamed = String::from_utf8(out).expect("utf8 SSE");
    assert!(streamed.contains("reference page could not be used: import URL is not allowed"));
    assert!(
        streamed.contains("Done —"),
        "orchestrator did not continue: {streamed}"
    );
    assert!(
        !state
            .lock()
            .expect("state")
            .editor
            .active_children()
            .is_empty(),
        "the design route should still write generated content"
    );
}

#[test]
fn screenshot_reference_is_persisted_and_reaches_planning() {
    let image = ChatAttachment {
        name: "reference.png".into(),
        media_type: "image/png".into(),
        data: vec![1, 2, 3],
    };
    let vision_response = r#"# Design System: Screenshot App

## 1. Visual Theme & Atmosphere
Calm and focused.

## 2. Color Palette & Roles
- **Blue** (#2563EB) — Primary action
<<<SKELETON>>>
{"source":"screenshot","width":1440,"sections":[{"role":"navbar","heightRatio":0.05,"childCount":3,"layout":"horizontal","hasImage":false}],"navKind":"topBar","heroKind":"split","columnRhythm":[3]}"#;
    let root = r#"{"rootFrame":{"id":"root","name":"Page","width":375,"height":200,"layout":"vertical"},"subtasks":[{"id":"hero","label":"Hero","elements":"headline","region":{"width":375,"height":200}}]}"#;
    let subtask = r#"I(null,{"type":"frame","name":"Hero","x":0,"y":0,"width":375,"height":200,"children":[{"type":"text","content":"Home","fontSize":18}]});"#;
    let (provider, seen) = QueuedProvider::with_seen(&[vision_response, root, subtask]);
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    let snapshot = state.lock().expect("state").editor.clone();
    let hub = SseHub::default();
    let mut out = Vec::new();

    stream_new_design_route(
        &mut out,
        new_design_request("参考这张截图做首页", vec![image]),
        snapshot,
        Box::new(provider),
        None,
        CanvasWriteTarget {
            state: &state,
            hub: &hub,
            write_barrier: None,
        },
    )
    .expect("new-design route");

    let guard = state.lock().expect("state");
    assert_eq!(
        guard
            .editor
            .doc
            .design_md
            .as_ref()
            .and_then(|spec| spec.project_name.as_deref()),
        Some("Screenshot App")
    );
    let seen = seen.lock().expect("seen requests");
    assert!(seen.iter().any(|request| {
        request.attachments.len() == 1
            && request.attachments[0].media_type == "image/png"
            && request.user_message.contains("Extract the design system")
    }));
    assert!(seen.iter().any(|request| {
        request.user_message.contains("REFERENCE SKELETON")
            && request.user_message.contains("source: screenshot")
    }));
}

#[test]
fn url_reference_is_attempted_before_screenshot_fallback() {
    let image = ChatAttachment {
        name: "reference.png".into(),
        media_type: "image/png".into(),
        data: vec![1, 2, 3],
    };
    let vision_response = r#"# Design System: Screenshot App
<<<SKELETON>>>
{"source":"screenshot","width":1440,"sections":[],"navKind":"none","heroKind":"none","columnRhythm":[]}"#;
    let root = r#"{"rootFrame":{"id":"root","name":"Page","width":375,"height":200,"layout":"vertical"},"subtasks":[{"id":"hero","label":"Hero","elements":"headline","region":{"width":375,"height":200}}]}"#;
    let subtask = r#"I(null,{"type":"frame","name":"Hero","x":0,"y":0,"width":375,"height":200,"children":[]});"#;
    let (provider, seen) = QueuedProvider::with_seen(&[vision_response, root, subtask]);
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    let snapshot = state.lock().expect("state").editor.clone();
    let hub = SseHub::default();
    let mut out = Vec::new();

    stream_new_design_route(
        &mut out,
        new_design_request("参考 http://127.0.0.1:1/ 和这张截图做首页", vec![image]),
        snapshot,
        Box::new(provider),
        None,
        CanvasWriteTarget {
            state: &state,
            hub: &hub,
            write_barrier: None,
        },
    )
    .expect("new-design route");

    let streamed = String::from_utf8(out).expect("utf8 SSE");
    assert!(streamed.contains("reference page could not be used: import URL is not allowed"));
    assert!(seen
        .lock()
        .expect("seen requests")
        .iter()
        .any(|request| request.attachments.len() == 1));
    assert_eq!(
        state
            .lock()
            .expect("state")
            .editor
            .doc
            .design_md
            .as_ref()
            .and_then(|spec| spec.project_name.as_deref()),
        Some("Screenshot App")
    );
}
