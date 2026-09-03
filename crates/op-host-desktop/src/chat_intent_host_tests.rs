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
use op_orchestrator::{AbortFlag, DesignRequest};

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
        // The orchestrator's retry ladder (attempt 2/3), geometry-echo
        // in-loop self-correction, and the end-of-run salvage pass can all
        // independently re-invoke the design provider for the SAME subtask
        // when an earlier attempt didn't land — real, by-design resilience
        // a live LLM always has something to say to (see
        // `op-orchestrator/src/concurrent.rs::run_subtask_retry_ladder` +
        // `maybe_geometry_echo`, `run.rs`'s salvage pass). A finite
        // scripted queue has no such reserve, so once exhausted this keeps
        // returning the LAST scripted line instead of panicking — this is
        // called from `ChatProviderLlmClient`'s per-call drain thread (one
        // new thread per attempt) whose `JoinHandle` is never joined, so a
        // panic here used to be silently contained EXCEPT for poisoning
        // `calls` for the rest of the process — starving any later,
        // legitimately-necessary call on this same instance.
        let mut calls = self.calls.lock().unwrap_or_else(|e| e.into_inner());
        if calls.len() > 1 {
            calls.pop_front();
        }
        let text = calls
            .front()
            .cloned()
            .expect("ScriptedCalls constructed with zero responses");
        drop(calls);
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
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
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

#[cfg_attr(
    target_os = "windows",
    ignore = "WINDOWS_WIDGET_HOST_NATIVE_TEST_ABORT: WidgetHostNative/Skia host tests abort in Windows CI; macOS and Linux keep coverage"
)]
#[test]
fn cli_new_design_clears_agent_frame_indicators_after_done() {
    // Held for the whole turn so an exact animation-deadline test can't
    // observe this run's streamed reveals mid-flight (shared registry).
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();
    let indicator_epoch = op_editor_core::agent_indicators::begin();
    let abort = AbortFlag::new();
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
        abort: abort.clone(),
        model: None,
    };
    let (chat_tx, _chat_rx) = mpsc::channel();
    let (executor, _tool_rx) = chat_tool_channel();
    let (delta_tx, delta_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let mut current = Some(DesignSession::from_channels_with_epoch_and_abort(
        delta_rx,
        cmd_rx,
        indicator_epoch,
        abort,
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
    // Graceful drain (finish_if_epoch): the frame ownership + reveal queue
    // persist after Done until the queued reveals play out, at which point
    // the paint-path maintenance clears the whole overlay. Drive time past
    // the drain window the way the render loop does, then assert it cleared
    // — the invariant is "no indicator leak after Done", not "cleared on the
    // same frame".
    let queued = op_editor_core::agent_indicators::snapshot();
    let drain_at = queued.reveals.values().copied().max().map_or(0, |last| {
        last + op_editor_core::agent_indicators::REVEAL_DURATION_MS + 1
    });
    let drained = op_editor_core::agent_indicators::snapshot_at(drain_at);
    assert!(
        drained.frames.is_empty() && drained.reveals.is_empty(),
        "agent frame ownership + reveals must clear once the drain plays out, got frames={:?} reveals={:?}",
        drained.frames,
        drained.reveals
    );
    op_editor_core::agent_indicators::clear();
}

/// Reproduces the real `app_handler.rs` per-frame sequence — `pump_commands`
/// → `pump_progress` → `design_loop_indicator::pump_design_session_indicator`
/// — for a CLI-launched new-design turn, and asserts the canvas radar-scan's
/// OWN activation gate (`AgentIndicators::frames` non-empty — see
/// `op_editor_ui::widgets::canvas_generation_scan::generating_paint_sets`,
/// which requires `!indicators.frames.is_empty()` before it paints anything
/// at all) actually fires while the turn is running.
///
/// The sibling test above only asserts the POST-drain "no leak" invariant —
/// it would pass unchanged even if `frames` were NEVER populated (empty
/// stays empty). This test is the missing "the scan gate actually opened"
/// assertion the bug report needed: it drives `pump_design_session_indicator`
/// (the ONLY caller of `agent_indicators::add_frame_if_absent`/`add_frame`
/// for the classic single-screen `Orchestrator::run` path — `run.rs` itself
/// never tags a root frame; only the multi-screen `run_screen_groups.rs`
/// scaffold does that directly) exactly like the real event loop does.
#[cfg_attr(
    target_os = "windows",
    ignore = "WINDOWS_WIDGET_HOST_NATIVE_TEST_ABORT: WidgetHostNative/Skia host tests abort in Windows CI; macOS and Linux keep coverage"
)]
#[test]
fn cli_new_design_populates_frame_indicators_the_canvas_scan_gates_on() {
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();
    let indicator_epoch = op_editor_core::agent_indicators::begin();
    let abort = AbortFlag::new();
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
        abort: abort.clone(),
        model: None,
    };
    let (chat_tx, _chat_rx) = mpsc::channel();
    let (executor, _tool_rx) = chat_tool_channel();
    let (delta_tx, delta_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let mut current = Some(DesignSession::from_channels_with_epoch_and_abort(
        delta_rx,
        cmd_rx,
        indicator_epoch,
        abort,
    ));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let mut design_indicator = None;

    let worker = thread::spawn(move || run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx));
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut saw_scan_gate_open = false;
    while current.is_some() && Instant::now() < deadline {
        let _ = pump_commands(&mut host, &mut current, 1440.0, 900.0);
        let _ = pump_progress(&mut host, &mut current, None);
        crate::design_loop_indicator::pump_design_session_indicator(
            &mut design_indicator,
            &current,
            host.editor_state_mut(),
            None,
        );
        if !op_editor_core::agent_indicators::snapshot()
            .frames
            .is_empty()
        {
            saw_scan_gate_open = true;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    worker.join().expect("worker exits");

    assert!(current.is_none(), "design session should finish");
    assert!(
        saw_scan_gate_open,
        "a CLI-launched new-design turn must tag its root frame via \
         agent_indicators::add_frame[_if_absent] at some point during the \
         run — the canvas radar-scan effect \
         (canvas_generation_scan::generating_paint_sets) never paints \
         anything for a run whose `frames` map stayed empty the whole time"
    );

    op_editor_core::agent_indicators::clear();
}
