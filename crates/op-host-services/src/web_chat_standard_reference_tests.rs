use super::*;

use std::collections::VecDeque;
use std::sync::Mutex;

use op_ai::chat_provider::{
    ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason, ThinkingMode,
};
use op_editor_core::EditorState;

use crate::ai_proxy::AiStreamRequest;
use crate::web_canvas_server::{SseHub, WebCanvasState};

struct QueuedProvider {
    responses: std::sync::Mutex<VecDeque<String>>,
}

impl QueuedProvider {
    fn new(responses: &[&str]) -> Self {
        Self {
            responses: std::sync::Mutex::new(
                responses
                    .iter()
                    .map(|response| (*response).to_string())
                    .collect(),
            ),
        }
    }
}

impl ChatProvider for QueuedProvider {
    fn provider_label(&self) -> &str {
        "queued-reference-test"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
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

#[test]
fn rejected_reference_is_reported_and_new_design_still_runs() {
    let request = WebStandardTurnRequest {
        ai: AiStreamRequest {
            provider: None,
            builtin_provider_id: None,
            model: "test-model".into(),
            skills: Vec::new(),
            user: "参考 http://127.0.0.1:1/ 做首页".into(),
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
        attachments: Vec::new(),
        transient_builtin: None,
    };
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
