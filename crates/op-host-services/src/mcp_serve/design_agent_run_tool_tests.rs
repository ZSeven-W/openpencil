//! `run_design_agent` unit tests — scripted loop drivers only, never network.

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use op_ai::chat_provider::{ChatDelta, ChatRequest, ChatToolExecutor, StopReason};
use op_editor_core::EditorState;
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};

use super::design_agent_run_tool::{DesignLoopDriver, HeadlessDesignExecutor, RunDesignAgentTool};

fn args(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// A driver that authors one frame through the REAL executor tool path
/// (`batch_design`), runs the real finalize, and ends the stream.
struct ScriptedDriver {
    executor: Arc<HeadlessDesignExecutor>,
}

impl DesignLoopDriver for ScriptedDriver {
    fn run(
        &self,
        _request: ChatRequest,
        _cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let program = serde_json::json!({
            "operations": "I(null, {type:'frame', name:'Landing', width:1200, height:800, fill:[{type:'solid', color:'#FFFFFF'}], layout:'vertical', children:[{type:'text', name:'Headline', content:'Hello', fontFamily:'Inter, system-ui, sans-serif', fontSize:32, fontWeight:700}]});"
        });
        let result = self.executor.execute("batch_design", &program.to_string());
        assert!(
            !result.is_error,
            "scripted batch_design must succeed: {}",
            result.content
        );
        let report = self.executor.finalize();
        assert!(report.quality.ran(), "finalize must run the quality passes");
        Box::new(
            [
                ChatDelta::ToolUse {
                    name: "batch_design".into(),
                    args: "{}".into(),
                },
                ChatDelta::TextDelta("done".into()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]
            .into_iter(),
        )
    }
}

/// A driver that never finishes; the tool's deadline must cancel it.
struct StalledDriver;

impl DesignLoopDriver for StalledDriver {
    fn run(
        &self,
        _request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        struct Stall {
            cancel: Arc<AtomicBool>,
        }
        impl Iterator for Stall {
            type Item = ChatDelta;
            fn next(&mut self) -> Option<ChatDelta> {
                loop {
                    if self.cancel.load(std::sync::atomic::Ordering::Acquire) {
                        return None;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
        Box::new(Stall { cancel })
    }
}

/// A driver that produces nothing and reports a provider error.
struct ErroringDriver;

impl DesignLoopDriver for ErroringDriver {
    fn run(
        &self,
        _request: ChatRequest,
        _cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(
            [
                ChatDelta::Error("401 Unauthorized".into()),
                ChatDelta::Done {
                    stop_reason: StopReason::Aborted,
                },
            ]
            .into_iter(),
        )
    }
}

#[test]
fn a_scripted_loop_lands_the_authored_design_as_one_batch() {
    let state = EditorState::new();
    let tool = RunDesignAgentTool::for_test(
        &state,
        Box::new(|executor| Box::new(ScriptedDriver { executor })),
    );
    let outcome = tool.call(&args(&[("brief", "landing page for a coffee brand")]));
    let ToolOutcome::OkJsonWithCommand(json, command) = outcome else {
        panic!("expected OkJsonWithCommand, got {outcome:?}");
    };
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("result json parses");
    assert_eq!(parsed["toolCalls"], 1);
    assert_eq!(parsed["stopReason"], "EndTurn");
    assert!(
        parsed["landedRoots"].as_u64().unwrap_or(0) >= 1,
        "at least one authored root lands: {json}"
    );
    assert!(
        parsed["finalize"].is_object(),
        "finalize report rides the result: {json}"
    );

    // The landing batch must replay onto the original state.
    let mut landed = state.clone();
    assert!(landed.apply(command), "landing batch applies");
    assert!(
        !landed.active_children().is_empty(),
        "landed document carries the authored design"
    );
}

#[test]
fn missing_brief_and_unknown_provider_are_structured_errors() {
    let state = EditorState::new();
    let tool = RunDesignAgentTool::for_test(
        &state,
        Box::new(|executor| Box::new(ScriptedDriver { executor })),
    );
    let ToolOutcome::Err(code, message) = tool.call(&args(&[])) else {
        panic!("missing brief must be an error");
    };
    assert_eq!(code, ToolErrorCode::InvalidArgument);
    assert!(message.contains("brief"), "{message}");

    // Production provider resolution: an empty settings store is the
    // structured no-provider error, never a panic or a network dial.
    let production = super::design_agent_run_tool::run_design_agent_snapshot(&state);
    let ToolOutcome::Err(code, message) = production.call(&args(&[("brief", "a page")])) else {
        panic!("missing provider must be an error");
    };
    assert_eq!(code, ToolErrorCode::InvalidArgument);
    assert!(
        message.contains("no builtin agent provider is configured"),
        "{message}"
    );

    let ToolOutcome::Err(code, message) =
        production.call(&args(&[("brief", "a page"), ("provider_id", "ghost")]))
    else {
        panic!("unknown provider id must be an error");
    };
    assert_eq!(code, ToolErrorCode::InvalidArgument);
    assert!(message.contains("ghost"), "{message}");
}

#[test]
fn a_stalled_loop_times_out_and_leaves_no_command() {
    let state = EditorState::new();
    let tool = RunDesignAgentTool::for_test(&state, Box::new(|_| Box::new(StalledDriver)));
    let outcome = tool.call(&args(&[("brief", "a page"), ("timeout_seconds", "1")]));
    let ToolOutcome::Err(code, message) = outcome else {
        panic!("stalled loop must time out, got {outcome:?}");
    };
    assert_eq!(code, ToolErrorCode::ToolFailed);
    assert!(message.contains("1s budget"), "{message}");
}

#[test]
fn a_provider_error_with_no_authored_result_is_terminal_and_unchanged() {
    let state = EditorState::new();
    let tool = RunDesignAgentTool::for_test(&state, Box::new(|_| Box::new(ErroringDriver)));
    let outcome = tool.call(&args(&[("brief", "a page")]));
    let ToolOutcome::Err(code, message) = outcome else {
        panic!("erroring loop must fail, got {outcome:?}");
    };
    assert_eq!(code, ToolErrorCode::ToolFailed);
    assert!(message.contains("401 Unauthorized"), "{message}");
}
