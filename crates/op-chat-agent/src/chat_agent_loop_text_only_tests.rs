//! Wire-level screenshot capability tests for OpenAI-compatible text models.

use super::*;

fn screenshot_tool_turn() -> String {
    [
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_shot","type":"function","function":{"name":"get_screenshot","arguments":"{\"nodeId\":\"root\"}"}}]}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n")
}

fn text_turn() -> String {
    [
        r#"data: {"choices":[{"delta":{"content":"Checked structurally."}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n")
}

fn request_body(request: &str) -> serde_json::Value {
    let body_start = request
        .find("\r\n\r\n")
        .map(|index| index + 4)
        .expect("request body separator");
    serde_json::from_str(&request[body_start..]).expect("request body JSON")
}

#[test]
fn glm_and_deepseek_second_requests_never_carry_screenshot_base64() {
    for model in ["glm-5.2", "deepseek-v4-pro"] {
        let (base, requests) = serve_sse_script(vec![screenshot_tool_turn(), text_turn()]);
        let screenshot_result = serde_json::json!({
            "success": true,
            "data": { "image_base64": TINY_PNG_B64, "format": "png" }
        })
        .to_string();
        let executor = ScriptedExecutor::ok(&screenshot_result);
        let cfg = AgentLoopConfig {
            url: format!("{base}/chat/completions"),
            api_key: "sk-test".into(),
            model: model.into(),
            system_prompt: "You are a design editor.".into(),
            history: Vec::new(),
            user_prompt: "render and check".into(),
            max_output_tokens: 6_144,
            tools: vec![get_screenshot_tool_def()],
            executor: executor.clone(),
            max_turns: 5,
            finalize_on_exit: false,
            disable_thinking: true,
            dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
        };

        let (outcome, _deltas) = run_loop_collect(cfg, false);
        assert_eq!(outcome, Ok(true), "model={model}");
        assert_eq!(executor.calls().len(), 1, "model={model}");
        let _first = requests.recv().expect("initial request");
        let second = requests.recv().expect("tool-result request");
        let body = request_body(&second);
        let wire = body["messages"].to_string();

        assert!(
            wire.contains(crate::chat_agent_context::TEXT_ONLY_SCREENSHOT_TEXT),
            "model={model}: {wire}"
        );
        assert!(!wire.contains(TINY_PNG_B64), "model={model}: {wire}");
        assert!(
            !wire.contains("data:image/png;base64,"),
            "model={model}: {wire}"
        );
        assert!(!wire.contains("image_url"), "model={model}: {wire}");
    }
}
