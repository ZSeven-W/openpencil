//! OpenAI-compatible reasoning-control and mixed-delta wire regressions.

use super::tests::{run_loop_collect, serve_sse_script, update_node_tool_def, ScriptedExecutor};
use super::*;
use serde_json::Value;

fn text_turn() -> String {
    [
        r#"data: {"choices":[{"delta":{"content":"done"}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n")
}

fn request_body(request: &str) -> Value {
    let body_start = request
        .find("\r\n\r\n")
        .map(|index| index + 4)
        .expect("request body separator");
    serde_json::from_str(&request[body_start..]).expect("request body JSON")
}

fn capture_agent_body(model: &str) -> Value {
    let (base, requests) = serve_sse_script(vec![text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let cfg = AgentLoopConfig {
        url: format!("{base}/chat/completions"),
        api_key: "sk-test".into(),
        model: model.into(),
        system_prompt: "You are a design editor.".into(),
        history: Vec::new(),
        user_prompt: "continue the design".into(),
        max_output_tokens: 512,
        tools: vec![update_node_tool_def()],
        executor,
        max_turns: 2,
        finalize_on_exit: false,
        disable_thinking: true,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    };
    let (outcome, deltas) = run_loop_collect(cfg, false);
    assert_eq!(outcome, Ok(true), "model={model}, deltas={deltas:?}");
    request_body(&requests.recv().expect("captured agent-loop request"))
}

#[test]
fn agent_loop_uses_the_same_mutually_exclusive_reasoning_controls_as_classic() {
    let k3 = capture_agent_body("kimi-k3");
    assert_eq!(k3["reasoning_effort"], "low");
    assert!(k3.get("thinking").is_none(), "K3 request: {k3}");

    for model in ["kimi-k2.5", "kimi-k2.6", "glm-5.2", "deepseek-v4-pro"] {
        let body = capture_agent_body(model);
        assert_eq!(body["thinking"]["type"], "disabled", "model={model}");
        assert!(
            body.get("reasoning_effort").is_none(),
            "model={model}: {body}"
        );
    }
}

#[test]
fn agent_loop_mixed_reasoning_and_content_delta_emits_both_in_order() {
    let mut collector = OpenAiCollector::default();
    let deltas = collector.handle(
        r#"{"choices":[{"delta":{"reasoning_content":"plan","content":"batch_design(...)"}}]}"#,
    );
    assert_eq!(
        deltas,
        vec![
            ChatDelta::Thinking("plan".into()),
            ChatDelta::TextDelta("batch_design(...)".into()),
        ]
    );
}
