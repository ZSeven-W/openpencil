//! OpenAI-compatible-wire half of the agent loop: streamed tool-call
//! accumulation (`OpenAiCollector`) and `run_openai_agent_loop`. Split out
//! of `chat_agent_loop.rs` to keep the spine under the 800-line cap; the
//! spine re-exports it so `chat_agent_loop::run_openai_agent_loop` still
//! resolves.

use super::*;

// ---------------------------------------------------------------------------
// OpenAI-compatible wire
// ---------------------------------------------------------------------------

// Streamed tool-call accumulation moved to `op_ai::chat_tool_sse` so the
// mobile FFI design loop shares this exact wire behavior; the alias keeps
// every existing `OpenAiCollector` path (including the wire tests) stable.
pub(super) use op_ai::chat_tool_sse::OpenAiToolCollector as OpenAiCollector;

/// Finalize-lifecycle invariant outer wrapper — same contract, same
/// rationale, as [`run_anthropic_agent_loop`]'s own wrapper above (this
/// loop's early-return sites mirror the Anthropic loop's exactly).
pub async fn run_openai_agent_loop(
    cfg: AgentLoopConfig,
    tx: &mpsc::Sender<ChatDelta>,
) -> Result<bool, BuiltinHttpError> {
    let executor = cfg.executor.clone();
    let enabled = cfg.finalize_on_exit;
    let result = run_openai_agent_loop_inner(cfg, tx).await;
    if result.is_err() {
        run_loop_finalize(&executor, enabled).await;
        emit_finalize_diagnostic(tx, enabled, "loop-exit").await;
    }
    result
}

/// Run the OpenAI-compatible agent loop to completion. Same contract
/// as [`run_anthropic_agent_loop_inner`].
pub(super) async fn run_openai_agent_loop_inner(
    cfg: AgentLoopConfig,
    tx: &mpsc::Sender<ChatDelta>,
) -> Result<bool, BuiltinHttpError> {
    let tools_json: Vec<Value> = cfg
        .tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": serde_json::from_str::<Value>(&t.input_schema_json)
                        .unwrap_or_else(|_| json!({ "type": "object" })),
                },
            })
        })
        .collect();
    let mut messages: Vec<Value> = Vec::new();
    if !cfg.system_prompt.trim().is_empty() {
        messages.push(json!({ "role": "system", "content": cfg.system_prompt }));
    }
    for (role, text) in &cfg.history {
        messages.push(json!({ "role": role.as_str(), "content": text }));
    }
    messages.push(json!({ "role": "user", "content": cfg.user_prompt }));

    // Tier 2 — under-budget per-screen fill-round budget, and Tier 2b —
    // post-exhaustion salvage budget — see the Anthropic loop above
    // (`FILL_BUDGET_MAX_ROUNDS_PER_SCREEN` / `SALVAGE_MAX_ROUNDS`'s doc
    // comments) for the full "budget never truncates committed work"
    // rationale and why these are two separate pools.
    let mut fill_attempts: HashMap<String, usize> = HashMap::new();
    let mut salvaged_screens: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut salvage_rounds_used = 0usize;
    // Tier 2c — unresolved-blocker corrective-round budget. A SEPARATE pool
    // from both of the above (see `chat_agent_loop_blockers::
    // BLOCKER_NUDGE_MAX_ROUNDS`'s doc comment): blockers and unfilled
    // screens are different failure modes with independent budgets.
    let mut blocker_rounds_used = 0usize;
    // Tier 2d — zero-write corrective-round budget; `wrote` flips on the
    // first successful write-level tool call (see `ZERO_WRITE_MAX_ROUNDS`).
    let mut wrote = false;
    let mut zero_write_rounds_used = 0usize;
    let mut write_retry = CorrectiveWriteRetry::default();
    let turn_cap = cfg.max_turns.max(1);
    let mut turn = 0usize;
    loop {
        // See the Anthropic path above: this one-shot request is model-authored
        // correction, not a blind replay, and owns its own hard budget.
        let corrective_write_round = write_retry.begin_round();
        if turn >= turn_cap && !corrective_write_round {
            if salvage_rounds_used >= SALVAGE_MAX_ROUNDS {
                let zero = zero_write_detail(cfg.finalize_on_exit, wrote, "turn budget exhausted");
                finalize_and_report(tx, &cfg.executor, cfg.finalize_on_exit, zero.as_deref()).await;
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::MaxTokens,
                    })
                    .await;
                return Ok(true);
            }
            let report = check_unfilled(&cfg.executor, cfg.finalize_on_exit).await;
            let eligible = salvage_eligible(&report.unfilled, &salvaged_screens);
            if eligible.is_empty() {
                let zero = zero_write_detail(cfg.finalize_on_exit, wrote, "turn budget exhausted");
                finalize_and_report(tx, &cfg.executor, cfg.finalize_on_exit, zero.as_deref()).await;
                let _ = tx
                    .send(ChatDelta::Done {
                        stop_reason: StopReason::MaxTokens,
                    })
                    .await;
                return Ok(true);
            }
            salvage_rounds_used += 1;
            for name in &eligible {
                salvaged_screens.insert(name.clone());
            }
            messages.push(
                json!({ "role": "user", "content": contract_nudge_text(&report.committed, &eligible) }),
            );
            // Falls through to send this dedicated salvage round below.
        }

        let mut body = json!({
            "model": cfg.model,
            "stream": true,
            "max_tokens": cfg.max_output_tokens,
            "messages": messages,
            "tools": tools_json,
        });
        // Turn OFF hidden reasoning for the families that can express it.
        // Without this a reasoning model's design turn spends its whole
        // `max_tokens` on `reasoning_content` and truncates the
        // `batch_design` mid-JSON. The failure is deceptively partial: the
        // read-only tools take `{}`-sized arguments and still fit in what
        // reasoning leaves behind, so the transcript shows a run of green
        // tool calls and then simply stops — measured with deepseek-v4-pro,
        // whose thinking defaults to `effort=high` (2026-07-31).
        //
        // The same helper builds the single-shot body. It maps Kimi K3 to
        // top-level `reasoning_effort:"low"`, while K2.5/K2.6, GLM,
        // DeepSeek, and MiniMax keep `thinking:{type:"disabled"}`. The two
        // mutually-exclusive fields can therefore never drift by path.
        crate::chat_builtin_http::apply_reasoning_wire_control(
            &mut body,
            &cfg.model,
            cfg.disable_thinking,
        );
        // Through the shared throttle/backoff: this tool-loop path used
        // to post raw, so a provider rate limit killed the design run with
        // no retries and a raw JSON error (measured: glm-5.2, 429
        // AccountRateLimitExceeded, 2026-07-12).
        let client = crate::provider_dial::client_for(cfg.dial_policy, &cfg.url).await?;
        let (max_retries, min_gap) = crate::chat_builtin_http::default_backoff_knobs();
        let resp = crate::chat_builtin_http::send_with_backoff(
            "openai-compatible",
            &cfg.url,
            max_retries,
            min_gap,
            || client.post(&cfg.url).bearer_auth(&cfg.api_key).json(&body),
        )
        .await?;
        let mut collector = OpenAiCollector::default();
        pump_sse(resp, tx, &mut collector).await?;
        if tx.is_closed() {
            return Ok(true);
        }
        if let Some(err) = collector.error {
            // Same in-stream provider failure as the Anthropic loop — see
            // `BuiltinHttpError::StreamReported`.
            return Err(BuiltinHttpError::StreamReported(err));
        }
        let calls = collector.pending_calls();
        if calls.is_empty() {
            if corrective_write_round {
                let _ = tx
                    .send(ChatDelta::TextDelta(CORRECTIVE_WRITE_EXHAUSTED.to_string()))
                    .await;
            }
            let reason = collector
                .finish_reason
                .as_deref()
                .map(map_openai_stop_reason)
                .unwrap_or(StopReason::EndTurn);
            let stop_content = || {
                if collector.text.is_empty() {
                    Value::Null
                } else {
                    Value::String(collector.text.clone())
                }
            };
            if turn >= turn_cap {
                // Already in salvage territory — the top-of-loop gate above
                // owns deciding whether another salvage round is owed.
                // Record this turn's reply and defer.
                messages.push(json!({ "role": "assistant", "content": stop_content() }));
                continue;
            }
            // Model voluntarily stopped, still within ordinary budget. Try
            // one dedicated fill round for any committed screen that's
            // still eligible — NOT gated by remaining turn budget (only by
            // the per-screen cap above).
            let report = check_unfilled(&cfg.executor, cfg.finalize_on_exit).await;
            let eligible = eligible_for_fill_round(&report.unfilled, &fill_attempts);
            if !eligible.is_empty() {
                spend_fill_round(&mut fill_attempts, &eligible);
                messages.push(json!({ "role": "assistant", "content": stop_content() }));
                messages.push(
                    json!({ "role": "user", "content": contract_nudge_text(&report.committed, &eligible) }),
                );
                continue; // Does not count against `turn`.
            }
            // No screen left to chase — try one dedicated corrective round
            // for any unresolved structural blocker, bounded by its own
            // `BLOCKER_NUDGE_MAX_ROUNDS` budget (see the Anthropic loop
            // above for the full rationale).
            if let Some(nudge) = blocker_nudge_if_owed(
                &cfg.executor,
                cfg.finalize_on_exit,
                &mut blocker_rounds_used,
            )
            .await
            {
                messages.push(json!({ "role": "assistant", "content": stop_content() }));
                messages.push(json!({ "role": "user", "content": nudge }));
                continue; // Does not count against `turn`.
            }
            // Zero-write guard (Tier 2d): the model stopped without ever
            // applying a write — the exact empty-canvas shape where a
            // starved/distracted model reads, narrates, and quits. One
            // dedicated contract round restates the build protocol; the
            // retry request re-applies the reasoning wire control like
            // every round, so a thinking-starved model actually has budget
            // to emit the write this time.
            if zero_write_round_owed(cfg.finalize_on_exit, wrote, zero_write_rounds_used) {
                zero_write_rounds_used += 1;
                messages.push(json!({ "role": "assistant", "content": stop_content() }));
                messages.push(json!({ "role": "user", "content": ZERO_WRITE_NUDGE }));
                continue; // Does not count against `turn`.
            }
            // Normal model-stop exit: run the Step-4 structural backstop ONCE
            // over the assembled doc BEFORE the Done delta. Tier 3 — honest
            // report — fires unconditionally if anything is still
            // unfilled/blocked.
            let stop_detail = collector
                .finish_reason
                .clone()
                .unwrap_or_else(|| "stop".into());
            let zero = zero_write_detail(cfg.finalize_on_exit, wrote, &stop_detail);
            finalize_and_report(tx, &cfg.executor, cfg.finalize_on_exit, zero.as_deref()).await;
            let _ = tx
                .send(ChatDelta::Done {
                    stop_reason: reason,
                })
                .await;
            return Ok(true);
        }
        if turn < turn_cap && !corrective_write_round {
            turn += 1;
        }

        let tool_calls_json: Vec<Value> = calls
            .iter()
            .map(|(_, call)| {
                json!({
                    "id": call.id,
                    "type": "function",
                    "function": { "name": call.name, "arguments": call.args_json },
                })
            })
            .collect();
        let content = if collector.text.is_empty() {
            Value::Null
        } else {
            Value::String(collector.text.clone())
        };
        messages.push(json!({
            "role": "assistant",
            "content": content,
            "tool_calls": tool_calls_json,
        }));
        let mut failed_write_tools = Vec::new();
        let mut corrective_write_seen = false;
        let mut corrective_write_failed = false;
        for (_, call) in &calls {
            let level = cfg.level_for(&call.name);
            let _ = tx
                .send(ChatDelta::ToolUse {
                    name: call.name.clone(),
                    args: tool_card_envelope(&level, &call.args_json),
                })
                .await;
            let result = execute_tool(&cfg.executor, &call.name, &call.args_json).await;
            if is_write_level(&level) && !result.is_error {
                wrote = true;
            }
            if is_write_level(&level) {
                if corrective_write_round {
                    corrective_write_seen = true;
                    corrective_write_failed |= result.is_error;
                } else if is_correctable_write_failure(&level, &result) {
                    failed_write_tools.push(call.name.clone());
                }
            }
            // OpenAI images follow the short role:tool acknowledgement.
            match prepare_screenshot_for_context(
                &cfg.model,
                cfg.max_output_tokens,
                &call.name,
                &result,
            ) {
                Some(PreparedScreenshot::Inline(b64)) => {
                    // Elide older images; keep intent, tool ids, and text.
                    for message in &mut messages {
                        elide_inline_screenshots(message);
                    }
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call.id,
                        "content": "Rendered screenshot attached as an image in the following message.",
                    }));
                    messages.push(json!({
                        "role": "user",
                        "content": [
                            { "type": "text", "text": "Rendered screenshot of the current design:" },
                            {
                                "type": "image_url",
                                "image_url": { "url": format!("data:image/png;base64,{b64}") },
                            },
                        ],
                    }));
                }
                Some(PreparedScreenshot::TextOnly) => messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call.id,
                    "content": TEXT_ONLY_SCREENSHOT_TEXT,
                })),
                Some(PreparedScreenshot::OverBudget) => messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call.id,
                    "content": OMITTED_SCREENSHOT_TEXT,
                })),
                None => {
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call.id,
                        "content": result.content,
                    }));
                }
            }
        }
        if corrective_write_round && (!corrective_write_seen || corrective_write_failed) {
            let _ = tx
                .send(ChatDelta::TextDelta(CORRECTIVE_WRITE_EXHAUSTED.to_string()))
                .await;
        } else if cfg.finalize_on_exit {
            if let Some(nudge) = write_retry.schedule(&failed_write_tools) {
                messages.push(json!({ "role": "user", "content": nudge }));
                let _ = tx
                    .send(ChatDelta::TextDelta(CORRECTIVE_WRITE_PROGRESS.to_string()))
                    .await;
            }
        }
    }
}
