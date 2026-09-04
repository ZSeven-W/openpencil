//! Anthropic-wire half of the agent loop: per-block SSE accumulation
//! (`AnthropicCollector`) and `run_anthropic_agent_loop`. Split out of
//! `chat_agent_loop.rs` to keep the spine under the 800-line cap; the spine
//! re-exports it so `chat_agent_loop::run_anthropic_agent_loop` still
//! resolves.

use super::*;

// ---------------------------------------------------------------------------
// Anthropic wire
// ---------------------------------------------------------------------------

// Per-block streamed accumulation moved to `op_ai::chat_tool_sse` so the
// mobile FFI design loop shares this exact wire behavior; the alias keeps
// every existing `AnthropicCollector` path stable.
pub(super) use op_ai::chat_tool_sse::AnthropicToolCollector as AnthropicCollector;

/// Finalize-lifecycle invariant outer wrapper (0718-1-k3-1 postmortem — see
/// [`run_loop_finalize`]'s doc + `op-host-desktop::chat_session::
/// finalize_design_session_if_needed`'s doc for the desktop-side half of
/// this same invariant).
///
/// ## The actual break, confirmed by reading this file
///
/// [`run_anthropic_agent_loop_inner`] / [`run_openai_agent_loop_inner`] each
/// call [`run_loop_finalize`] on every NORMAL exit (turn-cap exhausted,
/// salvage exhausted, model voluntarily stopped) — but FOUR early-return
/// sites per loop bypass it entirely: `client_for(...).await?`,
/// `send_with_backoff(...).await?`, `pump_sse(...).await?` (each via `?`
/// propagation), and the explicit `if let Some(err) = collector.error {
/// return Err(err); }` right after — the last one is what the 0718-1-k3-1
/// transcript's mid-stream `openai-compatible http 400` line hit: `pump_sse`
/// itself returns `Ok(())` (it drained the stream fine), but the collected
/// SSE-level error makes the caller return `Err` one line later, never
/// reaching the loop's own finalize call below it. `chat_builtin_http.rs`'s
/// `Err(e) => { tx.send(Error); tx.send(Done{Aborted}); }` catch-all never
/// finalizes either — it only forwards the error.
///
/// This wrapper is the single place ALL of a loop run's exits funnel
/// through, so it is where the invariant is actually enforced: on `Err`,
/// run the SAME best-effort finalize the inner loop's own normal-exit paths
/// already do, tagged `loop-exit` so a future occurrence is locatable by
/// grepping for that tag. Idempotent — [`run_loop_finalize`] no-ops when
/// `!cfg.finalize_on_exit`, and `apply_loop_finalize`'s own passes are each
/// individually idempotent, so this never double-mutates a document that an
/// inner normal-exit path already finalized (those paths return `Ok`, so
/// this wrapper's backstop only fires on the exact paths that skipped it).
pub async fn run_anthropic_agent_loop(
    cfg: AgentLoopConfig,
    tx: &mpsc::Sender<ChatDelta>,
) -> Result<bool, BuiltinHttpError> {
    let executor = cfg.executor.clone();
    let enabled = cfg.finalize_on_exit;
    let result = run_anthropic_agent_loop_inner(cfg, tx).await;
    if result.is_err() {
        run_loop_finalize(&executor, enabled).await;
        emit_finalize_diagnostic(tx, enabled, "loop-exit").await;
    }
    result
}

/// Run the Anthropic agent loop to completion. Returns `Ok(true)` when
/// a terminal `Done` was emitted; `Err` for transport / in-stream
/// errors (caller surfaces them as `Error + Done{Aborted}`).
/// Assistant `content[]` for the follow-up request. A thinking-only turn
/// accumulates NO replayable blocks (thinking is deliberately not replayed
/// — see `AnthropicToolCollector::assistant_content`), and Anthropic
/// rejects `content: []` with a 400 — which would poison the exact
/// corrective rounds that exist to rescue such a turn. Synthesize a
/// minimal text block instead.
fn assistant_turn_content(collector: &AnthropicCollector) -> Value {
    let content = collector.assistant_content();
    if content.is_empty() {
        json!([{ "type": "text", "text": "(no visible output this turn)" }])
    } else {
        json!(content)
    }
}

pub(super) async fn run_anthropic_agent_loop_inner(
    cfg: AgentLoopConfig,
    tx: &mpsc::Sender<ChatDelta>,
) -> Result<bool, BuiltinHttpError> {
    let tools_json: Vec<Value> = cfg
        .tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": serde_json::from_str::<Value>(&t.input_schema_json)
                    .unwrap_or_else(|_| json!({ "type": "object" })),
            })
        })
        .collect();
    let mut messages: Vec<Value> = Vec::new();
    for (role, text) in &cfg.history {
        messages.push(json!({ "role": role.as_str(), "content": text }));
    }
    messages.push(json!({ "role": "user", "content": cfg.user_prompt }));

    // Tier 2 — under-budget per-screen fill-round budget (see
    // `FILL_BUDGET_MAX_ROUNDS_PER_SCREEN`'s doc comment for the "budget
    // never truncates committed work" rationale).
    let mut fill_attempts: HashMap<String, usize> = HashMap::new();
    // Tier 2b — post-exhaustion salvage budget. A SEPARATE pool from
    // `fill_attempts` above (see `SALVAGE_MAX_ROUNDS`'s doc comment).
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
        // A failed design write gets one model-authored correction request,
        // even when that failure consumed the final ordinary turn. This is a
        // separate one-shot budget; the host never replays tool arguments.
        let corrective_write_round = write_retry.begin_round();
        // Ordinary turn budget exhausted — the ONLY reason to send another
        // request is a committed screen that's still eligible for its own
        // dedicated salvage round. This request, if sent, does NOT count
        // against `turn`; it draws from `SALVAGE_MAX_ROUNDS` instead.
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
                // Turn cap reached, and nothing committed is left worth
                // trying for — either nothing was ever unfilled, or every
                // unfilled screen already spent its one salvage round. Still
                // run the Step-4 structural backstop once over whatever the
                // run assembled, and report unconditionally.
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
            // Falls through to send this dedicated salvage round below —
            // bundling every eligible screen into one message means this
            // branch converges (nothing left eligible) after exactly one
            // round for the common case.
        }

        let mut body = json!({
            "model": cfg.model,
            "max_tokens": cfg.max_output_tokens,
            "stream": true,
            "messages": messages,
            "tools": tools_json,
        });
        if !cfg.system_prompt.trim().is_empty() {
            body.as_object_mut()
                .expect("anthropic request body is object")
                .insert("system".into(), json!(cfg.system_prompt));
        }
        // Turn OFF hidden reasoning for the families that can express it on
        // this wire — the empty-canvas postmortem: the OpenAI-compat loop
        // applied this control and the Anthropic loop did not, so a
        // reasoning model on its Anthropic-compatible endpoint burned the
        // whole turn budget on thinking and never emitted `batch_design`
        // (see `apply_reasoning_wire_control_anthropic`'s doc).
        crate::chat_builtin_http::apply_reasoning_wire_control_anthropic(
            &mut body,
            &cfg.model,
            cfg.disable_thinking,
        );
        let client = crate::provider_dial::client_for(cfg.dial_policy, &cfg.url).await?;
        let (max_retries, min_gap) = crate::chat_builtin_http::default_backoff_knobs();
        let resp = crate::chat_builtin_http::send_with_backoff(
            "anthropic",
            &cfg.url,
            max_retries,
            min_gap,
            || {
                client
                    .post(&cfg.url)
                    .header("x-api-key", &cfg.api_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&body)
            },
        )
        .await?;
        let mut collector = AnthropicCollector::default();
        pump_sse(resp, tx, &mut collector).await?;
        if tx.is_closed() {
            return Ok(true);
        }
        if let Some(err) = collector.error {
            // An HTTP-200 body that announced its own failure — see
            // `BuiltinHttpError::StreamReported`. The provider's message
            // rides verbatim, as it did when this was a bare `String`.
            return Err(BuiltinHttpError::StreamReported(err));
        }
        let calls = collector.tool_calls();
        if calls.is_empty() {
            if corrective_write_round {
                let _ = tx
                    .send(ChatDelta::TextDelta(CORRECTIVE_WRITE_EXHAUSTED.to_string()))
                    .await;
            }
            let reason = collector
                .stop_reason
                .as_deref()
                .map(map_anthropic_stop_reason)
                .unwrap_or(StopReason::EndTurn);
            if turn >= turn_cap {
                // Already in salvage territory — the top-of-loop gate above
                // owns deciding whether another salvage round is owed (it
                // re-checks fresh on the next pass, against `salvaged_screens`
                // / `salvage_rounds_used`). Record this turn's reply in
                // history and defer, rather than deciding again here —
                // deciding in both places would double-nudge.
                messages.push(
                    json!({ "role": "assistant", "content": assistant_turn_content(&collector) }),
                );
                continue;
            }
            // Model voluntarily stopped, still within ordinary budget. Try
            // one dedicated fill round for any committed screen that's still
            // eligible — NOT gated by remaining turn budget (only by the
            // per-screen cap above), so budget is never the reason a
            // promised screen ships empty.
            let report = check_unfilled(&cfg.executor, cfg.finalize_on_exit).await;
            let eligible = eligible_for_fill_round(&report.unfilled, &fill_attempts);
            if !eligible.is_empty() {
                spend_fill_round(&mut fill_attempts, &eligible);
                messages.push(
                    json!({ "role": "assistant", "content": assistant_turn_content(&collector) }),
                );
                messages.push(
                    json!({ "role": "user", "content": contract_nudge_text(&report.committed, &eligible) }),
                );
                continue; // Does not count against `turn`.
            }
            // No screen left to chase — try one dedicated corrective round
            // for any unresolved structural blocker (duplicate root / broken
            // ring / empty shell / unbound primary-mobile nav tab), bounded
            // by its own `BLOCKER_NUDGE_MAX_ROUNDS` budget so this can never
            // compound into an unbounded loop alongside the fill/salvage
            // budgets above.
            if let Some(nudge) = blocker_nudge_if_owed(
                &cfg.executor,
                cfg.finalize_on_exit,
                &mut blocker_rounds_used,
            )
            .await
            {
                messages.push(
                    json!({ "role": "assistant", "content": assistant_turn_content(&collector) }),
                );
                messages.push(json!({ "role": "user", "content": nudge }));
                continue; // Does not count against `turn`.
            }
            // Zero-write guard (Tier 2d): the model stopped without ever
            // applying a write — the empty-canvas shape (see the OpenAI
            // loop's twin comment). The retry round re-applies the
            // Anthropic-wire reasoning control, so a thinking-starved model
            // has real budget for the write this time.
            if zero_write_round_owed(cfg.finalize_on_exit, wrote, zero_write_rounds_used) {
                zero_write_rounds_used += 1;
                messages.push(
                    json!({ "role": "assistant", "content": assistant_turn_content(&collector) }),
                );
                messages.push(json!({ "role": "user", "content": ZERO_WRITE_NUDGE }));
                continue; // Does not count against `turn`.
            }
            // Nothing committed is left worth trying for: run the Step-4
            // structural backstop ONCE over the assembled doc BEFORE the
            // Done delta, so the finalized document is what the UI
            // persists/displays for this turn. Tier 3 — honest report —
            // fires unconditionally if anything is still unfilled/blocked (a
            // screen outside this run's committed set, a blocker whose
            // round budget ran out, or the executor being a no-op).
            let stop_detail = collector
                .stop_reason
                .clone()
                .unwrap_or_else(|| "end_turn".into());
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

        messages
            .push(json!({ "role": "assistant", "content": assistant_turn_content(&collector) }));
        let mut results: Vec<Value> = Vec::new();
        let mut failed_write_tools = Vec::new();
        let mut corrective_write_seen = false;
        let mut corrective_write_failed = false;
        for call in &calls {
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
            // Send screenshots as images only when the model supports them.
            let content: Value = match prepare_screenshot_for_context(
                &cfg.model,
                cfg.max_output_tokens,
                &call.name,
                &result,
            ) {
                Some(PreparedScreenshot::Inline(b64)) => {
                    // Keep only the newest visual observation.
                    for message in &mut messages {
                        elide_inline_screenshots(message);
                    }
                    for prior_result in &mut results {
                        elide_inline_screenshots(prior_result);
                    }
                    json!([
                    { "type": "text", "text": "Rendered screenshot of the current design:" },
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": b64,
                        },
                    },
                    ])
                }
                Some(PreparedScreenshot::TextOnly) => {
                    Value::String(TEXT_ONLY_SCREENSHOT_TEXT.to_string())
                }
                Some(PreparedScreenshot::OverBudget) => {
                    Value::String(OMITTED_SCREENSHOT_TEXT.to_string())
                }
                None => Value::String(result.content),
            };
            results.push(json!({
                "type": "tool_result",
                "tool_use_id": call.id,
                "content": content,
                "is_error": result.is_error,
            }));
        }
        if corrective_write_round && (!corrective_write_seen || corrective_write_failed) {
            let _ = tx
                .send(ChatDelta::TextDelta(CORRECTIVE_WRITE_EXHAUSTED.to_string()))
                .await;
        } else if cfg.finalize_on_exit {
            if let Some(nudge) = write_retry.schedule(&failed_write_tools) {
                results.push(json!({ "type": "text", "text": nudge }));
                let _ = tx
                    .send(ChatDelta::TextDelta(CORRECTIVE_WRITE_PROGRESS.to_string()))
                    .await;
            }
        }
        messages.push(json!({ "role": "user", "content": results }));
    }
}
