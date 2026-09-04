//! Screenshot-replay, fill-budget and salvage-round accounting tests for
//! the agent loop, plus the tool-card envelope shape. Split out of `chat_agent_loop_tests.rs` at
//! the 800-line cap; nested under that module so `use super::*` still
//! reaches its scripted-executor + loopback-SSE helpers.

use super::*;

#[test]
fn openai_loop_replays_screenshot_result_as_image_url_part() {
    let shot_turn = [
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_shot","type":"function","function":{"name":"get_screenshot","arguments":"{\"nodeId\":\"root\"}"}}]}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n");
    let text_turn = [
        r#"data: {"choices":[{"delta":{"content":"Looks good."}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n");
    // Third body: the zero-write guard's corrective round (a screenshot is
    // a read; this run never writes) — it stops again and the loop exits
    // with the honest zero-write report.
    let (base, req_rx) = serve_sse_script(vec![shot_turn, text_turn.clone(), text_turn]);
    let screenshot_result = serde_json::json!({
        "success": true,
        "data": { "image_base64": TINY_PNG_B64, "format": "png" }
    })
    .to_string();
    let executor = ScriptedExecutor::ok(&screenshot_result);
    let cfg = AgentLoopConfig {
        url: format!("{base}/chat/completions"),
        api_key: "sk-test".into(),
        model: "gpt-test".into(),
        system_prompt: "You are a design editor.".into(),
        history: Vec::new(),
        user_prompt: "render and check".into(),
        max_output_tokens: 512,
        tools: vec![get_screenshot_tool_def()],
        executor: executor.clone(),
        max_turns: 5,
        finalize_on_exit: true,
        disable_thinking: false,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    };
    let (outcome, _deltas) = run_loop_collect(cfg, false);
    assert_eq!(outcome, Ok(true));
    assert_eq!(executor.calls().len(), 1);

    let _first = req_rx.recv().expect("first request captured");
    let second = req_rx.recv().expect("second request captured");
    let body_start = second.find("\r\n\r\n").map(|i| i + 4).expect("body");
    let body: Value = serde_json::from_str(&second[body_start..]).expect("body JSON");
    let messages = body["messages"].as_array().expect("messages");

    // The role:"tool" message holds only a short text ack (string).
    let tool_msg = messages
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("tool message");
    assert!(
        tool_msg["content"].is_string(),
        "openai tool content must stay a string"
    );

    // A follow-up role:"user" message carries the image as an image_url
    // data URL — the only OpenAI-wire way to make the model see the render.
    let img_user = messages
        .iter()
        .filter(|m| m["role"] == "user")
        .find_map(|m| m["content"].as_array())
        .expect("a user message with multimodal content parts");
    let part = img_user
        .iter()
        .find(|p| p["type"] == "image_url")
        .expect("an image_url part");
    let url = part["image_url"]["url"].as_str().expect("data url");
    let expected_prefix = "data:image/png;base64,";
    assert!(url.starts_with(expected_prefix), "got {url}");
    assert_eq!(&url[expected_prefix.len()..], TINY_PNG_B64);
}

#[test]
fn openai_loop_keeps_only_latest_screenshot_without_dropping_user_intent() {
    let shot_turn = |id: &str| {
        [
            format!(
                r#"data: {{"choices":[{{"delta":{{"tool_calls":[{{"index":0,"id":"{id}","type":"function","function":{{"name":"get_screenshot","arguments":"{{\"nodeId\":\"root\"}}"}}}}]}}}}]}}"#
            ),
            String::new(),
            r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#.into(),
            String::new(),
            "data: [DONE]".into(),
            String::new(),
            String::new(),
        ]
        .join("\n")
    };
    let text_turn = [
        r#"data: {"choices":[{"delta":{"content":"Visual check complete."}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n");
    let (base, req_rx) = serve_sse_script(vec![
        shot_turn("call_shot_1"),
        shot_turn("call_shot_2"),
        text_turn.clone(),
        // The zero-write guard's corrective round — reads only, no writes.
        text_turn,
    ]);
    let first_screenshot_result = serde_json::json!({
        "success": true,
        "data": { "image_base64": TINY_PNG_B64, "format": "png" }
    })
    .to_string();
    let second_screenshot_result = serde_json::json!({
        "success": true,
        "data": { "image_base64": SECOND_TINY_PNG_B64, "format": "png" }
    })
    .to_string();
    let executor = ScriptedExecutor::sequence(&[
        first_screenshot_result.as_str(),
        second_screenshot_result.as_str(),
    ]);
    let cfg = AgentLoopConfig {
        url: format!("{base}/chat/completions"),
        api_key: "sk-test".into(),
        model: "MiniMax-M3".into(),
        system_prompt: "You are a design editor.".into(),
        history: Vec::new(),
        user_prompt: "Build the exact coffee landing page I requested".into(),
        max_output_tokens: 6_144,
        tools: vec![get_screenshot_tool_def()],
        executor,
        max_turns: 5,
        finalize_on_exit: true,
        disable_thinking: false,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    };
    let (outcome, _deltas) = run_loop_collect(cfg, false);
    assert_eq!(outcome, Ok(true));

    let _first = req_rx.recv().expect("initial request");
    let _second = req_rx.recv().expect("first screenshot request");
    let third = req_rx.recv().expect("second screenshot request");
    let body_start = third.find("\r\n\r\n").map(|i| i + 4).expect("body");
    let body: Value = serde_json::from_str(&third[body_start..]).expect("body JSON");
    let wire = body["messages"].to_string();

    assert_eq!(
        wire.matches("data:image/png;base64,").count(),
        1,
        "only the newest visual observation may ride the next request"
    );
    assert!(
        !wire.contains(TINY_PNG_B64),
        "the superseded first screenshot payload must be absent"
    );
    assert_eq!(wire.matches(SECOND_TINY_PNG_B64).count(), 1);
    assert!(wire.contains(crate::chat_agent_context::ELIDED_SCREENSHOT_TEXT));
    assert!(
        wire.contains("Build the exact coffee landing page I requested"),
        "context compaction must preserve the user's design intent"
    );
    assert!(wire.contains("call_shot_1"), "tool identity is preserved");
    assert!(
        wire.contains("call_shot_2"),
        "latest tool call is preserved"
    );
}

// ── "承诺-交付" invariant: dedicated fill-round budget + tier-3 honest report ──
// Budget only guards a runaway retry avalanche — it must never itself be the
// reason a promised screen ships empty (user policy upgrade, 2026-07-18).
// Dedicated fill rounds are therefore exempt from `max_turns`; only a
// per-screen cap (`FILL_BUDGET_MAX_ROUNDS_PER_SCREEN = 2`) stops the SAME
// screen from being nudged forever.

#[test]
fn model_stop_with_budget_left_gets_a_fill_round_then_reports_nothing_once_filled() {
    // Turn 1: model stops without calling any tool (calls.is_empty()) while
    // budget remains (max_turns: 3). The cheap check finds "Saved" unfilled
    // → the loop injects one dedicated fill round instead of finalizing
    // immediately. Turn 2: model stops again; this time the check reports
    // nothing left unfilled (the fill round "worked") → straight to
    // finalize, which also finds nothing left.
    // Leading write turn keeps the zero-write guard out of scope — this
    // test's subject is the fill-round tier alone.
    let (base, req_rx) = serve_sse_script(vec![
        anthropic_tool_use_turn(),
        anthropic_text_turn(),
        anthropic_text_turn(),
    ]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#)
        .with_unfilled_check(&["Trips", "Destination", "Saved"], &["Saved"])
        .with_unfilled_check(&["Trips", "Destination", "Saved"], &[]);
    let cfg = AgentLoopConfig {
        url: format!("{base}/v1/messages"),
        api_key: "sk-test".into(),
        model: "claude-test".into(),
        system_prompt: "You are a design editor.".into(),
        history: Vec::new(),
        user_prompt: "build the 3-screen app".into(),
        max_output_tokens: 512,
        tools: vec![update_node_tool_def()],
        executor: executor.clone(),
        max_turns: 3,
        finalize_on_exit: true,
        disable_thinking: false,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    };
    let (outcome, deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));

    // One fill-round probe finds it, a second confirms it's fixed; finalize
    // still runs exactly once at the real exit.
    assert_eq!(executor.unfilled_checks(), 2);
    assert_eq!(
        executor.finalizes(),
        1,
        "finalize still runs exactly once, at the real exit"
    );

    // The fill round's contract line — the FULL commitment, not just the
    // gap — actually rode the follow-up request as a real turn.
    let _first = req_rx.recv().expect("first request captured");
    let _second = req_rx.recv().expect("post-write request captured");
    let third = req_rx
        .recv()
        .expect("fill-round follow-up request captured");
    assert!(
        third.contains("You committed 3 screens (Trips/Destination/Saved)")
            && third.contains("Saved is still empty")
            && third.contains("Complete it before finishing"),
        "the fill round's contract line must state the full commitment, got: {third}"
    );

    // Nothing left unfilled after the fill round → no tier-3 report line.
    assert!(
        !deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains("left unfilled"))),
        "a screen the fill round filled must not also be reported as unfilled"
    );
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn
        })
    ));
}

#[test]
fn model_stop_repeatedly_failing_the_same_screen_is_capped_then_honestly_reported() {
    // The model voluntarily stops 3 times in a row, and "Saved" is STILL
    // unfilled every time — the per-screen cap (2 dedicated rounds) must
    // stop trying after the 2nd nudge and accept the failure on the 3rd
    // check, rather than nudging forever. Turn budget (max_turns: 5) is
    // deliberately generous so ONLY the per-screen cap — not the ordinary
    // turn budget — is what ends the retrying.
    let (base, _req_rx) = serve_sse_script(vec![
        anthropic_tool_use_turn(),
        anthropic_text_turn(),
        anthropic_text_turn(),
        anthropic_text_turn(),
    ]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#)
        .with_unfilled_check(&["Saved"], &["Saved"])
        .with_unfilled_check(&["Saved"], &["Saved"])
        .with_unfilled_check(&["Saved"], &["Saved"])
        .with_unfilled_finalize(&["Saved"], &["Saved"]);
    let cfg = AgentLoopConfig {
        url: format!("{base}/v1/messages"),
        api_key: "sk-test".into(),
        model: "claude-test".into(),
        system_prompt: "You are a design editor.".into(),
        history: Vec::new(),
        user_prompt: "build the 3-screen app".into(),
        max_output_tokens: 512,
        tools: vec![update_node_tool_def()],
        executor: executor.clone(),
        max_turns: 5,
        finalize_on_exit: true,
        disable_thinking: false,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    };
    let (outcome, deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));

    // 3 checks: round 1 (eligible, nudge), round 2 (still eligible, nudge),
    // round 3 (attempts == cap, no longer eligible → stop trying).
    assert_eq!(
        executor.unfilled_checks(),
        3,
        "the per-screen cap, not the ordinary turn budget, is what stops the retrying here"
    );
    assert_eq!(executor.finalizes(), 1);

    let report = deltas.iter().find_map(|d| match d {
        ChatDelta::TextDelta(s) if s.contains("left unfilled") => Some(s.clone()),
        _ => None,
    });
    assert!(
        report.is_some_and(|s| s.contains("Saved")),
        "a screen still unfilled after exhausting its dedicated fill budget must be reported by name, got: {deltas:?}"
    );
}

#[test]
fn turn_cap_exhausted_with_unfilled_committed_screen_gets_exactly_one_salvage_round() {
    // The model calls a tool on every turn until the ordinary budget
    // (max_turns: 2) is exhausted — the 0718-1-glm-1 incident's shape. Per
    // the "budget only guards runaway, never truncates committed work"
    // policy (2026-07-18 upgrade): running out of ordinary budget must NOT
    // itself be the reason "Saved" ships empty. The salvage pool is
    // SEPARATE from the under-budget fill pool and grants each unfilled
    // committed screen exactly ONE dedicated round (not the richer 2-round
    // budget) — bundling every eligible screen into one contract message
    // means this converges after exactly one salvage request here.
    let (base, req_rx) = serve_sse_script(vec![
        anthropic_tool_use_turn(), // turn 1/2 — normal budget
        anthropic_tool_use_turn(), // turn 2/2 — normal budget exhausted
        anthropic_text_turn(),     // the ONE dedicated salvage round
    ]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#)
        .with_unfilled_check(&["Saved"], &["Saved"])
        .with_unfilled_check(&["Saved"], &["Saved"])
        .with_unfilled_finalize(&["Saved"], &["Saved"]);
    let cfg = AgentLoopConfig {
        url: format!("{base}/v1/messages"),
        api_key: "sk-test".into(),
        model: "claude-test".into(),
        system_prompt: String::new(),
        history: Vec::new(),
        user_prompt: "build the 3-screen app".into(),
        max_output_tokens: 128,
        tools: vec![update_node_tool_def()],
        executor: executor.clone(),
        max_turns: 2,
        finalize_on_exit: true,
        disable_thinking: false,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    };
    let (outcome, deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));

    // 2 real tool-call turns consumed the ordinary budget, THEN exactly one
    // more request went out as the dedicated salvage round (uncounted
    // against max_turns) before the screen's own 1-round salvage cap
    // finally stopped the retrying.
    assert_eq!(executor.calls().len(), 2, "the 2 ordinary tool-call turns");
    for _ in 0..3 {
        req_rx
            .recv()
            .expect("all 3 requests, including the ONE dedicated salvage round, went out");
    }
    assert_eq!(
        executor.unfilled_checks(),
        2,
        "one check discovers the salvage-eligible screen; one more (next pass) confirms it's already been salvaged"
    );
    assert_eq!(executor.finalizes(), 1);
    assert!(
        deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains("left unfilled") && s.contains("Saved"))),
        "a screen still unfilled after its ONE salvage round must still get the honest report"
    );
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::MaxTokens
        })
    ));
}

#[test]
fn turn_cap_exhausted_without_unfilled_committed_screens_spends_zero_salvage_rounds() {
    // Nothing committed was ever left unfilled at exhaustion — the salvage
    // pool must never fire speculatively ("无未填屏 → 零专款"). Only the 2
    // ordinary tool-call turns' worth of requests may go out; a 3rd request
    // would mean a wasted salvage round nobody asked for.
    let (base, req_rx) =
        serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_tool_use_turn()]);
    let executor =
        ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#).with_unfilled_check(&[], &[]);
    let cfg = AgentLoopConfig {
        url: format!("{base}/v1/messages"),
        api_key: "sk-test".into(),
        model: "claude-test".into(),
        system_prompt: String::new(),
        history: Vec::new(),
        user_prompt: "build the 3-screen app".into(),
        max_output_tokens: 128,
        tools: vec![update_node_tool_def()],
        executor: executor.clone(),
        max_turns: 2,
        finalize_on_exit: true,
        disable_thinking: false,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    };
    let (outcome, deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));

    assert_eq!(executor.calls().len(), 2);
    let _first = req_rx.recv().expect("turn 1");
    let _second = req_rx.recv().expect("turn 2");
    assert!(
        req_rx.recv().is_err(),
        "no 3rd (salvage) request when nothing committed was ever left unfilled"
    );
    assert_eq!(
        executor.unfilled_checks(),
        1,
        "exactly one check, which found nothing worth salvaging"
    );
    assert_eq!(executor.finalizes(), 1);
    assert!(
        !deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains("left unfilled"))),
        "nothing to report when nothing was ever unfilled"
    );
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::MaxTokens
        })
    ));
}

#[test]
fn tool_card_envelope_wraps_args_with_level_and_running_status() {
    let envelope = tool_card_envelope("modify", r#"{"nodeId":"n1"}"#);
    let v: Value = serde_json::from_str(&envelope).unwrap();
    assert_eq!(v["level"], "modify");
    assert_eq!(v["status"], "running");
    assert_eq!(v["args"]["nodeId"], "n1");
    // Unparseable args degrade to a string payload instead of panicking.
    let envelope = tool_card_envelope("read", "not-json");
    let v: Value = serde_json::from_str(&envelope).unwrap();
    assert_eq!(v["args"], "not-json");
}
