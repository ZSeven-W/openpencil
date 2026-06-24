//! Host-coupled test for the CLI standard-mode design route (GAP #33).
//!
//! The bulk of `chat_intent`'s tests are headless and live beside the
//! module in `op_host_services::chat_intent` (accessing its `#[cfg(test)]`
//! internals via `super`). This one test is the exception: it drives the
//! desktop GUI design-session pumps (`design_session::{pump_commands,
//! pump_progress}`, which take `&mut WidgetHostNative` — orphan rule) to
//! prove the new-design route clears the agent frame indicators after the
//! turn finishes. `op-host-services` links `op-host-native` with
//! default-features off, so `WidgetHostNative` (gl-host gated) is absent
//! there; the test therefore lives host-side.

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, StopReason};
use op_editor_core::EditorState;
use op_orchestrator::DesignRequest;

use op_editor_host_core::design::DesignSession;
use op_host_native::WidgetHostNative;
use op_host_services::chat_canvas_tools::chat_tool_channel;
use op_host_services::chat_intent::{run_cli_turn, CliTurnPlan};

use crate::design_session::{pump_commands, pump_progress};

// ---------------------------------------------------------------------------
// Scripted providers (copied from the headless chat_intent test sibling —
// `op-host-services` keeps its own copies for the cross-crate tests there).
// ---------------------------------------------------------------------------

struct Scripted {
    deltas: Vec<ChatDelta>,
    delay: Duration,
}

impl Scripted {
    fn text(s: &str) -> Self {
        Self {
            deltas: vec![
                ChatDelta::TextDelta(s.to_string()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
            delay: Duration::ZERO,
        }
    }
}

impl ChatProvider for Scripted {
    fn provider_label(&self) -> &str {
        "scripted"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let deltas = self.deltas.clone();
        let delay = self.delay;
        Box::new(deltas.into_iter().inspect(move |_| {
            std::thread::sleep(delay);
        }))
    }
}

struct ScriptedCalls {
    calls: std::sync::Mutex<std::collections::VecDeque<String>>,
}

impl ScriptedCalls {
    fn new(calls: Vec<String>) -> Self {
        Self {
            calls: std::sync::Mutex::new(calls.into()),
        }
    }
}

impl ChatProvider for ScriptedCalls {
    fn provider_label(&self) -> &str {
        "scripted-calls"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let text = self
            .calls
            .lock()
            .unwrap()
            .pop_front()
            .expect("scripted call exhausted");
        Box::new(
            vec![
                ChatDelta::TextDelta(text),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]
            .into_iter(),
        )
    }
}

// ---------------------------------------------------------------------------
// Fixtures (copied verbatim from the headless chat_intent test sibling).
// ---------------------------------------------------------------------------

fn test_design_request() -> DesignRequest {
    DesignRequest {
        prompt: "p".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
    }
}

const ONE_SUBTASK_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Login", "width": 390, "height": 844,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "form", "label": "Form", "region": { "width": 390, "height": 300 } }
  ]
}"##;

fn one_node_json() -> String {
    r#"[{"type":"frame","id":"form-1","name":"Form","width":390,"height":120,"children":[{"type":"text","id":"form-title","content":"Welcome","fontSize":24}]}]"#.into()
}

// ---------------------------------------------------------------------------
// The host-coupled new-design route test.
// ---------------------------------------------------------------------------

#[test]
fn cli_new_design_clears_agent_frame_indicators_after_done() {
    op_editor_core::agent_indicators::clear();
    let indicator_epoch = op_editor_core::agent_indicators::begin();
    let plan = CliTurnPlan {
        user_text: "design a login page".into(),
        page_children_empty: true,
        classify_provider: Box::new(Scripted::text("DESIGN_NEW")),
        chat_provider: Box::new(Scripted::text("unused")),
        design_provider: Box::new(ScriptedCalls::new(vec![
            ONE_SUBTASK_PLAN_JSON.into(),
            one_node_json(),
        ])),
        chat_request: ChatRequest::default(),
        modify_request: None,
        design_request: test_design_request(),
        initial_state: EditorState::new(),
        indicator_epoch,
        model: None,
    };
    let (chat_tx, _chat_rx) = mpsc::channel();
    let (executor, _tool_rx) = chat_tool_channel();
    let (delta_tx, delta_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let mut current = Some(DesignSession::from_channels_with_epoch(
        delta_rx,
        cmd_rx,
        indicator_epoch,
    ));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());

    let worker = thread::spawn(move || run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx));
    let deadline = Instant::now() + Duration::from_secs(10);
    while current.is_some() && Instant::now() < deadline {
        let _ = pump_commands(&mut host, &mut current, 1440.0, 900.0);
        let _ = pump_progress(&mut host, &mut current, None);
        std::thread::sleep(Duration::from_millis(2));
    }
    worker.join().expect("worker exits");

    assert!(current.is_none(), "design session should finish");
    let snapshot = op_editor_core::agent_indicators::snapshot();
    assert!(
        snapshot.frames.is_empty(),
        "agent frame badges/borders must clear after Done, got {:?}",
        snapshot.frames
    );
    op_editor_core::agent_indicators::clear();
}
